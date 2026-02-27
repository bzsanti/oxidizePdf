//! Huffman decoding for JBIG2
//!
//! This module implements Huffman decoding per ITU-T T.88 Annex B.
//! JBIG2 uses standard Huffman tables (B.1-B.15) for decoding various
//! integer values such as symbol heights, widths, and coordinates.
//!
//! Key concepts:
//! - Standard tables are predefined (Tables B.1-B.15)
//! - User-defined tables can be specified in the bitstream
//! - Values are decoded using prefix-free codes

use super::bitstream::BitstreamReader;

/// Error type for Huffman decoding
#[derive(Debug, Clone, PartialEq)]
pub enum HuffmanError {
    /// Invalid or unrecognized code
    InvalidCode,
    /// Table index out of range
    InvalidTableIndex(u8),
    /// Code too long (likely corrupt data)
    CodeTooLong,
    /// Out-of-band value encountered
    OutOfBand,
}

impl std::fmt::Display for HuffmanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HuffmanError::InvalidCode => write!(f, "Invalid Huffman code"),
            HuffmanError::InvalidTableIndex(idx) => {
                write!(f, "Invalid Huffman table index: {}", idx)
            }
            HuffmanError::CodeTooLong => write!(f, "Huffman code too long"),
            HuffmanError::OutOfBand => write!(f, "Out-of-band value in Huffman stream"),
        }
    }
}

impl std::error::Error for HuffmanError {}

/// Result type for Huffman operations
pub type HuffmanResult<T> = Result<T, HuffmanError>;

/// A single entry in a Huffman table
///
/// Each entry specifies how to decode a range of values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HuffmanEntry {
    /// Prefix code length in bits
    pub prefix_len: u8,
    /// Range check bits (additional bits to read)
    pub range_len: u8,
    /// Lower bound of the range
    pub range_low: i32,
    /// Whether this is an out-of-band marker
    pub is_oob: bool,
}

impl HuffmanEntry {
    /// Create a new Huffman entry
    pub const fn new(prefix_len: u8, range_len: u8, range_low: i32) -> Self {
        Self {
            prefix_len,
            range_len,
            range_low,
            is_oob: false,
        }
    }

    /// Create an out-of-band entry
    pub const fn oob(prefix_len: u8) -> Self {
        Self {
            prefix_len,
            range_len: 0,
            range_low: 0,
            is_oob: true,
        }
    }
}

/// Standard Huffman table identifiers per ITU-T T.88 Annex B
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StandardTable {
    /// Table B.1 - Generic integers
    B1 = 1,
    /// Table B.2 - Generic integers
    B2 = 2,
    /// Table B.3 - Generic integers
    B3 = 3,
    /// Table B.4 - Generic integers
    B4 = 4,
    /// Table B.5 - Generic integers
    B5 = 5,
    /// Table B.6 - Height class delta
    B6 = 6,
    /// Table B.7 - Height class delta
    B7 = 7,
    /// Table B.8 - Width delta
    B8 = 8,
    /// Table B.9 - Width delta
    B9 = 9,
    /// Table B.10 - Symbol size difference
    B10 = 10,
    /// Table B.11 - Symbol position difference
    B11 = 11,
    /// Table B.12 - Symbol RD (reference delta)
    B12 = 12,
    /// Table B.13 - Symbol RD (reference delta)
    B13 = 13,
    /// Table B.14 - Generic region aggregate
    B14 = 14,
    /// Table B.15 - Generic region aggregate
    B15 = 15,
}

impl StandardTable {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(StandardTable::B1),
            2 => Some(StandardTable::B2),
            3 => Some(StandardTable::B3),
            4 => Some(StandardTable::B4),
            5 => Some(StandardTable::B5),
            6 => Some(StandardTable::B6),
            7 => Some(StandardTable::B7),
            8 => Some(StandardTable::B8),
            9 => Some(StandardTable::B9),
            10 => Some(StandardTable::B10),
            11 => Some(StandardTable::B11),
            12 => Some(StandardTable::B12),
            13 => Some(StandardTable::B13),
            14 => Some(StandardTable::B14),
            15 => Some(StandardTable::B15),
            _ => None,
        }
    }
}

/// A compiled Huffman table for efficient decoding
///
/// Contains entries with their computed prefix codes for direct matching.
#[derive(Debug, Clone)]
pub struct CompiledHuffmanTable {
    /// Entries sorted by code length, with computed codes
    entries: Vec<(u32, u8, HuffmanEntry)>, // (code, code_len, entry)
}

