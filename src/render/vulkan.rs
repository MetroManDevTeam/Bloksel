use ash::{
    vk,
    Entry,
    Instance,
    Device,
    extensions::khr::{Surface, Swapchain},
};
use ash::ext::debug_utils;
use anyhow::{Context, Result};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::{
    ffi::{CStr, CString},
    sync::{Arc, Mutex, MutexGuard},
};

// Customizable settings
#[derive(Debug, Clone)]
pub struct VulkanSettings {
    pub application_name: String,
    pub engine_name: String,
    pub enable_validation: bool,
    pub required_device_extensions: Vec<String>,
    pub optional_device_extensions: Vec<String>,
    pub required_features: vk::PhysicalDeviceFeatures,
    pub preferred_device_types: Vec<vk::PhysicalDeviceType>,
    pub dedicated_allocations: bool,
    pub buffer_device_address: bool,
    pub ray_tracing: bool,
    pub mesh_shading: bool,
    pub concurrent_resources: usize,
    pub max_frames_in_flight: usize,
    pub enable_debug_markers: bool,
    pub gpu_memory_budget_mb: Option<u32>,
}

impl Default for VulkanSettings {
    fn default() -> Self {
        Self {
            application_name: "Vulkan Application".into(),
            engine_name: "Vulkan Engine".into(),
            enable_validation: cfg!(debug_assertions),
            required_device_extensions: vec![
                Swapchain::name().to_str().unwrap().to_string(),
            ],
            optional_device_extensions: vec![],
            required_features: vk::PhysicalDeviceFeatures::default(),
            preferred_device_types: vec![
                vk::PhysicalDeviceType::DISCRETE_GPU,
                vk::PhysicalDeviceType::INTEGRATED_GPU,
                vk::PhysicalDeviceType::VIRTUAL_GPU,
            ],
            dedicated_allocations: false,
            buffer_device_address: false,
            ray_tracing: false,
            mesh_shading: false,
            concurrent_resources: 3,
            max_frames_in_flight: 2,
            enable_debug_markers: cfg!(debug_assertions),
            gpu_memory_budget_mb: None,
        }
    }
}

#[derive(Debug)]
pub struct VulkanContext {
    pub entry: Entry,
    pub instance: Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: Device,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub transfer_queue: Option<vk::Queue>,
    pub compute_queue: Option<vk::Queue>,
    pub graphics_queue_family: u32,
    pub present_queue_family: u32,
    pub transfer_queue_family: Option<u32>,
    pub compute_queue_family: Option<u32>,
    pub surface_loader: Option<Surface>,
    pub swapchain_loader: Option<Swapchain>,
    pub debug_utils: Option<DebugUtilsWrapper>,
    pub settings: VulkanSettings,
    pub memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub device_properties: vk::PhysicalDeviceProperties,
    pub device_features: vk::PhysicalDeviceFeatures,
    frame_index: Mutex<usize>,
    resource_pools: Mutex<Vec<ResourcePool>>,
}

#[derive(Debug)]
struct DebugUtilsWrapper {
    loader: debug_utils::DebugUtils,
    messenger: vk::DebugUtilsMessengerEXT,
}

#[derive(Debug)]
struct ResourcePool {
    buffers: Vec<vk::Buffer>,
    images: Vec<vk::Image>,
    memories: Vec<vk::DeviceMemory>,
    command_pools: Vec<vk::CommandPool>,
}

#[derive(Debug, Clone, Copy)]
struct QueueFamilies {
    graphics: Option<u32>,
    present: Option<u32>,
    transfer: Option<u32>,
    compute: Option<u32>,
}

impl QueueFamilies {
    fn new() -> Self {
        Self {
            graphics: None,
            present: None,
            transfer: None,
            compute: None,
        }
    }
}

