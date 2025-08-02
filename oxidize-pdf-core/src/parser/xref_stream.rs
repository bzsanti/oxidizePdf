//! Cross-reference stream support for PDF 1.5+
//!
//! This module implements cross-reference streams according to
//! ISO 32000-1:2008 Section 7.5.8 (Cross-Reference Streams).
//!
//! Cross-reference streams are an alternative to traditional xref tables,
//! providing more compact representation and supporting compressed object streams.

use crate::parser::filters::{apply_filter, Filter};
use crate::parser::objects::{PdfArray, PdfDictionary, PdfName, PdfObject};
use crate::parser::ParseOptions;
use crate::parser::{ParseError, ParseResult};
use std::io::{Read, Seek};

/// Cross-reference entry
#[derive(Debug, Clone, PartialEq)]
pub enum XRefEntry {
    /// Free object entry
    Free {
        /// Next free object number
        next_free_object: u32,
        /// Generation number
        generation: u16,
    },
    /// In-use object entry
    InUse {
        /// Byte offset in the file
        offset: u64,
        /// Generation number
        generation: u16,
    },
    /// Compressed object entry (PDF 1.5+)
    Compressed {
        /// Object number of the object stream containing this object
        stream_object_number: u32,
        /// Index of this object within the object stream
        index_within_stream: u32,
    },
}

/// Cross-reference stream parser
pub struct XRefStream {
    /// Stream dictionary
    pub dict: PdfDictionary,
    /// Decoded stream data
    pub data: Vec<u8>,
    /// Field widths from W array
    pub widths: Vec<usize>,
    /// Index array (pairs of [first_object_number, count])
    pub index: Vec<(u32, u32)>,
}