impl CompiledHuffmanTable {
    /// Build a compiled table from entries
    ///
    /// Uses canonical Huffman code construction where codes are assigned
    /// in ascending order within each length group.
    pub fn new(entries: &[HuffmanEntry]) -> Self {
        // Sort entries by prefix length (stable sort to preserve order within groups)
        let mut sorted: Vec<_> = entries.iter().cloned().collect();
        sorted.sort_by_key(|e| e.prefix_len);

        let mut result = Vec::with_capacity(sorted.len());
        let mut code: u32 = 0;
        let mut last_len: u8 = 0;

        for entry in sorted {
            if entry.prefix_len > last_len {
                code <<= entry.prefix_len - last_len;
                last_len = entry.prefix_len;
            }
            result.push((code, entry.prefix_len, entry));
            code += 1;
        }

        Self { entries: result }
    }

    /// Get the entries for iteration
    pub fn entries(&self) -> &[(u32, u8, HuffmanEntry)] {
        &self.entries
    }
}

/// Huffman decoder for JBIG2
///
/// Decodes integer values from a bitstream using predefined
/// or user-defined Huffman tables.
#[derive(Debug)]
pub struct HuffmanDecoder {
    /// Maximum code length to prevent infinite loops
    max_code_len: u8,
}

impl Default for HuffmanDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl HuffmanDecoder {
    /// Create a new Huffman decoder
    pub fn new() -> Self {
        Self { max_code_len: 32 }
    }

    /// Decode a single integer value using a standard table
    ///
    /// # Arguments
    /// * `reader` - The bitstream to read from
    /// * `table` - The standard table to use
    ///
    /// # Returns
    /// The decoded integer value, or `OutOfBand` if OOB marker found
    pub fn decode_int(
        &self,
        reader: &mut BitstreamReader<'_>,
        table: StandardTable,
    ) -> HuffmanResult<i32> {
        let entries = self.get_standard_table(table);
        self.decode_with_entries(reader, &entries)
    }

    /// Decode using a custom table (list of entries)
    ///
    /// The entries are compiled into a proper Huffman table with
    /// canonical prefix codes.
    pub fn decode_with_entries(
        &self,
        reader: &mut BitstreamReader<'_>,
        entries: &[HuffmanEntry],
    ) -> HuffmanResult<i32> {
        let compiled = CompiledHuffmanTable::new(entries);
        self.decode_with_compiled_table(reader, &compiled)
    }

    /// Decode using a pre-compiled table (more efficient for repeated use)
    pub fn decode_with_compiled_table(
        &self,
        reader: &mut BitstreamReader<'_>,
        table: &CompiledHuffmanTable,
    ) -> HuffmanResult<i32> {
        let mut code: u32 = 0;
        let mut code_len: u8 = 0;

        // Build code bit by bit until we find a match
        loop {
            if code_len >= self.max_code_len {
                return Err(HuffmanError::CodeTooLong);
            }

            let bit = reader.read_bit().map_err(|_| HuffmanError::InvalidCode)?;
            code = (code << 1) | (bit as u32);
            code_len += 1;

            // Try to match against entries with this prefix length and code
            for &(entry_code, entry_len, ref entry) in table.entries() {
                if entry_len == code_len && entry_code == code {
                    // Found matching entry
                    if entry.is_oob {
                        return Err(HuffmanError::OutOfBand);
                    }

                    // Read additional range bits
                    if entry.range_len > 0 {
                        let extra = reader
                            .read_bits(entry.range_len)
                            .map_err(|_| HuffmanError::InvalidCode)?;
                        return Ok(entry.range_low + extra as i32);
                    } else {
                        return Ok(entry.range_low);
                    }
                }
            }
        }
    }

    /// Get the entries for a standard table
    ///
    /// Returns the Huffman entries for the specified standard table
    /// per ITU-T T.88 Annex B.
    pub fn get_standard_table(&self, table: StandardTable) -> Vec<HuffmanEntry> {
        match table {
            StandardTable::B1 => self.table_b1(),
            StandardTable::B2 => self.table_b2(),
            StandardTable::B3 => self.table_b3(),
            StandardTable::B4 => self.table_b4(),
            StandardTable::B5 => self.table_b5(),
            StandardTable::B6 => self.table_b6(),
            StandardTable::B7 => self.table_b7(),
            StandardTable::B8 => self.table_b8(),
            StandardTable::B9 => self.table_b9(),
            StandardTable::B10 => self.table_b10(),
            StandardTable::B11 => self.table_b11(),
            StandardTable::B12 => self.table_b12(),
            StandardTable::B13 => self.table_b13(),
            StandardTable::B14 => self.table_b14(),
            StandardTable::B15 => self.table_b15(),
        }
    }

