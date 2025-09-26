use oxidize_pdf::PdfReader;
use std::error::Error;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    println!("🧪 Testing Issue #47 - Corrupted PDF with XRef stream");

    let pdf_path = "test-pdfs/Cold_Email_Hacks.pdf";

    if !Path::new(pdf_path).exists() {
        println!("❌ Test PDF not found: {}", pdf_path);
        println!("💡 Download it from: https://github.com/user-attachments/files/22399799/Cold.Email.Hacks.pdf");
        return Ok(());
    }

    println!("📖 Opening PDF: {}", pdf_path);

    match PdfReader::open(pdf_path) {
        Ok(mut reader) => {
            println!("✅ PDF opened successfully");
            println!("📄 PDF version: {}", reader.version());

            match reader.page_count() {
                Ok(count) => {
                    println!("✅ Page count: {}", count);

                    // Try to get basic info about first page
                    if count > 0 {
                        match reader.get_page(0) {
                            Ok(page) => {
                                let size = page.media_box;
                                println!(
                                    "✅ First page size: {}x{}",
                                    size[2] - size[0],
                                    size[3] - size[1]
                                );
                            }
                            Err(e) => {
                                println!("⚠️  Could not get first page: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to get page count: {}", e);
                    println!("🔍 Error chain:");
                    let mut current = e.source();
                    while let Some(err) = current {
                        println!("  → {}", err);
                        current = err.source();
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to open PDF: {}", e);
            println!("🔍 Error chain:");
            let mut current = e.source();
            while let Some(err) = current {
                println!("  → {}", err);
                current = err.source();
            }
        }
    }

    Ok(())
}
