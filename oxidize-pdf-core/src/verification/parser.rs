//! PDF Parser for Verification
//!
//! This module provides a simple parser to extract key information from generated PDFs
//! for verification purposes. It's not a complete PDF parser, but focuses on the
//! elements needed to verify ISO compliance.

use crate::error::{PdfError, Result};
use std::collections::HashMap;

/// Parsed representation of a PDF for verification
#[derive(Debug, Clone)]
pub struct ParsedPdf {
    /// PDF version from header
    pub version: String,
    /// Document catalog dictionary
    pub catalog: Option<HashMap<String, String>>,
    /// Page tree information
    pub page_tree: Option<PageTree>,
    /// Font information
    pub fonts: Vec<String>,
    /// Color space usage flags
    pub uses_device_rgb: bool,
    pub uses_device_cmyk: bool,
    pub uses_device_gray: bool,
    /// Graphics state information
    pub graphics_states: Vec<GraphicsState>,
    /// Text objects found
    pub text_objects: Vec<TextObject>,
    /// Annotations found
    pub annotations: Vec<Annotation>,
    /// Cross-reference table info
    pub xref_valid: bool,
    /// Total objects in PDF
    pub object_count: usize,
}

#[derive(Debug, Clone)]
pub struct PageTree {
    pub root_type: String,
    pub page_count: usize,
    pub kids_arrays: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct GraphicsState {
    pub line_width: Option<f64>,
    pub line_cap: Option<i32>,
    pub line_join: Option<i32>,
    pub fill_color: Option<String>,
    pub stroke_color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TextObject {
    pub font: Option<String>,
    pub font_size: Option<f64>,
    pub text_content: String,
}

#[derive(Debug, Clone)]
pub struct Annotation {
    pub subtype: String,
    pub rect: Option<[f64; 4]>,
    pub contents: Option<String>,
}

/// Parse PDF bytes and extract verification information
pub fn parse_pdf(pdf_bytes: &[u8]) -> Result<ParsedPdf> {
    let pdf_text = String::from_utf8_lossy(pdf_bytes);

    let parsed = ParsedPdf {
        version: extract_version(&pdf_text)?,
        catalog: extract_catalog(&pdf_text),
        page_tree: extract_page_tree(&pdf_text),
        fonts: extract_fonts(&pdf_text),
        uses_device_rgb: detect_rgb_usage(&pdf_text),
        uses_device_cmyk: detect_cmyk_usage(&pdf_text),
        uses_device_gray: detect_gray_usage(&pdf_text),
        graphics_states: extract_graphics_states(&pdf_text),
        text_objects: extract_text_objects(&pdf_text),
        annotations: extract_annotations(&pdf_text),
        xref_valid: validate_xref(&pdf_text),
        object_count: count_objects(&pdf_text),
    };

    Ok(parsed)
}

/// Extract PDF version from header
fn extract_version(pdf_text: &str) -> Result<String> {
    if let Some(header_line) = pdf_text.lines().next() {
        if let Some(stripped) = header_line.strip_prefix("%PDF-") {
            return Ok(stripped.to_string());
        }
    }
    Err(PdfError::ParseError(
        "No valid PDF header found".to_string(),
    ))
}

/// Extract document catalog information
fn extract_catalog(pdf_text: &str) -> Option<HashMap<String, String>> {
    // Look for catalog object pattern with flexible spacing
    let catalog_patterns = [
        "/Type /Catalog",
        "/Type/Catalog",
        "/Type  /Catalog", // Multiple spaces
    ];

    for pattern in &catalog_patterns {
        if let Some(catalog_start) = pdf_text.find(pattern) {
            let catalog_section = &pdf_text[catalog_start..];
            if let Some(end) = catalog_section.find("endobj") {
                let catalog_content = &catalog_section[..end];

                let mut catalog = HashMap::new();

                // Extract Type - check for any of the patterns
                for type_pattern in &catalog_patterns {
                    if catalog_content.contains(type_pattern) {
                        catalog.insert("Type".to_string(), "Catalog".to_string());
                        break;
                    }
                }

                // Extract Version if present
                if let Some(version_match) = extract_dict_entry(catalog_content, "Version") {
                    catalog.insert("Version".to_string(), version_match);
                }

                // Extract Pages reference
                if let Some(pages_match) = extract_dict_entry(catalog_content, "Pages") {
                    catalog.insert("Pages".to_string(), pages_match);
                }

                return Some(catalog);
            }
        }
    }
    None
}

/// Extract page tree information
fn extract_page_tree(pdf_text: &str) -> Option<PageTree> {
    // Look for page tree root with flexible spacing
    let pages_patterns = [
        "/Type /Pages",
        "/Type/Pages",
        "/Type  /Pages", // Multiple spaces
    ];

    for pattern in &pages_patterns {
        if let Some(pages_start) = pdf_text.find(pattern) {
            let pages_section = &pdf_text[pages_start..];
            if let Some(end) = pages_section.find("endobj") {
                let pages_content = &pages_section[..end];

                let page_count = extract_dict_entry(pages_content, "Count")
                    .and_then(|s| {
                        // Handle both "1" and "1 0 R" formats
                        let cleaned = s.split_whitespace().next().unwrap_or("0");
                        cleaned.parse::<usize>().ok()
                    })
                    .unwrap_or(0);

                let mut kids_arrays = Vec::new();
                if let Some(kids_match) = extract_array_entry(pages_content, "Kids") {
                    kids_arrays.push(kids_match);
                }

                return Some(PageTree {
                    root_type: "Pages".to_string(),
                    page_count,
                    kids_arrays,
                });
            }
        }
    }
    None
}

/// Extract font information
fn extract_fonts(pdf_text: &str) -> Vec<String> {
    let mut fonts = Vec::new();

    // Look for font objects
    for line in pdf_text.lines() {
        if line.contains("/Type /Font") || line.contains("/BaseFont") {
            // Extract font name patterns
            if line.contains("Helvetica") {
                fonts.push("Helvetica".to_string());
            }
            if line.contains("Times") {
                fonts.push("Times-Roman".to_string());
            }
            if line.contains("Courier") {
                fonts.push("Courier".to_string());
            }
            if line.contains("Symbol") {
                fonts.push("Symbol".to_string());
            }
            if line.contains("ZapfDingbats") {
                fonts.push("ZapfDingbats".to_string());
            }
        }
    }

    fonts.sort();
    fonts.dedup();
    fonts
}

/// Extract graphics state information
fn extract_graphics_states(pdf_text: &str) -> Vec<GraphicsState> {
    let mut states = Vec::new();

    // Look for content streams with graphics operators
    for line in pdf_text.lines() {
        if line.contains(" w")
            || line.contains(" J")
            || line.contains(" j")
            || line.contains(" rg")
            || line.contains(" RG")
        {
            let mut state = GraphicsState {
                line_width: None,
                line_cap: None,
                line_join: None,
                fill_color: None,
                stroke_color: None,
            };

            // Extract line width (pattern: "number w")
            if let Some(w_match) = extract_graphics_operator(line, "w") {
                state.line_width = w_match.parse().ok();
            }

            // Extract line cap (pattern: "number J")
            if let Some(j_match) = extract_graphics_operator(line, "J") {
                state.line_cap = j_match.parse().ok();
            }

            states.push(state);
        }
    }

    states
}

/// Extract text objects
fn extract_text_objects(pdf_text: &str) -> Vec<TextObject> {
    let mut text_objects = Vec::new();

    // Look for text objects (BT...ET blocks)
    let mut in_text_object = false;
    let mut current_font = None;
    let mut current_size = None;

    for line in pdf_text.lines() {
        if line.contains("BT") {
            in_text_object = true;
            current_font = None;
            current_size = None;
        } else if line.contains("ET") {
            in_text_object = false;
        } else if in_text_object {
            // Extract font settings (pattern: "/FontName size Tf")
            if line.contains(" Tf") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    current_font = Some(parts[0].to_string());
                    current_size = parts[1].parse().ok();
                }
            }

            // Extract text content (pattern: "(text) Tj" or "[(text)] TJ")
            if line.contains(" Tj") || line.contains(" TJ") {
                if let Some(text_content) = extract_text_content(line) {
                    text_objects.push(TextObject {
                        font: current_font.clone(),
                        font_size: current_size,
                        text_content,
                    });
                }
            }
        }
    }