impl XRefStream {
    /// Parse a cross-reference stream
    pub fn parse<R: Read + Seek>(
        _reader: &mut R,
        stream_dict: PdfDictionary,
        stream_data: Vec<u8>,
        _options: &ParseOptions,
    ) -> ParseResult<Self> {
        // Get the W (widths) array
        let widths = stream_dict
            .get("W")
            .and_then(|obj| obj.as_array())
            .ok_or_else(|| ParseError::MissingKey("W array in xref stream".to_string()))?
            .0
            .iter()
            .map(|obj| {
                obj.as_integer()
                    .ok_or_else(|| ParseError::SyntaxError {
                        position: 0,
                        message: "Invalid width in W array".to_string(),
                    })
                    .map(|n| n as usize)
            })
            .collect::<ParseResult<Vec<_>>>()?;

        if widths.len() != 3 {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: format!(
                    "W array must have 3 elements, found {len}",
                    len = widths.len()
                ),
            });
        }

        // Get the Index array if present
        let index =
            if let Some(index_array) = stream_dict.get("Index").and_then(|obj| obj.as_array()) {
                let mut index_pairs = Vec::new();
                let mut i = 0;
                while i + 1 < index_array.len() {
                    let first =
                        index_array.0[i]
                            .as_integer()
                            .ok_or_else(|| ParseError::SyntaxError {
                                position: 0,
                                message: "Invalid first object number in Index".to_string(),
                            })? as u32;
                    let count = index_array.0[i + 1].as_integer().ok_or_else(|| {
                        ParseError::SyntaxError {
                            position: 0,
                            message: "Invalid count in Index".to_string(),
                        }
                    })? as u32;
                    index_pairs.push((first, count));
                    i += 2;
                }
                index_pairs
            } else {
                // Default: start at 0, count is Size
                let size = stream_dict
                    .get("Size")
                    .and_then(|obj| obj.as_integer())
                    .ok_or_else(|| ParseError::MissingKey("Size in xref stream".to_string()))?
                    as u32;
                vec![(0, size)]
            };

        // Decode the stream data
        let decoded_data = if let Some(filter_obj) = stream_dict.get("Filter") {
            // Apply filters
            match filter_obj {
                PdfObject::Name(filter_name) => apply_filter(
                    &stream_data,
                    Filter::from_name(filter_name.as_str()).ok_or_else(|| {
                        ParseError::StreamDecodeError(format!("Unknown filter: {filter_name:?}"))
                    })?,
                )?,
                PdfObject::Array(filters) => {
                    let mut data = stream_data;
                    for filter in filters.0.iter() {
                        if let Some(filter_name) = filter.as_name() {
                            data = apply_filter(
                                &data,
                                Filter::from_name(filter_name.as_str()).ok_or_else(|| {
                                    ParseError::StreamDecodeError(format!(
                                        "Unknown filter: {filter_name:?}"
                                    ))
                                })?,
                            )?;
                        }
                    }
                    data
                }
                _ => stream_data,
            }
        } else {
            stream_data
        };

        Ok(XRefStream {
            dict: stream_dict,
            data: decoded_data,
            widths,
            index,
        })
    }

    /// Convert the cross-reference stream to XRefTable entries
    pub fn to_xref_entries(&self) -> ParseResult<Vec<(u32, XRefEntry)>> {
        let mut entries = Vec::new();
        let entry_size = self.widths.iter().sum::<usize>();

        if entry_size == 0 {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "Invalid entry size (0) in xref stream".to_string(),
            });
        }

        let mut data_offset = 0;

        for &(first_obj, count) in &self.index {
            for i in 0..count {
                if data_offset + entry_size > self.data.len() {
                    return Err(ParseError::SyntaxError {
                        position: data_offset,
                        message: "Xref stream data truncated".to_string(),
                    });
                }

                // Read fields according to widths
                let mut field_offset = data_offset;
                let mut fields = Vec::new();

                for &width in &self.widths {
                    let field_value = if width == 0 {
                        0 // Default value when width is 0
                    } else {
                        read_field(&self.data[field_offset..field_offset + width])
                    };
                    fields.push(field_value);
                    field_offset += width;
                }

                // Interpret fields based on type
                let entry_type = fields[0];
                let obj_num = first_obj + i;

                let entry = match entry_type {
                    0 => {
                        // Type 0: Free object
                        XRefEntry::Free {
                            next_free_object: fields[1] as u32,
                            generation: fields[2] as u16,
                        }
                    }
                    1 => {
                        // Type 1: Uncompressed object
                        XRefEntry::InUse {
                            offset: fields[1],
                            generation: fields[2] as u16,
                        }
                    }
                    2 => {
                        // Type 2: Compressed object
                        XRefEntry::Compressed {
                            stream_object_number: fields[1] as u32,
                            index_within_stream: fields[2] as u32,
                        }
                    }
                    _ => {
                        return Err(ParseError::SyntaxError {
                            position: data_offset,
                            message: format!("Invalid xref entry type: {entry_type}"),
                        });
                    }
                };

                entries.push((obj_num, entry));
                data_offset += entry_size;
            }
        }

        Ok(entries)
    }

    /// Get the trailer dictionary from the xref stream
    pub fn trailer_dict(&self) -> &PdfDictionary {
        &self.dict
    }

    /// Check if this is a hybrid reference file
    pub fn is_hybrid(&self) -> bool {
        // A hybrid file has both xref stream and traditional xref table
        self.dict.get("XRefStm").is_some()
    }

    /// Get the offset to additional xref stream (for hybrid files)
    pub fn get_xref_stm_offset(&self) -> Option<u64> {
        self.dict
            .get("XRefStm")
            .and_then(|obj| obj.as_integer())
            .map(|n| n as u64)
    }

    /// Get the previous xref offset
    pub fn get_prev_offset(&self) -> Option<u64> {
        self.dict
            .get("Prev")
            .and_then(|obj| obj.as_integer())
            .map(|n| n as u64)
    }
}

/// Read a field from bytes (big-endian)
fn read_field(bytes: &[u8]) -> u64 {
    let mut value = 0u64;
    for &byte in bytes {
        value = (value << 8) | (byte as u64);
    }
    value
}

/// XRef stream builder for creating new xref streams
pub struct XRefStreamBuilder {
    /// Entries to include in the stream
    entries: Vec<(u32, XRefEntry)>,
    /// Additional trailer dictionary entries
    trailer_entries: PdfDictionary,
}