impl VulkanContext {
    pub fn new(settings: VulkanSettings) -> Result<Arc<Self>> {
        let entry = Entry::linked();

        // Layers and extensions
        let mut instance_extensions = ash_window::enumerate_required_extensions()?
            .iter()
            .map(|&e| e.as_ptr())
            .collect::<Vec<_>>();

        let layers = if cfg!(debug_assertions) {
            vec![CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0").unwrap()]
        } else {
            vec![]
        };

        let layer_ptrs: Vec<*const i8> = layers.iter().map(|layer| layer.as_ptr()).collect();

        // Application info
        let app_name = CString::new(settings.application_name.clone())?;
        let engine_name = CString::new(settings.engine_name.clone())?;
        let app_info = vk::ApplicationInfo::builder()
            .application_name(app_name.as_c_str())
            .application_version(vk::make_version(1, 0, 0))
            .engine_name(engine_name.as_c_str())
            .engine_version(vk::make_version(1, 0, 0))
            .api_version(vk::make_version(1, 2, 0));

        // Create instance
        let instance = unsafe {
            entry.create_instance(
                &vk::InstanceCreateInfo::builder()
                    .application_info(&app_info)
                    .enabled_layer_names(&layer_ptrs)
                    .enabled_extension_names(&instance_extensions),
                None,
            )
        }
        .context("Failed to create Vulkan instance")?;

        // Debug utils setup
        let debug_utils = if settings.enable_validation {
            let debug_utils_loader = debug_utils::DebugUtils::new(&entry, &instance);
            let messenger = unsafe {
                debug_utils_loader.create_debug_utils_messenger(
                    &vk::DebugUtilsMessengerCreateInfoEXT::builder()
                        .message_severity(
                            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR |
                            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
                        )
                        .message_type(
                            vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION |
                            vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                        )
                        .pfn_user_callback(Some(vulkan_debug_callback)),
                    None,
                )
            }
            .context("Failed to create debug utils messenger")?;

            Some(DebugUtilsWrapper {
                loader: debug_utils_loader,
                messenger,
            })
        } else {
            None
        };

        // Physical device selection
        let (physical_device, queue_families) = Self::select_physical_device(
            &instance,
            None,
            &settings,
        )
        .context("Failed to select physical device")?;

        // Get device properties and features
        let device_properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let device_features = unsafe { instance.get_physical_device_features(physical_device) };
        let memory_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };

        // Device creation
        let mut queue_create_infos = Vec::new();
        let queue_priorities = [1.0]; // Reused for all queues

        let graphics_queue_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_families.graphics.expect("Graphics queue family not found"))
            .queue_priorities(&queue_priorities);

