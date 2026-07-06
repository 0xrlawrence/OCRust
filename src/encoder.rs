use image::DynamicImage;

#[cfg(not(target_arch = "wasm32"))]
use webp::{Encoder, WebPConfig};

/// Encodes a `DynamicImage` into a WebP byte vector.
///
/// Under desktop/NDK targets, it uses the `webp` crate (FFI bindings to Google's `libwebp`) for lossy encoding.
/// Under wasm32 targets, it compiles out-of-the-box using the pure-Rust `image` crate WebP encoder.
///
/// # Errors
///
/// Returns an error string if the encoder cannot be constructed from the
/// provided image.
#[cfg(not(target_arch = "wasm32"))]
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

/// Fallback WebP encoder implementation for wasm32 using pure Rust image codecs.
#[cfg(target_arch = "wasm32")]
pub fn encode_webp(
    image: &DynamicImage,
    _quality: f32,
) -> Result<Vec<u8>, String> {
    use image::codecs::webp::WebPEncoder;
    let mut buffer = Vec::new();
    let encoder = WebPEncoder::new_lossless(&mut buffer);
    image.write_with_encoder(encoder)
        .map_err(|e| format!("Wasm WebP encoding failed: {}", e))?;
    Ok(buffer)
}
