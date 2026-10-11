//! CPU-side procedural height-map generation ported from Skin Studio 2.4.76.
//!
//! This is the Rust counterpart of `generate_pore_normal_image()` in the original
//! Blender add-on. The generated values are linear/non-color height data, not RGB
//! albedo and not a tangent-space normal map. Its lattice and micro-wave frequencies
//! are periodic so the texture can be sampled with Repeat addressing.

use std::{error::Error, f64::consts::PI, fmt};

pub const DEFAULT_PORE_MAP_RESOLUTION: u32 = 1024;
pub const DEFAULT_PORE_DENSITY: f32 = 35.0;
pub const DEFAULT_PORE_RELIEF: f32 = 4.0;
pub const DEFAULT_PORE_SEED: u32 = 42;

/// Settings corresponding to the original add-on's Pore Resolution, Pore Density,
/// Pore Relief and Pore Seed controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoreMapSettings {
    pub resolution: u32,
    pub density: f32,
    pub relief: f32,
    pub seed: u32,
}

impl Default for PoreMapSettings {
    fn default() -> Self {
        Self {
            resolution: DEFAULT_PORE_MAP_RESOLUTION,
            density: DEFAULT_PORE_DENSITY,
            relief: DEFAULT_PORE_RELIEF,
            seed: DEFAULT_PORE_SEED,
        }
    }
}

impl PoreMapSettings {
    pub fn validate(self) -> Result<Self, PoreMapError> {
        if !(256..=4096).contains(&self.resolution) {
            return Err(PoreMapError::InvalidSetting(
                "resolution must be between 256 and 4096".to_owned(),
            ));
        }
        if !self.density.is_finite() || !(5.0..=100.0).contains(&self.density) {
            return Err(PoreMapError::InvalidSetting(
                "density must be between 5 and 100".to_owned(),
            ));
        }
        if !self.relief.is_finite() || !(0.5..=10.0).contains(&self.relief) {
            return Err(PoreMapError::InvalidSetting(
                "relief must be between 0.5 and 10".to_owned(),
            ));
        }
        Ok(self)
    }
}

/// A single-channel, linear height image. GPU upload code converts values to R16Float.
#[derive(Clone, Debug)]
pub struct PoreHeightMap {
    pub resolution: u32,
    pub values: Vec<f32>,
}