    // ========================================================================
    // Standard Huffman Tables per ITU-T T.88 Annex B
    // ========================================================================

    /// Table B.1 - Used for HTOOB = 0
    fn table_b1(&self) -> Vec<HuffmanEntry> {
        vec![
            HuffmanEntry::new(1, 4, 0),      // 0: 0-15
            HuffmanEntry::new(2, 8, 16),     // 10: 16-271
            HuffmanEntry::new(3, 16, 272),   // 110: 272-65807
            HuffmanEntry::new(3, 32, 65808), // 111: 65808+
        ]
    }

    /// Table B.2 - Used for HTOOB = 0, different ranges
    fn table_b2(&self) -> Vec<HuffmanEntry> {
        vec![
            HuffmanEntry::new(1, 0, 0),   // 0: 0
            HuffmanEntry::new(2, 0, 1),   // 10: 1
            HuffmanEntry::new(3, 0, 2),   // 110: 2
            HuffmanEntry::new(4, 3, 3),   // 1110: 3-10
            HuffmanEntry::new(5, 6, 11),  // 11110: 11-74
            HuffmanEntry::new(6, 32, 75), // 111110: 75+
            HuffmanEntry::oob(6),         // 111111: OOB
        ]
    }

    /// Table B.3 - Used for signed integers
    fn table_b3(&self) -> Vec<HuffmanEntry> {
        vec![
            HuffmanEntry::new(8, 8, -256),  // Low values
            HuffmanEntry::new(1, 0, 0),     // 0: 0
            HuffmanEntry::new(2, 0, 1),     // 10: 1
            HuffmanEntry::new(3, 0, 2),     // 110: 2
            HuffmanEntry::new(4, 3, 3),     // 1110: 3-10
            HuffmanEntry::new(5, 6, 11),    // 11110: 11-74
            HuffmanEntry::new(8, 32, -257), // Negative extension
        ]
    }

    /// Table B.4 - Height class delta for symbol dictionaries
    fn table_b4(&self) -> Vec<HuffmanEntry> {
        vec![
            HuffmanEntry::new(1, 0, 1),   // 0: 1
            HuffmanEntry::new(2, 0, 2),   // 10: 2
            HuffmanEntry::new(3, 0, 3),   // 110: 3
            HuffmanEntry::new(4, 3, 4),   // 1110: 4-11
            HuffmanEntry::new(5, 6, 12),  // 11110: 12-75
            HuffmanEntry::new(5, 32, 76), // 11111: 76+
        ]
    }

    /// Table B.5 - Width delta for symbol dictionaries
    fn table_b5(&self) -> Vec<HuffmanEntry> {
        vec![
            HuffmanEntry::new(7, 8, -255), // Negative values
            HuffmanEntry::new(1, 0, 1),    // 0: 1
            HuffmanEntry::new(2, 0, 2),    // 10: 2
            HuffmanEntry::new(3, 0, 3),    // 110: 3
            HuffmanEntry::new(4, 3, 4),    // 1110: 4-11
            HuffmanEntry::new(5, 6, 12),   // 11110: 12-75
            HuffmanEntry::new(7, 32, 76),  // 76+
        ]
    }

    /// Table B.6 - BMSIZE (bitmap size)
    fn table_b6(&self) -> Vec<HuffmanEntry> {
        vec![
            HuffmanEntry::new(5, 10, -2048), // Negative
            HuffmanEntry::new(4, 9, -1024),
            HuffmanEntry::new(4, 8, -512),
            HuffmanEntry::new(4, 7, -256),
            HuffmanEntry::new(5, 6, -128),
            HuffmanEntry::new(5, 5, -64),
            HuffmanEntry::new(4, 5, -32),
            HuffmanEntry::new(2, 7, 0),   // 0-127
            HuffmanEntry::new(3, 7, 128), // 128-255
            HuffmanEntry::new(3, 8, 256), // 256-511
            HuffmanEntry::new(4, 9, 512),
            HuffmanEntry::new(4, 10, 1024),
            HuffmanEntry::new(6, 32, 2048),  // Large positive
            HuffmanEntry::new(6, 32, -2049), // Large negative
        ]
    }

