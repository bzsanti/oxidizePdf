//! PDF object scanner for recovery operations

use crate::error::Result;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Scanner for finding valid PDF objects
pub struct ObjectScanner {
    /// Found objects indexed by ID
    objects: HashMap<u32, ScannedObject>,
    /// Current scan statistics
    stats: ScanStats,
}

/// A scanned PDF object
#[derive(Debug, Clone)]
pub struct ScannedObject {
    /// Object ID
    pub id: u32,
    /// Generation number
    pub generation: u16,
    /// File offset
    pub offset: u64,
    /// Object type if detected
    pub object_type: Option<ObjectType>,
    /// Whether object appears valid
    pub is_valid: bool,
}

/// Types of PDF objects
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectType {
    Page,
    Pages,
    Catalog,
    Font,
    Image,
    Stream,
    Dictionary,
    Array,
    Other(String),
}

/// Scan statistics
#[derive(Debug, Default, Clone)]
pub struct ScanStats {
    /// Total bytes scanned
    pub bytes_scanned: u64,
    /// Number of objects found
    pub objects_found: usize,
    /// Number of valid objects
    pub valid_objects: usize,
    /// Number of pages found
    pub pages_found: usize,
    /// Scan duration in milliseconds
    pub duration_ms: u64,
}

/// Result of scanning operation
#[derive(Debug)]
pub struct ScanResult {
    /// All found objects
    pub objects: Vec<ScannedObject>,
    /// Total objects found
    pub total_objects: usize,
    /// Valid objects
    pub valid_objects: usize,
    /// Estimated page count
    pub estimated_pages: u32,
    /// Scan statistics
    pub stats: ScanStats,
}

