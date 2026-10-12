//! Tileable reference-height generator ported from Skin Studio 2.4.76.
//!
//! Mirrors the original add-on functions _ref_skin_height(), _ref_noise() and
//! _ref_worley(): two warped Worley networks, shallow cell domes, cross-cell
//! furrows, junction pores, smooth periodic interior noise and grain.
//! Values are linear/non-color height data, normalized around 0.5 for a Bump layer.

use std::{error::Error, f32::consts::PI, fmt};

use rustfft::{FftPlanner, num_complex::Complex};

pub const DEFAULT_REFERENCE_RESOLUTION: u32 = 1024;
pub const DEFAULT_REFERENCE_SEED: u32 = 11;
pub const DEFAULT_REFERENCE_CELL_MM: f32 = 1.16;
pub const DEFAULT_REFERENCE_PORES: f32 = 1.0;

/// Settings corresponding to the original Reference Height generator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReferenceHeightSettings {
    pub resolution: u32,
    pub seed: u32,
    pub pores: f32,
}

impl Default for ReferenceHeightSettings {
    fn default() -> Self {
        Self {
            resolution: DEFAULT_REFERENCE_RESOLUTION,
            seed: DEFAULT_REFERENCE_SEED,
            pores: DEFAULT_REFERENCE_PORES,
        }
    }
}

impl ReferenceHeightSettings {
    pub fn validate(self) -> Result<Self, ReferenceHeightError> {
        if !(256..=4096).contains(&self.resolution) {
            return Err(ReferenceHeightError::InvalidSetting(
                "resolution must be between 256 and 4096".to_owned(),
            ));
        }
        if !self.pores.is_finite() || !(0.0..=3.0).contains(&self.pores) {
            return Err(ReferenceHeightError::InvalidSetting(
                "reference pores must be between 0 and 3".to_owned(),
            ));
        }
        Ok(self)
    }
}

/// Single-channel linear height map; upload it as R16Float, never as sRGB color.
#[derive(Clone, Debug)]
pub struct ReferenceHeightMap {
    pub resolution: u32,
    pub cells_per_tile: u32,
    pub values: Vec<f32>,
}

impl ReferenceHeightMap {
    pub fn flat(resolution: u32) -> Self {
        Self {
            resolution,
            cells_per_tile: (resolution / 8).max(8),
            values: vec![0.5; resolution as usize * resolution as usize],
        }
    }

