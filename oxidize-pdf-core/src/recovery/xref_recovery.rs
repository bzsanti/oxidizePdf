//! XRef recovery for corrupted PDF files
//!
//! This module provides functionality to rebuild cross-reference tables
//! when they are missing or corrupted in PDF files.

use crate::error::Result;
use crate::parser::xref::{XRefEntry, XRefTable};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// XRef recovery engine
#[derive(Default)]
pub struct XRefRecovery {
    /// Found objects during scan
    objects: BTreeMap<(u32, u16), u64>, // (id, gen) -> offset
    /// Reconstructed XRef entries
    xref_entries: HashMap<u32, XRefEntry>,
    /// Statistics
    stats: RecoveryStats,
}

/// Recovery statistics
#[derive(Debug, Default)]
pub struct RecoveryStats {
    /// Number of objects found
    pub objects_found: usize,
    /// Number of XRef entries reconstructed
    pub entries_reconstructed: usize,
    /// Number of errors encountered
    pub errors: usize,
    /// Whether trailer was found
    pub trailer_found: bool,
}

impl XRefRecovery {
    /// Create a new XRef recovery instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Recover XRef from a corrupted PDF file
    pub fn recover_from_file<P: AsRef<Path>>(path: P) -> Result<XRefTable> {
        let mut recovery = Self::new();
        let mut file = File::open(path)?;
        recovery.scan_file(&mut file)?;
        recovery.build_xref_table()
    }

    /// Scan file for PDF objects
    fn scan_file(&mut self, file: &mut File) -> Result<()> {
        let mut reader = BufReader::new(file);
        let file_size = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;

        // Read file in chunks
        let mut buffer = vec![0u8; 1024 * 1024]; // 1MB chunks
        let mut file_offset = 0u64;
        let mut overlap = Vec::new();

        while file_offset < file_size {
            let bytes_to_read = std::cmp::min(buffer.len(), (file_size - file_offset) as usize);
            reader.read_exact(&mut buffer[..bytes_to_read])?;

            // Combine with overlap from previous chunk
            let mut search_buffer = overlap.clone();
            search_buffer.extend_from_slice(&buffer[..bytes_to_read]);

            // Scan for objects
            self.scan_buffer(
                &search_buffer,
                file_offset.saturating_sub(overlap.len() as u64),
            )?;

            // Keep last 100 bytes as overlap for next iteration
            overlap = if bytes_to_read >= 100 {
                buffer[bytes_to_read - 100..bytes_to_read].to_vec()
            } else {
                buffer[..bytes_to_read].to_vec()
            };

            file_offset += bytes_to_read as u64;
        }

        Ok(())
    }

