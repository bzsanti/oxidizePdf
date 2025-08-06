//! Simple test to verify text extraction functionality

use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::TextExtractor;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple valid PDF manually
    let pdf_content = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>
endobj
4 0 obj
<< /Length 44 >>
stream
BT
/F1 12 Tf
100 700 Td
(Hello World) Tj
ET
endstream
endobj
5 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
xref
0 6
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000241 00000 n 
0000000334 00000 n 
trailer
<< /Size 6 /Root 1 0 R >>
startxref
404
%%EOF";

    // Write to a file
    let mut file = std::fs::File::create("test_manual.pdf")?;
    file.write_all(pdf_content)?;
    drop(file);

    println!("Created test_manual.pdf");

    // Try to parse it
    match PdfReader::open_document("test_manual.pdf") {
        Ok(document) => {
            println!("✓ PDF parsed successfully!");

            // Try to extract text
            let extractor = TextExtractor::new();
            match extractor.extract_from_document(&document) {
                Ok(pages) => {
                    println!("✓ Text extraction successful!");
                    for (i, page) in pages.iter().enumerate() {
                        println!("\nPage {}: {}", i + 1, page.text);
                    }
                }
                Err(e) => {
                    println!("✗ Text extraction failed: {e:?}");
                }
            }
        }
        Err(e) => {
            println!("✗ PDF parsing failed: {e:?}");
            println!("\nThis indicates an issue with the PDF parser xref handling.");
        }
    }

    Ok(())
}
