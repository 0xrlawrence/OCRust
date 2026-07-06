# Native Screen Compressor & OCRust Container Format 🦀⚡

An ultra-lightweight, memory-safe Android NDK engine written in Rust. It compresses full-resolution device screenshots into **20KB–35KB** grayscale WebP images, extracts screen text, and bundles them into a developer-friendly `.ocrust` JSON container.

This engine is built specifically for **timeline capture utilities, offline AI memory engines, local logging, and background agents** where storage, network bandwidth, and fast AI consumption are critical.

---

## 📊 Performance Benchmarks
*Tested on Snapdragon 8 Gen 3 / Galaxy S24 Ultra*

| Metric | Original JPG | Direct WebP (Q50) | Our Engine (640px, Grayscale, Q20) |
|---|---|---|---|
| **File Size** | `189 KB` | `86.4 KB` | **`23.6 KB`** |
| **Container (`.ocrust`)** | — | — | **`33.8 KB`** (Includes 2KB extracted text) |
| **Size Reduction** | — | 54.3% | **87.5% (Image) / 82.1% (Container)** |
| **Processing Latency** | — | — | **~12ms** (Release NDK execution loop) |

### 🔍 Side-by-Side Compression Comparison
Here is how our custom pipeline (using `method = 6` compression effort) stacks up against standard WebP encoding options when converting a standard screenshot input:

| Strategy | Output Size | Reduction vs. Original | Purpose / Verdict |
|---|---|---|---|
| **Original JPG** | `189 KB` | — | Input screenshot |
| **1. Direct WebP** (Orig Size, Color, Q50) | `86.4 KB` | **54.3%** | Standard WebP conversion |
| **2. Direct WebP** (Orig Size, Color, Q20) | `61.0 KB` | **67.8%** | Low-quality standard WebP |
| **3. Direct WebP** (Orig Size, Grayscale, Q20) | `57.7 KB` | **69.5%** | Grayscale standard WebP |
| **4. Our WebP** (640px Height, Grayscale, Q20) | **`23.6 KB`** | **`87.5%`** | **Optimal engine configuration** |

---

## 🛠️ Quick Start (Desktop Test Drive)

Try the compression and extraction pipeline on your own machine in under a minute. No Android device or NDK required.

### 1. Clone and Build
Ensure you have the Rust toolchain installed, then run:
```bash
git clone https://github.com/0xrlawrence/OCRust.git
cd OCRust
cargo build --release
```

### 2. Compress and OCR an Image
Use the included desktop example to compress any image on your disk. On macOS, this will automatically run the native Vision framework to perform OCR and embed the text:
```bash
cargo run --example compress_sample -- test_screenshot.png 640 20
```
This generates two files in your current directory:
- `test_screenshot_compressed.webp` (the raw grayscale WebP)
- `test_screenshot_compressed.ocrust` (the JSON container file)

### 3. Read and Decode the `.ocrust` File
Inspect the generated `.ocrust` container with the inspection tool:
```bash
# Fast Path: Read metadata and extracted text only (no image decoding)
cargo run --example read_ocrust -- test_screenshot_compressed.ocrust

# Full Path: Extract the WebP image back to a binary file
cargo run --example read_ocrust -- test_screenshot_compressed.ocrust output.webp
```

### 4. Run the Automated Tests
Verify library behavior and roundtrip encoding:
```bash
cargo test
```

---

## 📦 Understanding the `.ocrust` Container

The `.ocrust` format is a **pure JSON file**. It stores the compressed WebP image as a Base64-encoded data URL, meaning it is 100% human-readable, editable in any text editor, and natively parseable in any programming language.

### Schema Structure
```json
{
  "version": 1,
  "timestamp": "2026-07-06T05:05:33.377680+00:00",
  "source": {
    "width": 1536,
    "height": 1024,
    "format": "jpg",
    "size_bytes": 189330
  },
  "output": {
    "width": 960,
    "height": 640,
    "quality": 20,
    "size_bytes": 23638
  },
  "text": "SEO\nGoogle\nbest project management tools...",
  "context": {
    "device": "Galaxy S24 Ultra",
    "app": "com.android.settings",
    "os_version": "Android 15"
  },
  "simhash": "2f65a1b3c9d8e7f0",
  "embedding": [0.0125, -0.0456, 0.1876],
  "image": "data:image/webp;base64,UklGRvBhAABXRUJQVlA4..."
}
```

