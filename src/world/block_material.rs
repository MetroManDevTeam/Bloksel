use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

impl BlockMaterial {
    pub fn apply_modifiers(&mut self, modifiers: &MaterialModifiers) {
        if let Some(albedo_factor) = modifiers.albedo_factor {
            self.albedo[0] *= albedo_factor[0];
            self.albedo[1] *= albedo_factor[1];
            self.albedo[2] *= albedo_factor[2];
        }
        if let Some(roughness_offset) = modifiers.roughness_offset {
            self.roughness = (self.roughness + roughness_offset).clamp(0.0, 1.0);
        }
        if let Some(metallic_offset) = modifiers.metallic_offset {
            self.metallic = (self.metallic + metallic_offset).clamp(0.0, 1.0);
        }
        if let Some(emissive_boost) = modifiers.emissive_boost {
            self.emission[0] += emissive_boost[0];
            self.emission[1] += emissive_boost[1];
            self.emission[2] += emissive_boost[2];
        }
    }

    pub fn apply_tint(&mut self, color: [f32; 3], settings: &TintSettings) {
        if !settings.enabled {
            return;
        }

        let strength = settings.strength;
        match settings.blend_mode {
            TintBlendMode::Multiply => {
                if settings.affects_albedo {
                    self.albedo[0] *= color[0] * strength;
                    self.albedo[1] *= color[1] * strength;
                    self.albedo[2] *= color[2] * strength;
                }
                if settings.affects_emissive {
                    self.emission[0] *= color[0] * strength;
                    self.emission[1] *= color[1] * strength;
                    self.emission[2] *= color[2] * strength;
                }
            }
            TintBlendMode::Overlay => {
                if settings.affects_albedo {
                    for i in 0..3 {
                        if self.albedo[i] < 0.5 {
                            self.albedo[i] = 2.0 * self.albedo[i] * color[i] * strength;
                        } else {
                            self.albedo[i] =
                                1.0 - 2.0 * (1.0 - self.albedo[i]) * (1.0 - color[i] * strength);
                        }
                    }
                }
                if settings.affects_emissive {
                    for i in 0..3 {
                        if self.emission[i] < 0.5 {
                            self.emission[i] = 2.0 * self.emission[i] * color[i] * strength;
                        } else {
                            self.emission[i] =
                                1.0 - 2.0 * (1.0 - self.emission[i]) * (1.0 - color[i] * strength);
                        }
                    }
                }
            }
            TintBlendMode::Screen => {
                if settings.affects_albedo {
                    for i in 0..3 {
                        self.albedo[i] = 1.0 - (1.0 - self.albedo[i]) * (1.0 - color[i] * strength);
                    }
                }
                if settings.affects_emissive {
                    for i in 0..3 {
                        self.emission[i] =
                            1.0 - (1.0 - self.emission[i]) * (1.0 - color[i] * strength);
                    }
                }
            }
            TintBlendMode::Additive => {
                if settings.affects_albedo {
                    for i in 0..3 {
                        self.albedo[i] = (self.albedo[i] + color[i] * strength).min(1.0);
                    }
                }
                if settings.affects_emissive {
                    for i in 0..3 {
                        self.emission[i] = (self.emission[i] + color[i] * strength).min(1.0);
                    }
                }
            }
            TintBlendMode::Replace => {
                if settings.affects_albedo {
                    for i in 0..3 {
                        self.albedo[i] = color[i] * strength;
                    }
                }
                if settings.affects_emissive {
                    for i in 0..3 {
                        self.emission[i] = color[i] * strength;
                    }
                }
            }
        }

        if settings.affects_roughness {
            self.roughness =
                (self.roughness * (1.0 - strength) + color[0] * strength).clamp(0.0, 1.0);
        }
        if settings.affects_metallic {
            self.metallic =
                (self.metallic * (1.0 - strength) + color[0] * strength).clamp(0.0, 1.0);
        }
    }
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
