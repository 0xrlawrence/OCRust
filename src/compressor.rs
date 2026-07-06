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
    bitonal: bool,
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

    let mut grayscale = scaled.into_luma8();

    if bitonal {
        apply_otsu_threshold(&mut grayscale);
    }

    // Force desaturation to single-channel grayscale
    Ok(DynamicImage::ImageLuma8(grayscale))
}

/// Applies Otsu's binarization thresholding in place to convert a grayscale image
/// to a 1-bit black and white (bitonal) image.
fn apply_otsu_threshold(img: &mut image::GrayImage) {
    let threshold = otsu_threshold(img);
    for pixel in img.pixels_mut() {
        pixel[0] = if pixel[0] < threshold { 0 } else { 255 };
    }
}

/// Computes the optimal threshold value using Otsu's method.
fn otsu_threshold(img: &image::GrayImage) -> u8 {
    let mut histogram = [0u32; 256];
    for pixel in img.pixels() {
        histogram[pixel[0] as usize] += 1;
    }

    let total = img.width() * img.height();
    let mut sum: f32 = 0.0;
    for i in 0..256 {
        sum += i as f32 * histogram[i] as f32;
    }

    let mut sum_b: f32 = 0.0;
    let mut w_b: u32 = 0;

    let mut var_max: f32 = 0.0;
    let mut threshold: u8 = 128;

    for i in 0..256 {
        w_b += histogram[i];
        if w_b == 0 {
            continue;
        }
        let w_f = total - w_b;
        if w_f == 0 {
            break;
        }

        sum_b += i as f32 * histogram[i] as f32;

        let m_b = sum_b / w_b as f32;
        let m_f = (sum - sum_b) / w_f as f32;

        // Calculate Between Class Variance
        let var_between = w_b as f32 * w_f as f32 * (m_b - m_f) * (m_b - m_f);

        if var_between > var_max {
            var_max = var_between;
            threshold = i as u8;
        }
    }

    threshold
}
