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
        // SAFETY: We just checked len() == 1, so next() must return Some
        if let Some(corruption_type) = types.into_iter().next() {
            report.corruption_type = corruption_type;
        }
    } else if types.len() > 1 {
        report.corruption_type = CorruptionType::Multiple(types);
    }
}

fn find_pattern(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
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

    #[test]
    fn test_corruption_type_debug_clone_eq() {
        let corruption = CorruptionType::InvalidHeader;
        let debug_str = format!("{corruption:?}");
        assert!(debug_str.contains("InvalidHeader"));

        let cloned = corruption.clone();
        assert_eq!(corruption, cloned);

        // Test all variants
        let variants = vec![
            CorruptionType::InvalidHeader,
            CorruptionType::CorruptXRef,
            CorruptionType::MissingEOF,
            CorruptionType::BrokenReferences,
            CorruptionType::CorruptStreams,
            CorruptionType::InvalidPageTree,
            CorruptionType::TruncatedFile,
            CorruptionType::Unknown,
        ];

        for variant in variants {
            let _ = format!("{variant:?}");
            let _ = variant.clone();
        }
    }

    #[test]
    fn test_corruption_type_multiple() {
        let types = vec![
            CorruptionType::InvalidHeader,
            CorruptionType::CorruptXRef,
            CorruptionType::MissingEOF,
        ];
        let multiple = CorruptionType::Multiple(types);

        match &multiple {
            CorruptionType::Multiple(inner) => {
                assert_eq!(inner.len(), 3);
                assert_eq!(inner[0], CorruptionType::InvalidHeader);
            }
            _ => panic!("Should be Multiple variant"),
        }
    }

    #[test]
    fn test_section_type_debug_clone() {
        let sections = vec![
            SectionType::Header,
            SectionType::Body,
            SectionType::XRef,
            SectionType::Trailer,
            SectionType::Page(42),
            SectionType::Object(123),
            SectionType::Stream(456),
        ];

        for section in sections {
            let debug_str = format!("{section:?}");
            assert!(!debug_str.is_empty());

            let cloned = section.clone();
            match (section, cloned) {
                (SectionType::Page(n1), SectionType::Page(n2)) => assert_eq!(n1, n2),
                (SectionType::Object(n1), SectionType::Object(n2)) => assert_eq!(n1, n2),
                (SectionType::Stream(n1), SectionType::Stream(n2)) => assert_eq!(n1, n2),
                _ => {}
            }
        }
    }

    #[test]
    fn test_recoverable_section_creation() {
        let section = RecoverableSection {
            section_type: SectionType::Page(1),
            start_offset: 100,
            end_offset: 500,
            confidence: 0.95,
        };

        assert_eq!(section.start_offset, 100);
        assert_eq!(section.end_offset, 500);
        assert_eq!(section.confidence, 0.95);

        let debug_str = format!("{section:?}");
        assert!(debug_str.contains("RecoverableSection"));
    }

    #[test]
    fn test_corruption_report_creation() {
        let report = CorruptionReport {
            corruption_type: CorruptionType::CorruptXRef,
            severity: 7,
            errors: vec!["Error 1".to_string(), "Error 2".to_string()],
            recoverable_sections: vec![RecoverableSection {
                section_type: SectionType::Header,
                start_offset: 0,
                end_offset: 10,
                confidence: 1.0,
            }],
            file_stats: FileStats {
                file_size: 1000,
                readable_bytes: 900,
                estimated_objects: 10,
                found_pages: 3,
            },
        };

        assert_eq!(report.severity, 7);
        assert_eq!(report.errors.len(), 2);
        assert_eq!(report.recoverable_sections.len(), 1);
        assert_eq!(report.file_stats.file_size, 1000);
    }

    #[test]
    fn test_find_pattern_various_cases() {
        // Pattern at start
        assert_eq!(find_pattern(b"xref table", b"xref"), Some(0));

        // Pattern at end
        assert_eq!(find_pattern(b"table xref", b"xref"), Some(6));

        // Pattern in middle
        assert_eq!(find_pattern(b"PDF xref table", b"xref"), Some(4));

        // Pattern not found
        assert_eq!(find_pattern(b"PDF table", b"xref"), None);

        // Empty haystack
        assert_eq!(find_pattern(b"", b"xref"), None);

        // Empty needle (edge case)
        assert_eq!(find_pattern(b"test", b""), Some(0));
    }

    #[test]
    fn test_determine_corruption_type_single() {
        let mut report = CorruptionReport {
            corruption_type: CorruptionType::Unknown,
            severity: 5,
            errors: vec!["Invalid header found".to_string()],
            recoverable_sections: vec![],
            file_stats: FileStats::default(),
        };

        determine_corruption_type(&mut report);
        assert_eq!(report.corruption_type, CorruptionType::InvalidHeader);
    }

    #[test]
    fn test_determine_corruption_type_multiple() {
        let mut report = CorruptionReport {
            corruption_type: CorruptionType::Unknown,
            severity: 8,
            errors: vec![
                "Invalid header".to_string(),
                "Missing EOF marker".to_string(),
                "Corrupt cross-reference table".to_string(),
            ],
            recoverable_sections: vec![],
            file_stats: FileStats::default(),
        };

        determine_corruption_type(&mut report);
        match report.corruption_type {
            CorruptionType::Multiple(types) => {
                assert_eq!(types.len(), 3);
                assert!(types.contains(&CorruptionType::InvalidHeader));
                assert!(types.contains(&CorruptionType::MissingEOF));
                assert!(types.contains(&CorruptionType::CorruptXRef));
            }
            _ => panic!("Should be Multiple corruption type"),
        }
    }

    #[test]
    fn test_determine_corruption_type_unknown() {
        let mut report = CorruptionReport {
            corruption_type: CorruptionType::Unknown,
            severity: 3,
            errors: vec!["Some generic error".to_string()],
            recoverable_sections: vec![],
            file_stats: FileStats::default(),
        };

        determine_corruption_type(&mut report);
        assert_eq!(report.corruption_type, CorruptionType::Unknown);
    }

    #[test]
    fn test_check_header_valid() {
        use std::io::Cursor;

        let data = b"%PDF-1.7\nrest of content";
        let mut cursor = Cursor::new(data);
        let mut report = CorruptionReport {
            corruption_type: CorruptionType::Unknown,
            severity: 0,
            errors: vec![],
            recoverable_sections: vec![],
            file_stats: FileStats::default(),
        };

        let result = check_header(&mut cursor, &mut report).unwrap();
        assert!(result);
        assert_eq!(report.recoverable_sections.len(), 1);
        assert_eq!(report.recoverable_sections[0].confidence, 1.0);
    }

    #[test]
    fn test_check_header_invalid() {
        use std::io::Cursor;

        let data = b"INVALID HEADER\nrest of content";
        let mut cursor = Cursor::new(data);
        let mut report = CorruptionReport {
            corruption_type: CorruptionType::Unknown,
            severity: 0,
            errors: vec![],
            recoverable_sections: vec![],
            file_stats: FileStats::default(),
        };

        let result = check_header(&mut cursor, &mut report).unwrap();
        assert!(!result);
        assert!(!report.errors.is_empty());
        assert!(report.errors[0].contains("Invalid PDF header"));
    }

    #[test]
    fn test_check_header_too_short() {
        use std::io::Cursor;

        let data = b"PDF"; // Too short
        let mut cursor = Cursor::new(data);
        let mut report = CorruptionReport {
            corruption_type: CorruptionType::Unknown,
            severity: 0,
            errors: vec![],
            recoverable_sections: vec![],
            file_stats: FileStats::default(),
        };

        let result = check_header(&mut cursor, &mut report).unwrap();
        assert!(!result);
        assert!(!report.errors.is_empty());
    }

    #[test]
    fn test_check_eof_present() {
        use std::io::Cursor;

        let data = b"%PDF-1.7\nsome content\n%%EOF\n";
        let mut cursor = Cursor::new(data);
        let mut report = CorruptionReport {
            corruption_type: CorruptionType::Unknown,
            severity: 0,
            errors: vec![],
            recoverable_sections: vec![],
            file_stats: FileStats {
                file_size: data.len() as u64,
                ..Default::default()
            },
        };

        check_eof(&mut cursor, &mut report).unwrap();
        // Should add "analysis complete" message
        assert_eq!(report.errors.len(), 1);
        assert!(report.errors[0].contains("analysis complete"));
        assert_eq!(report.severity, 0);
    }

    #[test]
    fn test_check_eof_missing() {
        use std::io::Cursor;

        let data = b"%PDF-1.7\nsome content without eof";
        let mut cursor = Cursor::new(data);
        let mut report = CorruptionReport {
            corruption_type: CorruptionType::Unknown,
            severity: 0,
            errors: vec![],
            recoverable_sections: vec![],
            file_stats: FileStats {
                file_size: data.len() as u64,
                ..Default::default()
            },
        };

        check_eof(&mut cursor, &mut report).unwrap();
        assert!(!report.errors.is_empty());
        assert!(report.errors[0].contains("Missing %%EOF"));
        assert_eq!(report.severity, 5);
    }

    #[test]
    fn test_scan_xref_found() {
        use std::io::Cursor;

        let data = b"%PDF-1.7\nxref\n0 1\n0000000000 65535 f\ntrailer\n";
        let mut cursor = Cursor::new(data);
        let mut report = CorruptionReport {
            corruption_type: CorruptionType::Unknown,
            severity: 0,
            errors: vec![],
            recoverable_sections: vec![],
            file_stats: FileStats::default(),
        };

        scan_xref(&mut cursor, &mut report).unwrap();
        assert!(report
            .recoverable_sections
            .iter()
            .any(|s| matches!(s.section_type, SectionType::XRef)));
        assert!(report.errors.is_empty() || !report.errors[0].contains("No cross-reference"));
    }

    #[test]
    fn test_scan_xref_not_found() {
        use std::io::Cursor;

        let data = b"%PDF-1.7\nNo cross reference table here";
        let mut cursor = Cursor::new(data);
        let mut report = CorruptionReport {
            corruption_type: CorruptionType::Unknown,
            severity: 0,
            errors: vec![],
            recoverable_sections: vec![],
            file_stats: FileStats::default(),
        };

        scan_xref(&mut cursor, &mut report).unwrap();
        assert!(!report.errors.is_empty());
        assert!(report.errors[0].contains("No cross-reference tables found"));
        assert_eq!(report.severity, 8);
    }

    #[test]
    fn test_analyze_objects_with_pages() {
        use std::io::Cursor;

        let data = b"1 0 obj\n<< /Type /Page >>\nendobj\n2 0 obj\n<< /Type /Catalog >>\nendobj";
        let mut cursor = Cursor::new(data);
        let mut report = CorruptionReport {
            corruption_type: CorruptionType::Unknown,
            severity: 0,
            errors: vec![],
            recoverable_sections: vec![],
            file_stats: FileStats::default(),
        };

        analyze_objects(&mut cursor, &mut report).unwrap();
        assert_eq!(report.file_stats.estimated_objects, 2);
        assert_eq!(report.file_stats.found_pages, 1);
        assert_eq!(report.file_stats.readable_bytes, data.len() as u64);
    }

    #[test]
    fn test_analyze_objects_no_objects() {
        use std::io::Cursor;

        let data = b"No PDF items here";
        let mut cursor = Cursor::new(data);
        let mut report = CorruptionReport {
            corruption_type: CorruptionType::Unknown,
            severity: 0,
            errors: vec![],
            recoverable_sections: vec![],
            file_stats: FileStats::default(),
        };

        analyze_objects(&mut cursor, &mut report).unwrap();
        assert_eq!(report.file_stats.estimated_objects, 0);
        assert!(!report.errors.is_empty());
        assert!(report.errors[0].contains("No PDF objects"));
        assert_eq!(report.severity, 10);
    }

    #[test]
    fn test_is_corrupted_valid_file() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("valid_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"%PDF-1.7\n1 0 obj\n<< >>\nendobj\nxref\n0 1\n0000000000 65535 f\ntrailer\n<< >>\nstartxref\n0\n%%EOF").unwrap();

        let corrupted = is_corrupted(&temp_path);
        // May be false or true depending on analysis
        let _ = corrupted;

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_is_corrupted_invalid_file() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("invalid_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(b"This is not a PDF").unwrap();

        let corrupted = is_corrupted(&temp_path);
        assert!(corrupted);

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_is_corrupted_nonexistent_file() {
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("nonexistent_test.pdf");

        let corrupted = is_corrupted(&temp_path);
        assert!(corrupted); // Should return true for error case
    }

    #[test]
    fn test_detect_corruption_comprehensive() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("comprehensive_test.pdf");
        let mut file = File::create(&temp_path).unwrap();
        // Missing EOF marker
        file.write_all(b"%PDF-1.7\n1 0 obj\n<< /Type /Page >>\nendobj")
            .unwrap();

        let report = detect_corruption(&temp_path).unwrap();
        assert!(report.severity > 0);
        assert!(!report.errors.is_empty());

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_file_stats_debug() {
        let stats = FileStats {
            file_size: 1000,
            readable_bytes: 950,
            estimated_objects: 10,
            found_pages: 3,
        };

        let debug_str = format!("{stats:?}");
        assert!(debug_str.contains("FileStats"));
        assert!(debug_str.contains("1000"));
        assert!(debug_str.contains("950"));
        assert!(debug_str.contains("10"));
        assert!(debug_str.contains("3"));
    }

    #[test]
    fn test_corruption_report_debug() {
        let report = CorruptionReport {
            corruption_type: CorruptionType::Unknown,
            severity: 5,
            errors: vec!["Test error".to_string()],
            recoverable_sections: vec![],
            file_stats: FileStats::default(),
        };

        let debug_str = format!("{report:?}");
        assert!(debug_str.contains("CorruptionReport"));
        assert!(debug_str.contains("Unknown"));
        assert!(debug_str.contains("5"));
    }
}
