//! PDF Object Stream Parser
//!
//! Handles compressed objects stored in object streams (PDF 1.5+)

use super::lexer::Lexer;
use super::objects::{PdfObject, PdfStream};
use super::xref::XRefEntry;
use super::{ParseError, ParseOptions, ParseResult};
use std::collections::HashMap;
use std::io::Cursor;

/// Represents a PDF object stream containing compressed objects
#[derive(Debug)]
pub struct ObjectStream {
    /// Stream containing the objects
    stream: PdfStream,
    /// Number of objects in the stream
    n: u32,
    /// Offset of first object
    first: u32,
    /// Cached parsed objects
    objects: HashMap<u32, PdfObject>,
}

impl ObjectStream {
    /// Parse an object stream
    pub fn parse(stream: PdfStream, options: &ParseOptions) -> ParseResult<Self> {
        // Get required entries from stream dictionary
        let dict = &stream.dict;

        let n = dict
            .get("N")
            .and_then(|obj| obj.as_integer())
            .ok_or_else(|| ParseError::MissingKey("N".to_string()))? as u32;

        let first = dict
            .get("First")
            .and_then(|obj| obj.as_integer())
            .ok_or_else(|| ParseError::MissingKey("First".to_string()))? as u32;

        let mut obj_stream = ObjectStream {
            stream,
            n,
            first,
            objects: HashMap::new(),
        };

        // Parse all objects eagerly
        obj_stream.parse_objects(options)?;

        Ok(obj_stream)
    }

    /// Parse all objects in the stream
    fn parse_objects(&mut self, options: &ParseOptions) -> ParseResult<()> {
        // Decode the stream data
        let data = self.stream.decode(options)?;

        // Create a cursor for reading
        let mut cursor = Cursor::new(&data);
        // TODO: Accept options parameter in parse() to pass here
        let mut lexer = Lexer::new(&mut cursor);

        // Read object number/offset pairs
        let mut offsets = Vec::new();
        for _ in 0..self.n {
            // Read object number
            let obj_num = match lexer.next_token()? {
                super::lexer::Token::Integer(n) => n as u32,
                _ => {
                    return Err(ParseError::SyntaxError {
                        position: 0,
                        message: "Expected object number in object stream".to_string(),
                    })
                }
            };

            // Read offset
            let offset = match lexer.next_token()? {
                super::lexer::Token::Integer(n) => n as u32,
                _ => {
                    return Err(ParseError::SyntaxError {
                        position: 0,
                        message: "Expected offset in object stream".to_string(),
                    })
                }
            };

            offsets.push((obj_num, offset));
        }

        // Parse each object
        for (obj_num, offset) in offsets.iter() {
            // Calculate absolute offset
            let abs_offset = self.first + offset;

            // Seek to object start
            cursor.set_position(abs_offset as u64);
            let mut obj_lexer = Lexer::new(&mut cursor);

            // Parse the object
            // TODO: Accept options parameter in parse() to pass here
            let obj = PdfObject::parse(&mut obj_lexer)?;

            // Store in cache
            self.objects.insert(*obj_num, obj);
        }

        Ok(())
    }

    /// Get an object by its object number
    pub fn get_object(&self, obj_num: u32) -> Option<&PdfObject> {
        self.objects.get(&obj_num)
    }

    /// Get all objects
    pub fn objects(&self) -> &HashMap<u32, PdfObject> {
        &self.objects
    }
}

/// Extended XRef entry to handle compressed objects
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum XRefEntryType {
    /// Free object
    Free { next_free_obj: u32, generation: u16 },
    /// Uncompressed object
    InUse { offset: u64, generation: u16 },
    /// Compressed object in object stream
    Compressed {
        stream_obj_num: u32,
        index_in_stream: u32,
    },
}

impl XRefEntryType {
    /// Convert to simple XRefEntry for compatibility
    pub fn to_simple_entry(&self) -> XRefEntry {
        match self {
            XRefEntryType::Free { generation, .. } => XRefEntry {
                offset: 0,
                generation: *generation,
                in_use: false,
            },
            XRefEntryType::InUse { offset, generation } => XRefEntry {
                offset: *offset,
                generation: *generation,
                in_use: true,
            },
            XRefEntryType::Compressed { .. } => XRefEntry {
                offset: 0,
                generation: 0,
                in_use: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::objects::{PdfDictionary, PdfName};
    use super::*;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::collections::HashMap;
    use std::io::Write;

    fn create_test_stream_data() -> Vec<u8> {
        // Create test data with proper format:
        // Object numbers and offsets: "1 0 2 2"
        // Then the objects starting at offset 10 (after "1 0 2 2    ")
        // Simple objects: "true false"
        let data = b"1 0 2 2    true false";
        data.to_vec()
    }

    fn create_compressed_stream_data() -> Vec<u8> {
        let data = create_test_stream_data();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&data).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn test_xref_entry_type_free() {
        let entry = XRefEntryType::Free {
            next_free_obj: 5,
            generation: 65535,
        };

        let simple = entry.to_simple_entry();
        assert_eq!(simple.offset, 0);
        assert_eq!(simple.generation, 65535);
        assert!(!simple.in_use);
    }

    #[test]
    fn test_xref_entry_type_in_use() {
        let entry = XRefEntryType::InUse {
            offset: 1234,
            generation: 0,
        };

        let simple = entry.to_simple_entry();
        assert_eq!(simple.offset, 1234);
        assert_eq!(simple.generation, 0);
        assert!(simple.in_use);
    }

    #[test]
    fn test_xref_entry_type_compressed() {
        let entry = XRefEntryType::Compressed {
            stream_obj_num: 10,
            index_in_stream: 3,
        };

        let simple = entry.to_simple_entry();
        assert_eq!(simple.offset, 0); // Compressed entries have offset 0
        assert_eq!(simple.generation, 0);
        assert!(simple.in_use);
    }

    // Note: These tests are simplified because creating valid object stream data
    // that passes through the full parser is complex. The real testing happens
    // in integration tests with actual PDF files.

    #[test]
    fn test_object_stream_parse_missing_n() {
        // Test that missing N field causes error
        let mut dict = PdfDictionary(HashMap::new());
        dict.0.insert(
            PdfName("Type".to_string()),
            PdfObject::Name(PdfName("ObjStm".to_string())),
        );
        dict.0
            .insert(PdfName("First".to_string()), PdfObject::Integer(10));

        let stream = PdfStream { dict, data: vec![] };

        let options = ParseOptions::default();
        let result = ObjectStream::parse(stream, &options);

        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::MissingKey(key) => assert_eq!(key, "N"),
            _ => panic!("Expected MissingKey error"),
        }
    }

    #[test]
    fn test_object_stream_parse_missing_first() {
        // Test that missing First field causes error
        let mut dict = PdfDictionary(HashMap::new());
        dict.0.insert(
            PdfName("Type".to_string()),
            PdfObject::Name(PdfName("ObjStm".to_string())),
        );
        dict.0
            .insert(PdfName("N".to_string()), PdfObject::Integer(2));

        let stream = PdfStream { dict, data: vec![] };

        let options = ParseOptions::default();
        let result = ObjectStream::parse(stream, &options);

        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::MissingKey(key) => assert_eq!(key, "First"),
            _ => panic!("Expected MissingKey error"),
        }
    }
}