    /// Table B.7 - AGGINSTCOUNT (aggregate instance count)
    fn table_b7(&self) -> Vec<HuffmanEntry> {
        vec![
            HuffmanEntry::new(4, 9, -1024),
            HuffmanEntry::new(3, 8, -512),
            HuffmanEntry::new(4, 7, -256),
            HuffmanEntry::new(5, 6, -128),
            HuffmanEntry::new(5, 5, -64),
            HuffmanEntry::new(4, 5, -32),
            HuffmanEntry::new(4, 5, 0),
            HuffmanEntry::new(5, 5, 32),
            HuffmanEntry::new(5, 6, 64),
            HuffmanEntry::new(4, 7, 128),
            HuffmanEntry::new(3, 8, 256),
            HuffmanEntry::new(3, 9, 512),
            HuffmanEntry::new(3, 10, 1024),
            HuffmanEntry::new(5, 32, 2048),
            HuffmanEntry::new(5, 32, -2048),
        ]
    }

    /// Table B.8 - EXRUNLENGTH (export run length)
    fn table_b8(&self) -> Vec<HuffmanEntry> {
        vec![
            HuffmanEntry::new(8, 3, -15),
            HuffmanEntry::new(9, 1, -7),
            HuffmanEntry::new(8, 1, -5),
            HuffmanEntry::new(9, 0, -3),
            HuffmanEntry::new(7, 0, -2),
            HuffmanEntry::new(4, 0, -1),
            HuffmanEntry::new(2, 1, 0),
            HuffmanEntry::new(5, 0, 2),
            HuffmanEntry::new(6, 0, 3),
            HuffmanEntry::new(3, 4, 4),
            HuffmanEntry::new(6, 1, 20),
            HuffmanEntry::new(4, 4, 22),
            HuffmanEntry::new(4, 5, 38),
            HuffmanEntry::new(5, 6, 70),
            HuffmanEntry::new(5, 7, 134),
            HuffmanEntry::new(6, 7, 262),
            HuffmanEntry::new(7, 8, 390),
            HuffmanEntry::new(6, 10, 646),
            HuffmanEntry::new(9, 32, 1670),
            HuffmanEntry::new(9, 32, -16),
            HuffmanEntry::oob(2),
        ]
    }

    /// Table B.9 - SYMBINSTCOUNT
    fn table_b9(&self) -> Vec<HuffmanEntry> {
        vec![
            HuffmanEntry::new(8, 4, -31),
            HuffmanEntry::new(9, 2, -15),
            HuffmanEntry::new(8, 2, -11),
            HuffmanEntry::new(9, 1, -7),
            HuffmanEntry::new(7, 1, -5),
            HuffmanEntry::new(4, 1, -3),
            HuffmanEntry::new(3, 1, -1),
            HuffmanEntry::new(3, 1, 1),
            HuffmanEntry::new(5, 1, 3),
            HuffmanEntry::new(6, 1, 5),
            HuffmanEntry::new(3, 5, 7),
            HuffmanEntry::new(6, 2, 39),
            HuffmanEntry::new(4, 5, 43),
            HuffmanEntry::new(4, 6, 75),
            HuffmanEntry::new(5, 7, 139),
            HuffmanEntry::new(5, 8, 267),
            HuffmanEntry::new(6, 8, 523),
            HuffmanEntry::new(7, 9, 779),
            HuffmanEntry::new(6, 11, 1291),
            HuffmanEntry::new(9, 32, 3339),
            HuffmanEntry::new(9, 32, -32),
            HuffmanEntry::oob(2),
        ]
    }

    /// Table B.10 - RDWIDTH (reference delta width)
    fn table_b10(&self) -> Vec<HuffmanEntry> {
        vec![
            HuffmanEntry::new(7, 4, -21),
            HuffmanEntry::new(8, 0, -5),
            HuffmanEntry::new(7, 0, -4),
            HuffmanEntry::new(5, 0, -3),
            HuffmanEntry::new(2, 2, -2),
            HuffmanEntry::new(5, 0, 2),
            HuffmanEntry::new(6, 0, 3),
            HuffmanEntry::new(7, 0, 4),
            HuffmanEntry::new(8, 0, 5),
            HuffmanEntry::new(2, 6, 6),
            HuffmanEntry::new(5, 5, 70),
            HuffmanEntry::new(6, 5, 102),
            HuffmanEntry::new(6, 6, 134),
            HuffmanEntry::new(6, 7, 198),
            HuffmanEntry::new(6, 8, 326),
            HuffmanEntry::new(6, 9, 582),
            HuffmanEntry::new(6, 10, 1094),
            HuffmanEntry::new(7, 11, 2118),
            HuffmanEntry::new(8, 32, 4166),
            HuffmanEntry::new(8, 32, -22),
            HuffmanEntry::oob(2),
        ]
    }

