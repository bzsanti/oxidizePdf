//! Debug PDF structure to verify interactive features

use oxidize_pdf::parser::{PdfDocument, PdfObject, PdfReader};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_file>", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[1];
    println!("🔍 Analyzing PDF structure: {file_path}");

    let reader = PdfReader::open(file_path)?;
    let pdf_doc = PdfDocument::new(reader);

    println!("\n📄 Document Info:");
    println!("  Pages: {}", pdf_doc.page_count()?);

    // Check for interactive features in the catalog
    println!("\n📚 Checking Catalog:");

    // Try to access catalog through raw objects
    // The catalog is usually object 1 0
    match pdf_doc.get_object(1, 0) {
        Ok(PdfObject::Dictionary(catalog)) => {
            println!("  ✓ Found catalog dictionary");

            // Check for AcroForm
            if let Some(acro_form_ref) = catalog.get("AcroForm") {
                println!("  ✓ AcroForm present: {acro_form_ref:?}");

                // Try to get AcroForm object
                if let PdfObject::Reference(obj_num, gen_num) = acro_form_ref {
                    match pdf_doc.get_object(*obj_num, *gen_num) {
                        Ok(PdfObject::Dictionary(acro_form)) => {
                            println!(
                                "    - NeedAppearances: {:?}",
                                acro_form.get("NeedAppearances")
                            );
                            println!("    - Fields: {:?}", acro_form.get("Fields"));
                        }
                        _ => println!("    ⚠️  Could not read AcroForm dictionary"),
                    }
                }
            } else {
                println!("  ❌ No AcroForm found");
            }

            // Check for Outlines
            if let Some(outlines_ref) = catalog.get("Outlines") {
                println!("  ✓ Outlines present: {outlines_ref:?}");

                // Try to get Outlines object
                if let PdfObject::Reference(obj_num, gen_num) = outlines_ref {
                    match pdf_doc.get_object(*obj_num, *gen_num) {
                        Ok(PdfObject::Dictionary(outlines)) => {
                            println!("    - Type: {:?}", outlines.get("Type"));
                            println!("    - Count: {:?}", outlines.get("Count"));
                            println!("    - First: {:?}", outlines.get("First"));
                            println!("    - Last: {:?}", outlines.get("Last"));

                            // Try to read first outline item
                            if let Some(PdfObject::Reference(first_num, first_gen)) =
                                outlines.get("First")
                            {
                                if let Ok(PdfObject::Dictionary(first_item)) =
                                    pdf_doc.get_object(*first_num, *first_gen)
                                {
                                    println!("\n    First outline item:");
                                    println!("      - Title: {:?}", first_item.get("Title"));
                                    println!("      - Dest: {:?}", first_item.get("Dest"));
                                    println!("      - Parent: {:?}", first_item.get("Parent"));
                                }
                            }
                        }
                        _ => println!("    ⚠️  Could not read Outlines dictionary"),
                    }
                }
            } else {
                println!("  ❌ No Outlines found");
            }
        }
        _ => println!("  ❌ Could not read catalog"),
    }

    // Check pages for annotations
    println!("\n📄 Checking Pages:");
    for i in 0..pdf_doc.page_count()? {
        let page = pdf_doc.get_page(i)?;
        println!("\n  Page {}:", i + 1);

        if let Some(annots) = page.get_annotations() {
            println!("    ✓ Has {} annotations", annots.len());

            // Try to examine first annotation
            if !annots.is_empty() {
                if let Some(first_annot) = annots.get(0) {
                    if let PdfObject::Reference(obj_num, gen_num) = first_annot {
                        if let Ok(PdfObject::Dictionary(annot_dict)) =
                            pdf_doc.get_object(*obj_num, *gen_num)
                        {
                            println!("      First annotation:");
                            println!("        - Type: {:?}", annot_dict.get("Type"));
                            println!("        - Subtype: {:?}", annot_dict.get("Subtype"));
                            println!("        - Rect: {:?}", annot_dict.get("Rect"));
                            println!("        - Parent: {:?}", annot_dict.get("Parent"));
                        }
                    }
                }
            }
        } else {
            println!("    - No annotations");
        }
    }

    Ok(())
}
