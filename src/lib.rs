pub mod compressor;
pub mod encoder;
pub mod format;

use crate::compressor::process_image;
use crate::encoder::encode_webp;
use crate::format::{
    calculate_simhash, decode, decode_metadata, encode_to_string, OcrustMetadata, OutputInfo,
    SourceInfo, ContextInfo, OcrustBlock,
};

uniffi::setup_scaffolding!();

#[derive(Debug)]
pub enum OcrustError {
    CompressionError,
    DecodingError,
    InvalidJson,
}

impl std::error::Error for OcrustError {}

impl std::fmt::Display for OcrustError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CompressionError => write!(f, "Compression failed"),
            Self::DecodingError => write!(f, "Decoding failed"),
            Self::InvalidJson => write!(f, "Invalid JSON format"),
        }
    }
}

pub fn compress_screen(
    input_bytes: Vec<u8>,
    max_height: u32,
    quality: u32,
    bitonal: bool,
) -> Result<Vec<u8>, OcrustError> {
    let grayscale_img = process_image(&input_bytes, max_height, bitonal)
        .map_err(|_| OcrustError::CompressionError)?;

    let webp_bytes = encode_webp(&grayscale_img, quality as f32)
        .map_err(|_| OcrustError::CompressionError)?;

    Ok(webp_bytes)
}

pub fn compress_screen_to_ocrust(
    input_bytes: Vec<u8>,
    max_height: u32,
    quality: u32,
    bitonal: bool,
    text: Option<String>,
    device: Option<String>,
    app: Option<String>,
    os_version: Option<String>,
    embedding: Option<Vec<f32>>,
    blocks: Option<Vec<OcrustBlock>>,
) -> Result<String, OcrustError> {
    let webp_bytes = compress_screen(input_bytes.clone(), max_height, quality, bitonal)?;

    // Detect source dimensions from input bytes
    let source_img = image::load_from_memory(&input_bytes)
        .map_err(|_| OcrustError::CompressionError)?;
    let src_w = source_img.width();
    let src_h = source_img.height();

    // Calculate final dimensions
    let (out_h, out_w) = if src_h > max_height {
        let ratio = max_height as f32 / src_h as f32;
        (max_height, (src_w as f32 * ratio).round() as u32)
    } else {
        (src_h, src_w)
    };

    // Calculate SimHash for semantic search
    let simhash = text.as_deref().map(calculate_simhash);

    let metadata = OcrustMetadata {
        version: format::FORMAT_VERSION,
        timestamp: Some(chrono::Utc::now().to_rfc3339()),
        source: SourceInfo {
            width: src_w,
            height: src_h,
            format: None,
            size_bytes: Some(input_bytes.len() as u64),
        },
        output: OutputInfo {
            width: out_w,
            height: out_h,
            quality,
            size_bytes: webp_bytes.len() as u64,
        },
        text,
        context: Some(ContextInfo {
            device,
            app,
            os_version,
        }),
        simhash,
        embedding,
        blocks,
    };

    encode_to_string(&metadata, &webp_bytes).map_err(|_| OcrustError::CompressionError)
}

pub fn decode_ocrust_text(ocrust_json: String) -> Result<Option<String>, OcrustError> {
    let mut cursor = std::io::Cursor::new(ocrust_json.as_bytes());
    let metadata = decode_metadata(&mut cursor).map_err(|_| OcrustError::InvalidJson)?;
    Ok(metadata.text)
}

pub fn decode_ocrust_image(ocrust_json: String) -> Result<Vec<u8>, OcrustError> {
    let mut cursor = std::io::Cursor::new(ocrust_json.as_bytes());
    let record = decode(&mut cursor).map_err(|_| OcrustError::InvalidJson)?;
    Ok(record.image_data)
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compress_screen_wasm(
    input_bytes: Vec<u8>,
    max_height: u32,
    quality: u32,
    bitonal: bool,
) -> Result<Vec<u8>, JsValue> {
    compress_screen(input_bytes, max_height, quality, bitonal)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compress_screen_to_ocrust_wasm(
    input_bytes: Vec<u8>,
    max_height: u32,
    quality: u32,
    bitonal: bool,
    text: Option<String>,
    device: Option<String>,
    app: Option<String>,
    os_version: Option<String>,
) -> Result<String, JsValue> {
    compress_screen_to_ocrust(
        input_bytes,
        max_height,
        quality,
        bitonal,
        text,
        device,
        app,
        os_version,
        None,
        None,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn decode_ocrust_text_wasm(ocrust_json: String) -> Result<Option<String>, JsValue> {
    decode_ocrust_text(ocrust_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn decode_ocrust_image_wasm(ocrust_json: String) -> Result<Vec<u8>, JsValue> {
    decode_ocrust_image(ocrust_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn calculate_simhash_wasm(text: String) -> String {
    format::calculate_simhash(&text)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn simhash_similarity_wasm(hash1: String, hash2: String) -> Result<f32, JsValue> {
    format::simhash_similarity(&hash1, &hash2)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

