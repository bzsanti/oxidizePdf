//! Example demonstrating parsing annotations from existing PDF files
//!
//! This example shows how to read and process annotations from a PDF file.

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
    println!("Opening PDF: {}", filename);
    let reader = PdfReader::open(filename)?;
    let document = PdfDocument::new(reader);

    // Get document info
    println!("\nDocument Information:");
    println!("  Version: {}", document.version()?);
    println!("  Pages: {}", document.page_count()?);

    // Get metadata
    let metadata = document.metadata()?;
    if let Some(title) = metadata.title {
        println!("  Title: {}", title);
    }
    if let Some(author) = metadata.author {
        println!("  Author: {}", author);
    }

    // Process all annotations
    println!("\nScanning for annotations...");
    let all_annotations = document.get_all_annotations()?;

    if all_annotations.is_empty() {
        println!("No annotations found in the document.");
    } else {
        println!("\nFound annotations on {} page(s):", all_annotations.len());

        for (page_idx, annotations) in all_annotations {
            println!(
                "\n  Page {} ({} annotations):",
                page_idx + 1,
                annotations.len()
            );

            for (i, annot) in annotations.iter().enumerate() {
                println!("    Annotation {}:", i + 1);

                // Get annotation type
                if let Some(subtype) = annot.get("Subtype").and_then(|t| t.as_name()) {
                    println!("      Type: {:?}", subtype);
                }

                // Get annotation contents
                if let Some(contents) = annot.get("Contents").and_then(|c| c.as_string()) {
                    println!("      Contents: {:?}", contents);
                }

                // Get annotation title/author
                if let Some(title) = annot.get("T").and_then(|t| t.as_string()) {
                    println!("      Author: {:?}", title);
                }

                // Get annotation rectangle
                if let Some(rect) = annot.get("Rect").and_then(|r| r.as_array()) {
                    if rect.len() == 4 {
                        let coords: Vec<f64> = rect.0.iter().filter_map(|v| v.as_real()).collect();
                        if coords.len() == 4 {
                            println!(
                                "      Position: ({:.1}, {:.1}) to ({:.1}, {:.1})",
                                coords[0], coords[1], coords[2], coords[3]
                            );
                        }
                    }
                }

                // Get annotation color
                if let Some(color) = annot.get("C").and_then(|c| c.as_array()) {
                    let color_vals: Vec<f64> = color.0.iter().filter_map(|v| v.as_real()).collect();
                    match color_vals.len() {
                        1 => println!("      Color: Gray({:.2})", color_vals[0]),
                        3 => println!(
                            "      Color: RGB({:.2}, {:.2}, {:.2})",
                            color_vals[0], color_vals[1], color_vals[2]
                        ),
                        4 => println!(
                            "      Color: CMYK({:.2}, {:.2}, {:.2}, {:.2})",
                            color_vals[0], color_vals[1], color_vals[2], color_vals[3]
                        ),
                        _ => {}
                    }
                }

                // Get annotation flags
                if let Some(flags) = annot.get("F").and_then(|f| f.as_integer()) {
                    let flag_names = parse_annotation_flags(flags as u32);
                    if !flag_names.is_empty() {
                        println!("      Flags: {}", flag_names.join(", "));
                    }
                }

                // Special handling for specific annotation types
                if let Some(subtype) = annot.get("Subtype").and_then(|t| t.as_name()) {
                    let subtype_str = format!("{:?}", subtype);
                    match subtype_str.as_str() {
                        "PdfName(\"Link\")" => {
                            if let Some(action) = annot.get("A").and_then(|a| a.as_dict()) {
                                if let Some(s) = action.get("S").and_then(|s| s.as_name()) {
                                    let s_str = format!("{:?}", s);
                                    match s_str.as_str() {
                                        s if s.contains("URI") => {
                                            if let Some(uri) =
                                                action.get("URI").and_then(|u| u.as_string())
                                            {
                                                println!("      Link URL: {:?}", uri);
                                            }
                                        }
                                        s if s.contains("GoTo") => {
                                            println!("      Link Type: Internal navigation");
                                        }
                                        _ => {
                                            println!("      Link Type: {:?}", s);
                                        }
                                    }
                                }
                            }
                        }
                        "PdfName(\"Text\")" => {
                            if let Some(icon) = annot.get("Name").and_then(|n| n.as_name()) {
                                println!("      Icon: {:?}", icon);
                            }
                        }
                        s if s.contains("Highlight")
                            || s.contains("Underline")
                            || s.contains("StrikeOut")
                            || s.contains("Squiggly") =>
                        {
                            if let Some(quad_points) =
                                annot.get("QuadPoints").and_then(|q| q.as_array())
                            {
                                let num_quads = quad_points.len() / 8;
                                println!("      Highlighted regions: {}", num_quads);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    println!("\nAnnotation parsing complete.");
    Ok(())
}

/// Parse annotation flags into human-readable names
fn parse_annotation_flags(flags: u32) -> Vec<&'static str> {
    let mut flag_names = Vec::new();

    if flags & 0x01 != 0 {
        flag_names.push("Invisible");
    }
    if flags & 0x02 != 0 {
        flag_names.push("Hidden");
    }
    if flags & 0x04 != 0 {
        flag_names.push("Print");
    }
    if flags & 0x08 != 0 {
        flag_names.push("NoZoom");
    }
    if flags & 0x10 != 0 {
        flag_names.push("NoRotate");
    }
    if flags & 0x20 != 0 {
        flag_names.push("NoView");
    }
    if flags & 0x40 != 0 {
        flag_names.push("ReadOnly");
    }
    if flags & 0x80 != 0 {
        flag_names.push("Locked");
    }
    if flags & 0x100 != 0 {
        flag_names.push("ToggleNoView");
    }
    if flags & 0x200 != 0 {
        flag_names.push("LockedContents");
    }

    flag_names
}