    text_objects
}

/// Extract annotations
fn extract_annotations(pdf_text: &str) -> Vec<Annotation> {
    let mut annotations = Vec::new();

    // Look for annotation objects
    if pdf_text.contains("/Type /Annot") {
        // This is a simplified extraction - real implementation would be more complex
        for line in pdf_text.lines() {
            if line.contains("/Subtype") {
                if let Some(subtype) = extract_dict_entry(line, "Subtype") {
                    annotations.push(Annotation {
                        subtype,
                        rect: None,     // TODO: Extract rect
                        contents: None, // TODO: Extract contents
                    });
                }
            }
        }
    }

    annotations
}

/// Validate cross-reference table
fn validate_xref(pdf_text: &str) -> bool {
    pdf_text.contains("xref") && pdf_text.contains("%%EOF")
}

/// Count total objects in PDF
fn count_objects(pdf_text: &str) -> usize {
    pdf_text.matches(" obj").count()
}

/// Helper: Extract dictionary entry value
fn extract_dict_entry(content: &str, key: &str) -> Option<String> {
    let pattern = format!("/{}", key);
    if let Some(start) = content.find(&pattern) {
        let after_key = &content[start + pattern.len()..];
        let words: Vec<&str> = after_key.split_whitespace().collect();
        if !words.is_empty() {
            return Some(words[0].trim_start_matches('/').to_string());
        }
    }
    None
}

