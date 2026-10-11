use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, fs, path::Path};

pub const PRESET_SCHEMA_VERSION: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkinMaterial {
    pub base_color: [f32; 3],
    pub pore_strength: f32,
    pub roughness: f32,
    pub specular: f32,
    pub subsurface: f32,
    pub micro_relief: f32,
    pub hemoglobin_strength: f32,
    pub freckle_strength: f32,
    pub oiliness: f32,
    pub pigment_variation: f32,
    /// Master amplitude for both the tileable image relief and procedural pore populations.
    pub pore_scale: f32,
    pub pore_color_strength: f32,
    pub pore_micro_detail: f32,
    pub pore_micro_scale: f32,
    pub pore_irregularity: f32,
    pub pore_size_variation: f32,
    pub pore_map_resolution: u32,
    pub pore_map_density: f32,
    pub pore_map_relief: f32,
    pub pore_seed: u32,
    pub fine_detail_strength: f32,
    pub fine_detail_scale: f32,
    pub micro_grain_strength: f32,
    pub micro_grain_scale: f32,
    pub micro_grain_roughness: f32,
    pub regional_relief: f32,
    pub pore_vis_nose: f32,
    pub pore_vis_cheeks: f32,
    pub pore_vis_forehead: f32,
    pub pore_vis_chin: f32,
    pub pore_vis_ears: f32,
    pub pore_vis_eye_area: f32,
    pub pore_vis_neck: f32,
}

impl Default for SkinMaterial {
    fn default() -> Self {
        Self {
            base_color: [0.68, 0.43, 0.33],
            pore_strength: 1.0,
            roughness: 0.58,
            specular: 0.24,
            subsurface: 0.18,
            micro_relief: 0.35,
            hemoglobin_strength: 0.28,
            freckle_strength: 0.12,
            oiliness: 0.22,
            pigment_variation: 0.35,
            pore_scale: 1.0,
            pore_color_strength: 0.15,
            pore_micro_detail: 0.38,
            pore_micro_scale: 145.0,
            pore_irregularity: 0.70,
            pore_size_variation: 0.65,
            pore_map_resolution: 1024,
            pore_map_density: 35.0,
            pore_map_relief: 4.0,
            pore_seed: 42,
            fine_detail_strength: 0.02,
            fine_detail_scale: 180.0,
            micro_grain_strength: 0.02,
            micro_grain_scale: 320.0,
            micro_grain_roughness: 0.035,
            regional_relief: 1.0,
            pore_vis_nose: 1.6,
            pore_vis_cheeks: 1.0,
            pore_vis_forehead: 0.55,
            pore_vis_chin: 1.1,
            pore_vis_ears: 0.5,
            pore_vis_eye_area: 0.25,
            pore_vis_neck: 0.5,
        }
    }
}

impl SkinMaterial {
    /// Rejects invalid values instead of silently changing a saved material.
    pub fn validate(&self) -> Result<(), PresetError> {
        for (index, value) in self.base_color.iter().enumerate() {
            validate_range(&format!("base_color[{index}]"), *value, 0.0, 1.0)?;
        }
        validate_range("pore_strength", self.pore_strength, 0.0, 2.0)?;
        validate_range("pore_scale", self.pore_scale, 0.25, 4.0)?;
        validate_range("pore_color_strength", self.pore_color_strength, 0.0, 1.0)?;
        validate_range("pore_micro_detail", self.pore_micro_detail, 0.0, 1.0)?;
        validate_range("pore_micro_scale", self.pore_micro_scale, 40.0, 320.0)?;
        validate_range("pore_irregularity", self.pore_irregularity, 0.0, 1.0)?;
        validate_range("pore_size_variation", self.pore_size_variation, 0.0, 1.0)?;
        if !(256..=4096).contains(&self.pore_map_resolution) {
            return Err(PresetError::InvalidParameter {
                name: "pore_map_resolution".to_owned(),
                value: self.pore_map_resolution as f32,
                min: 256.0,
                max: 4096.0,
            });
        }
        validate_range("pore_map_density", self.pore_map_density, 5.0, 100.0)?;
        validate_range("pore_map_relief", self.pore_map_relief, 0.5, 10.0)?;
        validate_range("fine_detail_strength", self.fine_detail_strength, 0.0, 0.5)?;
        validate_range("fine_detail_scale", self.fine_detail_scale, 10.0, 500.0)?;
        validate_range("micro_grain_strength", self.micro_grain_strength, 0.0, 0.20)?;
        validate_range("micro_grain_scale", self.micro_grain_scale, 50.0, 1000.0)?;
        validate_range("micro_grain_roughness", self.micro_grain_roughness, 0.0, 0.15)?;
        validate_range("regional_relief", self.regional_relief, 0.0, 1.0)?;
        validate_range("pore_vis_nose", self.pore_vis_nose, 0.0, 2.0)?;
        validate_range("pore_vis_cheeks", self.pore_vis_cheeks, 0.0, 2.0)?;
        validate_range("pore_vis_forehead", self.pore_vis_forehead, 0.0, 2.0)?;
        validate_range("pore_vis_chin", self.pore_vis_chin, 0.0, 2.0)?;
        validate_range("pore_vis_ears", self.pore_vis_ears, 0.0, 2.0)?;
        validate_range("pore_vis_eye_area", self.pore_vis_eye_area, 0.0, 2.0)?;
        validate_range("pore_vis_neck", self.pore_vis_neck, 0.0, 2.0)?;
        validate_range("roughness", self.roughness, 0.03, 1.0)?;
        validate_range("specular", self.specular, 0.0, 1.0)?;
        validate_range("subsurface", self.subsurface, 0.0, 0.7)?;
        validate_range("micro_relief", self.micro_relief, 0.0, 1.0)?;
        validate_range("hemoglobin_strength", self.hemoglobin_strength, 0.0, 1.0)?;
        validate_range("freckle_strength", self.freckle_strength, 0.0, 1.0)?;
        validate_range("oiliness", self.oiliness, 0.0, 1.0)?;
        validate_range("pigment_variation", self.pigment_variation, 0.0, 1.0)?;
        Ok(())
    }
}

