//! Core types for digital signature handling

use std::fmt;

/// Represents a byte range in a PDF document for signature calculation
///
/// ByteRange is used to specify which parts of the PDF are covered by
/// the signature. It typically consists of two ranges:
/// - From document start to before the /Contents value
/// - From after the /Contents value to document end
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteRange {
    /// The ranges as (offset, length) pairs
    ranges: Vec<(u64, u64)>,
}

impl ByteRange {
    /// Creates a new ByteRange from a list of (offset, length) pairs
    pub fn new(ranges: Vec<(u64, u64)>) -> Self {
        Self { ranges }
    }

    /// Creates a ByteRange from a PDF array [offset1 len1 offset2 len2 ...]
    pub fn from_array(values: &[i64]) -> Result<Self, String> {
        if values.len() % 2 != 0 {
            return Err("ByteRange array must have even number of elements".to_string());
        }
        if values.len() < 4 {
            return Err("ByteRange array must have at least 4 elements".to_string());
        }

        let mut ranges = Vec::with_capacity(values.len() / 2);
        for chunk in values.chunks(2) {
            let offset = chunk[0];
            let length = chunk[1];

            if offset < 0 {
                return Err(format!("ByteRange offset cannot be negative: {}", offset));
            }
            if length < 0 {
                return Err(format!("ByteRange length cannot be negative: {}", length));
            }

            ranges.push((offset as u64, length as u64));
        }

        Ok(Self { ranges })
    }

    /// Returns the ranges as (offset, length) pairs
    pub fn ranges(&self) -> &[(u64, u64)] {
        &self.ranges
    }

    /// Returns the number of range pairs
    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    /// Returns true if there are no ranges
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Calculates the total number of bytes covered by all ranges
    pub fn total_bytes(&self) -> u64 {
        self.ranges.iter().map(|(_, len)| len).sum()
    }

    /// Validates that the ByteRange covers the expected document structure
    ///
    /// A valid signature ByteRange should:
    /// - Have exactly 2 ranges (before and after /Contents)
    /// - First range starts at 0
    /// - Ranges don't overlap
    pub fn validate(&self) -> Result<(), String> {
        if self.ranges.len() != 2 {
            return Err(format!(
                "Expected 2 ranges for signature, got {}",
                self.ranges.len()
            ));
        }

        let (offset1, _len1) = self.ranges[0];
        if offset1 != 0 {
            return Err(format!(
                "First range should start at offset 0, got {}",
                offset1
            ));
        }

        // Check ranges don't overlap
        let (offset1, len1) = self.ranges[0];
        let (offset2, _len2) = self.ranges[1];
        if offset2 < offset1 + len1 {
            return Err("ByteRange ranges overlap".to_string());
        }

        Ok(())
    }
}

impl fmt::Display for ByteRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, (offset, length)) in self.ranges.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{} {}", offset, length)?;
        }
        write!(f, "]")
    }
}

/// Represents a signature field in a PDF document
#[derive(Debug, Clone)]
pub struct SignatureField {
    /// Field name (from /T entry)
    pub name: Option<String>,

    /// The byte range covered by the signature
    pub byte_range: ByteRange,

    /// The raw signature contents (PKCS#7/CMS data)
    pub contents: Vec<u8>,

    /// Signature filter (e.g., "Adobe.PPKLite")
    pub filter: String,

    /// Signature sub-filter (e.g., "adbe.pkcs7.detached", "ETSI.CAdES.detached")
    pub sub_filter: Option<String>,

    /// Signing reason (from /Reason entry)
    pub reason: Option<String>,

    /// Signing location (from /Location entry)
    pub location: Option<String>,

    /// Contact info (from /ContactInfo entry)
    pub contact_info: Option<String>,

    /// Signing time (from /M entry, PDF date format)
    pub signing_time: Option<String>,
}

impl SignatureField {
    /// Creates a new SignatureField with required fields only
    pub fn new(filter: String, byte_range: ByteRange, contents: Vec<u8>) -> Self {
        Self {
            name: None,
            byte_range,
            contents,
            filter,
            sub_filter: None,
            reason: None,
            location: None,
            contact_info: None,
            signing_time: None,
        }
    }

    /// Returns true if this is a PAdES signature
    pub fn is_pades(&self) -> bool {
        self.sub_filter
            .as_ref()
            .map(|sf| sf.contains("CAdES") || sf.contains("cades"))
            .unwrap_or(false)
    }

    /// Returns true if this is a PKCS#7 detached signature
    pub fn is_pkcs7_detached(&self) -> bool {
        self.sub_filter
            .as_ref()
            .map(|sf| sf.contains("pkcs7.detached"))
            .unwrap_or(false)
    }

    /// Returns the size of the signature contents in bytes
    pub fn contents_size(&self) -> usize {
        self.contents.len()
    }
}

