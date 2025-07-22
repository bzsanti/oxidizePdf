//! PDF Cross-Reference Table Parser
//!
//! Parses xref tables according to ISO 32000-1 Section 7.5.4

use super::{ParseError, ParseResult};
use crate::parser::reader::PDFLines;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

/// Cross-reference entry
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct XRefEntry {
    /// Byte offset in the file (for in-use entries)
    pub offset: u64,
    /// Generation number
    pub generation: u16,
    /// Whether this entry is in use
    pub in_use: bool,
}

/// Extended XRef entry information for compressed objects
#[derive(Debug, Clone, PartialEq)]
pub struct XRefEntryExt {
    /// Basic entry information
    pub basic: XRefEntry,
    /// Additional info for compressed objects
    pub compressed_info: Option<(u32, u32)>, // (stream_obj_num, index_in_stream)
}

/// Cross-reference table
#[derive(Debug, Clone)]
pub struct XRefTable {
    /// Map of object number to xref entry
    entries: HashMap<u32, XRefEntry>,
    /// Extended entry information (for compressed objects)
    extended_entries: HashMap<u32, XRefEntryExt>,
    /// Trailer dictionary
    trailer: Option<super::objects::PdfDictionary>,
    /// Offset of the xref table in the file
    xref_offset: u64,
}

impl Default for XRefTable {
    fn default() -> Self {
        Self::new()
    }
}

