//! XRef Stream Writer for PDF 1.5+
//!
//! This module implements writing cross-reference streams according to
//! ISO 32000-1:2008 Section 7.5.8.

use crate::error::Result;
use crate::objects::{Dictionary, Object, ObjectId};
use crate::parser::xref_stream::XRefEntry;
use std::io::Write;

/// Helper function to write object values
fn write_object_value<W: Write>(writer: &mut W, object: &Object) -> Result<()> {
    match object {
        Object::Null => write!(writer, "null")?,
        Object::Boolean(b) => write!(writer, "{}", if *b { "true" } else { "false" })?,
        Object::Integer(i) => write!(writer, "{}", i)?,
        Object::Real(f) => write!(writer, "{:.6}", f)?,
        Object::String(s) => {
            write!(writer, "(")?;
            writer.write_all(s.as_bytes())?;
            write!(writer, ")")?;
        }
        Object::Name(n) => write!(writer, "/{}", n)?,
        Object::Array(arr) => {
            write!(writer, "[")?;
            for (i, obj) in arr.iter().enumerate() {
                if i > 0 {
                    write!(writer, " ")?;
                }
                write_object_value(writer, obj)?;
            }
            write!(writer, "]")?;
        }
        Object::Dictionary(dict) => {
            write!(writer, "<<")?;
            for (key, value) in dict.iter() {
                write!(writer, " /{} ", key)?;
                write_object_value(writer, value)?;
            }
            write!(writer, " >>")?;
        }
        Object::Reference(id) => write!(writer, "{} {} R", id.number(), id.generation())?,
        _ => {
            return Err(crate::error::PdfError::InvalidStructure(
                "Cannot write stream object directly".to_string(),
            ))
        }
    }
    Ok(())
}

/// Writer for XRef streams
pub struct XRefStreamWriter {
    /// Entries to be written
    entries: Vec<XRefEntry>,
    /// Field widths [type, field2, field3]
    widths: [usize; 3],
    /// Object ID for this XRef stream
    stream_id: ObjectId,
    /// Trailer information
    root_id: Option<ObjectId>,
    info_id: Option<ObjectId>,
}

impl XRefStreamWriter {
    /// Create a new XRef stream writer
    pub fn new(stream_id: ObjectId) -> Self {
        Self {
            entries: Vec::new(),
            // Default widths: 1 byte for type, 3 bytes for offsets, 2 bytes for generation
            widths: [1, 3, 2],
            stream_id,
            root_id: None,
            info_id: None,
        }
    }

    /// Set trailer information
    pub fn set_trailer_info(&mut self, root_id: ObjectId, info_id: ObjectId) {
        self.root_id = Some(root_id);
        self.info_id = Some(info_id);
    }

    /// Add a free entry
    pub fn add_free_entry(&mut self, next_free: u32, generation: u16) {
        self.entries.push(XRefEntry::Free {
            next_free_object: next_free,
            generation,
        });
    }

    /// Add an in-use entry
    pub fn add_in_use_entry(&mut self, offset: u64, generation: u16) {
        self.entries.push(XRefEntry::InUse { offset, generation });

        // Update widths if needed
        let offset_bytes = Self::bytes_needed(offset);
        if offset_bytes > self.widths[1] {
            self.widths[1] = offset_bytes;
        }
    }

    /// Add a compressed entry
    pub fn add_compressed_entry(&mut self, stream_object_number: u32, index: u32) {
        self.entries.push(XRefEntry::Compressed {
            stream_object_number,
            index_within_stream: index,
        });

        // Update widths if needed
        let stream_bytes = Self::bytes_needed(stream_object_number as u64);
        if stream_bytes > self.widths[1] {
            self.widths[1] = stream_bytes;
        }

        let index_bytes = Self::bytes_needed(index as u64);
        if index_bytes > self.widths[2] {
            self.widths[2] = index_bytes;
        }
    }

    /// Calculate minimum bytes needed to represent a value
    fn bytes_needed(value: u64) -> usize {
        if value == 0 {
            1
        } else {
            ((value.ilog2() / 8) + 1) as usize
        }
    }

    /// Encode entries into binary data
    pub fn encode_entries(&self) -> Vec<u8> {
        let mut data = Vec::new();

        for entry in &self.entries {
            match entry {
                XRefEntry::Free {
                    next_free_object,
                    generation,
                } => {
                    // Type 0: free object
                    Self::write_field(&mut data, 0, self.widths[0]);
                    Self::write_field(&mut data, *next_free_object as u64, self.widths[1]);
                    Self::write_field(&mut data, *generation as u64, self.widths[2]);
                }
                XRefEntry::InUse { offset, generation } => {
                    // Type 1: in-use object
                    Self::write_field(&mut data, 1, self.widths[0]);
                    Self::write_field(&mut data, *offset, self.widths[1]);
                    Self::write_field(&mut data, *generation as u64, self.widths[2]);
                }
                XRefEntry::Compressed {
                    stream_object_number,
                    index_within_stream,
                } => {
                    // Type 2: compressed object
                    Self::write_field(&mut data, 2, self.widths[0]);
                    Self::write_field(&mut data, *stream_object_number as u64, self.widths[1]);
                    Self::write_field(&mut data, *index_within_stream as u64, self.widths[2]);
                }
            }
        }

        data
    }

