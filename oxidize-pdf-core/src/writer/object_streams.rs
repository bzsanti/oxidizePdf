//! Object Streams implementation (ISO 32000-1 Section 7.5.7)
//!
//! Object streams allow multiple non-stream objects to be compressed together,
//! significantly reducing PDF file size (11-61% reduction typical).
//!
//! Requirements:
//! - PDF version must be >= 1.5
//! - Only non-stream objects can be compressed
//! - Stream objects, encryption dictionaries, and object 0 cannot be in object streams

use crate::error::{PdfError, Result};
use crate::objects::{Dictionary, Object, ObjectId};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;

/// Configuration for object stream generation
#[derive(Debug, Clone)]
pub struct ObjectStreamConfig {
    /// Maximum number of objects per stream (default: 100)
    pub max_objects_per_stream: usize,
    /// Compression level (0-9, default: 6)
    pub compression_level: u32,
    /// Enable object streams (default: true for PDF 1.5+)
    pub enabled: bool,
}

impl Default for ObjectStreamConfig {
    fn default() -> Self {
        Self {
            max_objects_per_stream: 100,
            compression_level: 6,
            enabled: true,
        }
    }
}

/// Represents an object stream containing multiple compressed objects
#[derive(Debug, Clone)]
pub struct ObjectStream {
    /// Stream object ID
    pub stream_id: ObjectId,
    /// Objects contained in this stream (id, data)
    pub objects: Vec<(ObjectId, Vec<u8>)>,
    /// First position in stream (N parameter)
    pub first_offset: usize,
}

impl ObjectStream {
    /// Create a new empty object stream
    pub fn new(stream_id: ObjectId) -> Self {
        Self {
            stream_id,
            objects: Vec::new(),
            first_offset: 0,
        }
    }

    /// Add an object to this stream
    pub fn add_object(&mut self, id: ObjectId, data: Vec<u8>) {
        self.objects.push((id, data));
    }

    /// Check if stream is full
    pub fn is_full(&self, max_objects: usize) -> bool {
        self.objects.len() >= max_objects
    }

    /// Check if stream is empty
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Generate the compressed stream data
    pub fn generate_stream_data(&mut self, compression_level: u32) -> Result<Vec<u8>> {
        if self.objects.is_empty() {
            return Err(PdfError::ObjectStreamError(
                "Cannot generate stream from empty object list".to_string(),
            ));
        }

        // Build the index section (N pairs of "obj_num offset")
        let mut index_section = Vec::new();
        let mut object_section = Vec::new();

        let mut current_offset = 0;
        for (id, data) in &self.objects {
            // Write "obj_num offset " to index
            write!(index_section, "{} {} ", id.number(), current_offset).map_err(|e| {
                PdfError::ObjectStreamError(format!("Failed to write index: {}", e))
            })?;

            // Append object data to object section
            object_section.extend_from_slice(data);
            object_section.push(b' '); // Space separator

            current_offset = object_section.len();
        }

        // Store first offset (where objects start after index)
        self.first_offset = index_section.len();

        // Combine index + objects
        let mut uncompressed = index_section;
        uncompressed.extend_from_slice(&object_section);

        // Compress with zlib
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(compression_level.min(9)));
        encoder
            .write_all(&uncompressed)
            .map_err(|e| PdfError::ObjectStreamError(format!("Compression failed: {}", e)))?;

        encoder
            .finish()
            .map_err(|e| PdfError::ObjectStreamError(format!("Compression finish failed: {}", e)))
    }

    /// Generate the stream dictionary for this object stream
    pub fn generate_dictionary(&self, compressed_data: &[u8]) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("ObjStm".to_string()));
        dict.set("N", Object::Integer(self.objects.len() as i64));
        dict.set("First", Object::Integer(self.first_offset as i64));
        dict.set("Length", Object::Integer(compressed_data.len() as i64));
        dict.set("Filter", Object::Name("FlateDecode".to_string()));
        dict
    }
}

/// Writer for managing object streams
pub struct ObjectStreamWriter {
    config: ObjectStreamConfig,
    current_stream: Option<ObjectStream>,
    completed_streams: Vec<ObjectStream>,
    next_stream_id: u32,
}

