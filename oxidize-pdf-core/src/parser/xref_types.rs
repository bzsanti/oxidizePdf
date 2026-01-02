//! XRef Entry Type Definitions
//!
//! This module defines all valid XRef entry types according to the PDF specification.
//! It provides a comprehensive handling of cross-reference entry types beyond the basic 'n' and 'f'.

// Module for XRef type definitions - no parsing errors needed here

/// XRef entry type enumeration covering all valid types in PDF specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XRefEntryType {
    /// Type 0: Free object (f)
    Free,
    /// Type 1: Normal uncompressed object (n)
    Uncompressed,
    /// Type 2: Compressed object in an object stream
    Compressed,
    /// Other valid but less common types
    /// Some PDF writers use custom types for internal purposes
    Custom(u8),
}

impl XRefEntryType {
    /// Parse entry type from a numeric value
    pub fn from_value(value: u64) -> Self {
        match value {
            0 => XRefEntryType::Free,
            1 => XRefEntryType::Uncompressed,
            2 => XRefEntryType::Compressed,
            n if n <= 255 => XRefEntryType::Custom(n as u8),
            _ => XRefEntryType::Free, // Invalid types default to free
        }
    }

    /// Check if this type represents an in-use object
    pub fn is_in_use(&self) -> bool {
        match self {
            XRefEntryType::Free => false,
            XRefEntryType::Uncompressed => true,
            XRefEntryType::Compressed => true,
            // Custom types: treat as in-use unless proven otherwise
            // This is safer than discarding potentially valid objects
            XRefEntryType::Custom(_) => true,
        }
    }

    /// Get the numeric value for this type
    pub fn to_value(&self) -> u8 {
        match self {
            XRefEntryType::Free => 0,
            XRefEntryType::Uncompressed => 1,
            XRefEntryType::Compressed => 2,
            XRefEntryType::Custom(n) => *n,
        }
    }
}

/// Extended XRef entry information
#[derive(Debug, Clone, PartialEq)]
pub struct XRefEntryInfo {
    /// Entry type
    pub entry_type: XRefEntryType,
    /// Field 2 interpretation depends on type:
    /// - Free: next free object number
    /// - Uncompressed: byte offset
    /// - Compressed: object stream number
    pub field2: u64,
    /// Field 3 interpretation depends on type:
    /// - Free: generation number
    /// - Uncompressed: generation number
    /// - Compressed: index within object stream
    pub field3: u64,
}

impl XRefEntryInfo {
    /// Create a new XRef entry info
    pub fn new(entry_type: XRefEntryType, field2: u64, field3: u64) -> Self {
        Self {
            entry_type,
            field2,
            field3,
        }
    }

    /// Get byte offset for uncompressed objects
    pub fn get_offset(&self) -> Option<u64> {
        match self.entry_type {
            XRefEntryType::Uncompressed => Some(self.field2),
            _ => None,
        }
    }

    /// Get generation number
    pub fn get_generation(&self) -> u16 {
        match self.entry_type {
            XRefEntryType::Free | XRefEntryType::Uncompressed => self.field3 as u16,
            _ => 0,
        }
    }

    /// Get compressed object info (stream number, index)
    pub fn get_compressed_info(&self) -> Option<(u32, u32)> {
        match self.entry_type {
            XRefEntryType::Compressed => Some((self.field2 as u32, self.field3 as u32)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xref_entry_type_from_value() {
        assert_eq!(XRefEntryType::from_value(0), XRefEntryType::Free);
        assert_eq!(XRefEntryType::from_value(1), XRefEntryType::Uncompressed);
        assert_eq!(XRefEntryType::from_value(2), XRefEntryType::Compressed);
        assert_eq!(XRefEntryType::from_value(53), XRefEntryType::Custom(53));
        assert_eq!(XRefEntryType::from_value(255), XRefEntryType::Custom(255));
        // Values > 255 default to Free
        assert_eq!(XRefEntryType::from_value(256), XRefEntryType::Free);
    }

    #[test]
    fn test_is_in_use() {
        assert!(!XRefEntryType::Free.is_in_use());
        assert!(XRefEntryType::Uncompressed.is_in_use());
        assert!(XRefEntryType::Compressed.is_in_use());
        assert!(XRefEntryType::Custom(53).is_in_use());
    }

    #[test]
    fn test_entry_info() {
        let info = XRefEntryInfo::new(XRefEntryType::Uncompressed, 1234, 5);
        assert_eq!(info.get_offset(), Some(1234));
        assert_eq!(info.get_generation(), 5);
        assert_eq!(info.get_compressed_info(), None);

        let compressed = XRefEntryInfo::new(XRefEntryType::Compressed, 10, 20);
        assert_eq!(compressed.get_offset(), None);
        assert_eq!(compressed.get_generation(), 0);
        assert_eq!(compressed.get_compressed_info(), Some((10, 20)));
    }

    #[test]
    fn test_to_value() {
        assert_eq!(XRefEntryType::Free.to_value(), 0);
        assert_eq!(XRefEntryType::Uncompressed.to_value(), 1);
        assert_eq!(XRefEntryType::Compressed.to_value(), 2);
        assert_eq!(XRefEntryType::Custom(53).to_value(), 53);
        assert_eq!(XRefEntryType::Custom(255).to_value(), 255);
    }

    #[test]
    fn test_free_entry_info() {
        let free = XRefEntryInfo::new(XRefEntryType::Free, 100, 65535);
        assert_eq!(free.get_offset(), None);
        assert_eq!(free.get_generation(), 65535);
        assert_eq!(free.get_compressed_info(), None);
    }

    #[test]
    fn test_custom_entry_info() {
        let custom = XRefEntryInfo::new(XRefEntryType::Custom(99), 500, 10);
        assert_eq!(custom.get_offset(), None);
        assert_eq!(custom.get_generation(), 0);
        assert_eq!(custom.get_compressed_info(), None);
    }
}
