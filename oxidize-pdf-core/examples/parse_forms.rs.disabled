//! Example to parse and analyze forms in PDF files
//!
//! This example reads a PDF file and extracts information about its AcroForm
//! and form fields to verify that forms are being written correctly.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get filename from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_file>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    if !Path::new(filename).exists() {
        eprintln!("Error: File '{}' not found", filename);
        std::process::exit(1);
    }

    // Open the PDF document
    println!("🔍 Analyzing PDF forms in: {}", filename);
    let reader = PdfReader::open(filename)?;
    let document = PdfDocument::new(reader);

    // Get document info
    println!("\n📄 Document Information:");
    println!("  Version: {}", document.version()?);
    println!("  Pages: {}", document.page_count()?);

    // Check for AcroForm in catalog
    println!("\n📋 AcroForm Analysis:");

    // Try to get the catalog
    if let Ok(catalog) = document.get_catalog() {
        // Look for AcroForm entry
        if let Some(acro_form_ref) = catalog.get("AcroForm") {
            println!("  ✅ AcroForm found in catalog!");
            println!("  📎 AcroForm reference: {:?}", acro_form_ref);

            // Try to resolve the AcroForm object
            if let Ok(acro_form_obj) = document.resolve_reference(acro_form_ref) {
                if let Some(acro_form_dict) = acro_form_obj.as_dict() {
                    println!("  📁 AcroForm dictionary contents:");

                    // Check for Fields array
                    if let Some(fields) = acro_form_dict.get("Fields") {
                        println!("    • Fields: {:?}", fields);

                        if let Some(fields_array) = fields.as_array() {
                            println!("    • Number of form fields: {}", fields_array.len());

                            // Analyze each field
                            for (i, field_ref) in fields_array.0.iter().enumerate() {
                                println!("    📝 Field {} reference: {:?}", i + 1, field_ref);

                                if let Ok(field_obj) = document.resolve_reference(field_ref) {
                                    if let Some(field_dict) = field_obj.as_dict() {
                                        analyze_form_field(field_dict, i + 1);
                                    }
                                }
                            }
                        }
                    } else {
                        println!("    ❌ No Fields array found in AcroForm");
                    }

                    // Check other AcroForm properties
                    if let Some(need_appearances) = acro_form_dict.get("NeedAppearances") {
                        println!("    • NeedAppearances: {:?}", need_appearances);
                    }

                    if let Some(da) = acro_form_dict.get("DA") {
                        println!("    • Default Appearance: {:?}", da);
                    }

                    if let Some(sig_flags) = acro_form_dict.get("SigFlags") {
                        println!("    • Signature Flags: {:?}", sig_flags);
                    }
                } else {
                    println!("  ❌ AcroForm object is not a dictionary");
                }
            } else {
                println!("  ❌ Could not resolve AcroForm object");
            }
        } else {
            println!("  ❌ No AcroForm found in catalog");
            println!("  📋 Available catalog entries:");
            for (key, value) in catalog.0.iter() {
                println!("    • {}: {:?}", key, value);
            }
        }
    } else {
        println!("  ❌ Could not access document catalog");
    }

    // Check for form fields in pages (widget annotations)
    println!("\n📑 Page-level Form Analysis:");
    for page_idx in 0..document.page_count()? {
        if let Ok(page) = document.get_page(page_idx) {
            if page.has_annotations() {
                println!("  📄 Page {} has annotations:", page_idx + 1);

                if let Some(annotations) = page.get_annotations() {
                    let mut widget_count = 0;

                    for (i, _annot) in annotations.0.iter().enumerate() {
                        // We would need to resolve and check if it's a Widget annotation
                        // For now, just count them
                        widget_count += 1;
                    }

                    println!("    • Total annotations: {}", annotations.len());
                    println!("    • Potential widgets: {}", widget_count);
                }
            } else {
                println!("  📄 Page {} has no annotations", page_idx + 1);
            }
        }
    }

    println!("\n🎯 Forms Integration Summary:");
    println!("• Check if AcroForm exists in catalog ✓");
    println!("• Analyze form fields structure ✓");
    println!("• Check for widget annotations on pages ✓");
    println!("• Verify field-widget relationships (partial)");

    Ok(())
}

fn analyze_form_field(field_dict: &oxidize_pdf::parser::objects::PdfDictionary, field_num: usize) {
    println!("      🏷️  Field {} details:", field_num);

    // Field type
    if let Some(ft) = field_dict.get("FT") {
        println!("        ▫️ Type (FT): {:?}", ft);
    }

    // Field name
    if let Some(t) = field_dict.get("T") {
        println!("        ▫️ Name (T): {:?}", t);
    }

    // Field value
    if let Some(v) = field_dict.get("V") {
        println!("        ▫️ Value (V): {:?}", v);
    }

    // Default value
    if let Some(dv) = field_dict.get("DV") {
        println!("        ▫️ Default Value (DV): {:?}", dv);
    }

    // Field flags
    if let Some(ff) = field_dict.get("Ff") {
        println!("        ▫️ Flags (Ff): {:?}", ff);
    }

    // Appearance
    if let Some(ap) = field_dict.get("AP") {
        println!("        ▫️ Appearance (AP): {:?}", ap);
    }

    // Kids (for hierarchical fields)
    if let Some(kids) = field_dict.get("Kids") {
        println!("        ▫️ Kids: {:?}", kids);
    }

    // Parent
    if let Some(parent) = field_dict.get("Parent") {
        println!("        ▫️ Parent: {:?}", parent);
    }

    // Widget-specific properties
    if let Some(rect) = field_dict.get("Rect") {
        println!("        ▫️ Rectangle (Rect): {:?}", rect);
    }

    if let Some(subtype) = field_dict.get("Subtype") {
        println!("        ▫️ Subtype: {:?}", subtype);
    }
}
