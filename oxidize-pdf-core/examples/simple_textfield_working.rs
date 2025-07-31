//! Simple working text field - Direct approach
//!
//! This creates a text field using a more direct approach to ensure it works

use oxidize_pdf::objects::{Dictionary, Object, ObjectId};
use oxidize_pdf::{Document, Page, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

fn main() -> Result<()> {
    println!("📝 Creating simple working text field...");

    // Create a minimal PDF manually to ensure forms work
    let mut pdf_content = Vec::new();

    // PDF header
    pdf_content.extend_from_slice(b"%PDF-1.7\n");
    pdf_content.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n"); // Binary comment

    // Keep track of object positions
    let mut xref_positions = HashMap::new();
    let mut current_pos = pdf_content.len() as u64;

    // Object 1: Catalog
    xref_positions.insert(1, current_pos);
    let catalog = "1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n/AcroForm 3 0 R\n>>\nendobj\n";
    pdf_content.extend_from_slice(catalog.as_bytes());
    current_pos += catalog.len() as u64;

    // Object 2: Pages
    xref_positions.insert(2, current_pos);
    let pages = "2 0 obj\n<<\n/Type /Pages\n/Kids [4 0 R]\n/Count 1\n>>\nendobj\n";
    pdf_content.extend_from_slice(pages.as_bytes());
    current_pos += pages.len() as u64;

    // Object 3: AcroForm
    xref_positions.insert(3, current_pos);
    let acroform =
        "3 0 obj\n<<\n/Fields [5 0 R]\n/NeedAppearances true\n/DA (/Helv 12 Tf 0 g)\n>>\nendobj\n";
    pdf_content.extend_from_slice(acroform.as_bytes());
    current_pos += acroform.len() as u64;

    // Object 4: Page
    xref_positions.insert(4, current_pos);
    let page = "4 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Contents 6 0 R\n/Annots [5 0 R]\n/Resources <<\n/Font <<\n/Helv <<\n/Type /Font\n/Subtype /Type1\n/BaseFont /Helvetica\n>>\n>>\n>>\n>>\nendobj\n";
    pdf_content.extend_from_slice(page.as_bytes());
    current_pos += page.len() as u64;

    // Object 5: Text field (form field + widget annotation combined)
    xref_positions.insert(5, current_pos);
    let field = "5 0 obj\n<<\n/Type /Annot\n/Subtype /Widget\n/FT /Tx\n/T (test_field)\n/Rect [150 640 400 660]\n/P 4 0 R\n/F 4\n/DA (/Helv 12 Tf 0 g)\n/V (Type here)\n/DV (Type here)\n>>\nendobj\n";
    pdf_content.extend_from_slice(field.as_bytes());
    current_pos += field.len() as u64;

    // Object 6: Page content
    xref_positions.insert(6, current_pos);
    let content = "6 0 obj\n<<\n/Length 85\n>>\nstream\nBT\n/Helv 14 Tf\n50 700 Td\n(Simple Working Text Field) Tj\n0 -50 Td\n(Enter text:) Tj\nET\nendstream\nendobj\n";
    pdf_content.extend_from_slice(content.as_bytes());
    current_pos += content.len() as u64;

    // Write xref table
    let xref_start = pdf_content.len();
    pdf_content.extend_from_slice(b"xref\n");
    pdf_content.extend_from_slice(b"0 7\n");
    pdf_content.extend_from_slice(b"0000000000 65535 f \n");

    for i in 1..=6 {
        let pos = xref_positions.get(&i).unwrap();
        let entry = format!("{:010} 00000 n \n", pos);
        pdf_content.extend_from_slice(entry.as_bytes());
    }

    // Write trailer
    pdf_content.extend_from_slice(b"trailer\n");
    pdf_content.extend_from_slice(b"<<\n");
    pdf_content.extend_from_slice(b"/Size 7\n");
    pdf_content.extend_from_slice(b"/Root 1 0 R\n");
    pdf_content.extend_from_slice(b">>\n");
    pdf_content.extend_from_slice(b"startxref\n");
    pdf_content.extend_from_slice(format!("{}\n", xref_start).as_bytes());
    pdf_content.extend_from_slice(b"%%EOF\n");

    // Write to file
    let mut file = File::create("simple_textfield_working.pdf")?;
    file.write_all(&pdf_content)?;

    println!("\n✅ Created simple_textfield_working.pdf");
    println!("\n🔍 This PDF contains:");
    println!("  - Page with /Annots array pointing to field");
    println!("  - AcroForm with /Fields array");
    println!("  - Text field that is both annotation and form field");
    println!("  - Proper parent/child relationships");
    println!("\n📋 Key differences:");
    println!("  - Field is in page's /Annots array");
    println!("  - Field has /P reference to parent page");
    println!("  - Single object serves as both field and widget");

    Ok(())
}