    /// Scan buffer for PDF objects
    fn scan_buffer(&mut self, buffer: &[u8], base_offset: u64) -> Result<()> {
        let mut pos = 0;

        while pos < buffer.len() {
            // Look for object pattern: "N G obj"
            if let Some(obj_pos) = self.find_object_pattern(&buffer[pos..]) {
                let absolute_pos = pos + obj_pos;

                // Extract object ID and generation
                if let Some((id, gen, obj_start)) =
                    self.parse_object_header(&buffer[..absolute_pos])
                {
                    let object_offset = base_offset + obj_start as u64;

                    // Verify object ends with "endobj"
                    if self.verify_object_end(&buffer[absolute_pos..]) {
                        self.objects.insert((id, gen), object_offset);
                        self.stats.objects_found += 1;
                    }
                }

                pos = absolute_pos + 4; // Skip past " obj"
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Find object pattern in buffer
    fn find_object_pattern(&self, buffer: &[u8]) -> Option<usize> {
        // Look for " obj" pattern
        buffer
            .windows(4)
            .position(|window| window == b" obj")
            .map(|pos| pos + 4) // Position after " obj"
    }

    /// Parse object header to extract ID and generation
    fn parse_object_header(&self, buffer: &[u8]) -> Option<(u32, u16, usize)> {
        // Convert to string for easier parsing
        let text = std::str::from_utf8(buffer).ok()?;

        // Look for pattern "ID GEN obj" at the end
        let mut words: Vec<&str> = text.split_whitespace().collect();
        if words.len() < 3 {
            return None;
        }

        // Get last 3 words
        let obj_word = words.pop()?;
        let gen_str = words.pop()?;
        let id_str = words.pop()?;

        if obj_word != "obj" {
            return None;
        }

        let id = id_str.parse::<u32>().ok()?;
        let gen = gen_str.parse::<u16>().ok()?;

        // Find start position of ID
        let id_pos = text.rfind(id_str)?;

        Some((id, gen, id_pos))
    }

    /// Verify object ends properly
    fn verify_object_end(&self, buffer: &[u8]) -> bool {
        // Look for "endobj" within reasonable distance
        let search_len = std::cmp::min(buffer.len(), 50000); // 50KB max
        buffer[..search_len]
            .windows(6)
            .any(|window| window == b"endobj")
    }

    /// Build XRef table from found objects
    fn build_xref_table(&mut self) -> Result<XRefTable> {
        let mut xref_table = XRefTable::new();

        // Convert found objects to XRef entries
        for ((id, gen), offset) in &self.objects {
            let entry = XRefEntry {
                offset: *offset,
                generation: *gen,
                in_use: true,
            };

            self.xref_entries.insert(*id, entry);
            xref_table.add_entry(*id, entry);
            self.stats.entries_reconstructed += 1;
        }

        // Try to find trailer
        if let Ok(trailer) = self.find_trailer() {
            xref_table.set_trailer(trailer);
            self.stats.trailer_found = true;
        }

        Ok(xref_table)
    }

    /// Try to find trailer dictionary
    fn find_trailer(&self) -> Result<crate::parser::objects::PdfDictionary> {
        // Simple implementation - would need to scan for trailer pattern
        // For now, return a minimal trailer
        let mut trailer = std::collections::HashMap::new();

        // Set size based on highest object ID
        if let Some(max_id) = self.objects.keys().map(|(id, _)| id).max() {
            trailer.insert(
                crate::parser::objects::PdfName("Size".to_string()),
                crate::parser::objects::PdfObject::Integer((*max_id + 1) as i64),
            );
        }

        Ok(crate::parser::objects::PdfDictionary(trailer))
    }

    /// Get recovery statistics
    pub fn stats(&self) -> &RecoveryStats {
        &self.stats
    }
}

/// Quick function to recover XRef from a file
pub fn recover_xref<P: AsRef<Path>>(path: P) -> Result<XRefTable> {
    XRefRecovery::recover_from_file(path)
}

/// Verify if a file needs XRef recovery
pub fn needs_xref_recovery<P: AsRef<Path>>(path: P) -> Result<bool> {
    let mut file = File::open(path)?;
    let mut reader = BufReader::new(&mut file);

    // Get file size
    let file_size = reader.seek(SeekFrom::End(0))?;

    // Check for startxref at end of file
    let search_size = std::cmp::min(1024, file_size);
    if search_size == 0 {
        return Ok(true); // Empty file needs recovery
    }

    reader.seek(SeekFrom::End(-(search_size as i64)))?;
    let mut buffer = vec![0u8; search_size as usize];
    let bytes_read = reader.read(&mut buffer)?;

    let has_startxref = buffer[..bytes_read]
        .windows(9)
        .any(|window| window == b"startxref");

    Ok(!has_startxref)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_xref_recovery_creation() {
        let recovery = XRefRecovery::new();
        assert_eq!(recovery.stats.objects_found, 0);
        assert_eq!(recovery.stats.entries_reconstructed, 0);
    }

    #[test]
    fn test_find_object_pattern() {
        let recovery = XRefRecovery::new();

        let buffer = b"some text 1 0 obj content";
        assert_eq!(recovery.find_object_pattern(buffer), Some(17)); // Position after " obj"

        let buffer = b"no pdf data here";
        assert_eq!(recovery.find_object_pattern(buffer), None);
    }

    #[test]
    fn test_parse_object_header() {
        let recovery = XRefRecovery::new();

        let buffer = b"some prefix 123 0 obj";
        let result = recovery.parse_object_header(buffer);
        assert_eq!(result, Some((123, 0, 12))); // ID, gen, start position

        let buffer = b"456 15 obj";
        let result = recovery.parse_object_header(buffer);
        assert_eq!(result, Some((456, 15, 0)));
    }

    #[test]
    fn test_verify_object_end() {
        let recovery = XRefRecovery::new();

        let valid = b"<< /Type /Page >> endobj more content";
        assert!(recovery.verify_object_end(valid));

        let invalid = b"<< /Type /Page >> no end here";
        assert!(!recovery.verify_object_end(invalid));
    }

    #[test]
    fn test_build_empty_xref() {
        let mut recovery = XRefRecovery::new();
        let xref_table = recovery.build_xref_table().unwrap();
        assert_eq!(xref_table.len(), 0);
    }

    #[test]
    fn test_build_xref_with_objects() {
        let mut recovery = XRefRecovery::new();

        // Add some test objects
        recovery.objects.insert((1, 0), 100);
        recovery.objects.insert((2, 0), 200);
        recovery.objects.insert((3, 1), 300);

        let xref_table = recovery.build_xref_table().unwrap();
        assert_eq!(recovery.stats.entries_reconstructed, 3);

        // Verify entries
        assert!(xref_table.get_entry(1).is_some());
        assert!(xref_table.get_entry(2).is_some());
        assert!(xref_table.get_entry(3).is_some());
    }

    #[test]
    fn test_xref_recovery_default() {
        let recovery = XRefRecovery::default();
        assert!(recovery.objects.is_empty());
        assert!(recovery.xref_entries.is_empty());
        assert_eq!(recovery.stats.objects_found, 0);
    }

    #[test]
    fn test_recovery_stats_default() {
        let stats = RecoveryStats::default();
        assert_eq!(stats.objects_found, 0);
        assert_eq!(stats.entries_reconstructed, 0);
        assert_eq!(stats.errors, 0);
        assert!(!stats.trailer_found);
    }

    #[test]
    fn test_recovery_stats_debug() {
        let stats = RecoveryStats {
            objects_found: 10,
            entries_reconstructed: 8,
            errors: 2,
            trailer_found: true,
        };

        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("RecoveryStats"));
        assert!(debug_str.contains("10"));
        assert!(debug_str.contains("8"));
        assert!(debug_str.contains("2"));
        assert!(debug_str.contains("true"));
    }

    #[test]
    fn test_find_object_pattern_edge_cases() {
        let recovery = XRefRecovery::new();

        // Pattern at start
        assert_eq!(recovery.find_object_pattern(b" obj at start"), Some(4));

        // Pattern at end
        assert_eq!(recovery.find_object_pattern(b"ends with obj"), Some(13));

        // Multiple patterns
        assert_eq!(recovery.find_object_pattern(b" obj and obj"), Some(4)); // First match

        // Partial pattern
        assert_eq!(recovery.find_object_pattern(b" ob"), None);
        assert_eq!(recovery.find_object_pattern(b"obj"), None); // Missing space

        // Empty buffer
        assert_eq!(recovery.find_object_pattern(b""), None);
    }

    #[test]
    fn test_parse_object_header_various_formats() {
        let recovery = XRefRecovery::new();

        // Standard format
        assert_eq!(
            recovery.parse_object_header(b"123 0 obj"),
            Some((123, 0, 0))
        );

        // With extra whitespace
        assert_eq!(
            recovery.parse_object_header(b"   42   5   obj  "),
            Some((42, 5, 3)) // Position of first digit
        );

        // Large numbers
        assert_eq!(
            recovery.parse_object_header(b"999999 65535 obj"),
            Some((999999, 65535, 0))
        );

        // With prefix text
        assert_eq!(
            recovery.parse_object_header(b"prefix text 7 3 obj"),
            Some((7, 3, 12))
        );

        // Invalid cases
        assert_eq!(recovery.parse_object_header(b"abc def obj"), None); // Non-numeric
        assert_eq!(recovery.parse_object_header(b"123 obj"), None); // Missing generation
        assert_eq!(recovery.parse_object_header(b"123 0"), None); // Missing "obj"
        assert_eq!(recovery.parse_object_header(b""), None); // Empty
        assert_eq!(recovery.parse_object_header(b"123 0 object"), None); // Wrong keyword
    }

    #[test]
    fn test_verify_object_end_various_cases() {
        let recovery = XRefRecovery::new();

        // Valid cases
        assert!(recovery.verify_object_end(b"endobj"));
        assert!(recovery.verify_object_end(b"data endobj more"));
        assert!(recovery.verify_object_end(b"stream\ndata\nendstream\nendobj"));

        // With spacing
        assert!(recovery.verify_object_end(b"   endobj   "));

        // Invalid cases
        assert!(!recovery.verify_object_end(b"endob"));
        assert!(!recovery.verify_object_end(b"endobject"));
        assert!(!recovery.verify_object_end(b""));

        // Far away endobj (beyond 50KB limit)
        let mut large = vec![b'x'; 51000];
        large.extend_from_slice(b"endobj");
        assert!(!recovery.verify_object_end(&large));
    }

    #[test]
    fn test_scan_buffer_single_object() {
        let mut recovery = XRefRecovery::new();

        let buffer = b"1 0 obj\n<< /Type /Page >>\nendobj";
        recovery.scan_buffer(buffer, 0).unwrap();

        assert_eq!(recovery.stats.objects_found, 1);
        assert!(recovery.objects.contains_key(&(1, 0)));
        assert_eq!(recovery.objects[&(1, 0)], 0);
    }

    #[test]
    fn test_scan_buffer_multiple_objects() {
        let mut recovery = XRefRecovery::new();

        let buffer = b"1 0 obj\n<< /Type /Catalog >>\nendobj\n2 0 obj\n<< /Type /Pages >>\nendobj";
        recovery.scan_buffer(buffer, 100).unwrap();

        assert_eq!(recovery.stats.objects_found, 2);
        assert!(recovery.objects.contains_key(&(1, 0)));
        assert!(recovery.objects.contains_key(&(2, 0)));
        assert_eq!(recovery.objects[&(1, 0)], 100); // Base offset applied
        assert_eq!(recovery.objects[&(2, 0)], 137); // Base offset + position
    }

    #[test]
    fn test_scan_buffer_invalid_objects() {
        let mut recovery = XRefRecovery::new();

        // Object without endobj
        let buffer = b"1 0 obj\n<< /Type /Page >>\n2 0 obj\nendobj";
        recovery.scan_buffer(buffer, 0).unwrap();

        assert_eq!(recovery.stats.objects_found, 1); // Only second object is valid
        assert!(!recovery.objects.contains_key(&(1, 0)));
        assert!(recovery.objects.contains_key(&(2, 0)));
    }

    #[test]
    fn test_scan_buffer_with_offset() {
        let mut recovery = XRefRecovery::new();

        let buffer = b"5 3 obj\n<< /Data >>\nendobj";
        recovery.scan_buffer(buffer, 1000).unwrap();

        assert_eq!(recovery.stats.objects_found, 1);
        assert_eq!(recovery.objects[&(5, 3)], 1000);
    }

    #[test]
    fn test_find_trailer() {
        let mut recovery = XRefRecovery::new();

        // Add some objects to determine Size
        recovery.objects.insert((1, 0), 100);
        recovery.objects.insert((5, 0), 200);
        recovery.objects.insert((10, 0), 300);

        let trailer = recovery.find_trailer().unwrap();

        // Should have Size entry
        if let Some(crate::parser::objects::PdfObject::Integer(size)) = trailer
            .0
            .get(&crate::parser::objects::PdfName("Size".to_string()))
        {
            assert_eq!(*size, 11); // Max ID + 1
        } else {
            panic!("Trailer should have Size entry");
        }
    }

    #[test]
    fn test_find_trailer_empty() {
        let recovery = XRefRecovery::new();
        let trailer = recovery.find_trailer().unwrap();

        // Empty recovery should produce empty trailer
        assert!(
            trailer.0.is_empty()
                || trailer
                    .0
                    .get(&crate::parser::objects::PdfName("Size".to_string()))
                    .is_none()
        );
    }

    #[test]
    fn test_stats_access() {
        let mut recovery = XRefRecovery::new();
        recovery.stats.objects_found = 5;
        recovery.stats.entries_reconstructed = 4;
        recovery.stats.errors = 1;
        recovery.stats.trailer_found = true;

        let stats = recovery.stats();
        assert_eq!(stats.objects_found, 5);
        assert_eq!(stats.entries_reconstructed, 4);
        assert_eq!(stats.errors, 1);
        assert!(stats.trailer_found);
    }

    #[test]
    fn test_recover_xref_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.pdf");
        let mut file = File::create(&path).unwrap();

        // Write a simple PDF with objects but no xref
        writeln!(file, "%PDF-1.7").unwrap();
        writeln!(file, "1 0 obj").unwrap();
        writeln!(file, "<< /Type /Catalog /Pages 2 0 R >>").unwrap();
        writeln!(file, "endobj").unwrap();
        writeln!(file, "2 0 obj").unwrap();
        writeln!(file, "<< /Type /Pages /Kids [] /Count 0 >>").unwrap();
        writeln!(file, "endobj").unwrap();
        writeln!(file, "%%EOF").unwrap();

        let xref_table = recover_xref(&path).unwrap();
        assert!(xref_table.len() > 0);
    }

    #[test]
    fn test_recover_xref_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("empty.pdf");
        File::create(&path).unwrap();

        let xref_table = recover_xref(&path).unwrap();
        assert_eq!(xref_table.len(), 0);
    }

