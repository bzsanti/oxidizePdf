//! Diagnose PDF parsing failures from .private/oxidizepdf-failures/

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::Path;

fn main() {
    let failures_dir = Path::new("../.private/oxidizepdf-failures");

    if !failures_dir.exists() {
        eprintln!("Directory not found: {:?}", failures_dir);
        return;
    }

    let mut error_categories: HashMap<String, Vec<String>> = HashMap::new();
    let mut success_count = 0;
    let mut total_count = 0;

    // Get all PDF files
    let mut pdf_files: Vec<_> = fs::read_dir(failures_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "pdf"))
        .collect();

    pdf_files.sort_by_key(|e| e.path());

    println!("=== Diagnosing {} PDFs ===\n", pdf_files.len());

    for entry in pdf_files {
        let path = entry.path();
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        total_count += 1;

        // Read JSON for original filename
        let json_path = path.with_extension("json");
        let original_name = if json_path.exists() {
            if let Ok(json_str) = fs::read_to_string(&json_path) {
                serde_json::from_str::<serde_json::Value>(&json_str)
                    .ok()
                    .and_then(|v| v["FileName"].as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| filename.clone())
            } else {
                filename.clone()
            }
        } else {
            filename.clone()
        };

        // Try to parse the PDF
        let pdf_data = match fs::read(&path) {
            Ok(data) => data,
            Err(e) => {
                let error_key = format!("IO Error: {}", e);
                error_categories
                    .entry(error_key)
                    .or_default()
                    .push(original_name);
                continue;
            }
        };

        let cursor = Cursor::new(&pdf_data);
        match PdfReader::new(cursor) {
            Ok(reader) => {
                let document = PdfDocument::new(reader);
                match document.extract_text() {
                    Ok(pages) => {
                        let total_chars: usize = pages.iter().map(|p| p.text.len()).sum();
                        if total_chars > 0 {
                            success_count += 1;
                            println!("✓ {} - {} chars extracted", original_name, total_chars);
                        } else {
                            error_categories
                                .entry("Empty text extraction".to_string())
                                .or_default()
                                .push(original_name);
                        }
                    }
                    Err(e) => {
                        let error_key = categorize_error(&format!("{:?}", e));
                        error_categories
                            .entry(error_key)
                            .or_default()
                            .push(original_name);
                    }
                }
            }
            Err(e) => {
                let error_key = categorize_error(&format!("{:?}", e));
                error_categories
                    .entry(error_key)
                    .or_default()
                    .push(original_name);
            }
        }
    }

    // Print summary
    println!("\n=== SUMMARY ===");
    println!(
        "Total: {} | Success: {} | Failed: {}",
        total_count,
        success_count,
        total_count - success_count
    );
    println!("\n=== ERROR CATEGORIES ===\n");

    let mut categories: Vec<_> = error_categories.iter().collect();
    categories.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    for (category, files) in categories {
        println!("[{}] ({} files)", category, files.len());
        for file in files.iter().take(3) {
            println!("  - {}", file);
        }
        if files.len() > 3 {
            println!("  ... and {} more", files.len() - 3);
        }
        println!();
    }
}

fn categorize_error(error_str: &str) -> String {
    if error_str.contains("Encrypted") || error_str.contains("encrypted") {
        "Encrypted PDF".to_string()
    } else if error_str.contains("XRef") || error_str.contains("xref") {
        "XRef parsing error".to_string()
    } else if error_str.contains("decompress")
        || error_str.contains("Flate")
        || error_str.contains("zlib")
    {
        "Decompression error".to_string()
    } else if error_str.contains("Font") || error_str.contains("font") {
        "Font error".to_string()
    } else if error_str.contains("stream") {
        "Stream error".to_string()
    } else if error_str.contains("object") || error_str.contains("Object") {
        "Object error".to_string()
    } else if error_str.contains("Invalid") || error_str.contains("invalid") {
        "Invalid format".to_string()
    } else if error_str.contains("UTF") || error_str.contains("utf") {
        "UTF-8 encoding error".to_string()
    } else {
        // Take first 80 chars of error
        let truncated = if error_str.len() > 80 {
            format!("{}...", &error_str[..80])
        } else {
            error_str.to_string()
        };
        truncated
    }
}
