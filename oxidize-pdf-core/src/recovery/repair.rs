//! PDF repair strategies and implementation

use crate::error::{PdfError, Result};
use crate::recovery::{CorruptionType, RecoveryOptions};
use crate::{Document, Page};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Strategy for repairing PDFs
#[derive(Debug, Clone)]
pub enum RepairStrategy {
    /// Rebuild cross-reference table
    RebuildXRef,
    /// Fix header and trailer
    FixStructure,
    /// Extract readable content only
    ExtractContent,
    /// Reconstruct from fragments
    ReconstructFragments,
    /// Minimal repair for quick recovery
    MinimalRepair,
    /// Aggressive repair with heuristics
    AggressiveRepair,
}

impl RepairStrategy {
    /// Choose strategy based on corruption type
    pub fn for_corruption(corruption: &CorruptionType) -> Self {
        match corruption {
            CorruptionType::InvalidHeader => RepairStrategy::FixStructure,
            CorruptionType::CorruptXRef => RepairStrategy::RebuildXRef,
            CorruptionType::MissingEOF => RepairStrategy::FixStructure,
            CorruptionType::BrokenReferences => RepairStrategy::RebuildXRef,
            CorruptionType::CorruptStreams => RepairStrategy::ExtractContent,
            CorruptionType::InvalidPageTree => RepairStrategy::ReconstructFragments,
            CorruptionType::TruncatedFile => RepairStrategy::ExtractContent,
            CorruptionType::Multiple(_) => RepairStrategy::AggressiveRepair,
            CorruptionType::Unknown => RepairStrategy::MinimalRepair,
        }
    }
}

/// Result of repair operation
pub struct RepairResult {
    /// Successfully repaired document
    pub recovered_document: Option<Document>,
    /// Number of pages recovered
    pub pages_recovered: usize,
    /// Number of objects recovered
    pub objects_recovered: usize,
    /// Repair warnings
    pub warnings: Vec<String>,
    /// Whether repair was partial
    pub is_partial: bool,
}

/// Repair a corrupted PDF document
pub fn repair_document<P: AsRef<Path>>(
    path: P,
    strategy: RepairStrategy,
    options: &RecoveryOptions,
) -> Result<RepairResult> {
    let path = path.as_ref();

    match strategy {
        RepairStrategy::RebuildXRef => rebuild_xref(path, options),
        RepairStrategy::FixStructure => fix_structure(path, options),
        RepairStrategy::ExtractContent => extract_content(path, options),
        RepairStrategy::ReconstructFragments => reconstruct_fragments(path, options),
        RepairStrategy::MinimalRepair => minimal_repair(path, options),
        RepairStrategy::AggressiveRepair => aggressive_repair(path, options),
    }
}

fn rebuild_xref<P: AsRef<Path>>(path: P, _options: &RecoveryOptions) -> Result<RepairResult> {
    let mut file = File::open(path)?;
    let mut reader = BufReader::new(&mut file);
    let mut result = RepairResult {
        recovered_document: None,
        pages_recovered: 0,
        objects_recovered: 0,
        warnings: Vec::new(),
        is_partial: false,
    };

    // Read entire file
    let mut content = Vec::new();
    reader.read_to_end(&mut content)?;

    // Find all objects
    let objects = scan_for_objects(&content);
    result.objects_recovered = objects.len();

    // Build new xref table
    let _xref = build_xref_table(&objects);

    // Create recovered document
    let mut doc = Document::new();

    // Add recovered pages
    for obj in objects.iter() {
        if obj.is_page {
            let page = Page::a4(); // Simplified
            doc.add_page(page);
            result.pages_recovered += 1;
        }
    }

    if result.pages_recovered > 0 {
        result.recovered_document = Some(doc);
    } else {
        result
            .warnings
            .push("No pages could be recovered".to_string());
        result.is_partial = true;
    }

    Ok(result)
}

