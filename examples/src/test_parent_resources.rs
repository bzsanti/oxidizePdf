use oxidize_pdf::{PdfDocument, PdfReader};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf_path = "test-pdfs/Cold_Email_Hacks.pdf";

    if !Path::new(pdf_path).exists() {
        println!("❌ PDF not found: {}", pdf_path);
        return Ok(());
    }

    println!("🔍 Investigando Resources del objeto parent 134");
    println!("==============================================");

    let reader = PdfReader::open(pdf_path)?;
    let document = PdfDocument::new(reader);

    // Get page 14 (index 13)
    let page = document.get_page(13)?;
    println!("📄 Página 14 cargada correctamente");

    // Try to get the page tree root (113) first
    println!("🔍 Investigando page tree root (113)...");
    match document.get_object(113, 0) {
        Ok(root_obj) => {
            println!("✅ Objeto 113 (page tree root) encontrado");
            if let Some(root_dict) = root_obj.as_dict() {
                println!("📊 Page tree root tiene {} entradas", root_dict.0.len());

                if let Some(resources_obj) = root_dict.get("Resources") {
                    println!("🎯 Encontrado Resources en page tree root!");
                    match resources_obj {
                        oxidize_pdf::PdfObject::Dictionary(resources_dict) => {
                            println!("📝 Resources tiene {} entradas:", resources_dict.0.len());
                            for (key, _value) in resources_dict.0.iter() {
                                println!("  - {}", key.0);
                            }

                            // Check for Font specifically
                            if let Some(font_obj) = resources_dict.get("Font") {
                                println!("🔤 Font dictionary encontrado!");
                                if let Some(font_dict) = font_obj.as_dict() {
                                    println!(
                                        "📚 Font dictionary tiene {} fuentes:",
                                        font_dict.0.len()
                                    );
                                    for (font_name, font_ref) in font_dict.0.iter() {
                                        println!("  📖 {} -> {:?}", font_name.0, font_ref);
                                    }
                                }
                            } else {
                                println!("❌ No hay Font dictionary en Resources del root");
                            }
                        }
                        _ => println!("❌ Resources no es un dictionary"),
                    }
                } else {
                    println!("❌ No hay Resources en el page tree root");
                    println!("📋 Claves disponibles en root:");
                    for (key, _value) in root_dict.0.iter() {
                        println!("  - {}", key.0);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Error accediendo objeto 113: {}", e);
        }
    }

    println!("\n🔍 Investigando objeto parent 134...");
    // Try to get the parent object (134) directly
    match document.get_object(134, 0) {
        Ok(parent_obj) => {
            println!("✅ Objeto 134 (parent) encontrado");
            if let Some(parent_dict) = parent_obj.as_dict() {
                println!("📊 Objeto parent tiene {} entradas", parent_dict.0.len());

                // Look for Resources in parent
                if let Some(resources_obj) = parent_dict.get("Resources") {
                    println!("🎯 Encontrado Resources en parent!");
                    match resources_obj {
                        oxidize_pdf::PdfObject::Dictionary(resources_dict) => {
                            println!("📝 Resources tiene {} entradas:", resources_dict.0.len());
                            for (key, _value) in resources_dict.0.iter() {
                                println!("  - {}", key.0);
                            }

                            // Check for Font specifically
                            if let Some(font_obj) = resources_dict.get("Font") {
                                println!("🔤 Font dictionary encontrado!");
                                if let Some(font_dict) = font_obj.as_dict() {
                                    println!(
                                        "📚 Font dictionary tiene {} fuentes:",
                                        font_dict.0.len()
                                    );
                                    for (font_name, font_ref) in font_dict.0.iter() {
                                        println!("  📖 {} -> {:?}", font_name.0, font_ref);
                                    }
                                }
                            } else {
                                println!("❌ No hay Font dictionary en Resources del parent");
                            }
                        }
                        _ => println!("❌ Resources no es un dictionary"),
                    }
                } else {
                    println!("❌ No hay Resources en el objeto parent");
                    println!("📋 Claves disponibles en parent:");
                    for (key, _value) in parent_dict.0.iter() {
                        println!("  - {}", key.0);
                    }
                }
            } else {
                println!("❌ Objeto 134 no es un dictionary");
            }
        }
        Err(e) => {
            println!("❌ Error accediendo objeto 134: {}", e);
        }
    }

    Ok(())
}
