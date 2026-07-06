//! Desktop tool: read, inspect, and extract `.ocrust` container files.
//!
//! Usage:
//!     cargo run --example read_ocrust -- <input_ocrust_file> [extract_image_path]
//!
//! Examples:
//!     cargo run --example read_ocrust -- test2_compressed.ocrust
//!     cargo run --example read_ocrust -- test2_compressed.ocrust extracted.webp

use ocrust::format;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: read_ocrust <input_ocrust_file> [extract_image_path]");
        eprintln!();
        eprintln!("Arguments:");
        eprintln!("  input_ocrust_file   Path to a .ocrust file to read");
        eprintln!("  extract_image_path  Optional path where the embedded WebP image should be written");
        process::exit(1);
    }

    let input_path = &args[1];
    let extract_path = args.get(2);

    // Open file
    let mut file = match fs::File::open(input_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open file '{}': {}", input_path, e);
            process::exit(1);
        }
    };

    println!("Inspecting .ocrust container: {}", input_path);
    println!("--------------------------------------------------");

    // If extract path is not specified, we can use the fast-path decode_metadata
    if extract_path.is_none() {
        println!("Performing fast-path metadata decode (no image read)...");
        match format::decode_metadata(&mut file) {
            Ok(metadata) => {
                print_metadata(&metadata);
                println!("\nTo extract the WebP image, provide an output path:");
                println!("  cargo run --example read_ocrust -- {} output.webp", input_path);
            }
            Err(e) => {
                eprintln!("Failed to decode metadata: {}", e);
                process::exit(1);
            }
        }
    } else {
        let output_img_path = extract_path.unwrap();
        println!("Performing full decode (extracting metadata + image payload)...");
        match format::decode(&mut file) {
            Ok(record) => {
                print_metadata(&record.metadata);

                if record.image_data.is_empty() {
                    println!("\nNo embedded image payload found in this container.");
                } else {
                    println!("\nFound embedded image payload ({} bytes).", record.image_data.len());
                    match fs::write(output_img_path, &record.image_data) {
                        Ok(_) => {
                            println!("Successfully extracted WebP image to: {}", output_img_path);
                        }
                        Err(e) => {
                            eprintln!("Failed to write extracted image to '{}': {}", output_img_path, e);
                            process::exit(1);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to decode container: {}", e);
                process::exit(1);
            }
        }
    }
}

fn print_metadata(metadata: &format::OcrustMetadata) {
    println!("\nMetadata (JSON):");
    match serde_json::to_string_pretty(metadata) {
        Ok(json) => println!("{}", json),
        Err(e) => println!("Error formatting JSON: {}", e),
    }
}
