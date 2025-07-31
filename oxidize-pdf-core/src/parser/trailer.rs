//! PDF Trailer Parser
//!
//! Parses PDF trailer according to ISO 32000-1 Section 7.5.5

use super::objects::{PdfDictionary, PdfObject};
use super::{ParseError, ParseResult};

/// PDF Trailer information
#[derive(Debug, Clone)]
pub struct PdfTrailer {
    /// The trailer dictionary
    pub dict: PdfDictionary,
    /// Byte offset of previous xref section (if any)
    pub prev: Option<u64>,
    /// Byte offset of this xref section
    pub xref_offset: u64,
}

impl PdfTrailer {
    /// Parse trailer from a dictionary
    pub fn from_dict(dict: PdfDictionary, xref_offset: u64) -> ParseResult<Self> {
        // Extract previous xref offset if present
        let prev = dict
            .get("Prev")
            .and_then(|obj| obj.as_integer())
            .map(|i| i as u64);

        Ok(PdfTrailer {
            dict,
            prev,
            xref_offset,
        })
    }

    /// Get the size (number of entries in xref table)
    pub fn size(&self) -> ParseResult<u32> {
        self.dict
            .get("Size")
            .and_then(|obj| obj.as_integer())
            .map(|i| i as u32)
            .ok_or_else(|| ParseError::MissingKey("Size".to_string()))
    }

    /// Get the root object reference (document catalog)
    pub fn root(&self) -> ParseResult<(u32, u16)> {
        self.dict
            .get("Root")
            .and_then(|obj| obj.as_reference())
            .ok_or_else(|| ParseError::MissingKey("Root".to_string()))
    }

    /// Try to find root by scanning for Catalog object
    pub fn find_root_fallback(&self) -> Option<(u32, u16)> {
        // This is a placeholder - actual implementation would scan objects
        // For now, try common object numbers for catalog
        if let Some(obj_num) = [1, 2, 3, 4, 5].into_iter().next() {
            // Would need to check if object exists and is a Catalog
            // For now, return first attempt as a guess
            return Some((obj_num, 0));
        }
        None
    }

    /// Get the info object reference (document information dictionary)
    pub fn info(&self) -> Option<(u32, u16)> {
        self.dict.get("Info").and_then(|obj| obj.as_reference())
    }

    /// Get the ID array (file identifiers)
    pub fn id(&self) -> Option<&PdfObject> {
        self.dict.get("ID")
    }

    /// Check if this PDF is encrypted
    pub fn is_encrypted(&self) -> bool {
        self.dict.contains_key("Encrypt")
    }

    /// Get the encryption dictionary reference
    pub fn encrypt(&self) -> ParseResult<Option<(u32, u16)>> {
        Ok(self.dict.get("Encrypt").and_then(|obj| obj.as_reference()))
    }

    /// Validate the trailer dictionary
    pub fn validate(&self) -> ParseResult<()> {
        // Required entries
        self.size()?;
        self.root()?;

        // Note: Encryption is now handled by the reader, not rejected here

        Ok(())
    }

    /// Get access to the trailer dictionary
    pub fn dict(&self) -> &PdfDictionary {
        &self.dict
    }
}

/// Represents the complete trailer chain for PDFs with updates
#[derive(Debug)]
pub struct TrailerChain {
    /// List of trailers from newest to oldest
    trailers: Vec<PdfTrailer>,
}

impl TrailerChain {
    /// Create a new trailer chain with a single trailer
    pub fn new(trailer: PdfTrailer) -> Self {
        Self {
            trailers: vec![trailer],
        }
    }

    /// Add an older trailer to the chain
    pub fn add_previous(&mut self, trailer: PdfTrailer) {
        self.trailers.push(trailer);
    }

    /// Get the most recent trailer
    pub fn current(&self) -> &PdfTrailer {
        &self.trailers[0]
    }

    /// Get all trailers in the chain
    pub fn all(&self) -> &[PdfTrailer] {
        &self.trailers
    }

