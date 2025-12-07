#[cfg(feature = "compression")]
use crate::error::{PdfError, Result};
use crate::objects::Dictionary;

#[derive(Debug, Clone)]
pub struct Stream {
    dictionary: Dictionary,
    data: Vec<u8>,
}

impl Stream {
    pub fn new(data: Vec<u8>) -> Self {
        let mut dictionary = Dictionary::new();
        dictionary.set("Length", data.len() as i64);

        Self { dictionary, data }
    }

    pub fn with_dictionary(dictionary: Dictionary, data: Vec<u8>) -> Self {
        let mut dict = dictionary;
        dict.set("Length", data.len() as i64);

        Self {
            dictionary: dict,
            data,
        }
    }

    pub fn dictionary(&self) -> &Dictionary {
        &self.dictionary
    }

    pub fn dictionary_mut(&mut self) -> &mut Dictionary {
        &mut self.dictionary
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }

    pub fn set_filter(&mut self, filter: &str) {
        self.dictionary
            .set("Filter", crate::objects::Object::Name(filter.to_string()));
    }

    pub fn set_decode_params(&mut self, params: Dictionary) {
        self.dictionary.set("DecodeParms", params);
    }

    #[cfg(feature = "compression")]
    pub fn compress_flate(&mut self) -> Result<()> {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&self.data)
            .map_err(|e| PdfError::CompressionError(e.to_string()))?;
        let compressed = encoder
            .finish()
            .map_err(|e| PdfError::CompressionError(e.to_string()))?;

        self.data = compressed;
        self.dictionary.set("Length", self.data.len() as i64);
        self.set_filter("FlateDecode");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::Object;

    #[test]
    fn test_stream_new() {
        let data = vec![1, 2, 3, 4, 5];
        let stream = Stream::new(data.clone());

        assert_eq!(stream.data(), &data);
        assert_eq!(stream.dictionary().get("Length"), Some(&Object::Integer(5)));
    }

    #[test]
    fn test_stream_with_dictionary() {
        let mut dict = Dictionary::new();
        dict.set("Type", "XObject");
        dict.set("Subtype", "Image");

        let data = vec![0xFF, 0xD8, 0xFF];
        let stream = Stream::with_dictionary(dict, data.clone());

        assert_eq!(stream.data(), &data);
        assert_eq!(stream.dictionary().get("Length"), Some(&Object::Integer(3)));
        assert_eq!(
            stream.dictionary().get("Type"),
            Some(&Object::String("XObject".to_string()))
        );
        assert_eq!(
            stream.dictionary().get("Subtype"),
            Some(&Object::String("Image".to_string()))
        );
    }

    #[test]
    fn test_stream_accessors() {
        let data = vec![10, 20, 30];
        let stream = Stream::new(data);

        // Test dictionary accessor
        let dict = stream.dictionary();
        assert_eq!(dict.get("Length"), Some(&Object::Integer(3)));

        // Test data accessor
        assert_eq!(stream.data(), &[10, 20, 30]);
    }

    #[test]
    fn test_stream_mutators() {
        let data = vec![1, 2, 3];
        let mut stream = Stream::new(data);

        // Test dictionary_mut
        stream.dictionary_mut().set("CustomKey", "CustomValue");
        assert_eq!(
            stream.dictionary().get("CustomKey"),
            Some(&Object::String("CustomValue".to_string()))
        );

        // Test data_mut
        stream.data_mut().push(4);
        assert_eq!(stream.data(), &[1, 2, 3, 4]);

        // Note: Length is not automatically updated when using data_mut
        assert_eq!(stream.dictionary().get("Length"), Some(&Object::Integer(3)));
    }

    #[test]
    fn test_set_filter() {
        let mut stream = Stream::new(vec![1, 2, 3]);

        stream.set_filter("FlateDecode");
        assert_eq!(
            stream.dictionary().get("Filter"),
            Some(&Object::Name("FlateDecode".to_string()))
        );

        stream.set_filter("ASCII85Decode");
        assert_eq!(
            stream.dictionary().get("Filter"),
            Some(&Object::Name("ASCII85Decode".to_string()))
        );
    }

    #[test]
    fn test_set_decode_params() {
        let mut stream = Stream::new(vec![1, 2, 3]);

        let mut params = Dictionary::new();
        params.set("Predictor", 12);
        params.set("Colors", 1);
        params.set("BitsPerComponent", 8);
        params.set("Columns", 100);

        stream.set_decode_params(params.clone());

        if let Some(Object::Dictionary(decode_params)) = stream.dictionary().get("DecodeParms") {
            assert_eq!(decode_params.get("Predictor"), Some(&Object::Integer(12)));
            assert_eq!(decode_params.get("Colors"), Some(&Object::Integer(1)));
            assert_eq!(
                decode_params.get("BitsPerComponent"),
                Some(&Object::Integer(8))
            );
            assert_eq!(decode_params.get("Columns"), Some(&Object::Integer(100)));
        } else {
            panic!("Expected DecodeParms to be a Dictionary");
        }
    }

    #[test]
    fn test_empty_stream() {
        let stream = Stream::new(vec![]);

        assert_eq!(stream.data(), &[] as &[u8]);
        assert_eq!(stream.dictionary().get("Length"), Some(&Object::Integer(0)));
    }

    #[test]
    fn test_large_stream() {
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let stream = Stream::new(data);

        assert_eq!(stream.data().len(), 1000);
        assert_eq!(
            stream.dictionary().get("Length"),
            Some(&Object::Integer(1000))
        );
    }

    #[test]
    #[cfg(feature = "compression")]
    fn test_compress_flate() {
        // Use a longer, more repetitive string that will compress well
        let original_data = "Hello, this is a test string that should be compressed! "
            .repeat(10)
            .into_bytes();
        let mut stream = Stream::new(original_data.clone());

        // Compress the stream
        let result = stream.compress_flate();
        assert!(result.is_ok());

        // Check that the stream has been modified
        assert_ne!(stream.data(), &original_data);

        // For very small data, compression might not always reduce size due to headers
        // So we just check that it's different

        // Check that the filter has been set
        assert_eq!(
            stream.dictionary().get("Filter"),
            Some(&Object::Name("FlateDecode".to_string()))
        );

        // Check that the length has been updated
        assert_eq!(
            stream.dictionary().get("Length"),
            Some(&Object::Integer(stream.data().len() as i64))
        );
    }

    #[test]
    #[cfg(feature = "compression")]
    fn test_compress_flate_empty() {
        let mut stream = Stream::new(vec![]);

        let result = stream.compress_flate();
        assert!(result.is_ok());

        // Even empty data should have some compressed bytes (header/trailer)
        assert!(!stream.data().is_empty());

        assert_eq!(
            stream.dictionary().get("Filter"),
            Some(&Object::Name("FlateDecode".to_string()))
        );
    }

    #[test]
    fn test_stream_with_existing_length() {
        // Test that Length is properly updated even if dictionary already has it
        let mut dict = Dictionary::new();
        dict.set("Length", 999); // Wrong length
        dict.set("Type", "XObject");

        let data = vec![1, 2, 3, 4, 5];
        let stream = Stream::with_dictionary(dict, data);

        // Length should be corrected to actual data length
        assert_eq!(stream.dictionary().get("Length"), Some(&Object::Integer(5)));
    }
}