        let present_queue_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_families.present.expect("Present queue family not found"))
            .queue_priorities(&queue_priorities);

        let queue_infos = if queue_families.graphics != queue_families.present {
            vec![graphics_queue_info.build(), present_queue_info.build()]
        } else {
            vec![graphics_queue_info.build()]
        }

        if queue_families.present != queue_families.graphics {
            let present_queue_info = vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(queue_families.present)
                .queue_priorities(&queue_priorities);
            queue_create_infos.push(present_queue_info.build());
        }

        if let Some(transfer) = queue_families.transfer {
            let transfer_queue_info = vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(transfer)
                .queue_priorities(&queue_priorities);
            queue_create_infos.push(transfer_queue_info.build());
        }

        if let Some(compute) = queue_families.compute {
            let compute_queue_info = vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(compute)
                .queue_priorities(&queue_priorities);
            queue_create_infos.push(compute_queue_info.build());
        }

        // Enable device extensions
        let mut device_extensions = settings
            .required_device_extensions
            .iter()
            .map(|ext| CString::new(ext.as_str()).unwrap().as_ptr())
            .collect::<Vec<_>>();

        // Add optional extensions if supported
        for ext in &settings.optional_device_extensions {
            if Self::is_device_extension_supported(&instance, physical_device, ext)? {
                device_extensions.push(CString::new(ext.as_str()).unwrap().as_ptr());
            }
        }

        // Enable features
        let mut features = settings.required_features;
        let mut features_11 = vk::PhysicalDeviceVulkan11Features::builder()
            .shader_draw_parameters(true);
        let mut features_12 = vk::PhysicalDeviceVulkan12Features::builder()
            .buffer_device_address(settings.buffer_device_address)
            .descriptor_indexing(true);

        let mut rt_features = vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder()
            .ray_tracing_pipeline(settings.ray_tracing);
        let mut mesh_features = vk::PhysicalDeviceMeshShaderFeaturesNV::builder()
            .mesh_shader(settings.mesh_shading);

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extensions)
            .push_next(&mut features_11)
            .push_next(&mut features_12);

        let device_create_info = if settings.ray_tracing {
            device_create_info.push_next(&mut rt_features)
        } else {
            device_create_info
        };

        let device_create_info = if settings.mesh_shading {
            device_create_info.push_next(&mut mesh_features)
        } else {
            device_create_info
        };

        let device = unsafe {
            instance.create_device(
                physical_device,
                &device_create_info,
                None,
            )
        }
        .context("Failed to create logical device")?;

        // Get queues
        let graphics_queue = unsafe { device.get_device_queue(queue_families.graphics, 0) };
        let present_queue = unsafe { device.get_device_queue(queue_families.present, 0) };
        let transfer_queue = queue_families.transfer.map(|f| unsafe { device.get_device_queue(f, 0) });
        let compute_queue = queue_families.compute.map(|f| unsafe { device.get_device_queue(f, 0) });

        // Create resource pools
        let resource_pools = Mutex::new(
            (0..settings.concurrent_resources)
                .map(|_| ResourcePool {
                    buffers: Vec::new(),
                    images: Vec::new(),
                    memories: Vec::new(),
                    command_pools: Vec::new(),
                })
                .collect(),
        );

        Ok(Arc::new(Self {
            entry,
            instance,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            transfer_queue,
            compute_queue,
            graphics_queue_family: queue_families.graphics,
            present_queue_family: queue_families.present,
            transfer_queue_family: queue_families.transfer,
            compute_queue_family: queue_families.compute,
            surface_loader: None,
            swapchain_loader: None,
            debug_utils,
            settings,
            memory_properties,
            device_properties,
            device_features,
            frame_index: Mutex::new(0),
            resource_pools,
        }))
    }

    pub fn create_surface<W: HasRawWindowHandle + HasRawDisplayHandle>(
        &mut self,
        window: &W,
    ) -> Result<vk::SurfaceKHR> {
        let surface = unsafe {
            ash_window::create_surface(
                &self.entry,
                &self.instance,
                window.raw_display_handle().map_err(|e| anyhow::anyhow!("Failed to get display handle: {}", e))?,
                window.raw_window_handle().map_err(|e| anyhow::anyhow!("Failed to get window handle: {}", e))?,
                None,
            )?
            Surface::new(&self.entry, &self.instance)
        };

        // Initialize surface loader if not already initialized
        if self.surface_loader.is_none() {
            self.surface_loader = Some(Surface::new(&self.entry, &self.instance));
        }

        // Verify surface support
        let surface_loader = self.surface_loader.as_ref().unwrap();
        let supported = unsafe {
            surface_loader.get_physical_device_surface_support(
                self.physical_device,
                self.graphics_queue_family,
                surface,
            )?
        };

        if !supported {
            return Err(anyhow::anyhow!("Surface not supported by physical device"));
        }

        Ok(surface)
    }

    pub fn create_swapchain(
        &mut self,
        surface: vk::SurfaceKHR,
        width: u32,
        height: u32,
        old_swapchain: Option<vk::SwapchainKHR>,
    ) -> Result<(vk::SwapchainKHR, Vec<vk::Image>, vk::Format, vk::Extent2D)> {
        let surface_loader = self.surface_loader.as_ref().unwrap();
        let capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(
                self.physical_device,
                surface,
            )?
        };

        let formats = unsafe {
            surface_loader.get_physical_device_surface_formats(
                self.physical_device,
                surface,
            )?
        };

        let present_modes = unsafe {
            surface_loader.get_physical_device_surface_present_modes(
                self.physical_device,
                surface,
            )?
        };

        // Select surface format
        let format = formats
            .iter()
            .find(|f| {
                f.format == vk::Format::B8G8R8A8_SRGB
                    && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .or_else(|| formats.first())
            .context("No suitable surface format found")?;

        // Select present mode (prefer MAILBOX for triple buffering)
        let present_mode = present_modes
            .iter()
            .find(|&&m| m == vk::PresentModeKHR::MAILBOX)
            .or_else(|| present_modes.iter().find(|&&m| m == vk::PresentModeKHR::FIFO))
            .context("No suitable present mode found")?;

        // Determine swapchain extent
        let extent = if capabilities.current_extent.width != u32::MAX {
            capabilities.current_extent
        } else {
            vk::Extent2D {
                width: width.clamp(
                    capabilities.min_image_extent.width,
                    capabilities.max_image_extent.width,
                ),
                height: height.clamp(
                    capabilities.min_image_extent.height,
                    capabilities.max_image_extent.height,
                ),
            }
        };

        // Determine image count
        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
            image_count = capabilities.max_image_count;
        }

        // Create swapchain
        let mut swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(*present_mode)
            .clipped(true);

        if let Some(old_swapchain) = old_swapchain {
            swapchain_create_info = swapchain_create_info.old_swapchain(old_swapchain);
        }

        // Handle queue family sharing
        let queue_family_indices = if self.graphics_queue_family != self.present_queue_family {
            let indices = [self.graphics_queue_family, self.present_queue_family];
            swapchain_create_info = swapchain_create_info
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&indices);
            Some(indices)
        } else {
            None
        };

        // Initialize swapchain loader if not already initialized
        if self.swapchain_loader.is_none() {
            self.swapchain_loader = Some(Swapchain::new(&self.instance, &self.device));
        }
        let swapchain_loader = self.swapchain_loader.as_ref().unwrap();

        let swapchain = unsafe {
            swapchain_loader.create_swapchain(&swapchain_create_info, None)?
        };

        // Get swapchain images
        let swapchain_images = unsafe {
            swapchain_loader.get_swapchain_images(swapchain)?
        };

        Ok((swapchain, swapchain_images, format.format, extent))
    }

    fn select_physical_device(
        instance: &Instance,
        surface: Option<vk::SurfaceKHR>,
        settings: &VulkanSettings,
    ) -> Result<(vk::PhysicalDevice, QueueFamilies)> {
        let devices = unsafe { instance.enumerate_physical_devices()? };

        let mut candidates = devices
            .into_iter()
            .filter_map(|device| {
                // Check extensions
                if !Self::check_device_extensions(instance, device, settings).ok()? {
                    return None;
                }

                // Check queue families
                let queue_families = Self::find_queue_families(instance, device, surface).ok()?;
                if queue_families.graphics.is_none() || queue_families.present.is_none() {
                    return None;
                }

                // Check swapchain support
                if !Self::check_swapchain_support(instance, device, surface).ok()? {
                    return None;
                }

                // Check features
                let features = unsafe { instance.get_physical_device_features(device) };
                if !Self::check_required_features(&features, &settings.required_features) {
                    return None;
                }

                // Score device
                let props = unsafe { instance.get_physical_device_properties(device) };
                let mut score = 0;

                // Prefer device types in order
                if let Some(pos) = settings.preferred_device_types.iter().position(|&t| t == props.device_type) {
                    score += 1000 - (pos as i32 * 100);
                }

                // Prefer more memory
                let mem_props = unsafe { instance.get_physical_device_memory_properties(device) };
                let device_local_memory = mem_props.memory_heaps
                    .iter()
                    .enumerate()
                    .filter(|(i, heap)| {
                        heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL)
                            && mem_props.memory_types.iter().any(|mt| mt.heap_index == *i as u32)
                    })
                    .map(|(_, heap)| heap.size)
                    .sum::<u64>();

                score += (device_local_memory / 1024 / 1024) as i32; // MB

                // Prefer higher limits
                score += (props.limits.max_image_dimension2_d / 1024) as i32;
                score += props.limits.max_descriptor_set_samplers as i32 / 16;

                Some((device, queue_families, score))
            })
            .collect::<Vec<_>>();

        candidates.sort_by(|a, b| b.2.cmp(&a.2));

        candidates
            .into_iter()
            .next()
            .map(|(device, families, _)| (device, families))
            .context("No suitable GPU found")
    }

    fn check_device_extensions(
        instance: &Instance,
        device: vk::PhysicalDevice,
        settings: &VulkanSettings,
    ) -> Result<bool> {
        let available_extensions = unsafe {
            instance
                .enumerate_device_extension_properties(device)?
                .iter()
                .map(|ext| ext.extension_name)
                .collect::<Vec<_>>()
        };

        for required in &settings.required_device_extensions {
            let cstr = CString::new(required.as_str())?;
            if !available_extensions.iter().any(|&ext| {
                unsafe { CStr::from_ptr(ext.as_ptr()) == cstr.as_c_str() }
            }) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn is_device_extension_supported(
        instance: &Instance,
        device: vk::PhysicalDevice,
        extension: &str,
    ) -> Result<bool> {
        let available_extensions = unsafe {
            instance
                .enumerate_device_extension_properties(device)?
                .iter()
                .map(|ext| ext.extension_name)
                .collect::<Vec<_>>()
        };

        let cstr = CString::new(extension)?;
        Ok(available_extensions.iter().any(|&ext| {
            unsafe { CStr::from_ptr(ext.as_ptr()) == cstr.as_c_str() }
        }))
    }

    fn check_required_features(
        available: &vk::PhysicalDeviceFeatures,
        required: &vk::PhysicalDeviceFeatures,
    ) -> bool {
        (required.robust_buffer_access == 0 || available.robust_buffer_access != 0)
            && (required.full_draw_index_uint32 == 0 || available.full_draw_index_uint32 != 0)
            && (required.image_cube_array == 0 || available.image_cube_array != 0)
            && (required.independent_blend == 0 || available.independent_blend != 0)
            && (required.geometry_shader == 0 || available.geometry_shader != 0)
            && (required.tessellation_shader == 0 || available.tessellation_shader != 0)
            && (required.sample_rate_shading == 0 || available.sample_rate_shading != 0)
            && (required.dual_src_blend == 0 || available.dual_src_blend != 0)
            && (required.logic_op == 0 || available.logic_op != 0)
            && (required.multi_draw_indirect == 0 || available.multi_draw_indirect != 0)
            && (required.draw_indirect_first_instance == 0 || available.draw_indirect_first_instance != 0)
            && (required.depth_clamp == 0 || available.depth_clamp != 0)
            && (required.depth_bias_clamp == 0 || available.depth_bias_clamp != 0)
            && (required.fill_mode_non_solid == 0 || available.fill_mode_non_solid != 0)
            && (required.depth_bounds == 0 || available.depth_bounds != 0)
            && (required.wide_lines == 0 || available.wide_lines != 0)
            && (required.large_points == 0 || available.large_points != 0)
            && (required.alpha_to_one == 0 || available.alpha_to_one != 0)
            && (required.multi_viewport == 0 || available.multi_viewport != 0)
            && (required.sampler_anisotropy == 0 || available.sampler_anisotropy != 0)
            && (required.texture_compression_etc2 == 0 || available.texture_compression_etc2 != 0)
            && (required.texture_compression_astc_ldr == 0 || available.texture_compression_astc_ldr != 0)
            && (required.texture_compression_bc == 0 || available.texture_compression_bc != 0)
            && (required.occlusion_query_precise == 0 || available.occlusion_query_precise != 0)
            && (required.pipeline_statistics_query == 0 || available.pipeline_statistics_query != 0)
            && (required.vertex_pipeline_stores_and_atomics == 0 || available.vertex_pipeline_stores_and_atomics != 0)
            && (required.fragment_stores_and_atomics == 0 || available.fragment_stores_and_atomics != 0)
            && (required.shader_tessellation_and_geometry_point_size == 0 || available.shader_tessellation_and_geometry_point_size != 0)
            && (required.shader_image_gather_extended == 0 || available.shader_image_gather_extended != 0)
            && (required.shader_storage_image_extended_formats == 0 || available.shader_storage_image_extended_formats != 0)
            && (required.shader_storage_image_multisample == 0 || available.shader_storage_image_multisample != 0)
            && (required.shader_storage_image_read_without_format == 0 || available.shader_storage_image_read_without_format != 0)
            && (required.shader_storage_image_write_without_format == 0 || available.shader_storage_image_write_without_format != 0)
            && (required.shader_uniform_buffer_array_dynamic_indexing == 0 || available.shader_uniform_buffer_array_dynamic_indexing != 0)
            && (required.shader_sampled_image_array_dynamic_indexing == 0 || available.shader_sampled_image_array_dynamic_indexing != 0)
            && (required.shader_storage_buffer_array_dynamic_indexing == 0 || available.shader_storage_buffer_array_dynamic_indexing != 0)
            && (required.shader_storage_image_array_dynamic_indexing == 0 || available.shader_storage_image_array_dynamic_indexing != 0)
            && (required.shader_clip_distance == 0 || available.shader_clip_distance != 0)
            && (required.shader_cull_distance == 0 || available.shader_cull_distance != 0)
            && (required.shader_float64 == 0 || available.shader_float64 != 0)
            && (required.shader_int64 == 0 || available.shader_int64 != 0)
            && (required.shader_int16 == 0 || available.shader_int16 != 0)
            && (required.shader_resource_residency == 0 || available.shader_resource_residency != 0)
            && (required.shader_resource_min_lod == 0 || available.shader_resource_min_lod != 0)
            && (required.sparse_binding == 0 || available.sparse_binding != 0)
            && (required.sparse_residency_buffer == 0 || available.sparse_residency_buffer != 0)
            && (required.sparse_residency_image2_d == 0 || available.sparse_residency_image2_d != 0)
            && (required.sparse_residency_image3_d == 0 || available.sparse_residency_image3_d != 0)
            && (required.sparse_residency2_samples == 0 || available.sparse_residency2_samples != 0)
            && (required.sparse_residency4_samples == 0 || available.sparse_residency4_samples != 0)
            && (required.sparse_residency8_samples == 0 || available.sparse_residency8_samples != 0)
            && (required.sparse_residency16_samples == 0 || available.sparse_residency16_samples != 0)
            && (required.sparse_residency_aliased == 0 || available.sparse_residency_aliased != 0)
            && (required.variable_multisample_rate == 0 || available.variable_multisample_rate != 0)
            && (required.inherited_queries == 0 || available.inherited_queries != 0)
    }

    fn find_queue_families(
        instance: &Instance,
        device: vk::PhysicalDevice,
        surface: Option<vk::SurfaceKHR>,
    ) -> Result<QueueFamilies> {
        let queue_properties = unsafe { instance.get_physical_device_queue_family_properties(device) };

        let mut families = QueueFamilies::new();

        // Find graphics and present queues (may be the same)
        for (index, properties) in queue_properties.iter().enumerate() {
            let index = index as u32;

            if properties.queue_count > 0 {
                if properties.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    if families.graphics.is_none() {
                        families.graphics = Some(index);
                    }
                }

                if properties.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                    if families.compute.is_none() {
                        families.compute = Some(index);
                    }
                }

                // Prefer dedicated transfer queue
                if properties.queue_flags.contains(vk::QueueFlags::TRANSFER)
                    && !properties.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                    && !properties.queue_flags.contains(vk::QueueFlags::COMPUTE)
                {
                    families.transfer = Some(index);
                }

                // Check surface support if needed
                if let Some(surface) = surface {
                    let supported = unsafe {
                        Surface::new(instance)
                            .get_physical_device_surface_support(device, index, surface)?
                    };
                    if supported && families.present.is_none() {
                        families.present = Some(index);
                    }
                }
            }
        }

        // Fallbacks
        if families.transfer.is_none() {
            families.transfer = families.graphics;
        }

        if families.compute.is_none() {
            families.compute = families.graphics;
        }

        if surface.is_some() && families.present.is_none() {
            families.present = families.graphics;
        }

        Ok(families)
    }

    pub fn find_memory_type(
        &self,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        self.memory_properties.memory_types
            .iter()
            .enumerate()
            .find(|(i, memory_type)| {
                (type_filter & (1 << i)) != 0
                    && (memory_type.property_flags & properties) == properties
            })
            .map(|(i, _)| i as u32)
    }

    pub fn begin_single_time_commands(
        &self,
        command_pool: vk::CommandPool,
    ) -> Result<vk::CommandBuffer> {
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool)
            .command_buffer_count(1);

        let command_buffer = unsafe {
            self.device.allocate_command_buffers(&alloc_info)?
                .first()
                .copied()
                .context("Failed to allocate command buffer")?
        };

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.device.begin_command_buffer(command_buffer, &begin_info)?;
        }

        Ok(command_buffer)
    }

    pub fn end_single_time_commands(
        &self,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        command_buffer: vk::CommandBuffer,
    ) -> Result<()> {
        unsafe {
            self.device.end_command_buffer(command_buffer)?;
        }

        let command_buffers = [command_buffer];
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&command_buffers);

        unsafe {
            self.device.queue_submit(queue, &[submit_info.build()], vk::Fence::null())?;
            self.device.queue_wait_idle(queue)?;
        }

        unsafe {
            self.device.free_command_buffers(command_pool, &[command_buffer]);
        }

        Ok(())
    }

    pub fn transition_image_layout(
        &self,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        image: vk::Image,
        format: vk::Format,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) -> Result<()> {
        let command_buffer = self.begin_single_time_commands(command_pool)?;

        let aspect_mask = if new_layout == vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL {
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
        } else {
            vk::ImageAspectFlags::COLOR
        };

        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };

        let (src_stage, dst_stage, barrier) = if old_layout == vk::ImageLayout::UNDEFINED && new_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL {
            let barrier = vk::ImageMemoryBarrier::builder()
                .old_layout(old_layout)
                .new_layout(new_layout)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(subresource_range)
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .build();
            (vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::TRANSFER, barrier)
        } else if old_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL && new_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL {
            let barrier = vk::ImageMemoryBarrier::builder()
                .old_layout(old_layout)
                .new_layout(new_layout)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(subresource_range)
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .build();
            (vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::FRAGMENT_SHADER, barrier)
        } else if old_layout == vk::ImageLayout::UNDEFINED && new_layout == vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL {
            let barrier = vk::ImageMemoryBarrier::builder()
                .old_layout(old_layout)
                .new_layout(new_layout)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(subresource_range)
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
                .build();
            (vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS, barrier)
        } else {
            return Err(anyhow::anyhow!("Unsupported layout transition"));
        };

        unsafe {
            self.device.cmd_pipeline_barrier(
                command_buffer,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }

        self.end_single_time_commands(command_pool, queue, command_buffer)
    }

    pub fn copy_buffer_to_image(
        &self,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        buffer: vk::Buffer,
        image: vk::Image,
        width: u32,
        height: u32,
    ) -> Result<()> {
        let command_buffer = self.begin_single_time_commands(command_pool)?;

        let region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            });

        unsafe {
            self.device.cmd_copy_buffer_to_image(
                command_buffer,
                buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region.build()],
            );
        }

        self.end_single_time_commands(command_pool, queue, command_buffer)
    }

    pub fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe {
            self.device.create_buffer(&buffer_info, None)
                .context("Failed to create buffer")?
        };

        let mem_requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(
                self.find_memory_type(mem_requirements.memory_type_bits, properties)
                    .context("Failed to find suitable memory type")?,
            );

        let buffer_memory = unsafe {
            self.device.allocate_memory(&alloc_info, None)
                .context("Failed to allocate buffer memory")?
        };

        unsafe {
            self.device.bind_buffer_memory(buffer, buffer_memory, 0)
                .context("Failed to bind buffer memory")?;
        }

        Ok((buffer, buffer_memory))
    }

    pub fn create_image(
        &self,
        width: u32,
        height: u32,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Image, vk::DeviceMemory)> {
        let image_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(tiling)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);

        let image = unsafe {
            self.device.create_image(&image_info, None)
                .context("Failed to create image")?
        };

        let mem_requirements = unsafe { self.device.get_image_memory_requirements(image) };

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(
                self.find_memory_type(mem_requirements.memory_type_bits, properties)
                    .context("Failed to find suitable memory type")?,
            );

        let image_memory = unsafe {
            self.device.allocate_memory(&alloc_info, None)
                .context("Failed to allocate image memory")?
        };

        unsafe {
            self.device.bind_image_memory(image, image_memory, 0)
                .context("Failed to bind image memory")?;
        }

        Ok((image, image_memory))
    }

    pub fn create_image_view(
        &self,
        image: vk::Image,
        format: vk::Format,
        aspect_flags: vk::ImageAspectFlags,
    ) -> Result<vk::ImageView> {
        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask: aspect_flags,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };

        let create_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(subresource_range);

        unsafe {
            self.device.create_image_view(&create_info, None)
                .context("Failed to create image view")
        }
    }

    pub fn create_command_pool(&self, queue_family_index: u32) -> Result<vk::CommandPool> {
        let pool_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        unsafe {
            self.device.create_command_pool(&pool_info, None)
                .context("Failed to create command pool")
        }
    }

    pub fn create_framebuffer(
        &self,
        render_pass: vk::RenderPass,
        attachments: &[vk::ImageView],
        width: u32,
        height: u32,
    ) -> Result<vk::Framebuffer> {
        let create_info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(attachments)
            .width(width)
            .height(height)
            .layers(1);

        unsafe {
            self.device.create_framebuffer(&create_info, None)
                .context("Failed to create framebuffer")
        }
    }

    pub fn create_render_pass(
        &self,
        color_format: vk::Format,
        depth_format: Option<vk::Format>,
    ) -> Result<vk::RenderPass> {
        let color_attachment = vk::AttachmentDescription::builder()
            .format(color_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let mut attachments = vec![color_attachment.build()];
        let color_reference = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let mut depth_reference = None;
        if let Some(depth_format) = depth_format {
            let depth_attachment = vk::AttachmentDescription::builder()
                .format(depth_format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

            depth_reference = Some(
                vk::AttachmentReference::builder()
                    .attachment(1)
                    .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .build(),
            );

            attachments.push(depth_attachment.build());
        }

        let mut subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&[color_reference.build()])
            .depth_stencil_attachment(depth_reference.as_ref().expect("Depth attachment reference not found"));

        let dependency = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_READ |
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            );

        let create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&[subpass.build()])
            .dependencies(&[dependency.build()]);

        unsafe {
            self.device.create_render_pass(&create_info, None)
                .context("Failed to create render pass")
        }
    }

    pub fn create_shader_module(&self, code: &[u8]) -> Result<vk::ShaderModule> {
        let code = unsafe {
            std::slice::from_raw_parts(
                code.as_ptr() as *const u32,
                code.len() / std::mem::size_of::<u32>(),
            )
        };

        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(code);

        unsafe {
            self.device.create_shader_module(&create_info, None)
                .context("Failed to create shader module")
        }
    }

    pub fn create_semaphore(&self) -> Result<vk::Semaphore> {
        let create_info = vk::SemaphoreCreateInfo::builder();

        unsafe {
            self.device.create_semaphore(&create_info, None)
                .context("Failed to create semaphore")
        }
    }

    pub fn create_fence(&self, signaled: bool) -> Result<vk::Fence> {
        let mut create_info = vk::FenceCreateInfo::builder();
        if signaled {
            create_info = create_info.flags(vk::FenceCreateFlags::SIGNALED);
        }

        unsafe {
            self.device.create_fence(&create_info, None)
                .context("Failed to create fence")
        }
    }

    pub fn create_descriptor_set_layout(
        &self,
        bindings: &[vk::DescriptorSetLayoutBinding],
    ) -> Result<vk::DescriptorSetLayout> {
        let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(bindings);

        unsafe {
            self.device.create_descriptor_set_layout(&create_info, None)
                .context("Failed to create descriptor set layout")
        }
    }

    pub fn create_descriptor_pool(
        &self,
        max_sets: u32,
        pool_sizes: &[vk::DescriptorPoolSize],
    ) -> Result<vk::DescriptorPool> {
        let create_info = vk::DescriptorPoolCreateInfo::builder()
            .max_sets(max_sets)
            .pool_sizes(pool_sizes);

        unsafe {
            self.device.create_descriptor_pool(&create_info, None)
                .context("Failed to create descriptor pool")
        }
    }

    pub fn allocate_descriptor_sets(
        &self,
        pool: vk::DescriptorPool,
        layouts: &[vk::DescriptorSetLayout],
    ) -> Result<Vec<vk::DescriptorSet>> {
        let allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(pool)
            .set_layouts(layouts);

        unsafe {
            self.device.allocate_descriptor_sets(&allocate_info)
                .context("Failed to allocate descriptor sets")
        }
    }

    pub fn update_descriptor_sets(
        &self,
        descriptor_writes: &[vk::WriteDescriptorSet],
        descriptor_copies: &[vk::CopyDescriptorSet],
    ) {
        unsafe {
            self.device.update_descriptor_sets(descriptor_writes, descriptor_copies);
        }
    }

    pub fn create_pipeline_layout(
        &self,
        set_layouts: &[vk::DescriptorSetLayout],
        push_constant_ranges: &[vk::PushConstantRange],
    ) -> Result<vk::PipelineLayout> {
        let create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(set_layouts)
            .push_constant_ranges(push_constant_ranges);

        unsafe {
            self.device.create_pipeline_layout(&create_info, None)
                .context("Failed to create pipeline layout")
        }
    }

    pub fn create_graphics_pipeline(
        &self,
        pipeline_info: &vk::GraphicsPipelineCreateInfo,
    ) -> Result<vk::Pipeline> {
        unsafe {
            self.device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[*pipeline_info],
                    None,
                )
                .map_err(|(_, e)| e)
                .context("Failed to create graphics pipeline")?
                .first()
                .copied()
                .ok_or_else(|| anyhow::anyhow!("No pipeline created"))
        }
    }

    pub fn create_compute_pipeline(
        &self,
        pipeline_info: &vk::ComputePipelineCreateInfo,
    ) -> Result<vk::Pipeline> {
        unsafe {
            self.device
                .create_compute_pipelines(
                    vk::PipelineCache::null(),
                    &[*pipeline_info],
                    None,
                )
                .map_err(|(_, e)| e)
                .context("Failed to create compute pipeline")?
                .first()
                .copied()
                .ok_or_else(|| anyhow::anyhow!("No pipeline created"))
        }
    }

    pub fn begin_command_buffer(
        &self,
        command_buffer: vk::CommandBuffer,
    ) -> Result<()> {
        let begin_info = vk::CommandBufferBeginInfo::builder();

        unsafe {
            self.device.begin_command_buffer(command_buffer, &begin_info)
                .context("Failed to begin command buffer")
        }
    }

    pub fn end_command_buffer(
        &self,
        command_buffer: vk::CommandBuffer,
    ) -> Result<()> {
        unsafe {
            self.device.end_command_buffer(command_buffer)
                .context("Failed to end command buffer")
        }
    }

    pub fn submit_command_buffers(
        &self,
        queue: vk::Queue,
        command_buffers: &[vk::CommandBuffer],
        wait_semaphores: &[vk::Semaphore],
        wait_stages: &[vk::PipelineStageFlags],
        signal_semaphores: &[vk::Semaphore],
        fence: vk::Fence,
    ) -> Result<()> {
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_stages)
            .command_buffers(command_buffers)
            .signal_semaphores(signal_semaphores);

        unsafe {
            self.device.queue_submit(queue, &[submit_info.build()], fence)
                .context("Failed to submit command buffer")
        }
    }

    pub fn acquire_next_image(
        &self,
        swapchain: vk::SwapchainKHR,
        semaphore: vk::Semaphore,
        fence: vk::Fence,
    ) -> Result<(u32, bool)> {
        let swapchain_loader = self.swapchain_loader.as_ref().unwrap();

        let result = unsafe {
            swapchain_loader.acquire_next_image(
                swapchain,
                std::u64::MAX,
                semaphore,
                fence,
            )
        };

        match result {
            Ok((image_index, suboptimal)) => {
                Ok((image_index, suboptimal == vk::Result::SUBOPTIMAL_KHR))
            }
            Err(e) => Err(anyhow::anyhow!("Failed to acquire swapchain image: {}", e))
        }
    }

    pub fn queue_present(
        &self,
        queue: vk::Queue,
        swapchain: vk::SwapchainKHR,
        image_index: u32,
        wait_semaphores: &[vk::Semaphore],
    ) -> Result<bool> {
        let swapchain_loader = self.swapchain_loader.as_ref().unwrap();

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(wait_semaphores)
            .swapchains(&[swapchain])
            .image_indices(&[image_index]);

        let result = unsafe {
            swapchain_loader.queue_present(queue, &present_info)
        };

        match result {
            Ok(suboptimal) => Ok(suboptimal == vk::Result::SUBOPTIMAL_KHR),
            Err(e) => Err(anyhow::anyhow!("Failed to present swapchain image: {}", e))
        }
    }

    pub fn wait_idle(&self) -> Result<()> {
        unsafe {
            self.device.device_wait_idle()
                .context("Failed to wait for device idle")
        }
    }

    pub fn destroy_swapchain(&self, swapchain: vk::SwapchainKHR) {
        if let Some(swapchain_loader) = &self.swapchain_loader {
            unsafe {
                swapchain_loader.destroy_swapchain(swapchain, None);
            }
        }
    }

    pub fn destroy_surface(&self, surface: vk::SurfaceKHR) {
        if let Some(surface_loader) = &self.surface_loader {
            unsafe {
                surface_loader.destroy_surface(surface, None);
            }
        }
    }

    pub fn next_frame_index(&self) -> usize {
        let mut frame_index = self.frame_index.lock().unwrap();
        *frame_index = (*frame_index + 1) % self.settings.concurrent_resources;
        *frame_index
    }

    pub fn current_frame_index(&self) -> usize {
        *self.frame_index.lock().unwrap()
    }

    pub fn get_resource_pool(&self, index: usize) -> Result<std::sync::MutexGuard<ResourcePool>> {
        let pools = self.resource_pools.lock().unwrap();
        if index >= pools.len() {
            return Err(anyhow::anyhow!("Resource pool index out of bounds"));
        }
        Ok(pools)
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    let severity = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[VERBOSE]",
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[INFO]",
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[WARNING]",
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[ERROR]",
        _ => "[UNKNOWN]",
    };

    let types = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[GENERAL]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[VALIDATION]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[PERFORMANCE]",
        _ => "[UNKNOWN]",
    };

    let message = unsafe { CStr::from_ptr((*p_callback_data).p_message) };
    eprintln!("{}{} {:?}", severity, types, message);

    vk::FALSE
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            // Free all resources
            let pools = self.resource_pools.lock().unwrap();
            for pool in pools.iter() {
                for &buffer in &pool.buffers {
                    self.device.destroy_buffer(buffer, None);
                }
                for &image in &pool.images {
                    self.device.destroy_image(image, None);
                }
                for &memory in &pool.memories {
                    self.device.free_memory(memory, None);
                }
                for &pool in &pool.command_pools {
                    self.device.destroy_command_pool(pool, None);
                }
            }

            if let Some(debug_utils) = &self.debug_utils {
                debug_utils
                    .loader
                    .destroy_debug_utils_messenger(debug_utils.messenger, None);
            }

            // Clean up swapchain loader
            if let Some(swapchain_loader) = &self.swapchain_loader {
                // Note: Swapchain should be destroyed explicitly before this point
            }

            // Clean up surface loader
            if let Some(surface_loader) = &self.surface_loader {
                // Note: Surface should be destroyed explicitly before this point
            }

            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
