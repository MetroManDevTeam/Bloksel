use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockMaterial {
    pub id: u32,
    pub name: String,
    pub color: [f32; 4],
    pub roughness: f32,
    pub metallic: f32,
    pub emissive: f32,
    pub texture_path: Option<String>,
    pub normal_map_path: Option<String>,
    pub occlusion_map_path: Option<String>,
    pub tintable: bool,
    pub grayscale_base: bool,
    pub tint_mask_path: Option<String>,
    pub vertex_colored: bool,
}

impl Default for BlockMaterial {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            color: [1.0, 1.0, 1.0, 1.0],
            roughness: 0.5,
            metallic: 0.0,
            emissive: 0.0,
            texture_path: None,
            normal_map_path: None,
            occlusion_map_path: None,
            tintable: true,
            grayscale_base: false,
            tint_mask_path: None,
            vertex_colored: false,
        }
    }
}

impl BlockMaterial {
    pub fn new(color: [f32; 4], roughness: f32, metallic: f32, emissive: f32) -> Self {
        Self {
            id: 0,
            name: String::new(),
            color,
            roughness,
            metallic,
            emissive,
            texture_path: None,
            normal_map_path: None,
            occlusion_map_path: None,
            tintable: true,
            grayscale_base: false,
            tint_mask_path: None,
            vertex_colored: false,
        }
    }

    pub fn apply_tint(&mut self, color: [f32; 4], settings: &TintSettings) {
        let [r, g, b, a] = color;
        let strength = settings.strength;

        self.color[0] = self.color[0] * (1.0 - strength) + r * strength;
        self.color[1] = self.color[1] * (1.0 - strength) + g * strength;
        self.color[2] = self.color[2] * (1.0 - strength) + b * strength;
        self.color[3] = self.color[3] * (1.0 - strength) + a * strength;

        if !settings.preserve_metallic {
            self.metallic *= 1.0 - strength;
        }
        if !settings.preserve_roughness {
            self.roughness = self.roughness * (1.0 - strength) + 0.5 * strength;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TintSettings {
    pub strength: f32,
    pub preserve_metallic: bool,
    pub preserve_roughness: bool,
}

impl Default for TintSettings {
    fn default() -> Self {
        Self {
            strength: 0.5,
            preserve_metallic: true,
            preserve_roughness: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MaterialModifiers {
    #[serde(default)]
    pub albedo_factor: Option<[f32; 3]>,
    #[serde(default)]
    pub roughness_offset: Option<f32>,
    #[serde(default)]
    pub metallic_offset: Option<f32>,
    #[serde(default)]
    pub emissive_boost: Option<[f32; 3]>,
    #[serde(default)]
    pub tint_strength: Option<f32>,
}