    /// Table B.11 - RDHEIGHT (reference delta height)
    fn table_b11(&self) -> Vec<HuffmanEntry> {
        vec![
            HuffmanEntry::new(1, 0, 0),
            HuffmanEntry::new(2, 1, 1),
            HuffmanEntry::new(4, 0, 3),
            HuffmanEntry::new(4, 1, 4),
            HuffmanEntry::new(5, 1, 6),
            HuffmanEntry::new(5, 2, 8),
            HuffmanEntry::new(6, 2, 12),
            HuffmanEntry::new(7, 2, 16),
            HuffmanEntry::new(7, 3, 20),
            HuffmanEntry::new(7, 4, 28),
            HuffmanEntry::new(7, 5, 44),
            HuffmanEntry::new(7, 6, 76),
            HuffmanEntry::new(7, 32, 140),
        ]
    }

    /// Table B.12 - RDXY (reference delta X/Y)
    fn table_b12(&self) -> Vec<HuffmanEntry> {
        vec![
            HuffmanEntry::new(1, 0, 0),
            HuffmanEntry::new(2, 0, 1),
            HuffmanEntry::new(3, 1, 2),
            HuffmanEntry::new(5, 0, 4),
            HuffmanEntry::new(5, 1, 5),
            HuffmanEntry::new(6, 1, 7),
            HuffmanEntry::new(7, 0, 9),
            HuffmanEntry::new(7, 1, 10),
            HuffmanEntry::new(7, 2, 12),
            HuffmanEntry::new(7, 3, 16),
            HuffmanEntry::new(7, 4, 24),
            HuffmanEntry::new(8, 5, 40),
            HuffmanEntry::new(8, 32, 72),
        ]
    }

    /// Table B.13 - BMSIZE alternative
    fn table_b13(&self) -> Vec<HuffmanEntry> {
        vec![
            HuffmanEntry::new(1, 0, 0),
            HuffmanEntry::new(3, 0, 1),
            HuffmanEntry::new(4, 0, 2),
            HuffmanEntry::new(5, 0, 3),
            HuffmanEntry::new(4, 1, 4),
            HuffmanEntry::new(3, 3, 6),
            HuffmanEntry::new(6, 1, 14),
            HuffmanEntry::new(6, 2, 16),
            HuffmanEntry::new(6, 3, 20),
            HuffmanEntry::new(6, 4, 28),
            HuffmanEntry::new(6, 5, 44),
            HuffmanEntry::new(7, 6, 76),
            HuffmanEntry::new(7, 32, 140),
        ]
    }

    /// Table B.14 - Symbol refinement X delta
    fn table_b14(&self) -> Vec<HuffmanEntry> {
        vec![
            HuffmanEntry::new(3, 0, -2),
            HuffmanEntry::new(3, 0, -1),
            HuffmanEntry::new(1, 0, 0),
            HuffmanEntry::new(3, 0, 1),
            HuffmanEntry::new(3, 0, 2),
        ]
    }

    /// Table B.15 - Symbol refinement Y delta
    fn table_b15(&self) -> Vec<HuffmanEntry> {
        vec![
            HuffmanEntry::new(3, 0, -2),
            HuffmanEntry::new(3, 0, -1),
            HuffmanEntry::new(1, 0, 0),
            HuffmanEntry::new(3, 0, 1),
            HuffmanEntry::new(3, 0, 2),
        ]
    }
}

