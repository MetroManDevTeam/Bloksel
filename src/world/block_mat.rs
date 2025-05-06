#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlockMaterial {
    pub id: u16,
    pub name: String,
    pub albedo: [f32:4],
    pub roughness: f32,
    pub metallic: f32,
    pub emissive: [f32; 3],
    pub texture_path: Option<String>,
    pub normal_map_path: Option<String>,
    pub occlusion_map_path: Option<String>,
    #[serde(default)]
    pub tintable: bool,
    #[serde(default)]
    pub grayscale_base: bool,
    #[serde(default)]
    pub tint_mask_path: Option<String>,
    #[serde(default)]
    pub vertex_colored: bool,
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