impl Default for ObjectScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectScanner {
    /// Create a new object scanner
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            stats: ScanStats::default(),
        }
    }

    /// Scan a file for PDF objects
    pub fn scan_file<P: AsRef<Path>>(&mut self, path: P) -> Result<ScanResult> {
        let start_time = std::time::Instant::now();

        let mut file = File::open(path)?;
        let mut reader = BufReader::new(&mut file);

        // Read file in chunks
        let mut buffer = vec![0u8; 1024 * 1024]; // 1MB chunks
        let mut file_offset = 0u64;

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    self.scan_buffer(&buffer[..n], file_offset)?;
                    file_offset += n as u64;
                    self.stats.bytes_scanned += n as u64;
                }
                Err(e) => return Err(e.into()),
            }
        }

        self.stats.duration_ms = start_time.elapsed().as_millis() as u64;

        // Build result
        let mut objects: Vec<_> = self.objects.values().cloned().collect();
        objects.sort_by_key(|obj| obj.id);

        let result = ScanResult {
            total_objects: objects.len(),
            valid_objects: objects.iter().filter(|o| o.is_valid).count(),
            estimated_pages: self.stats.pages_found as u32,
            objects,
            stats: self.stats.clone(),
        };

        Ok(result)
    }

    /// Scan a buffer for objects
    fn scan_buffer(&mut self, buffer: &[u8], base_offset: u64) -> Result<()> {
        let mut pos = 0;

        while pos < buffer.len() {
            // Look for object pattern: "N G obj"
            if let Some(obj_start) = find_object_start(&buffer[pos..]) {
                let absolute_pos = pos + obj_start;

                // Try to parse object header
                if let Some((id, gen)) = parse_object_header(&buffer[pos..absolute_pos]) {
                    let object_offset = base_offset + pos as u64;

                    // Scan object content
                    let object_type = detect_object_type(&buffer[absolute_pos..]);
                    let is_valid = validate_object(&buffer[absolute_pos..]);

                    if object_type == Some(ObjectType::Page) {
                        self.stats.pages_found += 1;
                    }

                    let scanned_obj = ScannedObject {
                        id,
                        generation: gen,
                        offset: object_offset,
                        object_type,
                        is_valid,
                    };

                    self.objects.insert(id, scanned_obj);
                    self.stats.objects_found += 1;

                    if is_valid {
                        self.stats.valid_objects += 1;
                    }
                }

                pos = absolute_pos + 4; // Skip past " obj"
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Get scan statistics
    pub fn stats(&self) -> &ScanStats {
        &self.stats
    }

    /// Reset scanner state
    pub fn reset(&mut self) {
        self.objects.clear();
        self.stats = ScanStats::default();
    }
}

fn find_object_start(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b" obj")
        .map(|pos| pos + 1) // Return position after the space in " obj"
}

fn parse_object_header(buffer: &[u8]) -> Option<(u32, u16)> {
    // Look backwards for "ID GEN obj" pattern
    let text = std::str::from_utf8(buffer).ok()?;
    let parts: Vec<&str> = text.split_whitespace().collect();

    if parts.len() >= 2 {
        let id = parts[parts.len() - 2].parse().ok()?;
        let gen = parts[parts.len() - 1].parse().ok()?;
        Some((id, gen))
    } else {
        None
    }
}

fn detect_object_type(content: &[u8]) -> Option<ObjectType> {
    // Simple type detection based on content
    if content.is_empty() {
        return None;
    }

    let text = String::from_utf8_lossy(&content[..content.len().min(200)]);

    if text.contains("/Type /Page") && !text.contains("/Type /Pages") {
        Some(ObjectType::Page)
    } else if text.contains("/Type /Pages") {
        Some(ObjectType::Pages)
    } else if text.contains("/Type /Catalog") {
        Some(ObjectType::Catalog)
    } else if text.contains("/Type /Font") {
        Some(ObjectType::Font)
    } else if text.contains("/Subtype /Image") {
        Some(ObjectType::Image)
    } else if text.contains("stream") {
        Some(ObjectType::Stream)
    } else if text.starts_with("<<") {
        Some(ObjectType::Dictionary)
    } else if text.starts_with('[') {
        Some(ObjectType::Array)
    } else {
        None
    }
}

fn validate_object(content: &[u8]) -> bool {
    // Check if object has proper ending
    if content.len() < 10 {
        return false;
    }

    // Look for endobj
    content.windows(6).any(|window| window == b"endobj")
}

/// Quick scan for basic file info
pub fn quick_scan<P: AsRef<Path>>(path: P) -> Result<ScanResult> {
    let mut scanner = ObjectScanner::new();
    scanner.scan_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_scanner_creation() {
        let scanner = ObjectScanner::new();
        assert_eq!(scanner.stats.objects_found, 0);
        assert_eq!(scanner.stats.valid_objects, 0);
    }

    #[test]
    fn test_find_object_start() {
        let buffer = b"some text 1 0 obj content";
        assert_eq!(find_object_start(buffer), Some(14));

        let buffer = b"this has no pdf data";
        assert_eq!(find_object_start(buffer), None);
    }

    #[test]
    fn test_parse_object_header() {
        let buffer = b"1 0";
        assert_eq!(parse_object_header(buffer), Some((1, 0)));

        let buffer = b"123 5";
        assert_eq!(parse_object_header(buffer), Some((123, 5)));
    }

    #[test]
    fn test_detect_object_type() {
        let page = b"<< /Type /Page /Parent 2 0 R >>";
        assert_eq!(detect_object_type(page), Some(ObjectType::Page));

        let catalog = b"<< /Type /Catalog /Pages 2 0 R >>";
        assert_eq!(detect_object_type(catalog), Some(ObjectType::Catalog));

        let stream = b"<< /Length 100 >> stream";
        assert_eq!(detect_object_type(stream), Some(ObjectType::Stream));
    }

    #[test]
    fn test_validate_object() {
        let valid = b"<< /Type /Page >> endobj";
        assert!(validate_object(valid));

        let invalid = b"<< /Type /Page >>";
        assert!(!validate_object(invalid));
    }

    #[test]
    fn test_object_scanner_default() {
        let scanner = ObjectScanner::default();
        assert_eq!(scanner.stats.objects_found, 0);
        assert_eq!(scanner.stats.valid_objects, 0);
        assert_eq!(scanner.stats.bytes_scanned, 0);
        assert_eq!(scanner.stats.pages_found, 0);
        assert_eq!(scanner.stats.duration_ms, 0);
    }

    #[test]
    fn test_scanned_object_creation() {
        let obj = ScannedObject {
            id: 42,
            generation: 0,
            offset: 1024,
            object_type: Some(ObjectType::Page),
            is_valid: true,
        };

        assert_eq!(obj.id, 42);
        assert_eq!(obj.generation, 0);
        assert_eq!(obj.offset, 1024);
        assert_eq!(obj.object_type, Some(ObjectType::Page));
        assert!(obj.is_valid);
    }

    #[test]
    fn test_scanned_object_debug_clone() {
        let obj = ScannedObject {
            id: 123,
            generation: 5,
            offset: 2048,
            object_type: Some(ObjectType::Font),
            is_valid: false,
        };

        let debug_str = format!("{:?}", obj);
        assert!(debug_str.contains("ScannedObject"));
        assert!(debug_str.contains("123"));

        let cloned = obj.clone();
        assert_eq!(cloned.id, obj.id);
        assert_eq!(cloned.generation, obj.generation);
        assert_eq!(cloned.offset, obj.offset);
        assert_eq!(cloned.object_type, obj.object_type);
        assert_eq!(cloned.is_valid, obj.is_valid);
    }

    #[test]
    fn test_object_type_variants() {
        let types = vec![
            ObjectType::Page,
            ObjectType::Pages,
            ObjectType::Catalog,
            ObjectType::Font,
            ObjectType::Image,
            ObjectType::Stream,
            ObjectType::Dictionary,
            ObjectType::Array,
            ObjectType::Other("Custom".to_string()),
        ];

        for obj_type in types {
            let debug_str = format!("{:?}", obj_type);
            assert!(!debug_str.is_empty());

            let cloned = obj_type.clone();
            assert_eq!(obj_type, cloned);
        }
    }

    #[test]
    fn test_object_type_equality() {
        assert_eq!(ObjectType::Page, ObjectType::Page);
        assert_ne!(ObjectType::Page, ObjectType::Pages);
        assert_ne!(ObjectType::Font, ObjectType::Image);

        assert_eq!(
            ObjectType::Other("Test".to_string()),
            ObjectType::Other("Test".to_string())
        );
        assert_ne!(
            ObjectType::Other("Test1".to_string()),
            ObjectType::Other("Test2".to_string())
        );
    }

    #[test]
    fn test_scan_stats_default() {
        let stats = ScanStats::default();
        assert_eq!(stats.bytes_scanned, 0);
        assert_eq!(stats.objects_found, 0);
        assert_eq!(stats.valid_objects, 0);
        assert_eq!(stats.pages_found, 0);
        assert_eq!(stats.duration_ms, 0);
    }

    #[test]
    fn test_scan_stats_debug_clone() {
        let stats = ScanStats {
            bytes_scanned: 1024 * 1024,
            objects_found: 100,
            valid_objects: 95,
            pages_found: 10,
            duration_ms: 250,
        };

        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("ScanStats"));

        let cloned = stats.clone();
        assert_eq!(cloned.bytes_scanned, stats.bytes_scanned);
        assert_eq!(cloned.objects_found, stats.objects_found);
        assert_eq!(cloned.valid_objects, stats.valid_objects);
        assert_eq!(cloned.pages_found, stats.pages_found);
        assert_eq!(cloned.duration_ms, stats.duration_ms);
    }

    #[test]
    fn test_scan_result_creation() {
        let objects = vec![
            ScannedObject {
                id: 1,
                generation: 0,
                offset: 100,
                object_type: Some(ObjectType::Catalog),
                is_valid: true,
            },
            ScannedObject {
                id: 2,
                generation: 0,
                offset: 200,
                object_type: Some(ObjectType::Pages),
                is_valid: true,
            },
        ];

        let result = ScanResult {
            objects: objects.clone(),
            total_objects: 2,
            valid_objects: 2,
            estimated_pages: 1,
            stats: ScanStats::default(),
        };

        assert_eq!(result.objects.len(), 2);
        assert_eq!(result.total_objects, 2);
        assert_eq!(result.valid_objects, 2);
        assert_eq!(result.estimated_pages, 1);
    }

    #[test]
    fn test_scan_result_debug() {
        let result = ScanResult {
            objects: vec![],
            total_objects: 0,
            valid_objects: 0,
            estimated_pages: 0,
            stats: ScanStats::default(),
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("ScanResult"));
    }

    #[test]
    fn test_scanner_reset() {
        let mut scanner = ObjectScanner::new();

        // Add some data
        scanner.objects.insert(
            1,
            ScannedObject {
                id: 1,
                generation: 0,
                offset: 100,
                object_type: Some(ObjectType::Page),
                is_valid: true,
            },
        );
        scanner.stats.objects_found = 5;
        scanner.stats.valid_objects = 4;
        scanner.stats.bytes_scanned = 1000;

        assert!(!scanner.objects.is_empty());
        assert_eq!(scanner.stats.objects_found, 5);

        // Reset
        scanner.reset();

        assert!(scanner.objects.is_empty());
        assert_eq!(scanner.stats.objects_found, 0);
        assert_eq!(scanner.stats.valid_objects, 0);
        assert_eq!(scanner.stats.bytes_scanned, 0);
    }

    #[test]
    fn test_detect_object_type_all_types() {
        // Test Pages type (must check before Page)
        let pages = b"<< /Type /Pages /Kids [1 0 R 2 0 R] >>";
        assert_eq!(detect_object_type(pages), Some(ObjectType::Pages));

        // Test Font
        let font = b"<< /Type /Font /Subtype /Type1 >>";
        assert_eq!(detect_object_type(font), Some(ObjectType::Font));

        // Test Image
        let image = b"<< /Subtype /Image /Width 100 >>";
        assert_eq!(detect_object_type(image), Some(ObjectType::Image));

        // Test Dictionary
        let dict = b"<< /Key /Value >>";
        assert_eq!(detect_object_type(dict), Some(ObjectType::Dictionary));

        // Test Array
        let array = b"[1 2 3 4]";
        assert_eq!(detect_object_type(array), Some(ObjectType::Array));

        // Test short content
        let short = b"abc";
        assert_eq!(detect_object_type(short), None);

        // Test empty
        let empty = b"";
        assert_eq!(detect_object_type(empty), None);
    }

    #[test]
    fn test_validate_object_various_cases() {
        // Valid with spaces
        assert!(validate_object(b"<< /Type /Page >>   endobj   "));

        // Valid with newlines
        assert!(validate_object(b"<< /Type /Page >>\nendobj"));

        // Valid in middle of content
        assert!(validate_object(
            b"<< /Type /Page >> stuff endobj more stuff"
        ));

        // Invalid - too short
        assert!(!validate_object(b"short"));

        // Invalid - no endobj
        assert!(!validate_object(b"<< /Type /Page >> no end marker"));

        // Invalid - partial endobj
        assert!(!validate_object(b"<< /Type /Page >> endob"));
    }

    #[test]
    fn test_parse_object_header_edge_cases() {
        // Normal case
        assert_eq!(parse_object_header(b"42 0"), Some((42, 0)));

        // With extra whitespace
        assert_eq!(parse_object_header(b"  42   0  "), Some((42, 0)));

        // With newlines
        assert_eq!(parse_object_header(b"42\n0"), Some((42, 0)));

        // Large numbers
        assert_eq!(parse_object_header(b"999999 65535"), Some((999999, 65535)));

        // Invalid - not numbers
        assert_eq!(parse_object_header(b"abc def"), None);

        // Invalid - only one number
        assert_eq!(parse_object_header(b"42"), None);

        // Invalid - empty
        assert_eq!(parse_object_header(b""), None);

        // Invalid - non-UTF8
        assert_eq!(parse_object_header(&[0xFF, 0xFE, 0xFD]), None);
    }

    #[test]
    fn test_find_object_start_multiple() {
        let buffer = b"first obj at 9 obj and another obj";
        assert_eq!(find_object_start(buffer), Some(6)); // First " obj"

        // Find next occurrence
        let next_search = &buffer[7..];
        assert_eq!(find_object_start(next_search), Some(8));

        // No obj pattern
        assert_eq!(find_object_start(b"no_object_pattern_here"), None);

        // obj at the very end
        assert_eq!(find_object_start(b"ends with obj"), Some(10));

        // obj at the beginning
        assert_eq!(find_object_start(b" obj starts here"), Some(1));
    }

    #[test]
    fn test_scanner_stats_access() {
        let scanner = ObjectScanner::new();
        let stats = scanner.stats();
        assert_eq!(stats.objects_found, 0);
        assert_eq!(stats.valid_objects, 0);
        assert_eq!(stats.bytes_scanned, 0);
        assert_eq!(stats.pages_found, 0);
        assert_eq!(stats.duration_ms, 0);
    }

    #[test]
    fn test_quick_scan_nonexistent_file() {
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("nonexistent_scanner_test.pdf");

        let result = quick_scan(&temp_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_file_empty() {
        use std::fs::File;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("empty_scan_test.pdf");
        let _file = File::create(&temp_path).unwrap();

        let mut scanner = ObjectScanner::new();
        let result = scanner.scan_file(&temp_path).unwrap();

        assert_eq!(result.total_objects, 0);
        assert_eq!(result.valid_objects, 0);
        assert_eq!(result.estimated_pages, 0);
        assert!(result.objects.is_empty());

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_scan_file_with_objects() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("objects_scan_test.pdf");
        let mut file = File::create(&temp_path).unwrap();

        // Write some PDF-like content
        file.write_all(b"%PDF-1.7\n").unwrap();
        file.write_all(b"1 0 obj\n<< /Type /Catalog >>\nendobj\n")
            .unwrap();
        file.write_all(b"2 0 obj\n<< /Type /Pages >>\nendobj\n")
            .unwrap();
        file.write_all(b"3 0 obj\n<< /Type /Page >>\nendobj\n")
            .unwrap();
        file.write_all(b"%%EOF").unwrap();

        let result = quick_scan(&temp_path).unwrap();

        assert!(result.total_objects > 0);
        assert_eq!(result.estimated_pages, 1); // One page found
        assert!(!result.objects.is_empty());

        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_scan_buffer_various_objects() {
        let mut scanner = ObjectScanner::new();

        let buffer = b"1 0 obj\n<< /Type /Page >>\nendobj\n2 0 obj\n<< /Type /Font >>\nendobj";
        scanner.scan_buffer(buffer, 0).unwrap();

        assert_eq!(scanner.stats.objects_found, 2);
        assert_eq!(scanner.stats.valid_objects, 2);
        assert_eq!(scanner.stats.pages_found, 1);
        assert_eq!(scanner.objects.len(), 2);
    }

    #[test]
    fn test_scan_buffer_invalid_objects() {
        let mut scanner = ObjectScanner::new();

        // Objects without endobj
        let buffer = b"1 0 obj\n<< /Type /Page >>\n2 0 obj\n<< /Type /Font >>";
        scanner.scan_buffer(buffer, 100).unwrap();

        assert_eq!(scanner.stats.objects_found, 2);
        assert_eq!(scanner.stats.valid_objects, 0); // No valid objects (missing endobj)
        assert_eq!(scanner.stats.pages_found, 1); // Page type still detected
    }

    #[test]
    fn test_scanned_object_with_offset() {
        let obj1 = ScannedObject {
            id: 1,
            generation: 0,
            offset: 0,
            object_type: Some(ObjectType::Catalog),
            is_valid: true,
        };

        let obj2 = ScannedObject {
            id: 2,
            generation: 0,
            offset: 1024,
            object_type: Some(ObjectType::Page),
            is_valid: true,
        };

        assert!(obj1.offset < obj2.offset);
        assert_ne!(obj1.object_type, obj2.object_type);
    }

    #[test]
    fn test_object_type_other_variant() {
        let other1 = ObjectType::Other("CustomType".to_string());
        let other2 = ObjectType::Other("CustomType".to_string());
        let other3 = ObjectType::Other("DifferentType".to_string());

        assert_eq!(other1, other2);
        assert_ne!(other1, other3);

        match &other1 {
            ObjectType::Other(name) => assert_eq!(name, "CustomType"),
            _ => panic!("Expected Other variant"),
        }
    }

    #[test]
    fn test_scan_result_sorting() {
        let mut objects = vec![
            ScannedObject {
                id: 3,
                generation: 0,
                offset: 300,
                object_type: Some(ObjectType::Page),
                is_valid: true,
            },
            ScannedObject {
                id: 1,
                generation: 0,
                offset: 100,
                object_type: Some(ObjectType::Catalog),
                is_valid: true,
            },
            ScannedObject {
                id: 2,
                generation: 0,
                offset: 200,
                object_type: Some(ObjectType::Pages),
                is_valid: true,
            },
        ];

        // Simulate what scan_file does
        objects.sort_by_key(|obj| obj.id);

        assert_eq!(objects[0].id, 1);
        assert_eq!(objects[1].id, 2);
        assert_eq!(objects[2].id, 3);
    }
}