impl XRefTable {
    /// Create a new empty xref table
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            extended_entries: HashMap::new(),
            trailer: None,
            xref_offset: 0,
        }
    }

    /// Parse xref table from a reader (handles both traditional and stream xrefs)
    pub fn parse<R: Read + Seek>(reader: &mut BufReader<R>) -> ParseResult<Self> {
        let mut table = Self::new();

        // Find and parse xref
        let xref_offset = Self::find_xref_offset(reader)?;
        table.xref_offset = xref_offset;
        reader.seek(SeekFrom::Start(xref_offset))?;

        // Check if this is a traditional xref table or xref stream
        let mut line = String::new();
        let pos = reader.stream_position()?;
        reader.read_line(&mut line)?;

        if line.trim() == "xref" {
            // Traditional xref table
            Self::parse_traditional_xref(reader, &mut table)?;
        } else {
            // Might be an xref stream, seek back
            reader.seek(SeekFrom::Start(pos))?;

            // Try to parse as an object
            let mut lexer = super::lexer::Lexer::new(reader);

            // Read object header
            let _obj_num = match lexer.next_token()? {
                super::lexer::Token::Integer(n) => n as u32,
                _ => return Err(ParseError::InvalidXRef),
            };

            let _gen_num = match lexer.next_token()? {
                super::lexer::Token::Integer(n) => n as u16,
                _ => return Err(ParseError::InvalidXRef),
            };

            match lexer.next_token()? {
                super::lexer::Token::Obj => {}
                _ => return Err(ParseError::InvalidXRef),
            };

            // Parse the object (should be a stream)
            let obj = super::objects::PdfObject::parse(&mut lexer)?;

            if let Some(stream) = obj.as_stream() {
                // Check if it's an xref stream
                if stream
                    .dict
                    .get("Type")
                    .and_then(|o| o.as_name())
                    .map(|n| n.as_str())
                    == Some("XRef")
                {
                    let xref_stream = XRefStream::parse(stream.clone())?;

                    // Copy entries from xref stream
                    for (obj_num, entry) in &xref_stream.entries {
                        table.entries.insert(*obj_num, *entry);
                    }

                    // Copy extended entries for compressed objects
                    for (obj_num, ext_entry) in &xref_stream.extended_entries {
                        table.extended_entries.insert(*obj_num, ext_entry.clone());
                    }

                    // Set trailer from xref stream
                    table.trailer = Some(xref_stream.trailer().clone());
                } else {
                    return Err(ParseError::InvalidXRef);
                }
            } else {
                return Err(ParseError::InvalidXRef);
            }
        }

        Ok(table)
    }

    /// Parse traditional xref table
    fn parse_traditional_xref<R: Read + Seek>(
        reader: &mut BufReader<R>,
        table: &mut XRefTable,
    ) -> ParseResult<()> {
        let mut line = String::new();

        // Parse subsections
        loop {
            line.clear();
            reader.read_line(&mut line)?;
            let trimmed_line = line.trim();

            // Skip empty lines
            if trimmed_line.is_empty() {
                continue;
            }

            // Check if we've reached the trailer
            if trimmed_line == "trailer" {
                break;
            }

            // Parse subsection header (first_obj_num count)
            let parts: Vec<&str> = trimmed_line.split_whitespace().collect();
            if parts.len() != 2 {
                // Invalid subsection header
                return Err(ParseError::InvalidXRef);
            }

            let first_obj_num = parts[0]
                .parse::<u32>()
                .map_err(|_| ParseError::InvalidXRef)?;
            let count = parts[1]
                .parse::<u32>()
                .map_err(|_| ParseError::InvalidXRef)?;

            // Parse entries
            // Parse xref entries
            for i in 0..count {
                line.clear();
                reader.read_line(&mut line)?;
                let entry = Self::parse_xref_entry(&line)?;
                table.entries.insert(first_obj_num + i, entry);
            }
            // Finished parsing xref entries
        }

        // Parse trailer dictionary
        let mut lexer = super::lexer::Lexer::new(reader);
        let trailer_obj = super::objects::PdfObject::parse(&mut lexer)?;
        // Trailer object parsed successfully

        table.trailer = trailer_obj.as_dict().cloned();

        // After parsing the trailer, the reader is positioned after the dictionary
        // We don't need to parse anything else - startxref/offset/%%EOF are handled elsewhere

        Ok(())
    }

    /// Find the xref offset by looking for startxref at the end of the file
    fn find_xref_offset<R: Read + Seek>(reader: &mut BufReader<R>) -> ParseResult<u64> {
        // Go to end of file
        reader.seek(SeekFrom::End(0))?;
        let file_size = reader.stream_position()?;

        // Read last 1024 bytes (should be enough for EOL + startxref + offset + %%EOF)
        let read_size = std::cmp::min(1024, file_size);
        reader.seek(SeekFrom::End(-(read_size as i64)))?;

        let mut buffer = vec![0u8; read_size as usize];
        reader.read_exact(&mut buffer)?;

        // Convert to string and find startxref
        let content = String::from_utf8_lossy(&buffer);
        let mut lines = content.pdf_lines();

        // Find startxref line - need to iterate forward after finding it
        while let Some(line) = lines.next() {
            if line.trim() == "startxref" {
                // The offset should be on the next line
                if let Some(offset_line) = lines.next() {
                    let offset = offset_line
                        .trim()
                        .parse::<u64>()
                        .map_err(|_| ParseError::InvalidXRef)?;
                    return Ok(offset);
                }
            }
        }

        Err(ParseError::InvalidXRef)
    }

    /// Parse a single xref entry line
    fn parse_xref_entry(line: &str) -> ParseResult<XRefEntry> {
        let line = line.trim();

        // Entry format: nnnnnnnnnn ggggg n/f
        // Where n = offset (10 digits), g = generation (5 digits), n/f = in use flag
        if line.len() < 18 {
            // Line too short
            return Err(ParseError::InvalidXRef);
        }

        let offset_str = &line[0..10];
        let gen_str = &line[11..16];
        let flag = line.chars().nth(17);

        // Parse xref entry

        let offset = offset_str
            .trim()
            .parse::<u64>()
            .map_err(|_| ParseError::InvalidXRef)?;
        let generation = gen_str
            .trim()
            .parse::<u16>()
            .map_err(|_| ParseError::InvalidXRef)?;

        let in_use = match flag {
            Some('n') => true,
            Some('f') => false,
            _ => return Err(ParseError::InvalidXRef),
        };

        Ok(XRefEntry {
            offset,
            generation,
            in_use,
        })
    }

    /// Get an xref entry by object number
    pub fn get_entry(&self, obj_num: u32) -> Option<&XRefEntry> {
        self.entries.get(&obj_num)
    }

    /// Get the trailer dictionary
    pub fn trailer(&self) -> Option<&super::objects::PdfDictionary> {
        self.trailer.as_ref()
    }

    /// Get the xref offset
    pub fn xref_offset(&self) -> u64 {
        self.xref_offset
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the table is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all entries
    pub fn iter(&self) -> impl Iterator<Item = (&u32, &XRefEntry)> {
        self.entries.iter()
    }

    /// Get extended entry information (for compressed objects)
    pub fn get_extended_entry(&self, obj_num: u32) -> Option<&XRefEntryExt> {
        self.extended_entries.get(&obj_num)
    }

    /// Check if an object is compressed
    pub fn is_compressed(&self, obj_num: u32) -> bool {
        self.extended_entries
            .get(&obj_num)
            .map(|e| e.compressed_info.is_some())
            .unwrap_or(false)
    }

    /// Add an entry to the xref table
    pub fn add_entry(&mut self, obj_num: u32, entry: XRefEntry) {
        self.entries.insert(obj_num, entry);
    }

    /// Set the trailer dictionary
    pub fn set_trailer(&mut self, trailer: super::objects::PdfDictionary) {
        self.trailer = Some(trailer);
    }

    /// Add an extended entry to the xref table
    pub fn add_extended_entry(&mut self, obj_num: u32, entry: XRefEntryExt) {
        self.extended_entries.insert(obj_num, entry);
    }
}

