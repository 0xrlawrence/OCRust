//! Desktop example: compress any JPEG or PNG file through the native pipeline.
//!
//! This bypasses the JNI layer and runs the same Rust compression engine
//! directly on your development machine, so you can verify output quality
//! and file sizes before deploying to Android.
//!
//! Produces both a `.webp` file and an `.ocrust` file (the custom format
//! that bundles compressed image data with structured metadata for AI agents).
//!
//! Usage:
//!     cargo run --example compress_sample -- <input_image> [max_height] [quality]
//!
//! Examples:
//!     cargo run --example compress_sample -- photo.png
//!     cargo run --example compress_sample -- screenshot.jpg 720 30

use ocrust::compressor::process_image;
use ocrust::encoder::encode_webp;
use ocrust::format::{self, OcrustMetadata, OutputInfo, SourceInfo};
use std::env;
use std::fs;
use std::path::Path;
use std::process;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: compress_sample <input_image> [max_height] [quality]");
        eprintln!();
        eprintln!("Arguments:");
        eprintln!("  input_image   Path to a JPEG or PNG file");
        eprintln!("  max_height    Maximum output height in pixels (default: 1080)");
        eprintln!("  quality       WebP lossy quality 0-100 (default: 50)");
        process::exit(1);
    }

    let input_path = &args[1];
    let max_height: u32 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(1080);
    let quality: f32 = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(50.0);

    // Read input file
    let raw_bytes = match fs::read(input_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Failed to read '{}': {}", input_path, e);
            process::exit(1);
        }
    };

    let input_size = raw_bytes.len();
    println!("Input:       {} ({} bytes)", input_path, input_size);
    println!("Max height:  {}px", max_height);
    println!("Quality:     {}%", quality);
    println!();

    // Run the compression pipeline with timing
    let start = Instant::now();

    let grayscale_img = match process_image(&raw_bytes, max_height) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Image processing failed: {}", e);
            process::exit(1);
        }
    };

    let webp_bytes = match encode_webp(&grayscale_img, quality) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("WebP encoding failed: {}", e);
            process::exit(1);
        }
    };

    let elapsed = start.elapsed();
    let output_size = webp_bytes.len();

    // Build output filenames
    let stem = Path::new(input_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = Path::new(input_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let webp_path = format!("{}_compressed.webp", stem);
    let ocrust_path = format!("{}_compressed.ocrust", stem);

    // Write the raw WebP file
    if let Err(e) = fs::write(&webp_path, &webp_bytes) {
        eprintln!("Failed to write '{}': {}", webp_path, e);
        process::exit(1);
    }

    // Detect source dimensions from the decoded image
    let source_img = image::load_from_memory(&raw_bytes).ok();
    let (src_w, src_h) = source_img
        .as_ref()
        .map(|img| (img.width(), img.height()))
        .unwrap_or((0, 0));

    // Run native OCR if on macOS to capture the image context/text
    println!("Extracting text content via native macOS Vision framework...");
    let extracted_text = perform_macos_ocr(input_path);
    if let Some(ref text) = extracted_text {
        println!("Successfully extracted text ({} chars)", text.len());
    } else {
        println!("No text detected or OCR not supported on this platform.");
    }

    // Build .ocrust metadata
    let metadata = OcrustMetadata {
        version: format::FORMAT_VERSION,
        timestamp: Some(chrono::Utc::now().to_rfc3339()),
        source: SourceInfo {
            width: src_w,
            height: src_h,
            format: Some(ext.to_string()),
            size_bytes: Some(input_size as u64),
        },
        output: OutputInfo {
            width: grayscale_img.width(),
            height: grayscale_img.height(),
            quality: quality as u32,
            size_bytes: output_size as u64,
        },
        text: extracted_text,
        context: None,
    };

    // Encode and write the .ocrust file
    let mut ocrust_file = match fs::File::create(&ocrust_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create '{}': {}", ocrust_path, e);
            process::exit(1);
        }
    };
    if let Err(e) = format::encode(&mut ocrust_file, &metadata, &webp_bytes) {
        eprintln!("Failed to write .ocrust file: {}", e);
        process::exit(1);
    }

    let ocrust_size = fs::metadata(&ocrust_path)
        .map(|m| m.len())
        .unwrap_or(0);

    // Report results
    let ratio = if input_size > 0 {
        (1.0 - (output_size as f64 / input_size as f64)) * 100.0
    } else {
        0.0
    };

    println!("WebP:        {} ({} bytes)", webp_path, output_size);
    println!(".ocrust:     {} ({} bytes)", ocrust_path, ocrust_size);
    println!("Reduction:   {:.1}%", ratio);
    println!("Time:        {:.2?}", elapsed);
}

#[cfg(target_os = "macos")]
fn perform_macos_ocr(image_path: &str) -> Option<String> {
    use std::process::Command;

    let swift_code = r#"
import Foundation
import Vision
import AppKit

guard CommandLine.arguments.count > 1 else {
    exit(1)
}

let imagePath = CommandLine.arguments[1]
guard let image = NSImage(contentsOfFile: imagePath) else {
    exit(1)
}

guard let cgImage = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
    exit(1)
}

let requestHandler = VNImageRequestHandler(cgImage: cgImage, options: [:])
let request = VNRecognizeTextRequest { request, error in
    guard let observations = request.results as? [VNRecognizedTextObservation] else { return }
    let recognizedStrings = observations.compactMap { observation in
        observation.topCandidates(1).first?.string
    }
    print(recognizedStrings.joined(separator: "\n"))
}

do {
    try requestHandler.perform([request])
} catch {
    exit(1)
}
"#;

    let temp_swift_path = "temp_ocr.swift";
    if std::fs::write(temp_swift_path, swift_code).is_err() {
        return None;
    }

    let output = Command::new("swift")
        .arg(temp_swift_path)
        .arg(image_path)
        .output();

    let _ = std::fs::remove_file(temp_swift_path);

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

#[cfg(not(target_os = "macos"))]
fn perform_macos_ocr(_image_path: &str) -> Option<String> {
    None
}