fn validate_range(name: &str, value: f32, min: f32, max: f32) -> Result<(), PresetError> {
    if !value.is_finite() || value < min || value > max {
        return Err(PresetError::InvalidParameter {
            name: name.to_owned(),
            value,
            min,
            max,
        });
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkinPreset {
    pub schema_version: u32,
    pub name: String,
    pub material: SkinMaterial,
}

#[derive(Debug, Deserialize)]
struct PresetHeader {
    schema_version: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkinMaterialV1 {
    base_color: [f32; 3],
    pore_strength: f32,
    roughness: f32,
    specular: f32,
    subsurface: f32,
    micro_relief: f32,
}

impl From<SkinMaterialV1> for SkinMaterial {
    fn from(old: SkinMaterialV1) -> Self {
        Self {
            base_color: old.base_color,
            pore_strength: old.pore_strength,
            roughness: old.roughness,
            specular: old.specular,
            subsurface: old.subsurface,
            micro_relief: old.micro_relief,
            ..Self::default()
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkinPresetV1 {
    schema_version: u32,
    name: String,
    material: SkinMaterialV1,
}

/// Exact schema retained by the first standalone Rust prototype (preset version 2).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkinMaterialV2 {
    base_color: [f32; 3],
    pore_strength: f32,
    roughness: f32,
    specular: f32,
    subsurface: f32,
    micro_relief: f32,
    hemoglobin_strength: f32,
    freckle_strength: f32,
    oiliness: f32,
    pigment_variation: f32,
}

impl From<SkinMaterialV2> for SkinMaterial {
    fn from(old: SkinMaterialV2) -> Self {
        Self {
            base_color: old.base_color,
            pore_strength: old.pore_strength,
            roughness: old.roughness,
            specular: old.specular,
            subsurface: old.subsurface,
            micro_relief: old.micro_relief,
            hemoglobin_strength: old.hemoglobin_strength,
            freckle_strength: old.freckle_strength,
            oiliness: old.oiliness,
            pigment_variation: old.pigment_variation,
            ..Self::default()
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkinPresetV2 {
    schema_version: u32,
    name: String,
    material: SkinMaterialV2,
}

impl SkinPreset {
    pub fn new(name: impl Into<String>, material: SkinMaterial) -> Result<Self, PresetError> {
        let preset = Self {
            schema_version: PRESET_SCHEMA_VERSION,
            name: name.into(),
            material,
        };
        preset.validate()?;
        Ok(preset)
    }

    pub fn validate(&self) -> Result<(), PresetError> {
        if self.schema_version != PRESET_SCHEMA_VERSION {
            return Err(PresetError::UnsupportedVersion(self.schema_version));
        }
        if self.name.trim().is_empty() {
            return Err(PresetError::EmptyName);
        }
        self.material.validate()
    }

    pub fn to_json_pretty(&self) -> Result<String, PresetError> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(PresetError::Json)
    }

    pub fn from_json(source: &str) -> Result<Self, PresetError> {
        let header: PresetHeader = serde_json::from_str(source).map_err(PresetError::Json)?;
        let preset = match header.schema_version {
            1 => {
                let old: SkinPresetV1 =
                    serde_json::from_str(source).map_err(PresetError::Json)?;
                debug_assert_eq!(old.schema_version, 1);
                Self {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: old.name,
                    material: old.material.into(),
                }
            }
            2 => {
                let old: SkinPresetV2 =
                    serde_json::from_str(source).map_err(PresetError::Json)?;
                debug_assert_eq!(old.schema_version, 2);
                Self {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: old.name,
                    material: old.material.into(),
                }
            }
            PRESET_SCHEMA_VERSION => serde_json::from_str(source).map_err(PresetError::Json)?,
            version => return Err(PresetError::UnsupportedVersion(version)),
        };
        preset.validate()?;
        Ok(preset)
    }
}

pub fn save_preset(path: impl AsRef<Path>, preset: &SkinPreset) -> Result<(), PresetError> {
    let json = preset.to_json_pretty()?;
    fs::write(path, json).map_err(PresetError::Io)
}

pub fn load_preset(path: impl AsRef<Path>) -> Result<SkinPreset, PresetError> {
    let json = fs::read_to_string(path).map_err(PresetError::Io)?;
    SkinPreset::from_json(&json)
}

#[derive(Debug)]
pub enum PresetError {
    EmptyName,
    UnsupportedVersion(u32),
    InvalidParameter { name: String, value: f32, min: f32, max: f32 },
    Json(serde_json::Error),
    Io(std::io::Error),
}

impl fmt::Display for PresetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "preset name cannot be empty"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported preset schema version: {version}")
            }
            Self::InvalidParameter { name, value, min, max } => {
                write!(f, "{name}={value} is outside the valid range {min}..={max}")
            }
            Self::Json(error) => write!(f, "invalid preset JSON: {error}"),
            Self::Io(error) => write!(f, "preset file error: {error}"),
        }
    }
}

impl Error for PresetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_material_is_valid() {
        assert!(SkinMaterial::default().validate().is_ok());
    }

    #[test]
    fn preset_round_trips_through_json() {
        let material = SkinMaterial {
            base_color: [0.2, 0.45, 0.7],
            roughness: 0.72,
            pore_scale: 1.25,
            ..SkinMaterial::default()
        };
        let preset = SkinPreset::new("Test skin", material).unwrap();
        let json = preset.to_json_pretty().unwrap();
        assert_eq!(SkinPreset::from_json(&json).unwrap(), preset);
    }

    #[test]
    fn rejects_non_finite_and_out_of_range_values() {
        let mut material = SkinMaterial::default();
        material.pore_strength = f32::NAN;
        assert!(material.validate().is_err());
        material.pore_strength = 1.2;
        assert!(material.validate().is_err());
    }

    #[test]
    fn migrates_v1_presets_and_keeps_existing_parameters() {
        let json = r#"{"schema_version":1,"name":"Legacy","material":{"base_color":[0.4,0.25,0.18],"pore_strength":0.6,"roughness":0.7,"specular":0.3,"subsurface":0.2,"micro_relief":0.5}}"#;
        let migrated = SkinPreset::from_json(json).unwrap();
        assert_eq!(migrated.schema_version, PRESET_SCHEMA_VERSION);
        assert_eq!(migrated.material.base_color, [0.4, 0.25, 0.18]);
        assert_eq!(migrated.material.pore_strength, 0.6);
        assert_eq!(migrated.material.hemoglobin_strength, SkinMaterial::default().hemoglobin_strength);
        assert_eq!(migrated.material.freckle_strength, SkinMaterial::default().freckle_strength);
        assert_eq!(migrated.material.oiliness, SkinMaterial::default().oiliness);
    }

    #[test]
    fn validates_new_skin_editor_parameters() {
        let mut material = SkinMaterial::default();
        material.oiliness = 1.1;
        assert!(material.validate().is_err());
        material.oiliness = 0.5;
        material.freckle_strength = f32::NAN;
        assert!(material.validate().is_err());
    }

    #[test]
    fn migrates_v2_presets_and_initializes_original_pore_controls() {
        let json = r#"{"schema_version":2,"name":"Legacy v2","material":{"base_color":[0.5,0.3,0.24],"pore_strength":0.42,"roughness":0.58,"specular":0.24,"subsurface":0.18,"micro_relief":0.35,"hemoglobin_strength":0.28,"freckle_strength":0.12,"oiliness":0.22,"pigment_variation":0.35}}"#;
        let migrated = SkinPreset::from_json(json).unwrap();
        assert_eq!(migrated.schema_version, PRESET_SCHEMA_VERSION);
        assert_eq!(migrated.material.pore_strength, 0.42);
        assert_eq!(migrated.material.pore_micro_scale, 145.0);
        assert_eq!(migrated.material.pore_map_resolution, 1024);
        assert_eq!(migrated.material.pore_seed, 42);
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let preset = SkinPreset {
            schema_version: PRESET_SCHEMA_VERSION + 1,
            name: "Future preset".to_owned(),
            material: SkinMaterial::default(),
        };
        assert!(matches!(preset.validate(), Err(PresetError::UnsupportedVersion(_))));
    }

    #[test]
    fn rejects_empty_names_and_invalid_json_materials() {
        assert!(matches!(
            SkinPreset::new("  ", SkinMaterial::default()),
            Err(PresetError::EmptyName)
        ));
        let json = r#"{"schema_version":1,"name":"Bad","material":{"base_color":[2.0,0.0,0.0],"pore_strength":0.4,"roughness":0.5,"specular":0.2,"subsurface":0.1,"micro_relief":0.2}}"#;
        assert!(SkinPreset::from_json(json).is_err());
    }
}