impl ObjectStreamWriter {
    /// Create a new object stream writer
    pub fn new(config: ObjectStreamConfig) -> Self {
        Self {
            config,
            current_stream: None,
            completed_streams: Vec::new(),
            next_stream_id: 1000000, // Start high to avoid conflicts
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(ObjectStreamConfig::default())
    }

    /// Check if object streams are enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check if an object can be compressed into an object stream
    pub fn can_compress(object: &Object) -> bool {
        match object {
            // Stream objects cannot be in object streams
            Object::Stream(_, _) => false,
            // Object 0 (null object) cannot be compressed
            Object::Null => false,
            // All other object types can be compressed
            _ => true,
        }
    }

    /// Add an object to be compressed
    pub fn add_object(&mut self, id: ObjectId, object_data: Vec<u8>) -> Result<()> {
        if !self.config.enabled {
            return Err(PdfError::ObjectStreamError(
                "Object streams are disabled".to_string(),
            ));
        }

        // Create new stream if needed
        let needs_new_stream = self.current_stream.is_none()
            || self
                .current_stream
                .as_ref()
                .map(|s| s.is_full(self.config.max_objects_per_stream))
                .unwrap_or(false);

        if needs_new_stream {
            self.flush_current_stream();
            let stream_id = ObjectId::new(self.next_stream_id, 0);
            self.next_stream_id += 1;
            self.current_stream = Some(ObjectStream::new(stream_id));
        }

        // Add to current stream
        if let Some(stream) = &mut self.current_stream {
            stream.add_object(id, object_data);
        }

        Ok(())
    }

    /// Flush current stream to completed list
    fn flush_current_stream(&mut self) {
        if let Some(stream) = self.current_stream.take() {
            if !stream.is_empty() {
                self.completed_streams.push(stream);
            }
        }
    }

    /// Finalize and get all completed object streams
    pub fn finalize(mut self) -> Result<Vec<ObjectStream>> {
        self.flush_current_stream();
        Ok(self.completed_streams)
    }

    /// Get compression statistics
    pub fn get_stats(&self) -> ObjectStreamStats {
        let total_objects: usize = self.completed_streams.iter().map(|s| s.objects.len()).sum();

        let current_objects = self
            .current_stream
            .as_ref()
            .map(|s| s.objects.len())
            .unwrap_or(0);

        ObjectStreamStats {
            total_streams: self.completed_streams.len(),
            total_objects: total_objects + current_objects,
            average_objects_per_stream: if !self.completed_streams.is_empty() {
                total_objects as f64 / self.completed_streams.len() as f64
            } else {
                0.0
            },
        }
    }
}

/// Statistics for object stream compression
#[derive(Debug, Clone)]
pub struct ObjectStreamStats {
    pub total_streams: usize,
    pub total_objects: usize,
    pub average_objects_per_stream: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_stream_creation() {
        let stream = ObjectStream::new(ObjectId::new(100, 0));
        assert_eq!(stream.stream_id, ObjectId::new(100, 0));
        assert!(stream.is_empty());
        assert!(!stream.is_full(10));
    }

    #[test]
    fn test_object_stream_add_object() {
        let mut stream = ObjectStream::new(ObjectId::new(100, 0));
        stream.add_object(ObjectId::new(1, 0), b"test data".to_vec());
        assert_eq!(stream.objects.len(), 1);
        assert!(!stream.is_empty());
    }

    #[test]
    fn test_object_stream_is_full() {
        let mut stream = ObjectStream::new(ObjectId::new(100, 0));
        for i in 0..5 {
            stream.add_object(ObjectId::new(i, 0), vec![]);
        }
        assert!(!stream.is_full(10));
        assert!(stream.is_full(5));
    }

    #[test]
    fn test_can_compress() {
        assert!(ObjectStreamWriter::can_compress(&Object::Integer(42)));
        assert!(ObjectStreamWriter::can_compress(&Object::Boolean(true)));
        assert!(ObjectStreamWriter::can_compress(&Object::Name(
            "Test".to_string()
        )));

        let dict = Dictionary::new();
        assert!(ObjectStreamWriter::can_compress(&Object::Dictionary(dict)));

        // Streams cannot be compressed
        let stream_dict = Dictionary::new();
        assert!(!ObjectStreamWriter::can_compress(&Object::Stream(
            stream_dict,
            vec![]
        )));
    }

    #[test]
    fn test_object_stream_generate_data() {
        let mut stream = ObjectStream::new(ObjectId::new(100, 0));
        stream.add_object(ObjectId::new(1, 0), b"<<>>".to_vec());
        stream.add_object(ObjectId::new(2, 0), b"42".to_vec());

        let result = stream.generate_stream_data(6);
        assert!(result.is_ok());
        let compressed = result.unwrap();
        assert!(!compressed.is_empty());
    }

    #[test]
    fn test_object_stream_writer_basic() {
        let config = ObjectStreamConfig {
            max_objects_per_stream: 2,
            compression_level: 6,
            enabled: true,
        };

        let mut writer = ObjectStreamWriter::new(config);

        writer
            .add_object(ObjectId::new(1, 0), b"data1".to_vec())
            .unwrap();
        writer
            .add_object(ObjectId::new(2, 0), b"data2".to_vec())
            .unwrap();

        let stats = writer.get_stats();
        assert_eq!(stats.total_objects, 2);
    }

    #[test]
    fn test_object_stream_writer_multiple_streams() {
        let config = ObjectStreamConfig {
            max_objects_per_stream: 2,
            compression_level: 6,
            enabled: true,
        };

        let mut writer = ObjectStreamWriter::new(config);

        // Add 5 objects (should create 3 streams: 2+2+1)
        for i in 1..=5 {
            writer
                .add_object(ObjectId::new(i, 0), format!("data{}", i).into_bytes())
                .unwrap();
        }

        let streams = writer.finalize().unwrap();
        assert_eq!(streams.len(), 3);
        assert_eq!(streams[0].objects.len(), 2);
        assert_eq!(streams[1].objects.len(), 2);
        assert_eq!(streams[2].objects.len(), 1);
    }

    #[test]
    fn test_disabled_object_streams() {
        let config = ObjectStreamConfig {
            enabled: false,
            ..Default::default()
        };

        let mut writer = ObjectStreamWriter::new(config);
        assert!(!writer.is_enabled());

        let result = writer.add_object(ObjectId::new(1, 0), vec![]);
        assert!(result.is_err());
    }
}