impl Default for XRefStreamBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl XRefStreamBuilder {
    /// Create a new XRef stream builder
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            trailer_entries: PdfDictionary::new(),
        }
    }

    /// Add an entry to the xref stream
    pub fn add_entry(&mut self, obj_num: u32, entry: XRefEntry) {
        self.entries.push((obj_num, entry));
    }

    /// Add a trailer dictionary entry
    pub fn add_trailer_entry(&mut self, key: &str, value: PdfObject) {
        self.trailer_entries.insert(key.to_string(), value);
    }

    /// Build the xref stream
    pub fn build(mut self) -> ParseResult<(PdfDictionary, Vec<u8>)> {
        // Sort entries by object number
        self.entries.sort_by_key(|(num, _)| *num);

        // Determine field widths
        let mut max_offset = 0u64;
        let mut max_obj_num = 0u32;
        let mut max_gen = 0u16;
        let mut _has_compressed = false;

        for (obj_num, entry) in &self.entries {
            max_obj_num = max_obj_num.max(*obj_num);
            match entry {
                XRefEntry::InUse { offset, generation } => {
                    max_offset = max_offset.max(*offset);
                    max_gen = max_gen.max(*generation);
                }
                XRefEntry::Free { generation, .. } => {
                    max_gen = max_gen.max(*generation);
                }
                XRefEntry::Compressed {
                    stream_object_number,
                    index_within_stream,
                } => {
                    _has_compressed = true;
                    max_obj_num = max_obj_num.max(*stream_object_number);
                    max_offset = max_offset.max(*index_within_stream as u64);
                }
            }
        }

        // Calculate minimum bytes needed for each field
        let w1 = 1; // Type field (0, 1, or 2)
        let w2 = bytes_needed(max_offset.max(max_obj_num as u64));
        let w3 = bytes_needed(max_gen as u64);

        // Build the stream data
        let mut stream_data = Vec::new();

        for (_obj_num, entry) in &self.entries {
            match entry {
                XRefEntry::Free {
                    next_free_object,
                    generation,
                } => {
                    write_field(&mut stream_data, 0, w1); // Type 0
                    write_field(&mut stream_data, *next_free_object as u64, w2);
                    write_field(&mut stream_data, *generation as u64, w3);
                }
                XRefEntry::InUse { offset, generation } => {
                    write_field(&mut stream_data, 1, w1); // Type 1
                    write_field(&mut stream_data, *offset, w2);
                    write_field(&mut stream_data, *generation as u64, w3);
                }
                XRefEntry::Compressed {
                    stream_object_number,
                    index_within_stream,
                } => {
                    write_field(&mut stream_data, 2, w1); // Type 2
                    write_field(&mut stream_data, *stream_object_number as u64, w2);
                    write_field(&mut stream_data, *index_within_stream as u64, w3);
                }
            }
        }

        // Build the stream dictionary
        let mut dict = self.trailer_entries;
        dict.insert(
            "Type".to_string(),
            PdfObject::Name(PdfName("XRef".to_string())),
        );
        dict.insert(
            "W".to_string(),
            PdfObject::Array(PdfArray(vec![
                PdfObject::Integer(w1 as i64),
                PdfObject::Integer(w2 as i64),
                PdfObject::Integer(w3 as i64),
            ])),
        );

        // Add Size
        let size = self.entries.iter().map(|(n, _)| n + 1).max().unwrap_or(0);
        dict.insert("Size".to_string(), PdfObject::Integer(size as i64));

        // Add Index array if not starting from 0
        if !self.entries.is_empty() {
            let first = self.entries[0].0;
            let count = self.entries.len() as u32;
            if first != 0 {
                dict.insert(
                    "Index".to_string(),
                    PdfObject::Array(PdfArray(vec![
                        PdfObject::Integer(first as i64),
                        PdfObject::Integer(count as i64),
                    ])),
                );
            }
        }

        // Add Length
        dict.insert(
            "Length".to_string(),
            PdfObject::Integer(stream_data.len() as i64),
        );

        // Apply compression (FlateDecode)
        let compressed = compress_data(&stream_data)?;
        dict.insert(
            "Filter".to_string(),
            PdfObject::Name(PdfName("FlateDecode".to_string())),
        );

        Ok((dict, compressed))
    }
}

/// Calculate minimum bytes needed to represent a value
fn bytes_needed(value: u64) -> usize {
    if value == 0 {
        1
    } else {
        ((64 - value.leading_zeros()).div_ceil(8)) as usize
    }
}

/// Write a field value with specified width (big-endian)
fn write_field(output: &mut Vec<u8>, value: u64, width: usize) {
    for i in (0..width).rev() {
        output.push((value >> (i * 8)) as u8);
    }
}