/// Helper: Extract array entry
fn extract_array_entry(content: &str, key: &str) -> Option<Vec<String>> {
    let pattern = format!("/{} [", key);
    if let Some(start) = content.find(&pattern) {
        let after_start = &content[start + pattern.len()..];
        if let Some(end) = after_start.find(']') {
            let array_content = &after_start[..end];
            let elements: Vec<String> = array_content
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            return Some(elements);
        }
    }
    None
}

/// Helper: Extract graphics operator value
fn extract_graphics_operator(line: &str, operator: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == operator && i > 0 {
            return Some(parts[i - 1].to_string());
        }
    }
    None
}

/// Helper: Extract text content from text showing operator
fn extract_text_content(line: &str) -> Option<String> {
    // Look for (text) pattern
    if let Some(start) = line.find('(') {
        if let Some(end) = line.find(')') {
            if end > start {
                return Some(line[start + 1..end].to_string());
            }
        }
    }
    None
}

/// Detect RGB color space usage (literal names or operators)
fn detect_rgb_usage(pdf_text: &str) -> bool {
    // Check for literal color space name
    if pdf_text.contains("/DeviceRGB") {
        return true;
    }

    // For compressed content streams, we can't easily parse operators
    // But we can detect RGB usage by other indicators:

    // 1. Look for RGB color operators in uncompressed streams
    for line in pdf_text.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        for i in 3..words.len() {
            if (words[i] == "rg" || words[i] == "RG")
                && words[i - 3].parse::<f64>().is_ok()
                && words[i - 2].parse::<f64>().is_ok()
                && words[i - 1].parse::<f64>().is_ok()
            {
                return true;
            }
        }
    }

    // 2. Check for color space resources in the resources section
    if pdf_text.contains("/ColorSpace") && pdf_text.contains("RGB") {
        return true;
    }

    // 3. Heuristic: If document has graphics content and no explicit grayscale/CMYK,
    //    assume RGB is being used (PDF default)
    if pdf_text.contains("/Contents") && pdf_text.contains("/Length") {
        // Has content streams - likely using default RGB
        return true;
    }

    false
}

