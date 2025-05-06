use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub struct BlockMaterial {
    pub id: u32,
    pub name: &'static str,
    pub albedo: [f32; 4],
    pub roughness: f32,
    pub metallic: f32,
    pub emission: [f32; 3],
    pub texture_index: u32,
    pub texture_path: Option<&'static str>,
    pub normal_map_path: Option<&'static str>,
    pub occlusion_map_path: Option<&'static str>,
    pub tintable: bool,
    pub grayscale_base: bool,
    pub tint_mask_path: Option<&'static str>,
    pub vertex_colored: bool,
}

impl Default for BlockMaterial {
    fn default() -> Self {
        Self {
            id: 0,
            name: "default",
            albedo: [1.0, 1.0, 1.0, 1.0],
            roughness: 1.0,
            metallic: 0.0,
            emission: [0.0, 0.0, 0.0],
            texture_index: 0,
            texture_path: None,
            normal_map_path: None,
            occlusion_map_path: None,
            tintable: false,
            grayscale_base: false,
            tint_mask_path: None,
            vertex_colored: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TintSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_tint_strength")]
    pub strength: f32,
    #[serde(default)]
    pub affects_albedo: bool,
    #[serde(default)]
    pub affects_emissive: bool,
    #[serde(default)]
    pub affects_roughness: bool,
    #[serde(default)]
    pub affects_metallic: bool,
    #[serde(default)]
    pub blend_mode: TintBlendMode,
    #[serde(default)]
    pub mask_channel: TintMaskChannel,
}

fn default_tint_strength() -> f32 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TintBlendMode {
    #[default]
    Multiply,
    Overlay,
    Screen,
    Additive,
    Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TintMaskChannel {
    #[default]
    Red,
    Green,
    Blue,
    Alpha,
    All,
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