/// Compress data using flate compression
fn compress_data(data: &[u8]) -> ParseResult<Vec<u8>> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data)
        .map_err(|e| ParseError::StreamDecodeError(format!("Compression failed: {e}")))?;
    encoder
        .finish()
        .map_err(|e| ParseError::StreamDecodeError(format!("Compression failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_field() {
        assert_eq!(read_field(&[0x00]), 0);
        assert_eq!(read_field(&[0xFF]), 255);
        assert_eq!(read_field(&[0x01, 0x23]), 0x0123);
        assert_eq!(read_field(&[0x12, 0x34, 0x56]), 0x123456);
    }

    #[test]
    fn test_write_field() {
        let mut data = Vec::new();
        write_field(&mut data, 0x1234, 2);
        assert_eq!(data, vec![0x12, 0x34]);

        data.clear();
        write_field(&mut data, 0xFF, 1);
        assert_eq!(data, vec![0xFF]);

        data.clear();
        write_field(&mut data, 0x123456, 3);
        assert_eq!(data, vec![0x12, 0x34, 0x56]);
    }

    #[test]
    fn test_bytes_needed() {
        assert_eq!(bytes_needed(0), 1);
        assert_eq!(bytes_needed(0xFF), 1);
        assert_eq!(bytes_needed(0x100), 2);
        assert_eq!(bytes_needed(0xFFFF), 2);
        assert_eq!(bytes_needed(0x10000), 3);
        assert_eq!(bytes_needed(0xFFFFFF), 3);
        assert_eq!(bytes_needed(0x1000000), 4);
    }

    #[test]
    fn test_xref_stream_builder() {
        let mut builder = XRefStreamBuilder::new();

        // Add some entries
        builder.add_entry(
            0,
            XRefEntry::Free {
                next_free_object: 0,
                generation: 65535,
            },
        );

        builder.add_entry(
            1,
            XRefEntry::InUse {
                offset: 15,
                generation: 0,
            },
        );

        builder.add_entry(
            2,
            XRefEntry::Compressed {
                stream_object_number: 5,
                index_within_stream: 0,
            },
        );

        let result = builder.build();
        assert!(result.is_ok());

        let (dict, _data) = result.unwrap();

        // Check dictionary entries
        assert_eq!(
            dict.get("Type")
                .and_then(|o| o.as_name())
                .map(|n| n.0.as_str()),
            Some("XRef")
        );
        assert!(dict.get("W").is_some());
        assert!(dict.get("Size").is_some());
        assert!(dict.get("Filter").is_some());
    }

    #[test]
    fn test_xref_entry_parsing() {
        // Test data for xref stream entries
        // Type 1 entry: offset=1000, generation=0
        let entry_data = vec![
            1, // Type 1
            0x03, 0xE8, // Offset = 1000 (0x03E8)
            0,    // Generation = 0
        ];

        let xref_stream = XRefStream {
            dict: PdfDictionary::new(),
            data: entry_data,
            widths: vec![1, 2, 1],
            index: vec![(10, 1)],
        };

        let entries = xref_stream.to_xref_entries().unwrap();
        assert_eq!(entries.len(), 1);

        let (obj_num, entry) = &entries[0];
        assert_eq!(*obj_num, 10);

        match entry {
            XRefEntry::InUse { offset, generation } => {
                assert_eq!(*offset, 1000);
                assert_eq!(*generation, 0);
            }
            _ => panic!("Expected InUse entry"),
        }
    }

    #[test]
    fn test_compressed_entry_parsing() {
        // Test compressed object entry
        let entry_data = vec![
            2, // Type 2 (compressed)
            0x00, 0x05, // Stream object number = 5
            0x00, 0x03, // Index within stream = 3
        ];

        let xref_stream = XRefStream {
            dict: PdfDictionary::new(),
            data: entry_data,
            widths: vec![1, 2, 2],
            index: vec![(20, 1)],
        };

        let entries = xref_stream.to_xref_entries().unwrap();
        assert_eq!(entries.len(), 1);

        let (obj_num, entry) = &entries[0];
        assert_eq!(*obj_num, 20);

        match entry {
            XRefEntry::Compressed {
                stream_object_number,
                index_within_stream,
            } => {
                assert_eq!(*stream_object_number, 5);
                assert_eq!(*index_within_stream, 3);
            }
            _ => panic!("Expected Compressed entry"),
        }
    }

    #[test]
    fn test_multiple_index_ranges() {
        // Test with multiple index ranges
        let entry_data = vec![
            // First range: objects 0-1 (each entry: 1+2+2=5 bytes)
            0, 0, 0, 0xFF, 0xFF, // Free object 0
            1, 0, 0x0A, 0, 0, // InUse object 1 at offset 10
            // Second range: objects 10-11
            1, 0, 0x14, 0, 0, // InUse object 10 at offset 20
            1, 0, 0x1E, 0, 0, // InUse object 11 at offset 30
        ];

        let xref_stream = XRefStream {
            dict: PdfDictionary::new(),
            data: entry_data,
            widths: vec![1, 2, 2],
            index: vec![(0, 2), (10, 2)],
        };

        let entries = xref_stream.to_xref_entries().unwrap();
        assert_eq!(entries.len(), 4);

        // Check object numbers
        assert_eq!(entries[0].0, 0);
        assert_eq!(entries[1].0, 1);
        assert_eq!(entries[2].0, 10);
        assert_eq!(entries[3].0, 11);
    }
}