    pub fn generate(settings: ReferenceHeightSettings) -> Result<Self, ReferenceHeightError> {
        let settings = settings.validate()?;
        let size = settings.resolution as usize;
        let n_small = (size / 8).max(8);
        let pitch = size as f32 / n_small as f32;

        // The original uses _ref_noise(size, seed + k) for periodic domain warp.
        let warp_x_small = scaled(&periodic_noise(size, settings.seed + 1), 0.25);
        let warp_y_small = scaled(&periodic_noise(size, settings.seed + 2), 0.25);
        let noise5 = periodic_noise(size, settings.seed + 5);
        let (f1, f2, id) = worley(size, n_small, settings.seed, &warp_x_small, &warp_y_small);
        drop(warp_x_small);
        drop(warp_y_small);

        let mut edge_distance = vec![0.0_f32; size * size];
        let mut height = vec![0.0_f32; size * size];
        for i in 0..height.len() {
            let edge = f2[i] - f1[i];
            edge_distance[i] = edge;
            let furrow = 1.0 - smoothstep(0.0, 0.12, edge);
            let depth_mask = 0.55 + 0.45 * smoothstep(-0.55, 0.65, noise5[i]);
            height[i] =
                0.30 * (1.0 - 0.6 * f1[i] * f1[i]) + 0.08 * id[i] - furrow * depth_mask;
        }
        drop(f1);
        drop(f2);
        drop(id);
        drop(noise5);

        // Second, finer and shallower network; parameters match _REF_SKIN.
        let n_fine = ((n_small as f32 * 2.3).round() as usize).max(16);
        let warp_x_fine = scaled(&periodic_noise(size, settings.seed + 3), 0.2);
        let warp_y_fine = scaled(&periodic_noise(size, settings.seed + 4), 0.2);
        let (fine_f1, fine_f2, fine_id) =
            worley(size, n_fine, settings.seed + 50, &warp_x_fine, &warp_y_fine);
        drop(warp_x_fine);
        drop(warp_y_fine);
        for i in 0..height.len() {
            let fine_furrow = 1.0 - smoothstep(0.0, 0.14, fine_f2[i] - fine_f1[i]);
            height[i] -= 0.30 * fine_furrow;
        }
        drop(fine_f1);
        drop(fine_f2);
        drop(fine_id);

        let mut rng = SeededRng::new(settings.seed as u64 + 9);
        if settings.pores > 0.0 {
            let band_count = edge_distance.iter().filter(|edge| **edge < 0.14).count().max(1);
            let total = 0.8 * (n_small * n_small) as f32;
            let probability = (total / band_count as f32).min(1.0);
            let impulse: Vec<f32> = edge_distance
                .iter()
                .map(|edge| {
                    if *edge < 0.14 && rng.unit_f32() < probability { 1.0 } else { 0.0 }
                })
                .collect();
            let sigma = (0.13 * pitch).max(0.7);
            let blurred = periodic_gaussian_blur(&impulse, size, sigma);
            let amplitude = settings.pores * 1.3 * (2.0 * PI * sigma * sigma);
            for (h, pore) in height.iter_mut().zip(blurred) {
                *h -= amplitude * pore;
            }
        }
        drop(edge_distance);

        // Low-amplitude interior texture and fine grain are distinct layers.
        let interior_sigma = (0.10 * pitch).max(0.8);
        let mut interior = vec![0.0; size * size];
        for value in &mut interior {
            *value = rng.standard_normal();
        }
        interior = periodic_gaussian_blur(&interior, size, interior_sigma);
        normalize_zero_mean_unit_std(&mut interior);
        let mut grain = vec![0.0; size * size];
        for value in &mut grain {
            *value = rng.standard_normal();
        }
        grain = periodic_gaussian_blur(&grain, size, 0.7);
        normalize_zero_mean_unit_std(&mut grain);

        for i in 0..height.len() {
            height[i] += 0.12 * interior[i] + 0.05 * grain[i];
        }
        normalize_zero_mean_unit_std(&mut height);
        let values = height
            .into_iter()
            .map(|h| (0.5 + 0.15 * h).clamp(0.0, 1.0))
            .collect();

        Ok(Self {
            resolution: settings.resolution,
            cells_per_tile: n_small as u32,
            values,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceHeightError {
    InvalidSetting(String),
}

impl fmt::Display for ReferenceHeightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSetting(message) => write!(f, "invalid reference-height setting: {message}"),
        }
    }
}

impl Error for ReferenceHeightError {}

fn scaled(source: &[f32], scale: f32) -> Vec<f32> {
    source.iter().map(|v| v * scale).collect()
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Periodic 2-D Gaussian noise. The Fourier filter follows the source add-on's
/// frequency-domain smoothing; the PRNG sequence is Rust-native but fixed per seed.
fn periodic_noise(size: usize, seed: u32) -> Vec<f32> {
    let sigma_frequency = 0.035 * 512.0 / size as f32;
    let sigma_pixels = 1.0 / (2.0 * PI * sigma_frequency);
    let mut rng = SeededRng::new(seed as u64);
    let mut field = vec![0.0; size * size];
    for value in &mut field {
        *value = rng.standard_normal();
    }
    let mut field = periodic_gaussian_blur(&field, size, sigma_pixels);
    normalize_zero_mean_unit_std(&mut field);
    field
}

fn normalize_zero_mean_unit_std(values: &mut [f32]) {
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    let variance = values
        .iter()
        .map(|v| {
            let delta = *v - mean;
            delta * delta
        })
        .sum::<f32>()
        / values.len() as f32;
    let std = variance.sqrt().max(1.0e-8);
    for value in values {
        *value = (*value - mean) / std;
    }
}

/// Periodic Gaussian convolution using a separable 2-D Fourier-domain Gaussian.
fn periodic_gaussian_blur(input: &[f32], size: usize, sigma_pixels: f32) -> Vec<f32> {
    let mut planner = FftPlanner::<f32>::new();
    let forward = planner.plan_fft_forward(size);
    let inverse = planner.plan_fft_inverse(size);
    let gaussian: Vec<f32> = (0..size)
        .map(|index| {
            let signed = if index <= size / 2 {
                index as f32
            } else {
                index as f32 - size as f32
            };
            let frequency = signed / size as f32;
            (-2.0 * PI * PI * sigma_pixels * sigma_pixels * frequency * frequency).exp()
        })
        .collect();

    let mut data: Vec<Complex<f32>> = input
        .iter()
        .map(|value| Complex::new(*value, 0.0))
        .collect();

    for row in data.chunks_mut(size) {
        forward.process(row);
        for (value, weight) in row.iter_mut().zip(&gaussian) {
            *value *= *weight;
        }
    }

    let mut column = vec![Complex::new(0.0, 0.0); size];
    for x in 0..size {
        for y in 0..size {
            column[y] = data[y * size + x];
        }
        forward.process(&mut column);
        for (value, weight) in column.iter_mut().zip(&gaussian) {
            *value *= *weight;
        }
        inverse.process(&mut column);
        for y in 0..size {
            data[y * size + x] = column[y];
        }
    }

    let normalization = 1.0 / (size * size) as f32;
    for row in data.chunks_mut(size) {
        inverse.process(row);
        for value in row {
            *value *= normalization;
        }
    }
    data.into_iter().map(|c| c.re).collect()
}

struct SeededRng {
    state: u64,
    spare_normal: Option<f32>,
}

impl SeededRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9E3779B97F4A7C15,
            spare_normal: None,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    fn unit_f32(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) / ((1_u32 << 24) as f32)
    }

