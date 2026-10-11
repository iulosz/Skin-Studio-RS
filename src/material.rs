use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, fs, path::Path};

pub const PRESET_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkinMaterial {
    pub base_color: [f32; 3],
    pub pore_strength: f32,
    pub roughness: f32,
    pub specular: f32,
    pub subsurface: f32,
    pub micro_relief: f32,
}

impl Default for SkinMaterial {
    fn default() -> Self {
        Self {
            base_color: [0.68, 0.43, 0.33],
            pore_strength: 0.42,
            roughness: 0.58,
            specular: 0.24,
            subsurface: 0.18,
            micro_relief: 0.35,
        }
    }
}

impl SkinMaterial {
    /// Rejects invalid values instead of silently changing a saved material.
    pub fn validate(&self) -> Result<(), PresetError> {
        for (index, value) in self.base_color.iter().enumerate() {
            validate_range(&format!("base_color[{index}]"), *value, 0.0, 1.0)?;
        }
        validate_range("pore_strength", self.pore_strength, 0.0, 1.0)?;
        validate_range("roughness", self.roughness, 0.03, 1.0)?;
        validate_range("specular", self.specular, 0.0, 1.0)?;
        validate_range("subsurface", self.subsurface, 0.0, 0.7)?;
        validate_range("micro_relief", self.micro_relief, 0.0, 1.0)?;
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
        let preset: Self = serde_json::from_str(source).map_err(PresetError::Json)?;
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
