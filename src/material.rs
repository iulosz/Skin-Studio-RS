use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, fs, path::Path};

pub const PRESET_SCHEMA_VERSION: u32 = 7;

/// Real-world length used by the source add-on's Coord Scale conversion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateUnit {
    #[default]
    Meter,
    Centimeter,
    Millimeter,
    Custom,
}

impl CoordinateUnit {
    pub fn label(self) -> &'static str {
        match self {
            Self::Meter => "Meters",
            Self::Centimeter => "Centimeters",
            Self::Millimeter => "Millimeters",
            Self::Custom => "Custom",
        }
    }

    pub fn meters_per_unit(self, custom: f32) -> f32 {
        match self {
            Self::Meter => 1.0,
            Self::Centimeter => 0.01,
            Self::Millimeter => 0.001,
            Self::Custom => custom,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
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
    pub spec_pore_occlusion: f32,
    pub coord_unit: CoordinateUnit,
    pub coord_custom_meters_per_unit: f32,
    pub reference_strength: f32,
    pub reference_relief: f32,
    pub reference_cell_mm: f32,
    pub reference_pores: f32,
    pub reference_seed: u32,
    pub reference_resolution: u32,
    pub reference_color: f32,
    pub reference_rough: f32,
    pub reference_regional: f32,
    pub skin_line_strength: f32,
    pub skin_line_scale: f32,
    pub skin_line_thickness: f32,
    pub skin_line_relief: f32,
    pub skin_line_shading: f32,
    pub variation_scale: f32,
    pub redness_strength: f32,
    pub redness_scale: f32,
    pub skin_warmth: f32,
    pub mottled_enable: bool,
    pub mottled_strength: f32,
    pub mottled_scale_macro: f32,
    pub mottled_scale_medium: f32,
    pub mottled_scale_micro: f32,
    pub mottled_variation: f32,
    pub mottled_contrast: f32,
    pub melanin_variation: f32,
    pub melanin_scale: f32,
    pub erythema_strength: f32,
    pub erythema_scale: f32,
    pub golden_variation: f32,
    pub golden_scale: f32,
    pub micro_color_strength: f32,
    pub micro_color_scale: f32,
}

impl Default for SkinMaterial {
    fn default() -> Self {
        Self {
            base_color: [0.4948, 0.2102, 0.1229],
            pore_strength: 1.0,
            roughness: 0.46,
            specular: 0.50,
            subsurface: 0.095,
            micro_relief: 0.35,
            hemoglobin_strength: 0.32,
            freckle_strength: 0.0,
            oiliness: 0.12,
            pigment_variation: 0.12,
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
            spec_pore_occlusion: 0.55,
            coord_unit: CoordinateUnit::Meter,
            coord_custom_meters_per_unit: 1.0,
            reference_strength: 0.0,
            reference_relief: 1.0,
            reference_cell_mm: 1.16,
            reference_pores: 1.0,
            reference_seed: 11,
            reference_resolution: 1024,
            reference_color: 0.0,
            reference_rough: 0.0,
            reference_regional: 0.0,
            skin_line_strength: 0.6,
            skin_line_scale: 80.0,
            skin_line_thickness: 0.35,
            skin_line_relief: 1.0,
            skin_line_shading: 0.18,
            variation_scale: 4.0,
            redness_strength: 0.08,
            redness_scale: 18.0,
            skin_warmth: 0.32,
            mottled_enable: true,
            mottled_strength: 0.62,
            mottled_scale_macro: 1.6,
            mottled_scale_medium: 6.0,
            mottled_scale_micro: 22.0,
            mottled_variation: 0.6,
            mottled_contrast: 0.45,
            melanin_variation: 0.16,
            melanin_scale: 3.2,
            erythema_strength: 0.11,
            erythema_scale: 14.0,
            golden_variation: 0.10,
            golden_scale: 9.0,
            micro_color_strength: 0.055,
            micro_color_scale: 55.0,
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
        validate_range("spec_pore_occlusion", self.spec_pore_occlusion, 0.0, 1.0)?;
        validate_range(
            "coord_custom_meters_per_unit",
            self.coord_custom_meters_per_unit,
            0.00001,
            1000.0,
        )?;
        validate_range("reference_strength", self.reference_strength, 0.0, 1.0)?;
        validate_range("reference_relief", self.reference_relief, 0.1, 8.0)?;
        validate_range("reference_cell_mm", self.reference_cell_mm, 0.02, 200.0)?;
        validate_range("reference_pores", self.reference_pores, 0.0, 3.0)?;
        if !(256..=4096).contains(&self.reference_resolution) {
            return Err(PresetError::InvalidParameter {
                name: "reference_resolution".to_owned(),
                value: self.reference_resolution as f32,
                min: 256.0,
                max: 4096.0,
            });
        }
        validate_range("reference_color", self.reference_color, 0.0, 1.0)?;
        validate_range("reference_rough", self.reference_rough, 0.0, 0.6)?;
        validate_range("reference_regional", self.reference_regional, 0.0, 1.0)?;
        validate_range("skin_line_strength", self.skin_line_strength, 0.0, 1.5)?;
        validate_range("skin_line_scale", self.skin_line_scale, 4.0, 400.0)?;
        validate_range("skin_line_thickness", self.skin_line_thickness, 0.05, 1.0)?;
        validate_range("skin_line_relief", self.skin_line_relief, 0.1, 8.0)?;
        validate_range("skin_line_shading", self.skin_line_shading, 0.0, 1.0)?;
        validate_range("variation_scale", self.variation_scale, 0.1, 30.0)?;
        validate_range("redness_strength", self.redness_strength, 0.0, 1.0)?;
        validate_range("redness_scale", self.redness_scale, 0.5, 60.0)?;
        validate_range("skin_warmth", self.skin_warmth, 0.0, 1.0)?;
        validate_range("mottled_strength", self.mottled_strength, 0.0, 1.0)?;
        validate_range("mottled_scale_macro", self.mottled_scale_macro, 0.4, 12.0)?;
        validate_range("mottled_scale_medium", self.mottled_scale_medium, 2.0, 35.0)?;
        validate_range("mottled_scale_micro", self.mottled_scale_micro, 10.0, 120.0)?;
        validate_range("mottled_variation", self.mottled_variation, 0.0, 1.0)?;
        validate_range("mottled_contrast", self.mottled_contrast, 0.0, 1.0)?;
        validate_range("melanin_variation", self.melanin_variation, 0.0, 1.0)?;
        validate_range("melanin_scale", self.melanin_scale, 0.5, 20.0)?;
        validate_range("erythema_strength", self.erythema_strength, 0.0, 1.0)?;
        validate_range("erythema_scale", self.erythema_scale, 2.0, 60.0)?;
        validate_range("golden_variation", self.golden_variation, 0.0, 1.0)?;
        validate_range("golden_scale", self.golden_scale, 2.0, 40.0)?;
        validate_range("micro_color_strength", self.micro_color_strength, 0.0, 1.0)?;
        validate_range("micro_color_scale", self.micro_color_scale, 10.0, 180.0)?;
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
            3 => {
                // Schema 3 introduced the pore subsystem. Schema 4 adds physical
                // coordinate units, so old schema-3 presets inherit meter units.
                let old: SkinPreset =
                    serde_json::from_str(source).map_err(PresetError::Json)?;
                Self {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: old.name,
                    material: old.material,
                }
            }
            4 => {
                // Schema 4 added physical coordinate units. Schema 5 adds the
                // reference-height generator and inherits its source-compatible defaults.
                let old: SkinPreset =
                    serde_json::from_str(source).map_err(PresetError::Json)?;
                Self {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: old.name,
                    material: old.material,
                }
            }
            5 => {
                // Schema 5 added the source Reference Height controls. Skin Line
                // Network controls inherit the original add-on defaults in schema 6.
                let old: SkinPreset =
                    serde_json::from_str(source).map_err(PresetError::Json)?;
                Self {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: old.name,
                    material: old.material,
                }
            }
            6 => {
                // Schema 6 introduced Skin Line Network. Multi-scale color controls
                // inherit the original add-on defaults in schema 7.
                let old: SkinPreset =
                    serde_json::from_str(source).map_err(PresetError::Json)?;
                Self {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: old.name,
                    material: old.material,
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
        material.pore_strength = 2.2;
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
    fn migrates_v3_presets_to_meter_coordinate_units() {
        let material = SkinMaterial::default();
        let v4 = SkinPreset::new("Schema 3", material).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_str(&v4.to_json_pretty().unwrap()).unwrap();
        value["schema_version"] = serde_json::Value::from(3);
        let material_object = value["material"]
            .as_object_mut()
            .expect("preset material should be a JSON object");
        material_object.remove("coord_unit");
        material_object.remove("coord_custom_meters_per_unit");
        let json = value.to_string();

        let migrated = SkinPreset::from_json(&json).unwrap();
        assert_eq!(migrated.schema_version, PRESET_SCHEMA_VERSION);
        assert_eq!(migrated.material.coord_unit, CoordinateUnit::Meter);
        assert_eq!(migrated.material.coord_custom_meters_per_unit, 1.0);
    }

    #[test]
    fn migrates_v4_presets_and_initializes_reference_height_defaults() {
        let preset = SkinPreset::new("Schema 4", SkinMaterial::default()).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_str(&preset.to_json_pretty().unwrap()).unwrap();
        value["schema_version"] = serde_json::Value::from(4);
        let object = value["material"]
            .as_object_mut()
            .expect("preset material should be a JSON object");
        for key in [
            "reference_strength",
            "reference_relief",
            "reference_cell_mm",
            "reference_pores",
            "reference_seed",
            "reference_resolution",
            "reference_color",
            "reference_rough",
            "reference_regional",
        ] {
            object.remove(key);
        }
        let migrated = SkinPreset::from_json(&value.to_string()).unwrap();
        assert_eq!(migrated.schema_version, PRESET_SCHEMA_VERSION);
        assert_eq!(migrated.material.reference_strength, 0.0);
        assert_eq!(migrated.material.reference_cell_mm, 1.16);
        assert_eq!(migrated.material.reference_resolution, 1024);
        assert_eq!(migrated.material.reference_seed, 11);
    }

    #[test]
    fn migrates_v5_presets_and_initializes_skin_line_controls() {
        let preset = SkinPreset::new("Schema 5", SkinMaterial::default()).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_str(&preset.to_json_pretty().unwrap()).unwrap();
        value["schema_version"] = serde_json::Value::from(5);
        let object = value["material"]
            .as_object_mut()
            .expect("preset material should be a JSON object");
        for key in [
            "skin_line_strength",
            "skin_line_scale",
            "skin_line_thickness",
            "skin_line_relief",
            "skin_line_shading",
        ] {
            object.remove(key);
        }
        let migrated = SkinPreset::from_json(&value.to_string()).unwrap();
        assert_eq!(migrated.schema_version, PRESET_SCHEMA_VERSION);
        assert_eq!(migrated.material.skin_line_strength, 0.6);
        assert_eq!(migrated.material.skin_line_scale, 80.0);
        assert_eq!(migrated.material.skin_line_thickness, 0.35);
        assert_eq!(migrated.material.skin_line_relief, 1.0);
        assert_eq!(migrated.material.skin_line_shading, 0.18);
    }

    #[test]
    fn migrates_v6_presets_and_initializes_multiscale_color_defaults() {
        let preset = SkinPreset::new("Schema 6", SkinMaterial::default()).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_str(&preset.to_json_pretty().unwrap()).unwrap();
        value["schema_version"] = serde_json::Value::from(6);
        let object = value["material"]
            .as_object_mut()
            .expect("preset material should be a JSON object");
        for key in [
            "variation_scale", "redness_strength", "redness_scale", "skin_warmth",
            "mottled_enable", "mottled_strength", "mottled_scale_macro",
            "mottled_scale_medium", "mottled_scale_micro", "mottled_variation",
            "mottled_contrast", "melanin_variation", "melanin_scale",
            "erythema_strength", "erythema_scale", "golden_variation",
            "golden_scale", "micro_color_strength", "micro_color_scale",
        ] {
            object.remove(key);
        }
        let migrated = SkinPreset::from_json(&value.to_string()).unwrap();
        assert_eq!(migrated.schema_version, PRESET_SCHEMA_VERSION);
        assert_eq!(migrated.material.variation_scale, 4.0);
        assert!(migrated.material.mottled_enable);
        assert_eq!(migrated.material.mottled_strength, 0.62);
        assert_eq!(migrated.material.melanin_scale, 3.2);
        assert_eq!(migrated.material.erythema_strength, 0.11);
        assert_eq!(migrated.material.golden_scale, 9.0);
        assert_eq!(migrated.material.micro_color_scale, 55.0);
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
