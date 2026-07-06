pub mod compressor;
pub mod encoder;
pub mod format;

use jni::errors::ThrowRuntimeExAndDefault;
use jni::objects::{JByteArray, JClass};
use jni::sys::jint;
use jni::{Env, EnvUnowned};

use crate::compressor::process_image;
use crate::encoder::encode_webp;

/// JNI entry point called from `ScreenCompressor.java`.
///
/// Accepts a raw image byte array (JPEG-encoded) from the Android layer,
/// applies downscaling and grayscale conversion, then returns a lossy
/// WebP-encoded byte array back across the JNI bridge.
///
/// # Arguments
///
/// * `input_bytes` - JPEG-encoded image bytes from `Bitmap.compress()`.
/// * `max_height`  - Maximum pixel height for the output. Images taller
///                   than this value are proportionally downscaled.
/// * `quality`     - WebP lossy compression quality (0-100). Lower values
///                   yield smaller files with more artifact degradation.
///
/// # Returns
///
/// A `jbyteArray` containing the WebP-encoded output, or an empty array
/// on failure. Errors are thrown as `java.lang.RuntimeException` on the
/// Java side.
#[no_mangle]
pub extern "system" fn Java_com_rfx_compressor_ScreenCompressor_compressScreenNative<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
    input_bytes: JByteArray<'local>,
    max_height: jint,
    quality: jint,
) -> JByteArray<'local> {
    unowned_env
        .with_env(|env| -> jni::errors::Result<JByteArray> {
            compress_inner(env, &input_bytes, max_height, quality)
                .map_err(|e| jni::errors::Error::ParseFailed(e.to_string()))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

/// Inner processing function that returns a `Result` so error handling
/// stays clean and does not require early-return gymnastics.
fn compress_inner<'local>(
    env: &mut Env<'local>,
    input_bytes: &JByteArray<'local>,
    max_height: jint,
    quality: jint,
) -> Result<JByteArray<'local>, Box<dyn std::error::Error>> {
    // 1. Unmarshal the Java byte array into owned Rust memory
    let raw_pixels = env.convert_byte_array(input_bytes)?;

    // 2. Decode, downscale, and convert to grayscale
    let grayscale_img = process_image(&raw_pixels, max_height as u32)?;

    // 3. Encode the processed image as lossy WebP
    let webp_bytes = encode_webp(&grayscale_img, quality as f32)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    // 4. Push the compressed bytes back across the JNI bridge
    let output_array = env.new_byte_array(webp_bytes.len())?;
    output_array.set_region(env, 0, bytemuck::cast_slice(&webp_bytes))?;

    Ok(output_array)
}
