use image::DynamicImage;
use webp::{Encoder, WebPConfig};

/// Encodes a `DynamicImage` into a lossy WebP byte vector.
///
/// Uses the `webp` crate (FFI bindings to Google's `libwebp`) for encoding.
/// The `quality` parameter controls the lossy compression aggressiveness:
///
/// - `100.0` = maximum visual fidelity, larger file size.
/// -  `50.0` = good balance for screen capture archival.
/// -  `20.0` = aggressive compression, visible block artifacts.
///
/// Grayscale (Luma8) images are converted to RGB before encoding because
/// libwebp does not accept single-channel input. The visual output remains
/// grayscale since all three channels carry the same luminance value.
///
/// # Errors
///
/// Returns an error string if the encoder cannot be constructed from the
/// provided image.
pub fn encode_webp(
    image: &DynamicImage,
    quality: f32,
) -> Result<Vec<u8>, String> {
    // libwebp requires RGB or RGBA input. Convert Luma8 to RGB8 so the
    // encoder receives a supported pixel layout.
    let rgb_image = DynamicImage::ImageRgb8(image.to_rgb8());

    let encoder = Encoder::from_image(&rgb_image)
        .map_err(|e| format!("WebP encoder initialization failed: {}", e))?;

    // Enable advanced configuration with compression method = 6 (maximum compression effort)
    let mut config = WebPConfig::new()
        .map_err(|_| "Failed to initialize WebPConfig".to_string())?;
    config.quality = quality;
    config.method = 6; // 0 to 6 (slowest speed, smallest file size)

    let webp_memory = encoder.encode_advanced(&config)
        .map_err(|e| format!("WebP encoding failed: {:?}", e))?;

    Ok(webp_memory.to_vec())
}
