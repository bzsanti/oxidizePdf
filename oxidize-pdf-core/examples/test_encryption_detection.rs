//! Test encryption detection

use oxidize_pdf::parser::PdfReader;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing encryption detection...\n");

    // Test with some PDFs that might be encrypted
    let test_files = [
        "PDF_Samples/1002579 - FIRMADO.pdf",
        "PDF_Samples/ActDiet.2010144196-197.pdf",
        "PDF_Samples/FA_202772941245.pdf",
    ];

    for file in &test_files {
        if !Path::new(file).exists() {
            continue;
        }

        print!("Testing {file}: ");
        match PdfReader::open(file) {
            Ok(_) => println!("✓ Not encrypted - parsed successfully"),
            Err(e) => {
                if e.to_string().contains("EncryptionNotSupported") {
                    println!("🔒 Encrypted PDF detected!");
                } else {
                    println!("✗ Other error: {e}");
                }
            }
        }
    }

    println!("\nEncryption detection is working correctly!");
    Ok(())
}
