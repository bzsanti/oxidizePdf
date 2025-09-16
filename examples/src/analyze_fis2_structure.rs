use oxidize_pdf::parser::objects::{PdfName, PdfObject};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::collections::HashMap;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 ANALYZING FIS2 PDF STRUCTURE");
    println!("================================");

    let pdf_path =
        PathBuf::from("/Users/santifdezmunoz/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf");

    if !pdf_path.exists() {
        println!("❌ FIS2 PDF not found");
        return Ok(());
    }

    println!("📄 File: {}", pdf_path.display());

    // Open the PDF document
    let document = PdfReader::open_document(&pdf_path)?;

    let page_count = document
        .page_count()
        .map_err(|e| format!("Failed to get page count: {e}"))?;

    println!("📊 Total pages: {}", page_count);

    // Analyze first few pages in detail
    let pages_to_analyze = page_count.min(3);

    for page_idx in 0..pages_to_analyze {
        println!("\n{}", "=".repeat(50));
        println!("📄 ANALYZING PAGE {}", page_idx + 1);
        println!("{}", "=".repeat(50));

        match analyze_page_structure(&document, page_idx) {
            Ok(_) => println!("✅ Page {} analysis completed", page_idx + 1),
            Err(e) => println!("❌ Page {} analysis failed: {}", page_idx + 1, e),
        }
    }

    Ok(())
}

fn analyze_page_structure(
    document: &PdfDocument<std::fs::File>,
    page_idx: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get the page object
    let page = document.get_page(page_idx)?;

    println!(
        "📐 Page dimensions: {:.1} x {:.1} points",
        page.width(),
        page.height()
    );

    // Get page resources
    match document.get_page_resources(&page) {
        Ok(Some(resources)) => {
            println!("📋 Page resources found!");
            analyze_resources(&resources.0)?;
        }
        Ok(None) => {
            println!("⚠️  No page resources found");
        }
        Err(e) => {
            println!("❌ Failed to get page resources: {}", e);
        }
    }

    // Get content streams
    match document.get_page_content_streams(&page) {
        Ok(content_streams) => {
            println!("📝 Content streams found: {}", content_streams.len());

            for (i, stream_data) in content_streams.iter().enumerate() {
                println!("   Stream {}: {} bytes", i + 1, stream_data.len());
                analyze_content_stream(stream_data, i)?;
            }
        }
        Err(e) => {
            println!("❌ Failed to get content streams: {}", e);
        }
    }

    Ok(())
}

fn analyze_resources(
    resources: &HashMap<PdfName, PdfObject>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Analyzing resources:");

    for (name, obj) in resources {
        println!("   Resource '{}': {:?}", name.0, get_object_type(obj));

        // Focus on XObject resources (where images usually are)
        if name.0 == "XObject" {
            println!("      🖼️  Found XObject resource!");
            analyze_xobjects(obj)?;
        }

        // Also check for ColorSpace, Font, etc.
        if name.0 == "ColorSpace" {
            println!("      🎨 Found ColorSpace resource");
        }

        if name.0 == "Font" {
            println!("      🔤 Found Font resource");
        }
    }

    Ok(())
}

fn analyze_xobjects(xobject_dict: &PdfObject) -> Result<(), Box<dyn std::error::Error>> {
    if let PdfObject::Dictionary(dict) = xobject_dict {
        println!(
            "      📦 XObject dictionary contains {} entries:",
            dict.0.len()
        );

        for (name, obj_ref) in &dict.0 {
            println!(
                "         XObject '{}': {:?}",
                name.0,
                get_object_type(obj_ref)
            );

            if let PdfObject::Reference(obj_num, gen_num) = obj_ref {
                println!(
                    "            📍 References object {} generation {}",
                    obj_num, gen_num
                );
                // TODO: Resolve reference and analyze the actual XObject
            }
        }
    } else {
        println!(
            "      ⚠️  XObject is not a dictionary: {:?}",
            get_object_type(xobject_dict)
        );
    }

    Ok(())
}

fn analyze_content_stream(
    stream_data: &[u8],
    stream_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Convert to string for analysis (this might fail for binary data)
    let content = String::from_utf8_lossy(stream_data);
    let content_preview = if content.len() > 200 {
        format!("{}...", &content[..200])
    } else {
        content.to_string()
    };

    println!("      📝 Stream {} content preview:", stream_index + 1);
    println!("         {}", content_preview.replace('\n', "\\n"));

    // Look for image operators
    let image_operators = [
        "BI", // Begin inline image
        "ID", // Inline image data
        "EI", // End inline image
        "Do", // Draw XObject (including images)
    ];

    let mut found_operators = Vec::new();
    for op in &image_operators {
        if content.contains(op) {
            found_operators.push(*op);
        }
    }

    if !found_operators.is_empty() {
        println!("      🖼️  Found image operators: {:?}", found_operators);

        // Count occurrences
        for op in &found_operators {
            let count = content.matches(op).count();
            println!("         '{}' appears {} times", op, count);
        }
    } else {
        println!("      ⚠️  No image operators found in this stream");
    }

    // Look for other interesting patterns
    if content.contains("cm") {
        println!("      🔄 Found coordinate transformations (cm operator)");
    }

    if content.contains("q") && content.contains("Q") {
        println!("      📦 Found graphics state save/restore (q/Q)");
    }

    Ok(())
}

fn get_object_type(obj: &PdfObject) -> &'static str {
    match obj {
        PdfObject::Null => "Null",
        PdfObject::Boolean(_) => "Boolean",
        PdfObject::Integer(_) => "Integer",
        PdfObject::Real(_) => "Real",
        PdfObject::String(_) => "String",
        PdfObject::Name(_) => "Name",
        PdfObject::Array(_) => "Array",
        PdfObject::Dictionary(_) => "Dictionary",
        PdfObject::Stream(_) => "Stream",
        PdfObject::Reference(_, _) => "Reference",
    }
}
