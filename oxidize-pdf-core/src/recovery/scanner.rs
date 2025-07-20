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
    if content.len() < 20 {
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
}