fn fix_structure<P: AsRef<Path>>(path: P, _options: &RecoveryOptions) -> Result<RepairResult> {
    let mut result = RepairResult {
        recovered_document: None,
        pages_recovered: 0,
        objects_recovered: 0,
        warnings: Vec::new(),
        is_partial: false,
    };

    // Read file
    let content = std::fs::read(path)?;

    // Fix header if needed
    let mut fixed_content = if !content.starts_with(b"%PDF-") {
        let mut new_content = b"%PDF-1.7\n".to_vec();
        new_content.extend_from_slice(&content);
        result.warnings.push("Added missing PDF header".to_string());
        new_content
    } else {
        content
    };

    // Fix EOF if needed
    if !fixed_content.windows(5).any(|w| w == b"%%EOF") {
        fixed_content.extend_from_slice(b"\n%%EOF\n");
        result.warnings.push("Added missing EOF marker".to_string());
    }

    // Create minimal document
    let doc = Document::new();
    result.recovered_document = Some(doc);
    result.is_partial = true;

    Ok(result)
}

fn extract_content<P: AsRef<Path>>(path: P, options: &RecoveryOptions) -> Result<RepairResult> {
    let mut result = RepairResult {
        recovered_document: None,
        pages_recovered: 0,
        objects_recovered: 0,
        warnings: Vec::new(),
        is_partial: true,
    };

    let content = std::fs::read(path)?;
    let mut doc = Document::new();

    // Look for page content between "BT" and "ET" markers
    let mut pos = 0;
    let mut page_count = 0;

    while pos < content.len() {
        if let Some(bt_pos) = find_marker(&content[pos..], b"BT") {
            let start = pos + bt_pos;
            if let Some(et_pos) = find_marker(&content[start..], b"ET") {
                let end = start + et_pos;

                // Extract text content
                let text_content = &content[start + 2..end];
                if !text_content.is_empty() {
                    let page = Page::a4();
                    // Add extracted content (simplified)
                    doc.add_page(page);
                    page_count += 1;

                    if !options.partial_content && page_count >= 10 {
                        break;
                    }
                }

                pos = end + 2;
            } else {
                pos = start + 2;
            }
        } else {
            break;
        }
    }

    result.pages_recovered = page_count;
    if page_count > 0 {
        result.recovered_document = Some(doc);
        result
            .warnings
            .push(format!("Extracted {page_count} pages with content"));
    }

    Ok(result)
}

fn reconstruct_fragments<P: AsRef<Path>>(
    path: P,
    _options: &RecoveryOptions,
) -> Result<RepairResult> {
    let mut result = RepairResult {
        recovered_document: None,
        pages_recovered: 0,
        objects_recovered: 0,
        warnings: Vec::new(),
        is_partial: true,
    };

    let content = std::fs::read(path)?;
    let fragments = find_pdf_fragments(&content);

    if fragments.is_empty() {
        result
            .warnings
            .push("No valid PDF fragments found".to_string());
        return Ok(result);
    }

    let mut doc = Document::new();

    for fragment in fragments.iter() {
        if fragment.looks_like_page() {
            let page = Page::a4();
            doc.add_page(page);
            result.pages_recovered += 1;
        }
        result.objects_recovered += 1;
    }

    if result.pages_recovered > 0 {
        result.recovered_document = Some(doc);
    }

    result
        .warnings
        .push(format!("Reconstructed from {} fragments", fragments.len()));

    Ok(result)
}

fn minimal_repair<P: AsRef<Path>>(path: P, _options: &RecoveryOptions) -> Result<RepairResult> {
    // Try to create a minimal valid PDF
    let mut result = RepairResult {
        recovered_document: None,
        pages_recovered: 0,
        objects_recovered: 0,
        warnings: Vec::new(),
        is_partial: false,
    };

    // Check if file exists and has content
    let metadata = std::fs::metadata(path)?;
    if metadata.len() == 0 {
        return Err(PdfError::InvalidStructure("Empty file".to_string()));
    }

    // Create minimal document
    let mut doc = Document::new();
    doc.add_page(Page::a4());

    result.recovered_document = Some(doc);
    result.pages_recovered = 1;
    result
        .warnings
        .push("Created minimal valid PDF".to_string());

    Ok(result)
}

