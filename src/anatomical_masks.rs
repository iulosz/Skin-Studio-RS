use std::path::Path;

/// Linear RGBA8 anatomical weight atlas sampled with the imported mesh UVs.
#[derive(Clone, Debug)]
pub struct AnatomicalMaskAtlas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}
impl AnatomicalMaskAtlas {
    pub fn blank() -> Self { Self { width: 1, height: 1, pixels: vec![0, 0, 0, 0] } }
    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0 || self.height == 0 || self.width > 4096 || self.height > 4096 {
            return Err("Mask atlas dimensions must be between 1 and 4096 pixels".to_owned());
        }
        let expected = (self.width as usize).checked_mul(self.height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| "Mask atlas dimensions overflow addressable memory".to_owned())?;
        if self.pixels.len() != expected {
            return Err(format!("Mask atlas has {} bytes; expected {expected}", self.pixels.len()));
        }
        Ok(())
    }
}
pub fn load_png(path: impl AsRef<Path>) -> Result<AnatomicalMaskAtlas, String> {
    let path = path.as_ref();
    let image = image::open(path).map_err(|e| format!("Could not decode {}: {e}", path.display()))?.to_rgba8();
    let atlas = AnatomicalMaskAtlas { width: image.width(), height: image.height(), pixels: image.into_raw() };
    atlas.validate()?;
    Ok(atlas)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blank_atlas_is_a_valid_zero_mask() {
        let atlas = AnatomicalMaskAtlas::blank();
        assert!(atlas.validate().is_ok());
        assert_eq!(atlas.pixels, [0, 0, 0, 0]);
    }
    #[test]
    fn rejects_a_mismatched_pixel_buffer() {
        let atlas = AnatomicalMaskAtlas { width: 2, height: 2, pixels: vec![0; 15] };
        assert!(atlas.validate().is_err());
    }
}
