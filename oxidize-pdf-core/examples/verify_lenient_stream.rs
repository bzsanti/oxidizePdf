use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::io::Cursor;

fn main() {
    println!("Creating a test PDF with incorrect stream length...\n");

    // Create a simple PDF with a stream that has incorrect length
    let pdf_content = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>
endobj
4 0 obj
<< /Length 20 >>
stream
This is a test stream that is definitely longer than 20 bytes!
endstream
endobj
xref
0 5
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000229 00000 n 
trailer
<< /Size 5 /Root 1 0 R >>
startxref
373
%%EOF";

    println!("PDF Details:");
    println!("- Declared stream length: 20 bytes");
    println!("- Actual stream content: \"This is a test stream that is definitely longer than 20 bytes!\"");
    println!("- Actual length: 62 bytes\n");

    // Test with strict mode
    println!("Testing STRICT mode:");
    let cursor = Cursor::new(pdf_content.to_vec());
    let strict_options = ParseOptions::strict();

    match PdfReader::new_with_options(cursor, strict_options) {
        Ok(mut reader) => {
            println!("  ✓ PDF parsed (xref recovery worked)");

            // Try to get the content stream
            match reader.get_object(4, 0) {
                Ok(obj) => {
                    println!("  ✓ Object 4 retrieved");
                    if let Some(stream) = obj.as_stream() {
                        let content = String::from_utf8_lossy(&stream.data);
                        println!("  Stream content: '{content}'");
                        println!("  Stream length: {} bytes", stream.data.len());
                    }
                }
                Err(e) => println!("  ✗ Failed to get object 4: {e}"),
            }
        }
        Err(e) => println!("  ✗ Failed to parse PDF: {e}"),
    }

    // Test with lenient mode
    println!("\nTesting LENIENT mode:");
    let cursor = Cursor::new(pdf_content.to_vec());
    let lenient_options = ParseOptions::lenient();

    match PdfReader::new_with_options(cursor, lenient_options) {
        Ok(mut reader) => {
            println!("  ✓ PDF parsed (xref recovery worked)");

            // Try to get the content stream
            match reader.get_object(4, 0) {
                Ok(obj) => {
                    println!("  ✓ Object 4 retrieved");
                    if let Some(stream) = obj.as_stream() {
                        let content = String::from_utf8_lossy(&stream.data);
                        println!("  Stream content: '{content}'");
                        println!("  Stream length: {} bytes", stream.data.len());

                        if stream.data.len() > 20 {
                            println!(
                                "\n  🎉 SUCCESS! Lenient mode recovered the full stream content!"
                            );
                            println!(
                                "     Expected only 20 bytes but got all {} bytes",
                                stream.data.len()
                            );
                        }
                    }
                }
                Err(e) => println!("  ✗ Failed to get object 4: {e}"),
            }
        }
        Err(e) => println!("  ✗ Failed to parse PDF: {e}"),
    }
}