/// Cross-reference stream (PDF 1.5+)
/// This is a more compact representation using streams
#[derive(Debug, Clone)]
pub struct XRefStream {
    /// The stream object containing xref data
    stream: super::objects::PdfStream,
    /// Decoded entries
    entries: HashMap<u32, XRefEntry>,
    /// Extended entries for compressed objects
    extended_entries: HashMap<u32, XRefEntryExt>,
}

impl XRefStream {
    /// Parse an xref stream object
    pub fn parse(stream: super::objects::PdfStream) -> ParseResult<Self> {
        let mut xref_stream = Self {
            stream,
            entries: HashMap::new(),
            extended_entries: HashMap::new(),
        };

        xref_stream.decode_entries()?;
        Ok(xref_stream)
    }

    /// Decode the xref stream entries
    fn decode_entries(&mut self) -> ParseResult<()> {
        // Get stream dictionary values
        let dict = &self.stream.dict;

        // Get the Size (number of entries)
        let size = dict
            .get("Size")
            .and_then(|obj| obj.as_integer())
            .ok_or_else(|| ParseError::MissingKey("Size".to_string()))?;

        // Get the Index array [first_obj_num, count, ...]
        let index = match dict.get("Index") {
            Some(obj) => {
                let array = obj.as_array().ok_or_else(|| ParseError::SyntaxError {
                    position: 0,
                    message: "Index must be an array".to_string(),
                })?;

                // Convert to pairs of (first_obj_num, count)
                let mut pairs = Vec::new();
                for chunk in array.0.chunks(2) {
                    if chunk.len() != 2 {
                        return Err(ParseError::SyntaxError {
                            position: 0,
                            message: "Index array must have even number of elements".to_string(),
                        });
                    }
                    let first = chunk[0]
                        .as_integer()
                        .ok_or_else(|| ParseError::SyntaxError {
                            position: 0,
                            message: "Index values must be integers".to_string(),
                        })? as u32;
                    let count = chunk[1]
                        .as_integer()
                        .ok_or_else(|| ParseError::SyntaxError {
                            position: 0,
                            message: "Index values must be integers".to_string(),
                        })? as u32;
                    pairs.push((first, count));
                }
                pairs
            }
            None => {
                // Default: single subsection starting at 0
                vec![(0, size as u32)]
            }
        };

        // Get the W array (field widths)
        let w_array = dict
            .get("W")
            .and_then(|obj| obj.as_array())
            .ok_or_else(|| ParseError::MissingKey("W".to_string()))?;

        if w_array.len() != 3 {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "W array must have exactly 3 elements".to_string(),
            });
        }

        let w: Vec<usize> = w_array
            .0
            .iter()
            .map(|obj| {
                obj.as_integer()
                    .ok_or_else(|| ParseError::SyntaxError {
                        position: 0,
                        message: "W values must be integers".to_string(),
                    })
                    .map(|i| i as usize)
            })
            .collect::<ParseResult<Vec<_>>>()?;

        // Decode the stream data
        let data = self.stream.decode()?;
        let mut offset = 0;

        // Process each subsection
        for (first_obj_num, count) in index {
            for i in 0..count {
                if offset + w[0] + w[1] + w[2] > data.len() {
                    return Err(ParseError::SyntaxError {
                        position: 0,
                        message: "Xref stream data truncated".to_string(),
                    });
                }

                // Read fields according to widths
                let field1 = Self::read_field(&data[offset..], w[0]);
                offset += w[0];

                let field2 = Self::read_field(&data[offset..], w[1]);
                offset += w[1];

                let field3 = Self::read_field(&data[offset..], w[2]);
                offset += w[2];

                // Interpret based on type (field1)
                let entry = match field1 {
                    0 => {
                        // Free object
                        XRefEntry {
                            offset: field2,
                            generation: field3 as u16,
                            in_use: false,
                        }
                    }
                    1 => {
                        // Uncompressed object
                        XRefEntry {
                            offset: field2,
                            generation: field3 as u16,
                            in_use: true,
                        }
                    }
                    2 => {
                        // Compressed object (in object stream)
                        // field2 = object stream number
                        // field3 = index within object stream
                        let entry = XRefEntry {
                            offset: 0,
                            generation: 0,
                            in_use: true,
                        };

                        // Store extended info for compressed objects
                        let ext_entry = XRefEntryExt {
                            basic: entry,
                            compressed_info: Some((field2 as u32, field3 as u32)),
                        };
                        self.extended_entries.insert(first_obj_num + i, ext_entry);

                        entry
                    }
                    _ => {
                        // Unknown type - treat as free object for compatibility
                        // PDF spec says unknown types should be ignored
                        eprintln!(
                            "Warning: Unknown xref entry type {} for object {}",
                            field1,
                            first_obj_num + i
                        );
                        XRefEntry {
                            offset: 0,
                            generation: 0,
                            in_use: false,
                        }
                    }
                };

                self.entries.insert(first_obj_num + i, entry);
            }
        }

        Ok(())
    }

    /// Read a field of given width from data
    fn read_field(data: &[u8], width: usize) -> u64 {
        let mut value = 0u64;
        for i in 0..width {
            if i < data.len() {
                value = (value << 8) | (data[i] as u64);
            }
        }
        value
    }

    /// Get an entry by object number
    pub fn get_entry(&self, obj_num: u32) -> Option<&XRefEntry> {
        self.entries.get(&obj_num)
    }

    /// Get the trailer dictionary from the stream
    pub fn trailer(&self) -> &super::objects::PdfDictionary {
        &self.stream.dict
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::parser::objects::{PdfDictionary, PdfObject};
    use std::io::Cursor;

    #[test]
    fn test_parse_xref_entry() {
        let entry1 = XRefTable::parse_xref_entry("0000000000 65535 f ").unwrap();
        assert_eq!(entry1.offset, 0);
        assert_eq!(entry1.generation, 65535);
        assert!(!entry1.in_use);

        let entry2 = XRefTable::parse_xref_entry("0000000017 00000 n ").unwrap();
        assert_eq!(entry2.offset, 17);
        assert_eq!(entry2.generation, 0);
        assert!(entry2.in_use);
    }

    #[test]
    fn test_xref_table_new() {
        let table = XRefTable::new();
        assert!(table.entries.is_empty());
        assert!(table.extended_entries.is_empty());
        assert!(table.trailer.is_none());
        assert_eq!(table.xref_offset, 0);
    }

    #[test]
    fn test_xref_table_default() {
        let table = XRefTable::default();
        assert!(table.entries.is_empty());
        assert!(table.extended_entries.is_empty());
        assert!(table.trailer.is_none());
    }

    #[test]
    fn test_xref_entry_struct() {
        let entry = XRefEntry {
            offset: 12345,
            generation: 7,
            in_use: true,
        };
        assert_eq!(entry.offset, 12345);
        assert_eq!(entry.generation, 7);
        assert!(entry.in_use);
    }

    #[test]
    fn test_xref_entry_equality() {
        let entry1 = XRefEntry {
            offset: 100,
            generation: 0,
            in_use: true,
        };
        let entry2 = XRefEntry {
            offset: 100,
            generation: 0,
            in_use: true,
        };
        assert_eq!(entry1, entry2);
    }

    #[test]
    fn test_xref_entry_clone() {
        let entry = XRefEntry {
            offset: 999,
            generation: 3,
            in_use: false,
        };
        let cloned = entry;
        assert_eq!(cloned.offset, 999);
        assert_eq!(cloned.generation, 3);
        assert!(!cloned.in_use);
    }

    #[test]
    fn test_xref_entry_ext() {
        let ext_entry = XRefEntryExt {
            basic: XRefEntry {
                offset: 500,
                generation: 0,
                in_use: true,
            },
            compressed_info: Some((10, 5)),
        };
        assert_eq!(ext_entry.basic.offset, 500);
        assert_eq!(ext_entry.compressed_info, Some((10, 5)));
    }

    #[test]
    fn test_xref_entry_ext_no_compression() {
        let ext_entry = XRefEntryExt {
            basic: XRefEntry {
                offset: 1000,
                generation: 1,
                in_use: true,
            },
            compressed_info: None,
        };
        assert!(ext_entry.compressed_info.is_none());
    }

    #[test]
    fn test_add_entry() {
        let mut table = XRefTable::new();
        table.add_entry(
            5,
            XRefEntry {
                offset: 1000,
                generation: 0,
                in_use: true,
            },
        );
        assert_eq!(table.entries.len(), 1);
        assert!(table.entries.contains_key(&5));
    }

    #[test]
    fn test_get_entry() {
        let mut table = XRefTable::new();
        let entry = XRefEntry {
            offset: 2000,
            generation: 1,
            in_use: true,
        };
        table.add_entry(10, entry);

        let retrieved = table.get_entry(10);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().offset, 2000);

        let missing = table.get_entry(999);
        assert!(missing.is_none());
    }

    #[test]
    fn test_set_trailer() {
        let mut table = XRefTable::new();
        let mut trailer = PdfDictionary::new();
        trailer.insert("Size".to_string(), PdfObject::Integer(10));

        table.set_trailer(trailer.clone());
        assert!(table.trailer.is_some());
        assert_eq!(
            table.trailer().unwrap().get("Size"),
            Some(&PdfObject::Integer(10))
        );
    }

    #[test]
    fn test_parse_xref_entry_invalid() {
        // Too short
        let result = XRefTable::parse_xref_entry("0000000000 65535");
        assert!(result.is_err());

        // Invalid format
        let result = XRefTable::parse_xref_entry("not_a_number 65535 f ");
        assert!(result.is_err());

        // Invalid flag
        let result = XRefTable::parse_xref_entry("0000000000 65535 x ");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_xref_entry_various_offsets() {
        // Small offset
        let entry = XRefTable::parse_xref_entry("0000000001 00000 n ").unwrap();
        assert_eq!(entry.offset, 1);

        // Large offset
        let entry = XRefTable::parse_xref_entry("9999999999 00000 n ").unwrap();
        assert_eq!(entry.offset, 9999999999);

        // Max generation
        let entry = XRefTable::parse_xref_entry("0000000000 65535 f ").unwrap();
        assert_eq!(entry.generation, 65535);
    }

    #[test]
    fn test_add_extended_entry() {
        let mut table = XRefTable::new();
        let ext_entry = XRefEntryExt {
            basic: XRefEntry {
                offset: 0,
                generation: 0,
                in_use: true,
            },
            compressed_info: Some((5, 10)),
        };

        table.add_extended_entry(15, ext_entry.clone());
        assert_eq!(table.extended_entries.len(), 1);
        assert!(table.extended_entries.contains_key(&15));
    }

    #[test]
    fn test_get_extended_entry() {
        let mut table = XRefTable::new();
        let ext_entry = XRefEntryExt {
            basic: XRefEntry {
                offset: 0,
                generation: 0,
                in_use: true,
            },
            compressed_info: Some((20, 3)),
        };

        table.add_extended_entry(7, ext_entry);

        let retrieved = table.get_extended_entry(7);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().compressed_info, Some((20, 3)));
    }

    #[test]
    fn test_xref_offset() {
        let mut table = XRefTable::new();
        assert_eq!(table.xref_offset(), 0);

        table.xref_offset = 12345;
        assert_eq!(table.xref_offset(), 12345);
    }

    #[test]
    fn test_find_xref_offset_simple() {
        let pdf_data = b"startxref\n12345\n%%EOF";
        let cursor = Cursor::new(pdf_data.to_vec());
        let mut reader = BufReader::new(cursor);

        let offset = XRefTable::find_xref_offset(&mut reader).unwrap();
        assert_eq!(offset, 12345);
    }

    #[test]
    fn test_find_xref_offset_with_spaces() {
        let pdf_data = b"startxref  \n  12345  \n%%EOF";
        let cursor = Cursor::new(pdf_data.to_vec());
        let mut reader = BufReader::new(cursor);

        let offset = XRefTable::find_xref_offset(&mut reader).unwrap();
        assert_eq!(offset, 12345);
    }

    #[test]
    fn test_find_xref_offset_missing() {
        let pdf_data = b"no startxref here";
        let cursor = Cursor::new(pdf_data.to_vec());
        let mut reader = BufReader::new(cursor);

        let result = XRefTable::find_xref_offset(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_trailer_getter() {
        let mut table = XRefTable::new();
        assert!(table.trailer().is_none());

        let trailer = PdfDictionary::new();
        table.set_trailer(trailer);
        assert!(table.trailer().is_some());
    }

    #[test]
    fn test_xref_table_clone() {
        let mut table = XRefTable::new();
        table.add_entry(
            1,
            XRefEntry {
                offset: 100,
                generation: 0,
                in_use: true,
            },
        );
        table.xref_offset = 5000;

        let cloned = table.clone();
        assert_eq!(cloned.entries.len(), 1);
        assert_eq!(cloned.xref_offset, 5000);
    }
}