impl fmt::Display for SignatureField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SignatureField {{ name: {:?}, filter: {}, sub_filter: {:?}, contents: {} bytes }}",
            self.name,
            self.filter,
            self.sub_filter,
            self.contents.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ByteRange tests

    #[test]
    fn test_byterange_from_array_valid() {
        let values = vec![0, 1000, 2000, 500];
        let br = ByteRange::from_array(&values).unwrap();
        assert_eq!(br.len(), 2);
        assert_eq!(br.ranges()[0], (0, 1000));
        assert_eq!(br.ranges()[1], (2000, 500));
    }

    #[test]
    fn test_byterange_from_array_odd_elements() {
        let values = vec![0, 1000, 2000];
        let result = ByteRange::from_array(&values);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("even"));
    }

    #[test]
    fn test_byterange_from_array_too_few_elements() {
        let values = vec![0, 1000];
        let result = ByteRange::from_array(&values);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least 4"));
    }

    #[test]
    fn test_byterange_from_array_negative_offset() {
        let values = vec![-1, 1000, 2000, 500];
        let result = ByteRange::from_array(&values);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("negative"));
    }

    #[test]
    fn test_byterange_from_array_negative_length() {
        let values = vec![0, -100, 2000, 500];
        let result = ByteRange::from_array(&values);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("negative"));
    }

    #[test]
    fn test_byterange_total_bytes() {
        let br = ByteRange::new(vec![(0, 1000), (2000, 500)]);
        assert_eq!(br.total_bytes(), 1500);
    }

    #[test]
    fn test_byterange_validate_valid() {
        let br = ByteRange::new(vec![(0, 1000), (2000, 500)]);
        assert!(br.validate().is_ok());
    }

    #[test]
    fn test_byterange_validate_wrong_range_count() {
        let br = ByteRange::new(vec![(0, 1000)]);
        let result = br.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("2 ranges"));
    }

    #[test]
    fn test_byterange_validate_first_not_zero() {
        let br = ByteRange::new(vec![(100, 1000), (2000, 500)]);
        let result = br.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("offset 0"));
    }

    #[test]
    fn test_byterange_validate_overlapping() {
        let br = ByteRange::new(vec![(0, 1000), (500, 500)]);
        let result = br.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("overlap"));
    }

    #[test]
    fn test_byterange_display() {
        let br = ByteRange::new(vec![(0, 1000), (2000, 500)]);
        assert_eq!(br.to_string(), "[0 1000 2000 500]");
    }

    #[test]
    fn test_byterange_is_empty() {
        let empty = ByteRange::new(vec![]);
        assert!(empty.is_empty());

        let non_empty = ByteRange::new(vec![(0, 100)]);
        assert!(!non_empty.is_empty());
    }

    // SignatureField tests

    #[test]
    fn test_signature_field_new() {
        let br = ByteRange::new(vec![(0, 1000), (2000, 500)]);
        let contents = vec![0x30, 0x82]; // Start of DER sequence
        let sig = SignatureField::new("Adobe.PPKLite".to_string(), br, contents);

        assert_eq!(sig.filter, "Adobe.PPKLite");
        assert!(sig.name.is_none());
        assert!(sig.sub_filter.is_none());
    }

    #[test]
    fn test_signature_field_is_pades() {
        let br = ByteRange::new(vec![(0, 1000), (2000, 500)]);
        let mut sig = SignatureField::new("Adobe.PPKLite".to_string(), br, vec![]);

        assert!(!sig.is_pades());

        sig.sub_filter = Some("ETSI.CAdES.detached".to_string());
        assert!(sig.is_pades());
    }

    #[test]
    fn test_signature_field_is_pkcs7_detached() {
        let br = ByteRange::new(vec![(0, 1000), (2000, 500)]);
        let mut sig = SignatureField::new("Adobe.PPKLite".to_string(), br, vec![]);

        assert!(!sig.is_pkcs7_detached());

        sig.sub_filter = Some("adbe.pkcs7.detached".to_string());
        assert!(sig.is_pkcs7_detached());
    }

    #[test]
    fn test_signature_field_contents_size() {
        let br = ByteRange::new(vec![(0, 1000), (2000, 500)]);
        let contents = vec![0u8; 4096];
        let sig = SignatureField::new("Adobe.PPKLite".to_string(), br, contents);

        assert_eq!(sig.contents_size(), 4096);
    }

    #[test]
    fn test_signature_field_display() {
        let br = ByteRange::new(vec![(0, 1000), (2000, 500)]);
        let mut sig = SignatureField::new("Adobe.PPKLite".to_string(), br, vec![0u8; 100]);
        sig.name = Some("Signature1".to_string());
        sig.sub_filter = Some("adbe.pkcs7.detached".to_string());

        let display = sig.to_string();
        assert!(display.contains("Signature1"));
        assert!(display.contains("Adobe.PPKLite"));
        assert!(display.contains("100 bytes"));
    }

    #[test]
    fn test_signature_field_clone() {
        let br = ByteRange::new(vec![(0, 1000), (2000, 500)]);
        let sig = SignatureField::new("Adobe.PPKLite".to_string(), br, vec![1, 2, 3]);
        let cloned = sig.clone();

        assert_eq!(sig.filter, cloned.filter);
        assert_eq!(sig.contents, cloned.contents);
    }
}