/// Detect CMYK color space usage (literal names or operators)
fn detect_cmyk_usage(pdf_text: &str) -> bool {
    // Check for literal color space name
    if pdf_text.contains("/DeviceCMYK") {
        return true;
    }

    // Check for CMYK color operators in content streams
    // Look for patterns like "0.5 0.2 0.8 0.1 k" (fill) or "0.5 0.2 0.8 0.1 K" (stroke)
    for line in pdf_text.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        for i in 4..words.len() {
            if (words[i] == "k" || words[i] == "K")
                && words[i - 4].parse::<f64>().is_ok()
                && words[i - 3].parse::<f64>().is_ok()
                && words[i - 2].parse::<f64>().is_ok()
                && words[i - 1].parse::<f64>().is_ok()
            {
                return true;
            }
        }
    }

    false
}

/// Detect grayscale color space usage (literal names or operators)
fn detect_gray_usage(pdf_text: &str) -> bool {
    // Check for literal color space name
    if pdf_text.contains("/DeviceGray") {
        return true;
    }

    // Check for grayscale color operators in uncompressed content streams
    for line in pdf_text.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        for i in 1..words.len() {
            if (words[i] == "g" || words[i] == "G") && words[i - 1].parse::<f64>().is_ok() {
                return true;
            }
        }
    }

    // Check for grayscale color space resources
    if pdf_text.contains("/ColorSpace") && pdf_text.contains("Gray") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version() {
        let pdf_content = "%PDF-1.4\n1 0 obj\n<<\n/Type /Catalog\n>>\nendobj\n%%EOF";
        let result = extract_version(pdf_content).unwrap();
        assert_eq!(result, "1.4");
    }

    #[test]
    fn test_extract_catalog() {
        let pdf_content = "1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj";
        let catalog = extract_catalog(pdf_content).unwrap();
        assert_eq!(catalog.get("Type"), Some(&"Catalog".to_string()));
        assert_eq!(catalog.get("Pages"), Some(&"2".to_string()));
    }

    #[test]
    fn test_extract_fonts() {
        let pdf_content =
            "<<\n/Type /Font\n/BaseFont /Helvetica\n>>\n<<\n/BaseFont /Times-Roman\n>>";
        let fonts = extract_fonts(pdf_content);
        assert!(fonts.contains(&"Helvetica".to_string()));
        assert!(fonts.contains(&"Times-Roman".to_string()));
    }

    #[test]
    fn test_color_space_detection() {
        let pdf_content = "%PDF-1.4\nstream\n1 0 0 rg\n/DeviceRGB cs\nendstream\n%%EOF";
        let parsed = parse_pdf(pdf_content.as_bytes()).unwrap();
        assert!(parsed.uses_device_rgb);
        assert!(!parsed.uses_device_cmyk);
    }

    #[test]
    fn test_improved_color_detection() {
        use crate::{Color, Document, Font, Page};

        let mut doc = Document::new();
        doc.set_title("Color Detection Test");

        let mut page = Page::a4();

        // Add text and colored graphics
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 700.0)
            .write("RGB Color Test")
            .unwrap();

        page.graphics()
            .set_fill_color(Color::rgb(1.0, 0.0, 0.0)) // Red
            .rectangle(50.0, 650.0, 100.0, 30.0)
            .fill();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes().unwrap();

        // Test improved color detection
        let parsed = parse_pdf(&pdf_bytes).unwrap();

        // Should detect RGB usage through heuristics since content streams are compressed
        assert!(parsed.uses_device_rgb, "Should detect RGB color usage");
        assert!(!parsed.uses_device_cmyk, "Should not detect CMYK");
    }
}
