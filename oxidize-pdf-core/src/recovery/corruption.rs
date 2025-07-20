//! PDF corruption detection and analysis

use crate::error::Result;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// Types of PDF corruption
#[derive(Debug, Clone, PartialEq)]
pub enum CorruptionType {
    /// Missing or invalid PDF header
    InvalidHeader,
    /// Corrupted cross-reference table
    CorruptXRef,
    /// Missing EOF marker
    MissingEOF,
    /// Invalid object references
    BrokenReferences,
    /// Corrupted content streams
    CorruptStreams,
    /// Invalid page tree
    InvalidPageTree,
    /// Truncated file
    TruncatedFile,
    /// Multiple corruption types
    Multiple(Vec<CorruptionType>),
    /// Unknown corruption
    Unknown,
}

/// Corruption analysis report
#[derive(Debug)]
pub struct CorruptionReport {
    /// Primary corruption type
    pub corruption_type: CorruptionType,
    /// Severity level (0-10)
    pub severity: u8,
    /// Detailed error messages
    pub errors: Vec<String>,
    /// Recoverable sections
    pub recoverable_sections: Vec<RecoverableSection>,
    /// File statistics
    pub file_stats: FileStats,
}

/// A potentially recoverable section
#[derive(Debug)]
pub struct RecoverableSection {
    /// Section type
    pub section_type: SectionType,
    /// Start offset in file
    pub start_offset: u64,
    /// End offset in file
    pub end_offset: u64,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
}

/// Types of PDF sections
#[derive(Debug, Clone)]
pub enum SectionType {
    Header,
    Body,
    XRef,
    Trailer,
    Page(u32),
    Object(u32),
    Stream(u32),
}

/// File statistics
#[derive(Debug, Default)]
pub struct FileStats {
    /// Total file size
    pub file_size: u64,
    /// Number of readable bytes
    pub readable_bytes: u64,
    /// Estimated object count
    pub estimated_objects: usize,
    /// Found page references
    pub found_pages: usize,
}

/// Detect corruption in a PDF file
pub fn detect_corruption<P: AsRef<Path>>(path: P) -> Result<CorruptionReport> {
    let mut file = File::open(path)?;
    let mut reader = BufReader::new(&mut file);

    let file_size = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(0))?;

    let mut report = CorruptionReport {
        corruption_type: CorruptionType::Unknown,
        severity: 0,
        errors: Vec::new(),
        recoverable_sections: Vec::new(),
        file_stats: FileStats {
            file_size,
            ..Default::default()
        },
    };

    // Check PDF header
    if !check_header(&mut reader, &mut report)? {
        report.corruption_type = CorruptionType::InvalidHeader;
        report.severity = 10;
        return Ok(report);
    }

    // Check for EOF marker
    check_eof(&mut reader, &mut report)?;

    // Scan for cross-reference tables
    scan_xref(&mut reader, &mut report)?;

    // Analyze object structure
    analyze_objects(&mut reader, &mut report)?;

    // Determine overall corruption type
    determine_corruption_type(&mut report);

    Ok(report)
}

fn check_header<R: Read + Seek>(reader: &mut R, report: &mut CorruptionReport) -> Result<bool> {
    let mut header = [0u8; 8];
    reader.seek(SeekFrom::Start(0))?;

    match reader.read_exact(&mut header) {
        Ok(_) => {
            if &header[0..5] == b"%PDF-" {
                report.recoverable_sections.push(RecoverableSection {
                    section_type: SectionType::Header,
                    start_offset: 0,
                    end_offset: 8,
                    confidence: 1.0,
                });
                Ok(true)
            } else {
                report.errors.push("Invalid PDF header".to_string());
                Ok(false)
            }
        }
        Err(e) => {
            report.errors.push(format!("Cannot read header: {e}"));
            Ok(false)
        }
    }
}

fn check_eof<R: Read + Seek>(reader: &mut R, report: &mut CorruptionReport) -> Result<()> {
    // Check last 1024 bytes for %%EOF
    let check_size = 1024.min(report.file_stats.file_size);
    let start_pos = report.file_stats.file_size.saturating_sub(check_size);

    reader.seek(SeekFrom::Start(start_pos))?;
    let mut buffer = vec![0u8; check_size as usize];
    reader.read_exact(&mut buffer)?;

    if !buffer.windows(5).any(|w| w == b"%%EOF") {
        report.errors.push("Missing %%EOF marker".to_string());
        report.severity = report.severity.max(5);
    }

    // Always report something for analysis
    if report.errors.is_empty() && report.severity == 0 {
        report
            .errors
            .push("PDF structure analysis complete".to_string());
    }

    Ok(())
}