impl PoreHeightMap {
    pub fn generate(settings: PoreMapSettings) -> Result<Self, PoreMapError> {
        let settings = settings.validate()?;
        let resolution = settings.resolution as usize;
        let cells_y = settings.density.round().max(4.0) as usize;
        let cells_x = (settings.density * 1.2).round().max(4.0) as usize;
        let relief = settings.relief as f64 / 4.0;

        // The random feature point for a lattice cell is precomputed. This is
        // algebraically equivalent to the NumPy expression in __init__.py, but
        // avoids repeating sin/cos once per pixel and neighboring-cell visit.
        let seed_x = settings.seed as f64 * 0.1731;
        let seed_y = settings.seed as f64 * 0.0977;
        let mut feature_x = vec![0.0_f64; cells_x * cells_y];
        let mut feature_y = vec![0.0_f64; cells_x * cells_y];
        for y in 0..cells_y {
            for x in 0..cells_x {
                let x = x as f64;
                let y = y as f64;
                let index = y as usize * cells_x + x as usize;
                let rx = ((x * 12.9898 + y * 78.233 + seed_x).sin() * 43758.5453)
                    .rem_euclid(1.0);
                let ry = ((x * 41.4140 + y * 9.9531 + seed_y).cos() * 23421.6311)
                    .rem_euclid(1.0);
                feature_x[index] = rx;
                feature_y[index] = ry;
            }
        }

        let mut output = vec![0.0_f32; resolution * resolution];
        let kx1 = ((25.0 * cells_x as f64) / (2.0 * PI)).round().max(1.0);
        let ky1 = ((25.0 * cells_y as f64) / (2.0 * PI)).round().max(1.0);
        let kx2 = ((55.0 * cells_x as f64) / (2.0 * PI)).round().max(1.0);

        for y in 0..resolution {
            let yy = y as f64 * (cells_y as f64 / resolution as f64);
            let yi = yy.floor() as isize;
            let yf = yy - yi as f64;

            for x in 0..resolution {
                let xx = x as f64 * (cells_x as f64 / resolution as f64);
                let xi = xx.floor() as isize;
                let xf = xx - xi as f64;
                let mut f1 = f64::INFINITY;
                let mut f2 = f64::INFINITY;

                for dy in -1_isize..=1 {
                    for dx in -1_isize..=1 {
                        let cell_x = (xi + dx).rem_euclid(cells_x as isize) as usize;
                        let cell_y = (yi + dy).rem_euclid(cells_y as isize) as usize;
                        let index = cell_y * cells_x + cell_x;
                        let px = dx as f64 + feature_x[index] * 0.85;
                        let py = dy as f64 + feature_y[index] * 0.85;
                        let distance = ((xf - px).powi(2) + (yf - py).powi(2)).sqrt();
                        if distance < f1 {
                            f2 = f1;
                            f1 = distance;
                        } else if distance < f2 {
                            f2 = distance;
                        }
                    }
                }

                let sulci_lines = f2 - f1;
                let cell_domes = 1.0 - (f1 * 1.5).clamp(0.0, 1.0);
                let pores = (1.0 - (f1 * 2.5).clamp(0.0, 1.0)).powi(4);
                // Integer wave counts maintain a seamless tile in each axis.
                let micro = ((2.0 * PI * kx1 * xx / cells_x as f64).sin()
                    * (2.0 * PI * ky1 * yy / cells_y as f64).cos()
                    * 0.03)
                    + (2.0 * PI * kx2 * xx / cells_x as f64).sin() * 0.015;
                let height =
                    cell_domes.powf(1.8) * 0.25 + sulci_lines * 0.35 - pores * 0.8 + micro;
                output[y * resolution + x] = (0.5 + 0.5 * height * relief).clamp(0.0, 1.0) as f32;
            }
        }

        Ok(Self {
            resolution: settings.resolution,
            values: output,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoreMapError {
    InvalidSetting(String),
}

impl fmt::Display for PoreMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSetting(detail) => write!(f, "invalid pore-map setting: {detail}"),
        }
    }
}

impl Error for PoreMapError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_original_addon_generator_settings() {
        let settings = PoreMapSettings::default();
        assert_eq!(settings.resolution, 1024);
        assert_eq!(settings.density, 35.0);
        assert_eq!(settings.relief, 4.0);
        assert_eq!(settings.seed, 42);
    }

    #[test]
    fn generated_height_is_finite_and_in_unit_range() {
        let map = PoreHeightMap::generate(PoreMapSettings {
            resolution: 256,
            ..PoreMapSettings::default()
        })
        .unwrap();
        assert_eq!(map.values.len(), 256 * 256);
        assert!(map.values.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)));
        let min = map.values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = map.values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        assert!(max - min > 0.1, "height map should contain visible relief");
    }

    #[test]
    fn generation_is_deterministic_and_seeded() {
        let settings = PoreMapSettings {
            resolution: 256,
            ..PoreMapSettings::default()
        };
        let a = PoreHeightMap::generate(settings).unwrap();
        let b = PoreHeightMap::generate(settings).unwrap();
        assert_eq!(a.values, b.values);

        let c = PoreHeightMap::generate(PoreMapSettings { seed: 43, ..settings }).unwrap();
        assert_ne!(a.values, c.values);
    }

    #[test]
    fn rejects_invalid_resolution_density_and_relief() {
        assert!(PoreMapSettings { resolution: 128, ..PoreMapSettings::default() }.validate().is_err());
        assert!(PoreMapSettings { density: f32::NAN, ..PoreMapSettings::default() }.validate().is_err());
        assert!(PoreMapSettings { relief: 11.0, ..PoreMapSettings::default() }.validate().is_err());
    }
}
