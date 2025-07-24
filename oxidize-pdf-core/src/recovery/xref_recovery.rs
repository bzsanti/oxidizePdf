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
}