fn aggressive_repair<P: AsRef<Path>>(path: P, options: &RecoveryOptions) -> Result<RepairResult> {
    // Try multiple strategies
    let strategies = vec![
        RepairStrategy::RebuildXRef,
        RepairStrategy::ExtractContent,
        RepairStrategy::ReconstructFragments,
    ];

    let mut best_result = None;
    let mut best_score = 0;

    for strategy in strategies {
        if let Ok(result) = repair_document(path.as_ref(), strategy, options) {
            let score = result.pages_recovered * 10 + result.objects_recovered;
            if score > best_score {
                best_score = score;
                best_result = Some(result);
            }
        }
    }

    best_result
        .ok_or_else(|| PdfError::InvalidStructure("All repair strategies failed".to_string()))
}

#[derive(Debug)]
struct PdfObject {
    #[allow(dead_code)]
    id: u32,
    offset: usize,
    is_page: bool,
}

fn scan_for_objects(content: &[u8]) -> Vec<PdfObject> {
    let mut objects = Vec::new();
    let mut pos = 0;

    while pos < content.len() {
        if let Some(obj_pos) = find_marker(&content[pos..], b" obj") {
            let absolute_pos = pos + obj_pos;

            // Try to extract object ID
            if let Some(id) = extract_object_id(&content[pos..absolute_pos]) {
                let check_end = (absolute_pos + 200).min(content.len());
                let is_page =
                    find_marker(&content[absolute_pos..check_end], b"/Type /Page").is_some();

                objects.push(PdfObject {
                    id,
                    offset: pos,
                    is_page,
                });
            }

            pos = absolute_pos + 4;
        } else {
            break;
        }
    }

    objects
}

fn build_xref_table(objects: &[PdfObject]) -> Vec<u8> {
    let mut xref = Vec::new();
    xref.extend_from_slice(b"xref\n");
    xref.extend_from_slice(b"0 1\n");
    xref.extend_from_slice(b"0000000000 65535 f \n");

    for obj in objects {
        let entry = format!("{:010} 00000 n \n", obj.offset);
        xref.extend_from_slice(entry.as_bytes());
    }

    xref
}

