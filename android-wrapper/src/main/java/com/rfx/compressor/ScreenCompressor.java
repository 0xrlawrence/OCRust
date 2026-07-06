package com.rfx.compressor;

import android.graphics.Bitmap;
import java.io.ByteArrayOutputStream;

/**
 * High-level Java API for the native Rust screen compression engine.
 *
 * <p>This class loads the compiled {@code libocrust.so} binary
 * and exposes a single static method to convert Android Bitmaps into compact,
 * grayscale WebP byte arrays suitable for local storage or network transfer.</p>
 *
 * <h3>Usage</h3>
 * <pre>{@code
 * Bitmap screenshot = captureScreen();
 * byte[] compressed = ScreenCompressor.optimize(screenshot, 1080, 50);
 * // Write compressed bytes to disk, database, or socket
 * }</pre>
 */
public class ScreenCompressor {

    static {
        System.loadLibrary("ocrust");
    }

    /**
     * Native bridge method linking to the Rust JNI export.
     *
     * @param inputBytes  JPEG-encoded image byte array.
     * @param maxHeight   Maximum output height in pixels.
     * @param quality     WebP lossy quality (0-100).
     * @return WebP-encoded byte array.
     */
    private static native byte[] compressScreenNative(
            byte[] inputBytes, int maxHeight, int quality);

    /**
     * Converts an Android Bitmap into a compact, grayscale WebP byte array.
     *
     * <p>The bitmap is first encoded to JPEG at 90% quality as an intermediate
     * format to cross the JNI boundary. The native Rust engine then decodes it,
     * applies downscaling and grayscale conversion, and re-encodes it as lossy
     * WebP at the specified compression quality.</p>
     *
     * @param bitmap             Source bitmap to compress.
     * @param maxHeight          Maximum pixel height. Images taller than this
     *                           are proportionally downscaled.
     * @param compressionQuality WebP lossy quality (0-100). Values around 50
     *                           produce good results for screen capture archival.
     * @return Byte array containing the WebP-encoded image, or an empty array
     *         if processing fails.
     * @throws IllegalArgumentException if bitmap is null.
     */
    public static byte[] optimize(
            Bitmap bitmap, int maxHeight, int compressionQuality) {
        if (bitmap == null) {
            throw new IllegalArgumentException("Bitmap must not be null");
        }

        ByteArrayOutputStream stream = new ByteArrayOutputStream();
        bitmap.compress(Bitmap.CompressFormat.JPEG, 90, stream);
        byte[] byteArray = stream.toByteArray();

        return compressScreenNative(byteArray, maxHeight, compressionQuality);
    }
}