    fn standard_normal(&mut self) -> f32 {
        if let Some(value) = self.spare_normal.take() {
            return value;
        }
        let u1 = self.unit_f32().max(1.0e-7);
        let u2 = self.unit_f32();
        let radius = (-2.0 * u1.ln()).sqrt();
        let angle = 2.0 * PI * u2;
        let first = radius * angle.cos();
        self.spare_normal = Some(radius * angle.sin());
        first
    }
}

fn worley(
    size: usize,
    cells: usize,
    seed: u32,
    warp_x: &[f32],
    warp_y: &[f32],
) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut rng = SeededRng::new(seed as u64);
    let mut points = vec![[0.0_f32; 2]; cells * cells];
    let mut identity = vec![0.0_f32; cells * cells];
    for index in 0..points.len() {
        points[index] = [rng.unit_f32(), rng.unit_f32()];
        identity[index] = rng.unit_f32();
    }

    let coords: Vec<f32> = (0..size)
        .map(|i| (i as f32 + 0.5) / size as f32 * cells as f32)
        .collect();
    let mut f1 = vec![0.0_f32; size * size];
    let mut f2 = vec![0.0_f32; size * size];
    let mut ids = vec![0.0_f32; size * size];

    for y in 0..size {
        for x in 0..size {
            let index = y * size + x;
            let px = coords[x] + warp_x[index];
            let py = coords[y] + warp_y[index];
            let cell_x = px.floor() as isize;
            let cell_y = py.floor() as isize;
            let fx = px - cell_x as f32;
            let fy = py - cell_y as f32;
            let mut near1 = f32::INFINITY;
            let mut near2 = f32::INFINITY;
            let mut near_id = 0.0_f32;

            for dy in -1_isize..=1 {
                for dx in -1_isize..=1 {
                    let ix = (cell_x + dx).rem_euclid(cells as isize) as usize;
                    let iy = (cell_y + dy).rem_euclid(cells as isize) as usize;
                    let point_index = iy * cells + ix;
                    let point = points[point_index];
                    let vx = point[0] + dx as f32 - fx;
                    let vy = point[1] + dy as f32 - fy;
                    let distance = (vx * vx + vy * vy).sqrt();
                    if distance < near1 {
                        near2 = near1;
                        near1 = distance;
                        near_id = identity[point_index];
                    } else if distance < near2 {
                        near2 = distance;
                    }
                }
            }

            f1[index] = near1;
            f2[index] = near2;
            ids[index] = near_id;
        }
    }
    (f1, f2, ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_defaults_match_original_generator() {
        assert_eq!(ReferenceHeightSettings::default().resolution, 1024);
        assert_eq!(ReferenceHeightSettings::default().seed, 11);
        assert_eq!(ReferenceHeightSettings::default().pores, 1.0);
    }

    #[test]
    fn skin_reference_map_is_deterministic_and_linear() {
        let settings = ReferenceHeightSettings { resolution: 256, ..ReferenceHeightSettings::default() };
        let first = ReferenceHeightMap::generate(settings).unwrap();
        let second = ReferenceHeightMap::generate(settings).unwrap();
        assert_eq!(first.values, second.values);
        assert_eq!(first.values.len(), 256 * 256);
        assert!(first.values.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)));
        let min = first.values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = first.values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        assert!(max - min > 0.1);
    }

    #[test]
    fn pores_control_only_changes_reference_map_detail() {
        let with_pores = ReferenceHeightMap::generate(ReferenceHeightSettings {
            resolution: 256, pores: 1.0, ..ReferenceHeightSettings::default()
        }).unwrap();
        let without_pores = ReferenceHeightMap::generate(ReferenceHeightSettings {
            resolution: 256, pores: 0.0, ..ReferenceHeightSettings::default()
        }).unwrap();
        assert_ne!(with_pores.values, without_pores.values);
    }

    #[test]
    fn invalid_reference_settings_are_rejected() {
        assert!(ReferenceHeightSettings { resolution: 128, ..ReferenceHeightSettings::default() }.validate().is_err());
        assert!(ReferenceHeightSettings { pores: 3.1, ..ReferenceHeightSettings::default() }.validate().is_err());
    }
}