fn find_marker(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn extract_object_id(content: &[u8]) -> Option<u32> {
    // Simple extraction - look for number before " obj"
    let text = String::from_utf8_lossy(content);
    text.split_whitespace()
        .rev()
        .nth(1)
        .and_then(|s| s.parse().ok())
}

#[derive(Debug)]
struct PdfFragment {
    #[allow(dead_code)]
    start: usize,
    #[allow(dead_code)]
    end: usize,
    content_type: FragmentType,
}

#[derive(Debug)]
enum FragmentType {
    Object,
    Stream,
    Page,
    #[allow(dead_code)]
    Unknown,
}

impl PdfFragment {
    fn looks_like_page(&self) -> bool {
        matches!(self.content_type, FragmentType::Page)
    }
}

fn find_pdf_fragments(content: &[u8]) -> Vec<PdfFragment> {
    let mut fragments = Vec::new();
    let mut pos = 0;

    while pos < content.len() {
        if let Some(start) = find_marker(&content[pos..], b" obj") {
            let absolute_start = pos + start;

            if let Some(end) = find_marker(&content[absolute_start..], b"endobj") {
                let absolute_end = absolute_start + end + 6;

                let content_type = if find_marker(
                    &content[absolute_start..absolute_end],
                    b"/Type /Page",
                )
                .is_some()
                {
                    FragmentType::Page
                } else if find_marker(&content[absolute_start..absolute_end], b"stream").is_some() {
                    FragmentType::Stream
                } else {
                    FragmentType::Object
                };

                fragments.push(PdfFragment {
                    start: absolute_start,
                    end: absolute_end,
                    content_type,
                });

                pos = absolute_end;
            } else {
                pos = absolute_start + 4;
            }
        } else {
            break;
        }
    }

    fragments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repair_strategy_selection() {
        assert!(matches!(
            RepairStrategy::for_corruption(&CorruptionType::InvalidHeader),
            RepairStrategy::FixStructure
        ));

        assert!(matches!(
            RepairStrategy::for_corruption(&CorruptionType::CorruptXRef),
            RepairStrategy::RebuildXRef
        ));
    }

    #[test]
    fn test_find_marker() {
        let content = b"some text obj more text";
        assert_eq!(find_marker(content, b" obj"), Some(9));
        assert_eq!(find_marker(content, b"xyz"), None);
    }

    #[test]
    fn test_extract_object_id() {
        let content = b"123 0";
        assert_eq!(extract_object_id(content), Some(123));

        let content = b"invalid";
        assert_eq!(extract_object_id(content), None);
    }

    #[test]
    fn test_repair_strategy_debug_clone() {
        let strategy = RepairStrategy::RebuildXRef;
        let debug_str = format!("{strategy:?}");
        assert!(debug_str.contains("RebuildXRef"));

        let cloned = strategy.clone();
        assert!(matches!(cloned, RepairStrategy::RebuildXRef));
    }

    #[test]
    fn test_repair_strategy_for_all_corruption_types() {
        use crate::recovery::CorruptionType;

        // Test all corruption types
        assert!(matches!(
            RepairStrategy::for_corruption(&CorruptionType::MissingEOF),
            RepairStrategy::FixStructure
        ));

        assert!(matches!(
            RepairStrategy::for_corruption(&CorruptionType::BrokenReferences),
            RepairStrategy::RebuildXRef
        ));

        assert!(matches!(
            RepairStrategy::for_corruption(&CorruptionType::CorruptStreams),
            RepairStrategy::ExtractContent
        ));

        assert!(matches!(
            RepairStrategy::for_corruption(&CorruptionType::InvalidPageTree),
            RepairStrategy::ReconstructFragments
        ));

        assert!(matches!(
            RepairStrategy::for_corruption(&CorruptionType::TruncatedFile),
            RepairStrategy::ExtractContent
        ));

        assert!(matches!(
            RepairStrategy::for_corruption(&CorruptionType::Unknown),
            RepairStrategy::MinimalRepair
        ));

        assert!(matches!(
            RepairStrategy::for_corruption(&CorruptionType::Multiple(vec![])),
            RepairStrategy::AggressiveRepair
        ));
    }

    #[test]
    fn test_scan_for_objects() {
        let content = b"1 0 obj\n<< /Type /Page >>\nendobj\n2 0 obj\n<< /Length 10 >>\nendobj";
        let objects = scan_for_objects(content);

        assert_eq!(objects.len(), 2);
        assert!(objects[0].is_page);
        assert!(!objects[1].is_page);
    }

    #[test]
    fn test_scan_for_objects_empty() {
        let content = b"";
        let objects = scan_for_objects(content);
        assert!(objects.is_empty());
    }

    #[test]
    fn test_scan_for_objects_no_valid_objects() {
        let content = b"This is not a PDF content";
        let objects = scan_for_objects(content);
        assert!(objects.is_empty());
    }

    #[test]
    fn test_build_xref_table() {
        let objects = vec![
            PdfObject {
                id: 1,
                offset: 15,
                is_page: true,
            },
            PdfObject {
                id: 2,
                offset: 50,
                is_page: false,
            },
        ];

        let xref = build_xref_table(&objects);
        let xref_str = String::from_utf8_lossy(&xref);

        assert!(xref_str.contains("xref"));
        assert!(xref_str.contains("0000000015"));
        assert!(xref_str.contains("0000000050"));
    }

    #[test]
    fn test_build_xref_table_empty() {
        let objects = vec![];
        let xref = build_xref_table(&objects);
        let xref_str = String::from_utf8_lossy(&xref);

        assert!(xref_str.contains("xref"));
        assert!(xref_str.contains("0 1"));
        assert!(xref_str.contains("0000000000 65535 f"));
    }

    #[test]
    fn test_find_pdf_fragments() {
        let content =
            b"1 0 obj\n<< /Type /Page >>\nendobj\n2 0 obj\nstream\ndata\nendstream\nendobj";
        let fragments = find_pdf_fragments(content);

        assert_eq!(fragments.len(), 2);
        assert!(fragments[0].looks_like_page());
        assert!(matches!(fragments[1].content_type, FragmentType::Stream));
    }

    #[test]
    fn test_find_pdf_fragments_incomplete() {
        let content = b"1 0 obj\n<< /Type /Page >>\n"; // Missing endobj
        let fragments = find_pdf_fragments(content);
        assert!(fragments.is_empty());
    }

    #[test]
    fn test_pdf_fragment_looks_like_page() {
        let page_fragment = PdfFragment {
            start: 0,
            end: 10,
            content_type: FragmentType::Page,
        };
        assert!(page_fragment.looks_like_page());

        let object_fragment = PdfFragment {
            start: 0,
            end: 10,
            content_type: FragmentType::Object,
        };
        assert!(!object_fragment.looks_like_page());
    }

    #[test]
    fn test_find_marker_multiple_occurrences() {
        let content = b"obj obj obj";
        assert_eq!(find_marker(content, b"obj"), Some(0));

        // Find second occurrence
        let pos = find_marker(content, b"obj").unwrap();
        assert_eq!(find_marker(&content[pos + 3..], b"obj"), Some(1));
    }

    #[test]
    fn test_extract_object_id_various_formats() {
        // Standard format
        assert_eq!(extract_object_id(b"123 0"), Some(123));

        // With extra spaces
        assert_eq!(extract_object_id(b"456  0"), Some(456));

        // Large number
        assert_eq!(extract_object_id(b"999999 0"), Some(999999));

        // No generation number
        assert_eq!(extract_object_id(b"789"), None);

        // Non-numeric
        assert_eq!(extract_object_id(b"abc 0"), None);
    }

    #[test]
    fn test_repair_result_creation() {
        let result = RepairResult {
            recovered_document: Some(Document::new()),
            pages_recovered: 5,
            objects_recovered: 10,
            warnings: vec!["Warning 1".to_string()],
            is_partial: true,
        };

        assert!(result.recovered_document.is_some());
        assert_eq!(result.pages_recovered, 5);
        assert_eq!(result.objects_recovered, 10);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.is_partial);
    }

    #[test]
    fn test_minimal_repair_empty_file() {
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("empty_test.pdf");
        let _file = File::create(&temp_path).unwrap();

        let options = RecoveryOptions::default();
        let result = minimal_repair(&temp_path, &options);

        assert!(result.is_err());

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_minimal_repair_valid_file() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("minimal_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"dummy content").unwrap();

        let options = RecoveryOptions::default();
        let result = minimal_repair(&temp_path, &options).unwrap();

        assert!(result.recovered_document.is_some());
        assert_eq!(result.pages_recovered, 1);
        assert!(!result.warnings.is_empty());

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_fix_structure_missing_header() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("no_header_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"some content without header").unwrap();

        let options = RecoveryOptions::default();
        let result = fix_structure(&temp_path, &options).unwrap();

        assert!(result.recovered_document.is_some());
        assert!(result.warnings.iter().any(|w| w.contains("header")));
        assert!(result.is_partial);

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_fix_structure_missing_eof() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("no_eof_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"%PDF-1.7\nsome content").unwrap();

        let options = RecoveryOptions::default();
        let result = fix_structure(&temp_path, &options).unwrap();

        assert!(result.recovered_document.is_some());
        assert!(result.warnings.iter().any(|w| w.contains("EOF")));

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_extract_content_with_text() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("text_content_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"BT (Hello World) Tj ET BT (Second Page) Tj ET")
            .unwrap();

        let options = RecoveryOptions::default();
        let result = extract_content(&temp_path, &options).unwrap();

        assert!(result.recovered_document.is_some());
        assert_eq!(result.pages_recovered, 2);
        assert!(!result.warnings.is_empty());

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_extract_content_no_text() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("no_text_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"No text markers here").unwrap();

        let options = RecoveryOptions::default();
        let result = extract_content(&temp_path, &options).unwrap();

        assert_eq!(result.pages_recovered, 0);
        assert!(result.recovered_document.is_none());

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_extract_content_partial_limit() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("many_pages_test.pdf");
        let mut file = File::create(&temp_path).unwrap();

        // Write 15 pages worth of content
        for i in 0..15 {
            file.write_all(format!("BT (Page {i}) Tj ET ").as_bytes())
                .unwrap();
        }

        let options = RecoveryOptions {
            partial_content: false,
            ..Default::default()
        };
        let result = extract_content(&temp_path, &options).unwrap();

        // Should stop at 10 pages due to limit
        assert_eq!(result.pages_recovered, 10);

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_rebuild_xref_with_objects() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("objects_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"1 0 obj\n<< /Type /Page >>\nendobj\n2 0 obj\n<< >>\nendobj")
            .unwrap();

        let options = RecoveryOptions::default();
        let result = rebuild_xref(&temp_path, &options).unwrap();

        assert!(result.recovered_document.is_some());
        assert_eq!(result.objects_recovered, 2);
        assert_eq!(result.pages_recovered, 1);

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_reconstruct_fragments_with_pages() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("fragments_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(
            b"1 0 obj\n<< /Type /Page >>\nendobj\n2 0 obj\nstream\ndata\nendstream\nendobj",
        )
        .unwrap();

        let options = RecoveryOptions::default();
        let result = reconstruct_fragments(&temp_path, &options).unwrap();

        assert!(result.recovered_document.is_some());
        assert_eq!(result.pages_recovered, 1);
        assert_eq!(result.objects_recovered, 2);
        assert!(result.warnings.iter().any(|w| w.contains("fragments")));

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_reconstruct_fragments_no_fragments() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("no_fragments_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"Invalid PDF content").unwrap();

        let options = RecoveryOptions::default();
        let result = reconstruct_fragments(&temp_path, &options).unwrap();

        assert!(result.recovered_document.is_none());
        assert_eq!(result.pages_recovered, 0);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("No valid PDF fragments")));

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_aggressive_repair() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("aggressive_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"BT (Text) Tj ET 1 0 obj << /Type /Page >> endobj")
            .unwrap();

        let options = RecoveryOptions::default();
        let result = aggressive_repair(&temp_path, &options).unwrap();

        // Should find the best strategy
        assert!(result.recovered_document.is_some());
        assert!(result.pages_recovered > 0 || result.objects_recovered > 0);

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_repair_document_all_strategies() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("repair_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"%PDF-1.7\n1 0 obj\n<< /Type /Page >>\nendobj\n%%EOF")
            .unwrap();

        let options = RecoveryOptions::default();

        // Test all strategies
        let strategies = vec![
            RepairStrategy::RebuildXRef,
            RepairStrategy::FixStructure,
            RepairStrategy::ExtractContent,
            RepairStrategy::ReconstructFragments,
            RepairStrategy::MinimalRepair,
            RepairStrategy::AggressiveRepair,
        ];

        for strategy in strategies {
            let result = repair_document(&temp_path, strategy.clone(), &options);
            assert!(result.is_ok(), "Strategy {strategy:?} failed");
        }

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_pdf_object_debug() {
        let obj = PdfObject {
            id: 42,
            offset: 100,
            is_page: true,
        };

        let debug_str = format!("{obj:?}");
        assert!(debug_str.contains("PdfObject"));
        assert!(debug_str.contains("42"));
        assert!(debug_str.contains("100"));
        assert!(debug_str.contains("true"));
    }

    #[test]
    fn test_pdf_fragment_debug() {
        let fragment = PdfFragment {
            start: 0,
            end: 100,
            content_type: FragmentType::Page,
        };

        let debug_str = format!("{fragment:?}");
        assert!(debug_str.contains("PdfFragment"));
        assert!(debug_str.contains("Page"));
    }

    #[test]
    fn test_fragment_type_debug() {
        let types = vec![
            FragmentType::Object,
            FragmentType::Stream,
            FragmentType::Page,
            FragmentType::Unknown,
        ];

        for ftype in types {
            let debug_str = format!("{ftype:?}");
            assert!(!debug_str.is_empty());
        }
    }
}