### 🧠 Local Semantic Search via SimHash
Each `.ocrust` file contains a `"simhash"` signature—a 64-bit fingerprint of the screen text. This allows you to instantly determine if two screen captures are semantically similar without hitting heavy machine learning models or cloud embedding APIs:
* **Hamming Distance**: Count the number of differing bits between two SimHashes. If the distance is low (e.g., $\le 6$), the screens are highly similar.
* **On-Device Deduplication**: Allows timeline capture apps to drop redundant screenshots (e.g., if the user is looking at the same static page for minutes) by calculating distance between sequential captures locally.

---

## 🤖 How AI Agents & Backends Consume `.ocrust`

Because `.ocrust` is structured JSON, AI agents can ingest screen contexts in two ways depending on token budget and latency needs:

### A. Text-Only Interaction (Fast & Cheap)
If the agent only needs text to answer a query (bypassing expensive Vision API costs), it reads the `"text"` field and ignores the `"image"` field entirely:

```python
# Python Agent
import json

def get_screen_text(filepath):
    with open(filepath, 'r') as f:
        data = json.load(f)
    return data.get("text", "")

# Send get_screen_text("capture.ocrust") directly to standard LLM
```

### B. Multimodal Interaction (Vision + Text)
If the agent needs the full layout or visual context, it extracts the `"image"` string. The Base64 data URL matches standard LLM input payloads exactly, allowing it to be sent directly to OpenAI, Gemini, or Claude APIs without conversion:

```javascript
// Node.js Agent sending to OpenAI / Gemini
const fs = require('fs');

const payload = JSON.parse(fs.readFileSync('capture.ocrust', 'utf8'));

const llmResponse = await openai.chat.completions.create({
  model: "gpt-4o",
  messages: [
    {
      role: "user",
      content: [
        { type: "text", text: "Analyze the layout and click the settings button." },
        {
          type: "image_url",
          image_url: {
            url: payload.image // Direct inject of base64 data URL
          }
        }
      ]
    }
  ]
});
```

---

## 📱 Android App Integration Tutorial

To consume this engine in a production Android application, compile the native libraries and orchestrate screen captures using the following architecture:

### 1. Project Setup
UniFFI automatically generates the Kotlin interface file (e.g. `ocrust.kt`) and compiles the corresponding `.so` shared libraries.

Place the compiled `.so` files into your Android project structure:
```
app/src/main/jniLibs/
  ├── arm64-v8a/libocrust.so
  ├── armeabi-v7a/libocrust.so
  ├── x86_64/libocrust.so
  └── x86/libocrust.so
```

Include the generated `ocrust.kt` file in your Kotlin source tree (e.g. under package `uniffi.ocrust`).

### 2. Standard Capture & Packer Pipeline (Kotlin)
On Android, you capture the screen as a JPEG byte array, run Optical Character Recognition (OCR) using Google's **ML Kit**, and pass the text/metadata to `compressScreenToOcrust` to generate the complete `.ocrust` JSON string in a single step:

```kotlin
import android.graphics.Bitmap
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import uniffi.ocrust.compressScreenToOcrust
import java.io.ByteArrayOutputStream
import java.io.File

object CapturePipeline {

    private val recognizer = TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS)

    fun processAndSaveScreen(bitmap: Bitmap, outputFile: File, deviceModel: String, packageName: String) {
        val inputImage = InputImage.fromBitmap(bitmap, 0)
        
        // 1. Perform OCR in the background using Google ML Kit
        recognizer.process(inputImage)
            .addOnSuccessListener { visionText ->
                val extractedText = visionText.text // Text block extracted from image
                
                // Convert Bitmap to JPEG byte array to send to NDK
                val stream = ByteArrayOutputStream()
                bitmap.compress(Bitmap.CompressFormat.JPEG, 100, stream)
                val jpegBytes = stream.toByteArray()

                try {
                    // 2. Compress screenshot, calculate SimHash, and format JSON in one call.
                    // Automatically populates timestamp, width, height, and generates the 64-bit SimHash.
                    val ocrustJson = compressScreenToOcrust(
                        inputBytes = jpegBytes.toList(),
                        maxHeight = 640.toLong(),
                        quality = 20.toLong(),
                        text = extractedText,
                        device = deviceModel,
                        app = packageName,
                        osVersion = "Android 15"
                    )

                    // 3. Write .ocrust file directly to disk
                    outputFile.writeText(ocrustJson)
                } catch (e: Exception) {
                    e.printStackTrace()
                }
            }
            .addOnFailureListener { e ->
                e.printStackTrace()
            }
    }
}
```

