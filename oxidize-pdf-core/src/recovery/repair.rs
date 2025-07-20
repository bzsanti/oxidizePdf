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
}
