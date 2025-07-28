use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::fs::File;
use std::path::Path;

fn main() {
    println!("Testing stream parsing with lenient mode...\n");

    // Create a test PDF with incorrect stream length
    let test_pdf = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << >> >>
endobj
4 0 obj
<< /Length 10 >>
stream
This is a much longer stream than the declared 10 bytes
endstream
endobj
xref
0 5
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000116 00000 n 
0000000232 00000 n 
trailer
<< /Size 5 /Root 1 0 R >>
startxref
352
%%EOF";

    // Write test PDF to a temporary file
    let temp_path = "/tmp/test_lenient_stream.pdf";
    std::fs::write(temp_path, test_pdf).unwrap();

    println!("Created test PDF with incorrect stream length:");
    println!("- Declared length: 10 bytes");
    println!("- Actual content: 'This is a much longer stream than the declared 10 bytes'");
    println!("- Actual length: 56 bytes\n");

    // Test strict mode
    println!("Testing STRICT mode:");
    let file = File::open(temp_path).unwrap();
    let options = ParseOptions::strict();

    match PdfReader::new_with_options(file, options) {
        Ok(mut reader) => {
            println!("  ✓ PDF parsed successfully");

            // Try to access the content stream
            match reader.get_object(4, 0) {
                Ok(obj) => {
                    println!("  ✓ Object 4 retrieved: {:?}", obj);
                    if let Some(stream) = obj.as_stream() {
                        let content = String::from_utf8_lossy(&stream.data);
                        println!("  Stream content: '{}'", content);
                        println!("  Stream length: {} bytes", stream.data.len());
                    }
                }
                Err(e) => println!("  ✗ Failed to get object 4: {}", e),
            }
        }
        Err(e) => println!("  ✗ Failed to parse PDF: {}", e),
    }

    // Test lenient mode
    println!("\nTesting LENIENT mode:");
    let file = File::open(temp_path).unwrap();
    let options = ParseOptions::lenient();

    match PdfReader::new_with_options(file, options) {
        Ok(mut reader) => {
            println!("  ✓ PDF parsed successfully");

            // Try to access the content stream
            match reader.get_object(4, 0) {
                Ok(obj) => {
                    println!("  ✓ Object 4 retrieved");
                    if let Some(stream) = obj.as_stream() {
                        let content = String::from_utf8_lossy(&stream.data);
                        println!("  Stream content: '{}'", content);
                        println!("  Stream length: {} bytes", stream.data.len());

                        if stream.data.len() > 10 {
                            println!("  🎉 LENIENT MODE RECOVERED FULL STREAM CONTENT!");
                        }
                    }
                }
                Err(e) => println!("  ✗ Failed to get object 4: {}", e),
            }
        }
        Err(e) => println!("  ✗ Failed to parse PDF: {}", e),
    }

    // Clean up
    std::fs::remove_file(temp_path).ok();

    println!("\nNow testing real PDFs for stream errors...\n");

    // Look for PDFs that might have stream length issues
    let fixtures_dir =
        Path::new("/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf/tests/fixtures");

    if let Ok(entries) = std::fs::read_dir(fixtures_dir) {
        for entry in entries.flatten().take(50) {
            // Test first 50 PDFs
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
                // Only test if strict mode works but we want to see if lenient helps
                let file = File::open(&path).unwrap();
                let strict_options = ParseOptions::default();

                if let Ok(mut reader) = PdfReader::new_with_options(file, strict_options) {
                    // Try to read all objects to find stream errors
                    for obj_num in 1..100 {
                        if let Ok(obj) = reader.get_object(obj_num, 0) {
                            if obj.as_stream().is_some() {
                                // Found a stream, no need to test further for this PDF
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}
