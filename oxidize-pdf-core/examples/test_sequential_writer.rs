//! Test with a more compatible PDF structure
//! This example manually creates a PDF with proper sequential object numbering

use std::fs::File;
use std::io::{Seek, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating test PDF with sequential object numbering...");

    let mut file = File::create("test_sequential.pdf")?;

    // PDF Header
    writeln!(file, "%PDF-1.7")?;
    file.write_all(&[b'%', 0xE2, 0xE3, 0xCF, 0xD3, b'\n'])?; // Binary marker

    // Object 1: Catalog
    writeln!(file, "1 0 obj")?;
    writeln!(file, "<<")?;
    writeln!(file, "/Type /Catalog")?;
    writeln!(file, "/Pages 2 0 R")?;
    writeln!(file, "/Outlines 3 0 R")?;
    writeln!(file, "/PageMode /UseOutlines")?; // Show bookmarks panel by default
    writeln!(file, ">>")?;
    writeln!(file, "endobj")?;

    // Object 2: Pages
    writeln!(file, "2 0 obj")?;
    writeln!(file, "<<")?;
    writeln!(file, "/Type /Pages")?;
    writeln!(file, "/Kids [4 0 R 5 0 R]")?;
    writeln!(file, "/Count 2")?;
    writeln!(file, ">>")?;
    writeln!(file, "endobj")?;

    // Object 3: Outlines root
    writeln!(file, "3 0 obj")?;
    writeln!(file, "<<")?;
    writeln!(file, "/Type /Outlines")?;
    writeln!(file, "/First 6 0 R")?;
    writeln!(file, "/Last 7 0 R")?;
    writeln!(file, "/Count 2")?;
    writeln!(file, ">>")?;
    writeln!(file, "endobj")?;

    // Object 4: Page 1
    writeln!(file, "4 0 obj")?;
    writeln!(file, "<<")?;
    writeln!(file, "/Type /Page")?;
    writeln!(file, "/Parent 2 0 R")?;
    writeln!(file, "/MediaBox [0 0 612 792]")?;
    writeln!(file, "/Resources << /Font << /F1 8 0 R >> >>")?;
    writeln!(file, "/Contents 9 0 R")?;
    writeln!(file, ">>")?;
    writeln!(file, "endobj")?;

    // Object 5: Page 2
    writeln!(file, "5 0 obj")?;
    writeln!(file, "<<")?;
    writeln!(file, "/Type /Page")?;
    writeln!(file, "/Parent 2 0 R")?;
    writeln!(file, "/MediaBox [0 0 612 792]")?;
    writeln!(file, "/Resources << /Font << /F1 8 0 R >> >>")?;
    writeln!(file, "/Contents 10 0 R")?;
    writeln!(file, ">>")?;
    writeln!(file, "endobj")?;

    // Object 6: First outline item
    writeln!(file, "6 0 obj")?;
    writeln!(file, "<<")?;
    writeln!(file, "/Title (First Page)")?;
    writeln!(file, "/Parent 3 0 R")?;
    writeln!(file, "/Next 7 0 R")?;
    writeln!(file, "/Dest [4 0 R /Fit]")?;
    writeln!(file, ">>")?;
    writeln!(file, "endobj")?;

    // Object 7: Second outline item
    writeln!(file, "7 0 obj")?;
    writeln!(file, "<<")?;
    writeln!(file, "/Title (Second Page)")?;
    writeln!(file, "/Parent 3 0 R")?;
    writeln!(file, "/Prev 6 0 R")?;
    writeln!(file, "/Dest [5 0 R /Fit]")?;
    writeln!(file, ">>")?;
    writeln!(file, "endobj")?;

    // Object 8: Font
    writeln!(file, "8 0 obj")?;
    writeln!(file, "<<")?;
    writeln!(file, "/Type /Font")?;
    writeln!(file, "/Subtype /Type1")?;
    writeln!(file, "/BaseFont /Helvetica")?;
    writeln!(file, ">>")?;
    writeln!(file, "endobj")?;

    // Object 9: Content stream for page 1
    let content1 = b"BT /F1 24 Tf 50 700 Td (Page 1) Tj ET";
    writeln!(file, "9 0 obj")?;
    writeln!(file, "<<")?;
    writeln!(file, "/Length {}", content1.len())?;
    writeln!(file, ">>")?;
    writeln!(file, "stream")?;
    file.write_all(content1)?;
    writeln!(file)?;
    writeln!(file, "endstream")?;
    writeln!(file, "endobj")?;

    // Object 10: Content stream for page 2
    let content2 = b"BT /F1 24 Tf 50 700 Td (Page 2) Tj ET";
    writeln!(file, "10 0 obj")?;
    writeln!(file, "<<")?;
    writeln!(file, "/Length {}", content2.len())?;
    writeln!(file, ">>")?;
    writeln!(file, "stream")?;
    file.write_all(content2)?;
    writeln!(file)?;
    writeln!(file, "endstream")?;
    writeln!(file, "endobj")?;

    // XRef table
    let xref_offset = file.stream_position()?;
    writeln!(file, "xref")?;
    writeln!(file, "0 11")?;
    writeln!(file, "0000000000 65535 f ")?;
    writeln!(file, "0000000015 00000 n ")?; // obj 1
    writeln!(file, "0000000120 00000 n ")?; // obj 2
    writeln!(file, "0000000189 00000 n ")?; // obj 3
    writeln!(file, "0000000269 00000 n ")?; // obj 4
    writeln!(file, "0000000393 00000 n ")?; // obj 5
    writeln!(file, "0000000517 00000 n ")?; // obj 6
    writeln!(file, "0000000608 00000 n ")?; // obj 7
    writeln!(file, "0000000699 00000 n ")?; // obj 8
    writeln!(file, "0000000780 00000 n ")?; // obj 9
    writeln!(file, "0000000875 00000 n ")?; // obj 10

    // Trailer
    writeln!(file, "trailer")?;
    writeln!(file, "<<")?;
    writeln!(file, "/Size 11")?;
    writeln!(file, "/Root 1 0 R")?;
    writeln!(file, ">>")?;
    writeln!(file, "startxref")?;
    writeln!(file, "{xref_offset}")?;
    writeln!(file, "%%EOF")?;

    println!("✓ Created test_sequential.pdf with proper object ordering");
    println!("This PDF should work in all viewers including Foxit and Adobe Reader");

    Ok(())
}
