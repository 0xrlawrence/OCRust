use image::{DynamicImage, RgbImage};
use ocrust::compressor::process_image;
use ocrust::encoder::encode_webp;
use std::io::Cursor;

/// Generates a synthetic 1920x1080 test image with color gradient blocks
/// and returns it as a JPEG byte vector (simulating what Android sends
/// over the JNI bridge).
fn generate_test_jpeg(width: u32, height: u32, _jpeg_quality: u8) -> Vec<u8> {
    let mut img = RgbImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            // Create a gradient pattern with varying color blocks
            let r = ((x as f32 / width as f32) * 255.0) as u8;
            let g = ((y as f32 / height as f32) * 255.0) as u8;
            let b = (((x + y) as f32 / (width + height) as f32) * 255.0) as u8;
            img.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }

    let dynamic = DynamicImage::ImageRgb8(img);
    let mut buffer = Cursor::new(Vec::new());
    dynamic
        .write_to(&mut buffer, image::ImageFormat::Jpeg)
        .expect("JPEG encoding should not fail for a valid RGB image");
    buffer.into_inner()
}

#[test]
fn full_pipeline_produces_smaller_output() {
    let input_jpeg = generate_test_jpeg(1920, 1080, 90);
    let input_size = input_jpeg.len();

    // Run through the same pipeline Android would use
    let processed = process_image(&input_jpeg, 1080)
        .expect("process_image should succeed on valid JPEG input");
    let webp_output = encode_webp(&processed, 50.0)
        .expect("encode_webp should succeed on a valid DynamicImage");

    let output_size = webp_output.len();

    assert!(
        output_size < input_size,
        "WebP output ({} bytes) should be smaller than JPEG input ({} bytes)",
        output_size,
        input_size,
    );

    // Verify the output is a valid WebP file by checking the RIFF header
    assert!(
        webp_output.len() >= 12,
        "WebP output is too small to contain a valid header",
    );
    assert_eq!(
        &webp_output[0..4],
        b"RIFF",
        "WebP output should start with RIFF header",
    );
    assert_eq!(
        &webp_output[8..12],
        b"WEBP",
        "WebP output should contain WEBP signature at offset 8",
    );
}

#[test]
fn downscale_reduces_height() {
    let input_jpeg = generate_test_jpeg(1080, 1920, 90);

    let processed = process_image(&input_jpeg, 720)
        .expect("process_image should succeed on valid JPEG input");

    assert_eq!(
        processed.height(),
        720,
        "Output height should match the requested max_height",
    );

    // Width should be proportionally scaled
    let expected_width = (1080.0_f64 * (720.0_f64 / 1920.0_f64)).round() as u32;
    assert_eq!(
        processed.width(),
        expected_width,
        "Output width should be proportionally scaled",
    );
}

#[test]
fn images_below_max_height_are_not_upscaled() {
    let input_jpeg = generate_test_jpeg(640, 480, 90);

    let processed = process_image(&input_jpeg, 1080)
        .expect("process_image should succeed on valid JPEG input");

    assert_eq!(
        processed.height(),
        480,
        "Images shorter than max_height should not be resized",
    );
    assert_eq!(
        processed.width(),
        640,
        "Images shorter than max_height should keep original width",
    );
}

#[test]
fn output_is_grayscale() {
    let input_jpeg = generate_test_jpeg(800, 600, 90);

    let processed = process_image(&input_jpeg, 1080)
        .expect("process_image should succeed on valid JPEG input");

    // The processed image should be Luma8 (single-channel grayscale)
    match processed {
        DynamicImage::ImageLuma8(_) => {} // expected
        other => panic!(
            "Expected ImageLuma8 grayscale output, got {:?}",
            other.color(),
        ),
    }
}

#[test]
fn quality_affects_output_size() {
    let input_jpeg = generate_test_jpeg(1920, 1080, 90);

    let processed = process_image(&input_jpeg, 1080)
        .expect("process_image should succeed on valid JPEG input");

    let low_quality = encode_webp(&processed, 20.0)
        .expect("encode_webp at quality 20 should succeed");
    let high_quality = encode_webp(&processed, 90.0)
        .expect("encode_webp at quality 90 should succeed");

    assert!(
        low_quality.len() < high_quality.len(),
        "Lower quality ({} bytes) should produce smaller output than higher quality ({} bytes)",
        low_quality.len(),
        high_quality.len(),
    );
}

#[test]
fn invalid_input_returns_error() {
    let garbage = vec![0u8, 1, 2, 3, 4, 5];

    let result = process_image(&garbage, 1080);
    assert!(
        result.is_err(),
        "process_image should return Err for invalid image data",
    );
}

#[test]
fn ocrust_roundtrip_succeeds() {
    use ocrust::format::{
        self, ContextInfo, OcrustMetadata, OutputInfo, SourceInfo,
    };

    let original_image = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let metadata = OcrustMetadata {
        version: format::FORMAT_VERSION,
        timestamp: Some("2026-07-06T12:00:00Z".to_string()),
        source: SourceInfo {
            width: 1920,
            height: 1080,
            format: Some("png".to_string()),
            size_bytes: Some(1024 * 1024),
        },
        output: OutputInfo {
            width: 1280,
            height: 720,
            quality: 50,
            size_bytes: 8,
        },
        text: Some("Detected screen text".to_string()),
        context: Some(ContextInfo {
            device: Some("Test Device".to_string()),
            app: Some("com.test.app".to_string()),
            os_version: Some("Android 14".to_string()),
        }),
        simhash: Some(format::calculate_simhash("Detected screen text")),
        embedding: Some(vec![0.1, 0.2, 0.3]),
    };

    // Encode
    let encoded = format::encode_to_string(&metadata, &original_image)
        .expect("Encoding should succeed");

    assert!(encoded.starts_with('{'), "Encoded output must be a JSON object");
    assert!(encoded.contains("\"image\":\"data:image/webp;base64,"), "Should contain base64 image data URL");

    // Decode full
    let mut decode_cursor = std::io::Cursor::new(encoded.as_bytes());
    let decoded = format::decode(&mut decode_cursor)
        .expect("Decoding should succeed");

    assert_eq!(decoded.metadata, metadata);
    assert_eq!(decoded.image_data, original_image);

    // Decode metadata only
    let mut meta_cursor = std::io::Cursor::new(encoded.as_bytes());
    let decoded_meta = format::decode_metadata(&mut meta_cursor)
        .expect("Decoding metadata only should succeed");

    assert_eq!(decoded_meta, metadata);
}

#[test]
fn ocrust_invalid_json_fails() {
    use ocrust::format;

    let bad_data = b"not a json structure";
    let mut cursor = std::io::Cursor::new(bad_data);
    let result = format::decode(&mut cursor);

    assert!(result.is_err());
}

#[test]
fn simhash_similarity_test() {
    use ocrust::format::{calculate_simhash, simhash_similarity};

    let text1 = "best project management tools Monday.com ClickUp Wrike";
    let text2 = "best project management tools Monday.com ClickUp Wrike extra";
    let text3 = "completely different text about programming rust code";

    let hash1 = calculate_simhash(text1);
    let hash2 = calculate_simhash(text2);
    let hash3 = calculate_simhash(text3);

    let sim_1_2 = simhash_similarity(&hash1, &hash2).unwrap();
    let sim_1_3 = simhash_similarity(&hash1, &hash3).unwrap();

    assert!(sim_1_2 > 0.8, "Similar text should have high similarity: {}", sim_1_2);
    assert!(sim_1_3 < 0.6, "Different text should have low similarity: {}", sim_1_3);
}