    /// Write a field with the specified width
    fn write_field(data: &mut Vec<u8>, value: u64, width: usize) {
        for i in (0..width).rev() {
            data.push(((value >> (i * 8)) & 0xFF) as u8);
        }
    }

    /// Create the XRef stream dictionary
    pub fn create_dictionary(&self, prev_xref: Option<u64>) -> Dictionary {
        let mut dict = Dictionary::new();

        // Required entries
        dict.set("Type", Object::Name("XRef".to_string()));
        dict.set("Size", Object::Integer(self.entries.len() as i64));

        // Trailer entries (Root and Info)
        if let Some(root_id) = self.root_id {
            dict.set("Root", Object::Reference(root_id));
        }
        if let Some(info_id) = self.info_id {
            dict.set("Info", Object::Reference(info_id));
        }

        // W array specifying field widths
        dict.set(
            "W",
            Object::Array(vec![
                Object::Integer(self.widths[0] as i64),
                Object::Integer(self.widths[1] as i64),
                Object::Integer(self.widths[2] as i64),
            ]),
        );

        // Index array (default is [0 Size])
        dict.set(
            "Index",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(self.entries.len() as i64),
            ]),
        );

        // Filter (always use FlateDecode for compression)
        dict.set("Filter", Object::Name("FlateDecode".to_string()));

        // Previous xref offset if this is an incremental update
        if let Some(prev) = prev_xref {
            dict.set("Prev", Object::Integer(prev as i64));
        }

        dict
    }

    /// Write the complete XRef stream object
    pub fn write_xref_stream<W: Write>(
        &self,
        writer: &mut W,
        _stream_position: u64,
        prev_xref: Option<u64>,
    ) -> Result<()> {
        // Encode the entries
        let uncompressed_data = self.encode_entries();

        // Compress with FlateDecode
        let compressed_data = crate::compression::compress(&uncompressed_data)?;

        // Create the stream dictionary
        let mut dict = self.create_dictionary(prev_xref);
        dict.set("Length", Object::Integer(compressed_data.len() as i64));

        // Write the object header
        writeln!(
            writer,
            "{} {} obj",
            self.stream_id.number(),
            self.stream_id.generation()
        )?;

        // Write the dictionary as a stream dictionary
        write!(writer, "<<")?;
        for (key, value) in dict.iter() {
            write!(writer, "\n/{} ", key)?;
            write_object_value(writer, value)?;
        }
        write!(writer, "\n>>")?;

        // Write the stream
        writeln!(writer, "\nstream")?;
        writer.write_all(&compressed_data)?;
        writeln!(writer, "\nendstream")?;
        writeln!(writer, "endobj")?;

        Ok(())
    }

    /// Get the number of entries
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get the stream object ID
    pub fn stream_id(&self) -> ObjectId {
        self.stream_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_needed() {
        assert_eq!(XRefStreamWriter::bytes_needed(0), 1);
        assert_eq!(XRefStreamWriter::bytes_needed(255), 1);
        assert_eq!(XRefStreamWriter::bytes_needed(256), 2);
        assert_eq!(XRefStreamWriter::bytes_needed(65535), 2);
        assert_eq!(XRefStreamWriter::bytes_needed(65536), 3);
        assert_eq!(XRefStreamWriter::bytes_needed(16777215), 3);
        assert_eq!(XRefStreamWriter::bytes_needed(16777216), 4);
    }

    #[test]
    fn test_encode_free_entry() {
        let mut writer = XRefStreamWriter::new(ObjectId::new(1, 0));
        writer.add_free_entry(42, 1);

        let data = writer.encode_entries();
        assert_eq!(data.len(), 6); // 1 + 3 + 2 bytes
        assert_eq!(data[0], 0); // Type 0
        assert_eq!(data[1], 0); // High byte of offset
        assert_eq!(data[2], 0); // Middle byte of offset
        assert_eq!(data[3], 42); // Low byte of offset
        assert_eq!(data[4], 0); // High byte of generation
        assert_eq!(data[5], 1); // Low byte of generation
    }

    #[test]
    fn test_encode_in_use_entry() {
        let mut writer = XRefStreamWriter::new(ObjectId::new(1, 0));
        writer.add_in_use_entry(0x123456, 0);

        let data = writer.encode_entries();
        assert_eq!(data.len(), 6);
        assert_eq!(data[0], 1); // Type 1
        assert_eq!(data[1], 0x12); // High byte of offset
        assert_eq!(data[2], 0x34); // Middle byte of offset
        assert_eq!(data[3], 0x56); // Low byte of offset
        assert_eq!(data[4], 0); // Generation high
        assert_eq!(data[5], 0); // Generation low
    }

    #[test]
    fn test_encode_compressed_entry() {
        let mut writer = XRefStreamWriter::new(ObjectId::new(1, 0));
        writer.add_compressed_entry(5, 3);

        let data = writer.encode_entries();
        assert_eq!(data.len(), 6);
        assert_eq!(data[0], 2); // Type 2
        assert_eq!(data[1], 0); // High byte of stream object
        assert_eq!(data[2], 0); // Middle byte
        assert_eq!(data[3], 5); // Low byte of stream object
        assert_eq!(data[4], 0); // Index high
        assert_eq!(data[5], 3); // Index low
    }

    #[test]
    fn test_width_adjustment() {
        let mut writer = XRefStreamWriter::new(ObjectId::new(1, 0));

        // Add entry with large offset that requires 4 bytes
        writer.add_in_use_entry(0x12345678, 0);

        assert_eq!(writer.widths[1], 4); // Should have adjusted to 4 bytes

        let data = writer.encode_entries();
        assert_eq!(data.len(), 7); // 1 + 4 + 2 bytes
    }

    #[test]
    fn test_set_trailer_info() {
        let mut writer = XRefStreamWriter::new(ObjectId::new(1, 0));
        let root_id = ObjectId::new(2, 0);
        let info_id = ObjectId::new(3, 0);

        writer.set_trailer_info(root_id, info_id);

        assert_eq!(writer.root_id, Some(root_id));
        assert_eq!(writer.info_id, Some(info_id));
    }

    #[test]
    fn test_create_dictionary_with_trailer() {
        let mut writer = XRefStreamWriter::new(ObjectId::new(1, 0));
        let root_id = ObjectId::new(2, 0);
        let info_id = ObjectId::new(3, 0);

        writer.set_trailer_info(root_id, info_id);
        writer.add_in_use_entry(100, 0);

        let dict = writer.create_dictionary(None);

        // Check required entries
        assert_eq!(dict.get("Type").and_then(|o| o.as_name()), Some("XRef"));
        assert_eq!(dict.get("Size").and_then(|o| o.as_integer()), Some(1));

        // Check trailer entries
        match dict.get("Root") {
            Some(Object::Reference(id)) => assert_eq!(*id, root_id),
            _ => panic!("Expected Root reference"),
        }
        match dict.get("Info") {
            Some(Object::Reference(id)) => assert_eq!(*id, info_id),
            _ => panic!("Expected Info reference"),
        }

        // Check other required entries
        assert!(dict.get("W").is_some());
        assert!(dict.get("Index").is_some());
        assert_eq!(
            dict.get("Filter").and_then(|o| o.as_name()),
            Some("FlateDecode")
        );
    }

    #[test]
    fn test_write_xref_stream() {
        use std::io::Cursor;

        let mut buffer = Vec::new();
        let mut writer = XRefStreamWriter::new(ObjectId::new(5, 0));

        writer.set_trailer_info(ObjectId::new(1, 0), ObjectId::new(2, 0));
        writer.add_free_entry(0, 65535);
        writer.add_in_use_entry(15, 0);
        writer.add_in_use_entry(94, 0);

        let result = writer.write_xref_stream(&mut Cursor::new(&mut buffer), 200, None);
        assert!(result.is_ok());

        let content = String::from_utf8_lossy(&buffer);

        // Check object header
        assert!(content.contains("5 0 obj"));

        // Check dictionary entries
        assert!(content.contains("/Type /XRef"));
        assert!(content.contains("/Root 1 0 R"));
        assert!(content.contains("/Info 2 0 R"));
        assert!(content.contains("/Filter /FlateDecode"));
        assert!(content.contains("/W ["));

        // Check stream markers
        assert!(content.contains("stream"));
        assert!(content.contains("endstream"));
        assert!(content.contains("endobj"));
    }

    #[test]
    fn test_multiple_entry_types() {
        let mut writer = XRefStreamWriter::new(ObjectId::new(1, 0));

        // Add different entry types
        writer.add_free_entry(0, 65535);
        writer.add_in_use_entry(100, 0);
        writer.add_compressed_entry(5, 3);
        writer.add_in_use_entry(200, 1);

        let data = writer.encode_entries();

        // Verify we have 4 entries
        assert_eq!(writer.entry_count(), 4);

        // Each entry should be 6 bytes (1 + 3 + 2)
        assert_eq!(data.len(), 24);

        // Verify first entry (free)
        assert_eq!(data[0], 0); // Type 0

        // Verify second entry (in-use)
        assert_eq!(data[6], 1); // Type 1

        // Verify third entry (compressed)
        assert_eq!(data[12], 2); // Type 2

        // Verify fourth entry (in-use with generation 1)
        assert_eq!(data[18], 1); // Type 1
        assert_eq!(data[23], 1); // Generation 1
    }
}
