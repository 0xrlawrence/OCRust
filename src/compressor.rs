use image::{DynamicImage, imageops::FilterType};

/// Decodes raw image bytes, applies proportional downscaling if the image
/// exceeds `max_height`, and converts the result to 8-bit grayscale.
///
/// # Processing Pipeline
///
/// 1. Decode the input buffer (supports JPEG, PNG, and any format enabled
///    via the `image` crate feature flags).
/// 2. If the decoded height exceeds `max_height`, compute a scale ratio
///    that preserves the aspect ratio and resize using bilinear (Triangle)
///    interpolation. This filter is chosen for its balance of speed and
///    quality when rendering high-contrast content such as text.
/// 3. Convert the RGB/RGBA pixel data to single-channel Luma8 grayscale,
///    halving the raw memory footprint.
///
/// # Errors
///
/// Returns an error if the input bytes cannot be decoded as a supported
/// image format.
pub fn process_image(
    raw_bytes: &[u8],
    max_height: u32,
) -> Result<DynamicImage, image::ImageError> {
    let img = image::load_from_memory(raw_bytes)?;

    // Proportional downscale when the source exceeds the height ceiling
    let scaled = if img.height() > max_height {
        let ratio = max_height as f32 / img.height() as f32;
        let target_width = (img.width() as f32 * ratio).round() as u32;
        img.resize_exact(target_width, max_height, FilterType::Triangle)
    } else {
        img
    };

    // Force desaturation to single-channel grayscale
    Ok(DynamicImage::ImageLuma8(scaled.into_luma8()))
}