---

## 🛠️ Cross-Compilation & Toolchains

The repository supports building native `.so` files out of the box.

### Local Compilation (Manual)
To build the `.so` files locally, install `cargo-ndk` and the Android targets:
```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
cargo install cargo-ndk

// Build all libraries
cargo ndk \
  -t arm64-v8a \
  -t armeabi-v7a \
  -t x86_64 \
  -t x86 \
  -o ./jniLibs \
  build --release
```

### CI/CD Pipeline (GitHub Actions)
The repository includes an automated build action in `.github/workflows/release.yml`. When you push a tag matching `v*`:
1. It spins up an environment with Rust and NDK toolchains.
2. Cross-compiles the dynamic libraries for `arm64-v8a`, `armeabi-v7a`, `x86_64`, and `x86`.
3. Compresses the `.so` binaries into a `.tar.gz` archive.
4. Automatically publishes a new GitHub Release with the compiled NDK binaries attached, eliminating the need for frontend developers to set up a local Rust compiler.

---

## 🔗 Dependencies

| Crate | Version | Purpose |
|---|---|---|
| [jni](https://crates.io/crates/jni) | 0.22 | JNI bindings for Kotlin/Java NDK bridge |
| [image](https://crates.io/crates/image) | 0.25 | Grayscale and bilinear downscaling pipeline |
| [webp](https://crates.io/crates/webp) | 0.3 | libwebp FFI wrapper for advanced lossy encoding |
| [serde](https://crates.io/crates/serde) | 1.0 | Serialization framework for metadata |
| [serde_json](https://crates.io/crates/serde_json) | 1.0 | Fast, minified JSON parsing & formatting |
| [base64](https://crates.io/crates/base64) | 0.22 | Safe Base64 conversions for standard JSON payloads |

---

## 📄 License
GNU GPLv3

---

## 🤖 LLM Prompt (Copy-Paste for AI Coders)

If you are using an AI coding assistant (like Claude, Gemini, or GPT) to write integrations, wrappers, or plugins for OCRust, copy and paste this system context block to help the model write correct code:

```markdown
You are an expert AI software engineer integrating the **OCRust** screen compression and capture format into our application.

### Key Facts:
1. **Purpose**: Crate `ocrust` is an Android NDK utility (written in Rust) that downscales screenshots, converts them to single-channel grayscale (halving raw memory), and encodes them as lossy WebP at quality 20 (compression method = 6) for maximum storage savings.
2. **JNI Signature**:
   `private static native byte[] compressScreenNative(byte[] inputBytes, int maxHeight, int quality);`
   - Loaded from native library: `"ocrust"` (outputs: `libocrust.so`)
   - Java API: `com.rfx.compressor.ScreenCompressor.optimize(Bitmap bitmap, int maxHeight, int quality)`
3. **Container Format (`.ocrust`)**:
   A pure JSON schema that packages metadata and the compressed image:
   ```json
   {
     "version": 1,
     "timestamp": "ISO 8601 string",
     "source": { "width": 1920, "height": 1080, "format": "png", "size_bytes": 1048576 },
     "output": { "width": 1280, "height": 720, "quality": 20, "size_bytes": 23638 },
     "text": "Pre-extracted text from OCR engine",
     "context": { "device": "Device Model", "app": "Package Name", "os_version": "Android 15" },
     "image": "data:image/webp;base64,<Base64 WebP bytes>"
   }
   ```
4. **Integration Task**:
   [Insert your custom task here, e.g., "Write a React Native bridge module", "Write a Python script to index .ocrust files in vector DB", or "Write a parser in Swift to decode it"].
```
```
