//! Generate a PDF fixture with embedded JPEG for testing JPEG extraction
//!
//! This creates a raw PDF with a JPEG image to test the byte duplication bug fix.

use std::fs::File;
use std::io::Write;

/// Generate a small JPEG image as bytes for embedding
fn generate_test_jpeg() -> Vec<u8> {
    // This is a minimal valid JPEG image
    let mut jpeg_data = vec![
        // JPEG SOI (Start of Image)
        0xFF, 0xD8, // JFIF APP0 segment
        0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, // "JFIF\0"
        0x01, 0x01, // Version 1.1
        0x01, // Units: dots per inch
        0x00, 0x48, 0x00, 0x48, // X and Y density (72 DPI)
        0x00, 0x00, // Thumbnail width and height (none)
        // Quantization table
        0xFF, 0xDB, 0x00, 0x43, 0x00,
    ];

    // Simple quantization values (64 bytes)
    for i in 0..64 {
        jpeg_data.push(if i == 0 { 8 } else { 16 });
    }

    // Frame header (SOF0) - 8x8 image
    jpeg_data.extend_from_slice(&[
        0xFF, 0xC0, 0x00, 0x11, 0x08, // Sample precision
        0x00, 0x08, 0x00, 0x08, // Height and width (8x8)
        0x01, // Number of components
        0x01, 0x11, 0x00, // Component 1: ID=1, sampling=1x1, quantization table=0
    ]);

    // Huffman table (simplified)
    jpeg_data.extend_from_slice(&[
        0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        0x09, 0x0A, 0x0B,
    ]);

    // Start of scan
    jpeg_data.extend_from_slice(&[
        0xFF, 0xDA, 0x00, 0x08, 0x01, // Number of components in scan
        0x01, 0x00, // Component 1, Huffman table IDs
        0x00, 0x3F, 0x00, // Start, end, successive approximation
    ]);

    // Add the specific 17-byte pattern that was being duplicated in the bug
    jpeg_data.extend_from_slice(&[
        0x1a, 0x1f, 0x28, 0x42, 0x2b, 0x28, 0x24, 0x24, 0x28, 0x51, 0x3a, 0x3d, 0x30, 0x42, 0x60,
        0x55, 0x64,
    ]);

    // Minimal compressed data and EOI
    jpeg_data.extend_from_slice(&[0xFF, 0x00, 0xFF, 0xD9]);

    jpeg_data
}

/// Generate a minimal PDF with the JPEG embedded
fn generate_pdf_with_jpeg() -> Vec<u8> {
    let jpeg_data = generate_test_jpeg();
    let jpeg_length = jpeg_data.len();

    // Build PDF content carefully
    let mut pdf_bytes = Vec::new();

    // PDF header
    pdf_bytes.extend_from_slice(b"%PDF-1.7\n");
    pdf_bytes.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n"); // Binary comment

    // Object 1: Catalog
    let obj1_start = pdf_bytes.len();
    pdf_bytes.extend_from_slice(b"1 0 obj\n");
    pdf_bytes.extend_from_slice(b"<< /Type /Catalog /Pages 2 0 R >>\n");
    pdf_bytes.extend_from_slice(b"endobj\n");

    // Object 2: Pages
    let obj2_start = pdf_bytes.len();
    pdf_bytes.extend_from_slice(b"2 0 obj\n");
    pdf_bytes.extend_from_slice(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>\n");
    pdf_bytes.extend_from_slice(b"endobj\n");

    // Object 3: Page
    let obj3_start = pdf_bytes.len();
    pdf_bytes.extend_from_slice(b"3 0 obj\n");
    pdf_bytes.extend_from_slice(b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /XObject << /Im1 4 0 R >> >> /Contents 5 0 R >>\n");
    pdf_bytes.extend_from_slice(b"endobj\n");

    // Object 4: Image
    let obj4_start = pdf_bytes.len();
    pdf_bytes.extend_from_slice(format!("4 0 obj\n<< /Type /XObject /Subtype /Image /Width 8 /Height 8 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n", jpeg_length).as_bytes());
    pdf_bytes.extend_from_slice(&jpeg_data);
    pdf_bytes.extend_from_slice(b"\nendstream\nendobj\n");

    // Object 5: Content stream
    let obj5_start = pdf_bytes.len();
    let content_stream = "q\n100 0 0 100 100 600 cm\n/Im1 Do\nQ\n";
    pdf_bytes.extend_from_slice(
        format!(
            "5 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            content_stream.len(),
            content_stream
        )
        .as_bytes(),
    );

    // Cross-reference table
    let xref_start = pdf_bytes.len();
    pdf_bytes.extend_from_slice(b"xref\n");
    pdf_bytes.extend_from_slice(b"0 6\n");
    pdf_bytes.extend_from_slice(b"0000000000 65535 f \n");
    pdf_bytes.extend_from_slice(format!("{:010} 00000 n \n", obj1_start).as_bytes());
    pdf_bytes.extend_from_slice(format!("{:010} 00000 n \n", obj2_start).as_bytes());
    pdf_bytes.extend_from_slice(format!("{:010} 00000 n \n", obj3_start).as_bytes());
    pdf_bytes.extend_from_slice(format!("{:010} 00000 n \n", obj4_start).as_bytes());
    pdf_bytes.extend_from_slice(format!("{:010} 00000 n \n", obj5_start).as_bytes());

    // Trailer
    pdf_bytes.extend_from_slice(b"trailer\n");
    pdf_bytes.extend_from_slice(b"<< /Size 6 /Root 1 0 R >>\n");
    pdf_bytes.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_start).as_bytes());

    pdf_bytes
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating JPEG test fixture PDF...");

    let pdf_data = generate_pdf_with_jpeg();
    let output_path = "test-pdfs/jpeg_extraction_test.pdf";

    println!("Writing PDF to: {}", output_path);
    let mut file = File::create(output_path)?;
    file.write_all(&pdf_data)?;

    let metadata = std::fs::metadata(output_path)?;
    println!("✅ PDF fixture created successfully!");
    println!("   File size: {} bytes", metadata.len());
    println!("   Contains JPEG with specific 17-byte pattern for regression testing");

    Ok(())
}