/// Build prefix codes for a table
///
/// Given a list of entries with prefix lengths, compute the actual
/// binary prefix codes for decoding.
pub fn build_prefix_codes(entries: &[HuffmanEntry]) -> Vec<(u32, HuffmanEntry)> {
    let mut result = Vec::new();

    // Sort entries by prefix length
    let mut sorted: Vec<_> = entries.iter().cloned().enumerate().collect();
    sorted.sort_by_key(|(_, e)| e.prefix_len);

    let mut code: u32 = 0;
    let mut last_len: u8 = 0;

    for (_, entry) in sorted {
        if entry.prefix_len > last_len {
            code <<= entry.prefix_len - last_len;
        }
        result.push((code, entry));
        code += 1;
        last_len = entry.prefix_len;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // HuffmanError Tests
    // ========================================================================

    #[test]
    fn test_huffman_error_display() {
        assert_eq!(
            format!("{}", HuffmanError::InvalidCode),
            "Invalid Huffman code"
        );
        assert_eq!(
            format!("{}", HuffmanError::InvalidTableIndex(99)),
            "Invalid Huffman table index: 99"
        );
        assert_eq!(
            format!("{}", HuffmanError::CodeTooLong),
            "Huffman code too long"
        );
        assert_eq!(
            format!("{}", HuffmanError::OutOfBand),
            "Out-of-band value in Huffman stream"
        );
    }

    #[test]
    fn test_huffman_error_clone() {
        let err1 = HuffmanError::InvalidCode;
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_huffman_error_debug() {
        let err = HuffmanError::InvalidTableIndex(5);
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidTableIndex"));
        assert!(debug_str.contains("5"));
    }

    #[test]
    fn test_huffman_error_std_error_trait() {
        let err = HuffmanError::InvalidCode;
        let std_err: &dyn std::error::Error = &err;
        assert!(std_err.to_string().contains("Huffman"));
    }

    // ========================================================================
    // HuffmanEntry Tests
    // ========================================================================

    #[test]
    fn test_huffman_entry_new() {
        let entry = HuffmanEntry::new(3, 5, 10);
        assert_eq!(entry.prefix_len, 3);
        assert_eq!(entry.range_len, 5);
        assert_eq!(entry.range_low, 10);
        assert!(!entry.is_oob);
    }

    #[test]
    fn test_huffman_entry_oob() {
        let entry = HuffmanEntry::oob(4);
        assert_eq!(entry.prefix_len, 4);
        assert_eq!(entry.range_len, 0);
        assert_eq!(entry.range_low, 0);
        assert!(entry.is_oob);
    }

    #[test]
    fn test_huffman_entry_clone() {
        let entry1 = HuffmanEntry::new(2, 3, -5);
        let entry2 = entry1.clone();
        assert_eq!(entry1, entry2);
    }

    #[test]
    fn test_huffman_entry_debug() {
        let entry = HuffmanEntry::new(4, 8, 256);
        let debug_str = format!("{:?}", entry);
        assert!(debug_str.contains("HuffmanEntry"));
        assert!(debug_str.contains("prefix_len: 4"));
    }

    // ========================================================================
    // StandardTable Tests
    // ========================================================================

    #[test]
    fn test_standard_table_from_u8_valid() {
        assert_eq!(StandardTable::from_u8(1), Some(StandardTable::B1));
        assert_eq!(StandardTable::from_u8(8), Some(StandardTable::B8));
        assert_eq!(StandardTable::from_u8(15), Some(StandardTable::B15));
    }

    #[test]
    fn test_standard_table_from_u8_invalid() {
        assert_eq!(StandardTable::from_u8(0), None);
        assert_eq!(StandardTable::from_u8(16), None);
        assert_eq!(StandardTable::from_u8(255), None);
    }

    #[test]
    fn test_standard_table_clone() {
        let table1 = StandardTable::B5;
        let table2 = table1.clone();
        assert_eq!(table1, table2);
    }

    #[test]
    fn test_standard_table_debug() {
        let table = StandardTable::B10;
        let debug_str = format!("{:?}", table);
        assert!(debug_str.contains("B10"));
    }

    #[test]
    fn test_standard_table_repr() {
        assert_eq!(StandardTable::B1 as u8, 1);
        assert_eq!(StandardTable::B15 as u8, 15);
    }

    // ========================================================================
    // HuffmanDecoder Tests
    // ========================================================================

    #[test]
    fn test_huffman_decoder_new() {
        let decoder = HuffmanDecoder::new();
        assert_eq!(decoder.max_code_len, 32);
    }

    #[test]
    fn test_huffman_decoder_default() {
        let decoder = HuffmanDecoder::default();
        assert_eq!(decoder.max_code_len, 32);
    }

    #[test]
    fn test_huffman_decoder_debug() {
        let decoder = HuffmanDecoder::new();
        let debug_str = format!("{:?}", decoder);
        assert!(debug_str.contains("HuffmanDecoder"));
    }

    #[test]
    fn test_huffman_decoder_get_all_tables() {
        let decoder = HuffmanDecoder::new();

        // Verify all standard tables are accessible
        for i in 1..=15 {
            let table = StandardTable::from_u8(i).unwrap();
            let entries = decoder.get_standard_table(table);
            assert!(!entries.is_empty(), "Table B.{} should have entries", i);
        }
    }

    #[test]
    fn test_table_b1_structure() {
        let decoder = HuffmanDecoder::new();
        let entries = decoder.get_standard_table(StandardTable::B1);

        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].prefix_len, 1);
        assert_eq!(entries[0].range_len, 4);
        assert_eq!(entries[0].range_low, 0);
    }

    #[test]
    fn test_table_b2_has_oob() {
        let decoder = HuffmanDecoder::new();
        let entries = decoder.get_standard_table(StandardTable::B2);

        let has_oob = entries.iter().any(|e| e.is_oob);
        assert!(has_oob, "Table B.2 should have OOB entry");
    }

    #[test]
    fn test_table_b14_b15_symmetry() {
        let decoder = HuffmanDecoder::new();
        let b14 = decoder.get_standard_table(StandardTable::B14);
        let b15 = decoder.get_standard_table(StandardTable::B15);

        // B.14 and B.15 have the same structure
        assert_eq!(b14.len(), b15.len());
        for (e14, e15) in b14.iter().zip(b15.iter()) {
            assert_eq!(e14.prefix_len, e15.prefix_len);
            assert_eq!(e14.range_len, e15.range_len);
            assert_eq!(e14.range_low, e15.range_low);
        }
    }

    // ========================================================================
    // Decoding Tests
    // ========================================================================

    #[test]
    fn test_decode_simple_value() {
        // Create a simple table for testing
        let entries = vec![
            HuffmanEntry::new(1, 0, 0), // 0 -> value 0
            HuffmanEntry::new(2, 0, 1), // 10 -> value 1
            HuffmanEntry::new(2, 0, 2), // 11 -> value 2
        ];

        let decoder = HuffmanDecoder::new();

        // Test decoding "0" (single 0 bit = value 0)
        let data = [0x00]; // 00000000
        let mut reader = BitstreamReader::new(&data);
        let result = decoder.decode_with_entries(&mut reader, &entries);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_decode_with_range_bits() {
        // Table: 0 -> value 0-7 (3 extra bits)
        let entries = vec![
            HuffmanEntry::new(1, 3, 0), // 0 + 3 bits -> 0-7
        ];

        let decoder = HuffmanDecoder::new();

        // 0 followed by 101 = 5
        let data = [0x50]; // 01010000
        let mut reader = BitstreamReader::new(&data);
        let result = decoder.decode_with_entries(&mut reader, &entries);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);
    }

    #[test]
    fn test_decode_oob() {
        // Canonical codes: length 1 entry gets code 0, length 2 OOB gets code 10
        let entries = vec![
            HuffmanEntry::new(1, 0, 0), // code 0 -> value 0
            HuffmanEntry::oob(2),       // code 10 -> OOB
        ];

        let decoder = HuffmanDecoder::new();

        // Code 10 (binary) = OOB
        // 0x80 = 10000000, first two bits are "10"
        let data = [0x80]; // 10000000
        let mut reader = BitstreamReader::new(&data);
        let result = decoder.decode_with_entries(&mut reader, &entries);

        assert_eq!(result.unwrap_err(), HuffmanError::OutOfBand);
    }

    #[test]
    fn test_decode_code_too_long() {
        // Empty table - no codes will match
        let entries = vec![];

        let mut decoder = HuffmanDecoder::new();
        decoder.max_code_len = 8; // Limit for faster test

        let data = [0xFF, 0xFF];
        let mut reader = BitstreamReader::new(&data);
        let result = decoder.decode_with_entries(&mut reader, &entries);

        assert_eq!(result.unwrap_err(), HuffmanError::CodeTooLong);
    }

    // ========================================================================
    // build_prefix_codes Tests
    // ========================================================================

    #[test]
    fn test_build_prefix_codes_simple() {
        let entries = vec![
            HuffmanEntry::new(1, 0, 0), // 0
            HuffmanEntry::new(2, 0, 1), // 10
            HuffmanEntry::new(2, 0, 2), // 11
        ];

        let codes = build_prefix_codes(&entries);

        assert_eq!(codes.len(), 3);
        // The first entry (length 1) should get code 0
        // The two length-2 entries should get codes 2 and 3 (10, 11)
    }

    #[test]
    fn test_build_prefix_codes_empty() {
        let entries: Vec<HuffmanEntry> = vec![];
        let codes = build_prefix_codes(&entries);
        assert!(codes.is_empty());
    }

    #[test]
    fn test_build_prefix_codes_single() {
        let entries = vec![HuffmanEntry::new(3, 0, 42)];

        let codes = build_prefix_codes(&entries);
        assert_eq!(codes.len(), 1);
        assert_eq!(codes[0].0, 0); // First code is always 0
        assert_eq!(codes[0].1.range_low, 42);
    }

    // ========================================================================
    // Integration Tests
    // ========================================================================

    #[test]
    fn test_decode_multiple_values_sequential() {
        // Simple table for predictable decoding with canonical codes:
        // Length 1: code 0 (binary "0") -> value 0
        // Length 2: code 2 (binary "10") -> value 1
        // Length 3: code 6 (binary "110") -> value 2
        // Length 3: code 7 (binary "111") -> value 3
        //
        // Wait, canonical code assignment works differently:
        // Sort by length, then assign sequentially:
        // - Length 1 entry: starts at 0, gets code 0
        // - Length 2 entry: 0 << 1 = 0, but we increment after length-1 used it
        //   Actually: code = (0 + 1) << (2-1) = 2 (binary "10")
        // - Length 3 entries: code = (2 + 1) << (3-2) = 6 (binary "110"), then 7
        //
        // So codes are: 0 -> val 0, 10 -> val 1, 110 -> val 2, 111 -> val 3
        let entries = vec![
            HuffmanEntry::new(1, 0, 0), // code 0 (binary "0") -> value 0
            HuffmanEntry::new(2, 0, 1), // code 2 (binary "10") -> value 1
            HuffmanEntry::new(3, 0, 2), // code 6 (binary "110") -> value 2
            HuffmanEntry::new(3, 0, 3), // code 7 (binary "111") -> value 3
        ];

        let decoder = HuffmanDecoder::new();

        // Let's verify the compiled codes first
        let compiled = CompiledHuffmanTable::new(&entries);
        // Should be: (0, 1, val0), (2, 2, val1), (6, 3, val2), (7, 3, val3)
        assert_eq!(compiled.entries()[0].0, 0); // code 0
        assert_eq!(compiled.entries()[1].0, 2); // code 10
        assert_eq!(compiled.entries()[2].0, 6); // code 110
        assert_eq!(compiled.entries()[3].0, 7); // code 111

        // Encode: 0 | 10 | 110 | 111 = 0_10_110_111 = 0101_1011_1 = 0x5B 0x80
        // Bit pattern: 0 10 110 111 = 01011011 1xxxxxxx
        let data = [0x5B, 0x80]; // 01011011 10000000
        let mut reader = BitstreamReader::new(&data);

        // Read first value: "0" -> value 0
        let v1 = decoder.decode_with_entries(&mut reader, &entries);
        assert_eq!(v1.unwrap(), 0);

        // Read second value: "10" -> value 1
        let v2 = decoder.decode_with_entries(&mut reader, &entries);
        assert_eq!(v2.unwrap(), 1);

        // Read third value: "110" -> value 2
        let v3 = decoder.decode_with_entries(&mut reader, &entries);
        assert_eq!(v3.unwrap(), 2);

        // Read fourth value: "111" -> value 3
        let v4 = decoder.decode_with_entries(&mut reader, &entries);
        assert_eq!(v4.unwrap(), 3);
    }

    #[test]
    fn test_all_standard_tables_valid() {
        let decoder = HuffmanDecoder::new();

        for i in 1..=15 {
            let table = StandardTable::from_u8(i).unwrap();
            let entries = decoder.get_standard_table(table);

            // Verify table has entries
            assert!(!entries.is_empty(), "Table B.{} is empty", i);

            // Verify all entries have valid prefix lengths
            for entry in &entries {
                assert!(entry.prefix_len > 0, "Table B.{} has zero prefix length", i);
                assert!(entry.prefix_len <= 32, "Table B.{} has prefix > 32", i);
            }
        }
    }

    #[test]
    fn test_negative_range_values() {
        // Several tables support negative values
        let decoder = HuffmanDecoder::new();

        let b3 = decoder.get_standard_table(StandardTable::B3);
        let has_negative = b3.iter().any(|e| e.range_low < 0);
        assert!(has_negative, "Table B.3 should support negative values");

        let b6 = decoder.get_standard_table(StandardTable::B6);
        let has_negative = b6.iter().any(|e| e.range_low < 0);
        assert!(has_negative, "Table B.6 should support negative values");
    }
}