    /// Check if there are previous versions
    pub fn has_previous(&self) -> bool {
        self.trailers.len() > 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::objects::{PdfArray, PdfObject, PdfString};

    #[test]
    fn test_trailer_basic() {
        let mut dict = PdfDictionary::new();
        dict.insert("Size".to_string(), PdfObject::Integer(100));
        dict.insert("Root".to_string(), PdfObject::Reference(1, 0));

        let trailer = PdfTrailer::from_dict(dict, 12345).unwrap();

        assert_eq!(trailer.size().unwrap(), 100);
        assert_eq!(trailer.root().unwrap(), (1, 0));
        assert!(trailer.info().is_none());
        assert!(!trailer.is_encrypted());
    }

    #[test]
    fn test_trailer_with_prev() {
        let mut dict = PdfDictionary::new();
        dict.insert("Size".to_string(), PdfObject::Integer(200));
        dict.insert("Root".to_string(), PdfObject::Reference(1, 0));
        dict.insert("Prev".to_string(), PdfObject::Integer(5000));

        let trailer = PdfTrailer::from_dict(dict, 20000).unwrap();

        assert_eq!(trailer.prev, Some(5000));
        assert_eq!(trailer.xref_offset, 20000);
    }

    #[test]
    fn test_trailer_validation() {
        // Missing Size
        let mut dict = PdfDictionary::new();
        dict.insert("Root".to_string(), PdfObject::Reference(1, 0));

        let trailer = PdfTrailer::from_dict(dict, 12345).unwrap();
        assert!(trailer.validate().is_err());

        // Missing Root
        let mut dict = PdfDictionary::new();
        dict.insert("Size".to_string(), PdfObject::Integer(100));

        let trailer = PdfTrailer::from_dict(dict, 12345).unwrap();
        assert!(trailer.validate().is_err());

        // Encrypted
        let mut dict = PdfDictionary::new();
        dict.insert("Size".to_string(), PdfObject::Integer(100));
        dict.insert("Root".to_string(), PdfObject::Reference(1, 0));
        dict.insert("Encrypt".to_string(), PdfObject::Reference(10, 0));

        let trailer = PdfTrailer::from_dict(dict, 12345).unwrap();
        assert!(matches!(
            trailer.validate(),
            Err(ParseError::EncryptionNotSupported)
        ));
    }

    #[test]
    fn test_trailer_with_info() {
        let mut dict = PdfDictionary::new();
        dict.insert("Size".to_string(), PdfObject::Integer(150));
        dict.insert("Root".to_string(), PdfObject::Reference(1, 0));
        dict.insert("Info".to_string(), PdfObject::Reference(2, 0));

        let trailer = PdfTrailer::from_dict(dict, 15000).unwrap();

        assert_eq!(trailer.info(), Some((2, 0)));
        assert_eq!(trailer.size().unwrap(), 150);
    }

    #[test]
    fn test_trailer_with_id() {
        let mut dict = PdfDictionary::new();
        dict.insert("Size".to_string(), PdfObject::Integer(100));
        dict.insert("Root".to_string(), PdfObject::Reference(1, 0));

        let mut id_array = PdfArray::new();
        id_array.push(PdfObject::String(PdfString(b"ID1".to_vec())));
        id_array.push(PdfObject::String(PdfString(b"ID2".to_vec())));
        dict.insert("ID".to_string(), PdfObject::Array(id_array));

        let trailer = PdfTrailer::from_dict(dict, 10000).unwrap();

        assert!(trailer.id().is_some());
        assert!(matches!(trailer.id().unwrap(), PdfObject::Array(_)));
    }

    #[test]
    fn test_trailer_size_missing() {
        let mut dict = PdfDictionary::new();
        dict.insert("Root".to_string(), PdfObject::Reference(1, 0));

        let trailer = PdfTrailer::from_dict(dict, 1000).unwrap();

        match trailer.size() {
            Err(ParseError::MissingKey(key)) => assert_eq!(key, "Size"),
            _ => panic!("Expected MissingKey error for Size"),
        }
    }

    #[test]
    fn test_trailer_root_missing() {
        let mut dict = PdfDictionary::new();
        dict.insert("Size".to_string(), PdfObject::Integer(100));

        let trailer = PdfTrailer::from_dict(dict, 1000).unwrap();

        match trailer.root() {
            Err(ParseError::MissingKey(key)) => assert_eq!(key, "Root"),
            _ => panic!("Expected MissingKey error for Root"),
        }
    }

    #[test]
    fn test_trailer_invalid_size_type() {
        let mut dict = PdfDictionary::new();
        dict.insert(
            "Size".to_string(),
            PdfObject::String(PdfString(b"not a number".to_vec())),
        );
        dict.insert("Root".to_string(), PdfObject::Reference(1, 0));

        let trailer = PdfTrailer::from_dict(dict, 1000).unwrap();

        assert!(trailer.size().is_err());
    }

    #[test]
    fn test_trailer_invalid_root_type() {
        let mut dict = PdfDictionary::new();
        dict.insert("Size".to_string(), PdfObject::Integer(100));
        dict.insert(
            "Root".to_string(),
            PdfObject::String(PdfString(b"not a reference".to_vec())),
        );

        let trailer = PdfTrailer::from_dict(dict, 1000).unwrap();

        assert!(trailer.root().is_err());
    }

    #[test]
    fn test_trailer_encrypt_reference() {
        let mut dict = PdfDictionary::new();
        dict.insert("Size".to_string(), PdfObject::Integer(100));
        dict.insert("Root".to_string(), PdfObject::Reference(1, 0));
        dict.insert("Encrypt".to_string(), PdfObject::Reference(5, 0));

        let trailer = PdfTrailer::from_dict(dict, 1000).unwrap();

        assert!(trailer.is_encrypted());
        assert_eq!(trailer.encrypt().unwrap(), Some((5, 0)));
    }

    #[test]
    fn test_trailer_chain_single() {
        let mut dict = PdfDictionary::new();
        dict.insert("Size".to_string(), PdfObject::Integer(100));
        dict.insert("Root".to_string(), PdfObject::Reference(1, 0));

        let trailer = PdfTrailer::from_dict(dict, 1000).unwrap();
        let chain = TrailerChain::new(trailer.clone());

        assert!(!chain.has_previous());
        assert_eq!(chain.all().len(), 1);
        assert_eq!(chain.current().xref_offset, 1000);
    }

    #[test]
    fn test_trailer_chain_multiple() {
        let mut dict1 = PdfDictionary::new();
        dict1.insert("Size".to_string(), PdfObject::Integer(100));
        dict1.insert("Root".to_string(), PdfObject::Reference(1, 0));
        dict1.insert("Prev".to_string(), PdfObject::Integer(500));
        let trailer1 = PdfTrailer::from_dict(dict1, 1000).unwrap();

        let mut dict2 = PdfDictionary::new();
        dict2.insert("Size".to_string(), PdfObject::Integer(80));
        dict2.insert("Root".to_string(), PdfObject::Reference(1, 0));
        let trailer2 = PdfTrailer::from_dict(dict2, 500).unwrap();

        let mut chain = TrailerChain::new(trailer1);
        chain.add_previous(trailer2);

        assert!(chain.has_previous());
        assert_eq!(chain.all().len(), 2);
        assert_eq!(chain.current().xref_offset, 1000);
        assert_eq!(chain.all()[1].xref_offset, 500);
    }

    #[test]
    fn test_trailer_prev_as_float() {
        let mut dict = PdfDictionary::new();
        dict.insert("Size".to_string(), PdfObject::Integer(100));
        dict.insert("Root".to_string(), PdfObject::Reference(1, 0));
        dict.insert("Prev".to_string(), PdfObject::Real(5000.0));

        let trailer = PdfTrailer::from_dict(dict, 10000).unwrap();

        // Real numbers should not be converted to prev offset
        assert_eq!(trailer.prev, None);
    }

    #[test]
    fn test_trailer_large_values() {
        let mut dict = PdfDictionary::new();
        dict.insert("Size".to_string(), PdfObject::Integer(i64::MAX));
        dict.insert("Root".to_string(), PdfObject::Reference(u32::MAX, u16::MAX));
        dict.insert("Prev".to_string(), PdfObject::Integer(i64::MAX));

        let trailer = PdfTrailer::from_dict(dict, u64::MAX).unwrap();

        assert_eq!(trailer.size().unwrap(), u32::MAX);
        assert_eq!(trailer.root().unwrap(), (u32::MAX, u16::MAX));
        assert_eq!(trailer.prev, Some(i64::MAX as u64));
        assert_eq!(trailer.xref_offset, u64::MAX);
    }

    #[test]
    fn test_trailer_all_optional_fields() {
        let mut dict = PdfDictionary::new();
        dict.insert("Size".to_string(), PdfObject::Integer(200));
        dict.insert("Root".to_string(), PdfObject::Reference(1, 0));
        dict.insert("Info".to_string(), PdfObject::Reference(2, 0));
        dict.insert("Prev".to_string(), PdfObject::Integer(1000));

        let mut id_array = PdfArray::new();
        id_array.push(PdfObject::String(PdfString(b"FirstID".to_vec())));
        id_array.push(PdfObject::String(PdfString(b"SecondID".to_vec())));
        dict.insert("ID".to_string(), PdfObject::Array(id_array));

        let trailer = PdfTrailer::from_dict(dict.clone(), 5000).unwrap();

        assert_eq!(trailer.size().unwrap(), 200);
        assert_eq!(trailer.root().unwrap(), (1, 0));
        assert_eq!(trailer.info(), Some((2, 0)));
        assert_eq!(trailer.prev, Some(1000));
        assert!(trailer.id().is_some());
        assert!(!trailer.is_encrypted());
        assert_eq!(trailer.xref_offset, 5000);

        // Verify validation passes
        assert!(trailer.validate().is_ok());
    }

    #[test]
    fn test_trailer_chain_ordering() {
        let trailers: Vec<PdfTrailer> = (0..5)
            .map(|i| {
                let mut dict = PdfDictionary::new();
                dict.insert("Size".to_string(), PdfObject::Integer(100 + i));
                dict.insert("Root".to_string(), PdfObject::Reference(1, 0));
                if i > 0 {
                    dict.insert("Prev".to_string(), PdfObject::Integer(i * 1000));
                }
                PdfTrailer::from_dict(dict, ((i + 1) * 1000) as u64).unwrap()
            })
            .collect();

        let mut chain = TrailerChain::new(trailers[0].clone());
        for trailer in trailers.iter().skip(1) {
            chain.add_previous(trailer.clone());
        }

        assert_eq!(chain.all().len(), 5);
        assert!(chain.has_previous());

        // Verify ordering (newest first)
        assert_eq!(chain.current().xref_offset, 1000);
        assert_eq!(chain.all()[0].xref_offset, 1000);
        assert_eq!(chain.all()[4].xref_offset, 5000);
    }
}