fn scan_xref<R: Read + Seek>(reader: &mut R, report: &mut CorruptionReport) -> Result<()> {
    reader.seek(SeekFrom::Start(0))?;
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    // Look for xref tables
    let mut xref_count = 0;
    let mut pos = 0;

    while let Some(xref_pos) = find_pattern(&buffer[pos..], b"xref") {
        let absolute_pos = pos + xref_pos;
        xref_count += 1;

        report.recoverable_sections.push(RecoverableSection {
            section_type: SectionType::XRef,
            start_offset: absolute_pos as u64,
            end_offset: (absolute_pos + 100) as u64, // Estimate
            confidence: 0.8,
        });

        pos = absolute_pos + 4;
    }

    if xref_count == 0 {
        report
            .errors
            .push("No cross-reference tables found".to_string());
        report.severity = report.severity.max(8);
    }

    Ok(())
}

fn analyze_objects<R: Read + Seek>(reader: &mut R, report: &mut CorruptionReport) -> Result<()> {
    reader.seek(SeekFrom::Start(0))?;
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    // Count objects
    let mut object_count = 0;
    let mut page_count = 0;
    let mut pos = 0;

    // Look for object definitions
    while pos < buffer.len() {
        if let Some(obj_pos) = find_pattern(&buffer[pos..], b" obj") {
            let absolute_pos = pos + obj_pos;
            object_count += 1;

            // Check if it's a page object
            let check_end = (absolute_pos + 200).min(buffer.len());
            if find_pattern(&buffer[absolute_pos..check_end], b"/Type /Page").is_some() {
                page_count += 1;
            }

            pos = absolute_pos + 4;
        } else {
            break;
        }
    }

    report.file_stats.estimated_objects = object_count;
    report.file_stats.found_pages = page_count;
    report.file_stats.readable_bytes = buffer.len() as u64;

    if object_count == 0 {
        report.errors.push("No PDF objects found".to_string());
        report.severity = 10;
    }

    Ok(())
}

fn determine_corruption_type(report: &mut CorruptionReport) {
    let mut types = Vec::new();

    for error in &report.errors {
        if error.contains("header") {
            types.push(CorruptionType::InvalidHeader);
        } else if error.contains("EOF") {
            types.push(CorruptionType::MissingEOF);
        } else if error.contains("cross-reference") || error.contains("xref") {
            types.push(CorruptionType::CorruptXRef);
        }
    }

    if types.is_empty() && report.severity > 0 {
        report.corruption_type = CorruptionType::Unknown;
    } else if types.len() == 1 {
        report.corruption_type = types.into_iter().next().unwrap();
    } else if types.len() > 1 {
        report.corruption_type = CorruptionType::Multiple(types);
    }
}

fn find_pattern(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Quick corruption check
pub fn is_corrupted<P: AsRef<Path>>(path: P) -> bool {
    detect_corruption(path)
        .map(|report| report.severity > 0)
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corruption_type() {
        let corruption = CorruptionType::InvalidHeader;
        assert_eq!(corruption, CorruptionType::InvalidHeader);

        let multiple = CorruptionType::Multiple(vec![
            CorruptionType::InvalidHeader,
            CorruptionType::CorruptXRef,
        ]);
        assert!(matches!(multiple, CorruptionType::Multiple(_)));
    }

    #[test]
    fn test_find_pattern() {
        let haystack = b"Hello PDF world";
        assert_eq!(find_pattern(haystack, b"PDF"), Some(6));
        assert_eq!(find_pattern(haystack, b"XYZ"), None);
    }

    #[test]
    fn test_file_stats_default() {
        let stats = FileStats::default();
        assert_eq!(stats.file_size, 0);
        assert_eq!(stats.readable_bytes, 0);
        assert_eq!(stats.estimated_objects, 0);
        assert_eq!(stats.found_pages, 0);
    }
}
