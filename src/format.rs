//! `.ocrust` format: Pure JSON Optimized Capture Record for AI agents.
//!
//! A developer-friendly JSON container that packages a compressed WebP image
//! (as a Base64-encoded data URL) with structured metadata and pre-extracted
//! text content. Easy to read in any text editor and directly consumable
//! by web browsers and LLM APIs.

use base64::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

/// Current format version.
pub const FORMAT_VERSION: u8 = 1;

/// Prefix used for the Base64 WebP image data URL.
pub const IMAGE_DATA_PREFIX: &str = "data:image/webp;base64,";

/// Metadata describing the source image before compression.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceInfo {
    /// Original width in pixels.
    pub width: u32,
    /// Original height in pixels.
    pub height: u32,
    /// Original image format (e.g., "png", "jpeg").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Original file size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

/// Metadata describing the compressed output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutputInfo {
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// WebP lossy quality setting used (0-100).
    pub quality: u32,
    /// Compressed image size in bytes.
    pub size_bytes: u64,
}

/// Optional device and application context.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ContextInfo {
    /// Device model name (e.g., "Galaxy S24 Ultra").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    /// Foreground application package name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    /// Operating system version string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
}

/// A complete `.ocrust` record containing metadata and the Base64 WebP image.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrustRecord {
    /// Format version.
    pub version: u8,
    /// ISO 8601 timestamp of when the capture was taken.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    /// Information about the source image before processing.
    pub source: SourceInfo,
    /// Information about the compressed output.
    pub output: OutputInfo,
    /// Pre-extracted text content from the screen capture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Optional device and application context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ContextInfo>,
    /// Lightweight 64-bit semantic SimHash signature (16-char hex string).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simhash: Option<String>,
    /// Optional high-dimensional vector embedding for semantic search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    /// Base64-encoded WebP image payload with the "data:image/webp;base64," prefix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

/// Lightweight structure representing the `.ocrust` record without the large
/// image payload. Used by fast-path deserializers to avoid parsing and
/// allocating the large Base64 image string.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrustMetadata {
    pub version: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    pub source: SourceInfo,
    pub output: OutputInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ContextInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simhash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

/// Helper structure for decoding a complete record and separating binary image data.
#[derive(Debug, Clone)]
pub struct DecodedRecord {
    /// Parsed metadata elements.
    pub metadata: OcrustMetadata,
    /// Decoded raw binary WebP image payload.
    pub image_data: Vec<u8>,
}

/// Encodes the metadata and image bytes into a formatted JSON string.
///
/// The image bytes are automatically Base64-encoded and prefixed as a data URL.
pub fn encode_to_string(
    metadata: &OcrustMetadata,
    image_bytes: &[u8],
) -> Result<String, serde_json::Error> {
    let base64_image = BASE64_STANDARD.encode(image_bytes);
    let data_url = format!("{}{}", IMAGE_DATA_PREFIX, base64_image);

    let record = OcrustRecord {
        version: metadata.version,
        timestamp: metadata.timestamp.clone(),
        source: metadata.source.clone(),
        output: metadata.output.clone(),
        text: metadata.text.clone(),
        context: metadata.context.clone(),
        simhash: metadata.simhash.clone(),
        embedding: metadata.embedding.clone(),
        image: Some(data_url),
    };

    serde_json::to_string(&record)
}

/// Encodes and writes the `.ocrust` JSON structure directly to a `Write` stream.
pub fn encode<W: Write>(
    writer: &mut W,
    metadata: &OcrustMetadata,
    image_bytes: &[u8],
) -> io::Result<()> {
    let json_string = encode_to_string(metadata, image_bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writer.write_all(json_string.as_bytes())?;
    writer.flush()
}

/// Decodes only the metadata section from an `.ocrust` JSON reader,
/// ignoring the large Base64 image payload. This is the fast path for
/// text-only AI agents.
pub fn decode_metadata<R: Read>(reader: &mut R) -> io::Result<OcrustMetadata> {
    serde_json::from_reader(reader)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Decodes the complete `.ocrust` record, parsing the JSON and extracting
/// the Base64 image payload back into raw binary bytes.
pub fn decode<R: Read>(reader: &mut R) -> io::Result<DecodedRecord> {
    let record: OcrustRecord = serde_json::from_reader(reader)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let image_data = if let Some(image_url) = record.image {
        if !image_url.starts_with(IMAGE_DATA_PREFIX) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid image format prefix in .ocrust file",
            ));
        }
        let base64_part = &image_url[IMAGE_DATA_PREFIX.len()..];
        BASE64_STANDARD
            .decode(base64_part)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
    } else {
        Vec::new()
    };

    let metadata = OcrustMetadata {
        version: record.version,
        timestamp: record.timestamp,
        source: record.source,
        output: record.output,
        text: record.text,
        context: record.context,
        simhash: record.simhash,
        embedding: record.embedding,
    };

    Ok(DecodedRecord {
        metadata,
        image_data,
    })
}

/// Calculates standard FNV-1a 64-bit hash.
fn fnv1a_hash(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in s.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Computes a 64-bit SimHash (semantic signature) for the given text.
/// Returns a 16-character hexadecimal string.
pub fn calculate_simhash(text: &str) -> String {
    let mut v = [0i32; 64];

    for word in text.split_whitespace() {
        // Clean word to ignore case, punctuation, and non-alphanumeric chars
        let clean_word: String = word
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(|c| c.to_lowercase())
            .collect();

        if clean_word.is_empty() {
            continue;
        }

        let hash = fnv1a_hash(&clean_word);
        for i in 0..64 {
            let bit = (hash >> i) & 1;
            if bit == 1 {
                v[i] += 1;
            } else {
                v[i] -= 1;
            }
        }
    }

    let mut fingerprint: u64 = 0;
    for i in 0..64 {
        if v[i] > 0 {
            fingerprint |= 1 << i;
        }
    }

    format!("{:016x}", fingerprint)
}

/// Helper function to calculate Hamming distance (number of differing bits)
/// between two 64-bit hex SimHash signatures.
pub fn simhash_distance(hash1: &str, hash2: &str) -> Result<u32, String> {
    let u1 = u64::from_str_radix(hash1, 16).map_err(|e| e.to_string())?;
    let u2 = u64::from_str_radix(hash2, 16).map_err(|e| e.to_string())?;
    Ok((u1 ^ u2).count_ones())
}

/// Helper function to calculate similarity (0.0 to 1.0) based on Hamming distance.
pub fn simhash_similarity(hash1: &str, hash2: &str) -> Result<f32, String> {
    let dist = simhash_distance(hash1, hash2)?;
    Ok(1.0 - (dist as f32 / 64.0))
}