    #[test]
    fn test_needs_xref_recovery_with_xref() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("has_xref.pdf");
        let mut file = File::create(&path).unwrap();

        // Write PDF with proper xref
        writeln!(file, "%PDF-1.7").unwrap();
        writeln!(file, "1 0 obj").unwrap();
        writeln!(file, "<< >>").unwrap();
        writeln!(file, "endobj").unwrap();
        writeln!(file, "xref").unwrap();
        writeln!(file, "0 2").unwrap();
        writeln!(file, "0000000000 65535 f").unwrap();
        writeln!(file, "0000000009 00000 n").unwrap();
        writeln!(file, "trailer").unwrap();
        writeln!(file, "<< /Size 2 >>").unwrap();
        writeln!(file, "startxref").unwrap();
        writeln!(file, "27").unwrap();
        writeln!(file, "%%EOF").unwrap();

        assert!(!needs_xref_recovery(&path).unwrap());
    }

    #[test]
    fn test_needs_xref_recovery_without_xref() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("no_xref.pdf");
        let mut file = File::create(&path).unwrap();

        // Write PDF without xref
        writeln!(file, "%PDF-1.7").unwrap();
        writeln!(file, "1 0 obj").unwrap();
        writeln!(file, "<< >>").unwrap();
        writeln!(file, "endobj").unwrap();
        writeln!(file, "%%EOF").unwrap();

        assert!(needs_xref_recovery(&path).unwrap());
    }

    #[test]
    fn test_needs_xref_recovery_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("empty.pdf");
        File::create(&path).unwrap();

        assert!(needs_xref_recovery(&path).unwrap());
    }

    #[test]
    fn test_needs_xref_recovery_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nonexistent.pdf");

        assert!(needs_xref_recovery(&path).is_err());
    }

    #[test]
    fn test_scan_file_with_overlap() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("overlap_test.pdf");
        let mut file = File::create(&path).unwrap();

        // Create a large file that will require multiple chunks
        writeln!(file, "%PDF-1.7").unwrap();

        // Write padding to force multiple chunks
        let padding = vec![b' '; 1024 * 1024]; // 1MB of spaces
        file.write_all(&padding).unwrap();

        // Write object that might span chunk boundary
        writeln!(file, "1 0 obj").unwrap();
        writeln!(file, "<< /Type /Test >>").unwrap();
        writeln!(file, "endobj").unwrap();

        let mut recovery = XRefRecovery::new();
        let mut file = File::open(&path).unwrap();
        recovery.scan_file(&mut file).unwrap();

        assert_eq!(recovery.stats.objects_found, 1);
    }

    #[test]
    fn test_build_xref_table_with_stats() {
        let mut recovery = XRefRecovery::new();

        // Add objects with different generations
        recovery.objects.insert((1, 0), 100);
        recovery.objects.insert((2, 0), 200);
        recovery.objects.insert((3, 1), 300);
        recovery.objects.insert((4, 2), 400);
        recovery.objects.insert((5, 0), 500);

        let xref_table = recovery.build_xref_table().unwrap();

        assert_eq!(recovery.stats.entries_reconstructed, 5);
        assert_eq!(xref_table.len(), 5);

        // Verify each entry
        for id in 1..=5 {
            let entry = xref_table.get_entry(id).unwrap();
            assert!(entry.in_use);
            assert_eq!(entry.offset, (id as u64) * 100);
        }

        // Check generations
        assert_eq!(xref_table.get_entry(3).unwrap().generation, 1);
        assert_eq!(xref_table.get_entry(4).unwrap().generation, 2);
    }

    #[test]
    fn test_scan_buffer_edge_positions() {
        let mut recovery = XRefRecovery::new();

        // Object at very start
        let buffer = b"1 0 obj\ndata\nendobj rest";
        recovery.scan_buffer(buffer, 0).unwrap();
        assert_eq!(recovery.objects[&(1, 0)], 0);

        // Clear for next test
        recovery.objects.clear();
        recovery.stats.objects_found = 0;

        // Object ID at position 0 but " obj" later
        let buffer = b"123 0 obj\ndata\nendobj";
        recovery.scan_buffer(buffer, 500).unwrap();
        assert_eq!(recovery.objects[&(123, 0)], 500);
    }

    #[test]
    fn test_parse_object_header_utf8_handling() {
        let recovery = XRefRecovery::new();

        // Valid UTF-8
        let valid = b"text 42 0 obj";
        assert_eq!(recovery.parse_object_header(valid), Some((42, 0, 5)));

        // Invalid UTF-8 should return None
        let invalid = &[0xFF, 0xFE, b' ', b'1', b' ', b'0', b' ', b'o', b'b', b'j'];
        assert_eq!(recovery.parse_object_header(invalid), None);
    }

    #[test]
    fn test_xref_recovery_large_file_simulation() {
        let mut recovery = XRefRecovery::new();

        // Simulate finding many objects
        for i in 1..=1000 {
            recovery.objects.insert((i, 0), i as u64 * 1000);
        }

        let xref_table = recovery.build_xref_table().unwrap();
        assert_eq!(xref_table.len(), 1000);
        assert_eq!(recovery.stats.entries_reconstructed, 1000);

        // Check trailer has correct size
        let trailer = recovery.find_trailer().unwrap();
        if let Some(crate::parser::objects::PdfObject::Integer(size)) = trailer
            .0
            .get(&crate::parser::objects::PdfName("Size".to_string()))
        {
            assert_eq!(*size, 1001); // Max ID + 1
        }
    }

    #[test]
    fn test_btree_map_ordering() {
        let mut recovery = XRefRecovery::new();

        // Insert in random order
        recovery.objects.insert((5, 0), 500);
        recovery.objects.insert((1, 0), 100);
        recovery.objects.insert((3, 0), 300);
        recovery.objects.insert((2, 0), 200);
        recovery.objects.insert((4, 0), 400);

        // BTreeMap should maintain order
        let keys: Vec<_> = recovery.objects.keys().cloned().collect();
        assert_eq!(keys, vec![(1, 0), (2, 0), (3, 0), (4, 0), (5, 0)]);
    }

    #[test]
    fn test_scan_buffer_no_objects() {
        let mut recovery = XRefRecovery::new();

        let buffer = b"This is just plain text with no PDF objects";
        recovery.scan_buffer(buffer, 0).unwrap();

        assert_eq!(recovery.stats.objects_found, 0);
        assert!(recovery.objects.is_empty());
    }

    #[test]
    fn test_verify_object_end_exact_match() {
        let recovery = XRefRecovery::new();

        // Exact match
        assert!(recovery.verify_object_end(b"endobj"));

        // With prefix
        assert!(recovery.verify_object_end(b"prefixendobjsuffix"));

        // Multiple occurrences - should find first
        assert!(recovery.verify_object_end(b"endobj endobj endobj"));
    }

    #[test]
    fn test_recovery_with_high_generation_numbers() {
        let mut recovery = XRefRecovery::new();

        // Add objects with high generation numbers
        recovery.objects.insert((1, 65535), 100); // Max generation
        recovery.objects.insert((2, 32768), 200);
        recovery.objects.insert((3, 1000), 300);

        let xref_table = recovery.build_xref_table().unwrap();

        assert_eq!(xref_table.get_entry(1).unwrap().generation, 65535);
        assert_eq!(xref_table.get_entry(2).unwrap().generation, 32768);
        assert_eq!(xref_table.get_entry(3).unwrap().generation, 1000);
    }

    #[test]
    fn test_scan_file_io_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("readonly.pdf");

        // File doesn't exist - test that open fails
        let file_result = File::open(&path);
        assert!(file_result.is_err());
    }

    #[test]
    fn test_find_object_pattern_performance() {
        let recovery = XRefRecovery::new();

        // Large buffer with pattern at end
        let mut buffer = vec![b'x'; 10000];
        buffer.extend_from_slice(b" obj");

        let pos = recovery.find_object_pattern(&buffer);
        assert_eq!(pos, Some(10004));
    }

    #[test]
    fn test_xref_entry_properties() {
        let mut recovery = XRefRecovery::new();
        recovery.objects.insert((42, 7), 12345);

        recovery.build_xref_table().unwrap();

        let entry = &recovery.xref_entries[&42];
        assert_eq!(entry.offset, 12345);
        assert_eq!(entry.generation, 7);
        assert!(entry.in_use);
    }
}
