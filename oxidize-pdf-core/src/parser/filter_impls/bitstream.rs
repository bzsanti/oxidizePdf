//! Bitstream reader for JBIG2 decoding
//!
//! This module provides a bitstream reader for reading variable-length
//! codes and values from JBIG2 compressed data per ITU-T T.88 Section 6.2.
//!
//! Key operations:
//! - Read individual bits
//! - Read multi-bit integers (MSB-first)
//! - Byte alignment for segment boundaries

use crate::parser::ParseError;

/// Error type for bitstream operations
#[derive(Debug, Clone, PartialEq)]
pub enum BitstreamError {
    /// End of data reached unexpectedly
    EndOfData,
    /// Invalid number of bits requested
    InvalidBitCount(u8),
}

impl std::fmt::Display for BitstreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BitstreamError::EndOfData => write!(f, "End of bitstream data"),
            BitstreamError::InvalidBitCount(n) => {
                write!(f, "Invalid bit count: {} (max 32)", n)
            }
        }
    }
}

impl std::error::Error for BitstreamError {}

impl From<BitstreamError> for ParseError {
    fn from(err: BitstreamError) -> Self {
        ParseError::StreamDecodeError(err.to_string())
    }
}

/// Result type for bitstream operations
pub type BitstreamResult<T> = Result<T, BitstreamError>;

/// Bitstream reader for JBIG2 decoding
///
/// Reads bits from a byte slice in MSB-first order as specified in ITU-T T.88.
/// Maintains current byte position and bit offset within the current byte.
#[derive(Debug)]
pub struct BitstreamReader<'a> {
    /// The underlying data buffer
    data: &'a [u8],
    /// Current byte position in the buffer
    byte_pos: usize,
    /// Current bit position within the current byte (0-7, 0 is MSB)
    bit_pos: u8,
}

impl<'a> BitstreamReader<'a> {
    /// Create a new bitstream reader from a byte slice
    ///
    /// # Arguments
    /// * `data` - The byte slice to read from
    ///
    /// # Returns
    /// A new `BitstreamReader` positioned at the start of the data
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    /// Read a single bit from the stream
    ///
    /// Returns the bit value (0 or 1) and advances the position.
    /// Per ITU-T T.88, bits are read MSB-first.
    pub fn read_bit(&mut self) -> BitstreamResult<u8> {
        if self.byte_pos >= self.data.len() {
            return Err(BitstreamError::EndOfData);
        }

        let byte = self.data[self.byte_pos];
        let bit = (byte >> (7 - self.bit_pos)) & 1;

        self.bit_pos += 1;
        if self.bit_pos >= 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }

        Ok(bit)
    }

    /// Read multiple bits as an unsigned integer
    ///
    /// Reads `n` bits and returns them as a u32, MSB-first.
    ///
    /// # Arguments
    /// * `n` - Number of bits to read (1-32)
    ///
    /// # Errors
    /// * `InvalidBitCount` - if n is 0 or greater than 32
    /// * `EndOfData` - if not enough data available
    pub fn read_bits(&mut self, n: u8) -> BitstreamResult<u32> {
        if n == 0 || n > 32 {
            return Err(BitstreamError::InvalidBitCount(n));
        }

        let mut value: u32 = 0;
        for _ in 0..n {
            value = (value << 1) | (self.read_bit()? as u32);
        }

        Ok(value)
    }

    /// Read a single byte (8 bits)
    ///
    /// Reads 8 bits and returns them as a byte.
    /// This is more efficient than read_bits(8) when byte-aligned.
    pub fn read_byte(&mut self) -> BitstreamResult<u8> {
        if self.bit_pos == 0 {
            // Byte-aligned read is faster
            if self.byte_pos >= self.data.len() {
                return Err(BitstreamError::EndOfData);
            }
            let byte = self.data[self.byte_pos];
            self.byte_pos += 1;
            Ok(byte)
        } else {
            // Not byte-aligned, read bit by bit
            self.read_bits(8).map(|v| v as u8)
        }
    }

    /// Align the reader to the next byte boundary
    ///
    /// If already byte-aligned, does nothing.
    /// Otherwise, advances to the start of the next byte,
    /// discarding any remaining bits in the current byte.
    pub fn align_to_byte(&mut self) {
        if self.bit_pos != 0 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
    }

    /// Check if the reader is at a byte boundary
    pub fn is_byte_aligned(&self) -> bool {
        self.bit_pos == 0
    }

    /// Get the current byte position
    pub fn byte_position(&self) -> usize {
        self.byte_pos
    }

    /// Get the current bit offset within the current byte
    pub fn bit_offset(&self) -> u8 {
        self.bit_pos
    }

    /// Get the total number of bits remaining
    pub fn bits_remaining(&self) -> usize {
        if self.byte_pos >= self.data.len() {
            return 0;
        }
        let full_bytes = self.data.len() - self.byte_pos - 1;
        let remaining_bits = 8 - self.bit_pos as usize;
        full_bytes * 8 + remaining_bits
    }

    /// Check if there are at least `n` bits remaining
    pub fn has_bits(&self, n: usize) -> bool {
        self.bits_remaining() >= n
    }

    /// Check if the stream has been fully consumed
    pub fn is_exhausted(&self) -> bool {
        self.byte_pos >= self.data.len()
    }

    /// Peek at the next bit without consuming it
    ///
    /// Returns None if no more data is available.
    pub fn peek_bit(&self) -> Option<u8> {
        if self.byte_pos >= self.data.len() {
            return None;
        }
        let byte = self.data[self.byte_pos];
        Some((byte >> (7 - self.bit_pos)) & 1)
    }

    /// Skip `n` bits
    ///
    /// Advances the position by `n` bits without reading.
    pub fn skip_bits(&mut self, n: usize) -> BitstreamResult<()> {
        let total_bits = self.bit_pos as usize + n;
        let new_byte_pos = self.byte_pos + total_bits / 8;
        let new_bit_pos = (total_bits % 8) as u8;

        if new_byte_pos > self.data.len() || (new_byte_pos == self.data.len() && new_bit_pos > 0) {
            return Err(BitstreamError::EndOfData);
        }

        self.byte_pos = new_byte_pos;
        self.bit_pos = new_bit_pos;
        Ok(())
    }

    /// Get a reference to the underlying data
    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Read a signed integer in two's complement
    ///
    /// Reads `n` bits and interprets them as a signed integer.
    pub fn read_signed_bits(&mut self, n: u8) -> BitstreamResult<i32> {
        if n == 0 || n > 32 {
            return Err(BitstreamError::InvalidBitCount(n));
        }

        let unsigned = self.read_bits(n)?;

        // Check sign bit
        let sign_bit = 1u32 << (n - 1);
        if unsigned & sign_bit != 0 {
            // Negative: sign-extend
            let mask = !((1u32 << n) - 1);
            Ok((unsigned | mask) as i32)
        } else {
            Ok(unsigned as i32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // BitstreamError Tests
    // ========================================================================

    #[test]
    fn test_bitstream_error_end_of_data_display() {
        let err = BitstreamError::EndOfData;
        assert_eq!(format!("{}", err), "End of bitstream data");
    }

    #[test]
    fn test_bitstream_error_invalid_bit_count_display() {
        let err = BitstreamError::InvalidBitCount(64);
        assert_eq!(format!("{}", err), "Invalid bit count: 64 (max 32)");
    }

    #[test]
    fn test_bitstream_error_clone() {
        let err1 = BitstreamError::EndOfData;
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_bitstream_error_debug() {
        let err = BitstreamError::InvalidBitCount(33);
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidBitCount"));
        assert!(debug_str.contains("33"));
    }

    #[test]
    fn test_bitstream_error_into_parse_error() {
        let err = BitstreamError::EndOfData;
        let parse_err: ParseError = err.into();
        assert!(parse_err.to_string().contains("bitstream"));
    }

    #[test]
    fn test_bitstream_error_std_error_trait() {
        let err = BitstreamError::EndOfData;
        let std_err: &dyn std::error::Error = &err;
        assert!(std_err.to_string().contains("bitstream"));
    }

    // ========================================================================
    // BitstreamReader Creation Tests
    // ========================================================================

    #[test]
    fn test_bitstream_reader_new() {
        let data = [0xAB, 0xCD];
        let reader = BitstreamReader::new(&data);
        assert_eq!(reader.byte_position(), 0);
        assert_eq!(reader.bit_offset(), 0);
        assert!(reader.is_byte_aligned());
    }

    #[test]
    fn test_bitstream_reader_new_empty() {
        let data: [u8; 0] = [];
        let reader = BitstreamReader::new(&data);
        assert!(reader.is_exhausted());
        assert_eq!(reader.bits_remaining(), 0);
    }

    #[test]
    fn test_bitstream_reader_debug() {
        let data = [0x00];
        let reader = BitstreamReader::new(&data);
        let debug_str = format!("{:?}", reader);
        assert!(debug_str.contains("BitstreamReader"));
    }

    // ========================================================================
    // Read Bit Tests
    // ========================================================================

    #[test]
    fn test_read_bit_msb_first() {
        // 0xAB = 0b10101011
        let data = [0xAB];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.read_bit().unwrap(), 1); // bit 7
        assert_eq!(reader.read_bit().unwrap(), 0); // bit 6
        assert_eq!(reader.read_bit().unwrap(), 1); // bit 5
        assert_eq!(reader.read_bit().unwrap(), 0); // bit 4
        assert_eq!(reader.read_bit().unwrap(), 1); // bit 3
        assert_eq!(reader.read_bit().unwrap(), 0); // bit 2
        assert_eq!(reader.read_bit().unwrap(), 1); // bit 1
        assert_eq!(reader.read_bit().unwrap(), 1); // bit 0
    }

    #[test]
    fn test_read_bit_advances_to_next_byte() {
        let data = [0xFF, 0x00];
        let mut reader = BitstreamReader::new(&data);

        // Read all bits from first byte
        for _ in 0..8 {
            assert_eq!(reader.read_bit().unwrap(), 1);
        }

        // Now reading from second byte
        assert_eq!(reader.byte_position(), 1);
        assert_eq!(reader.read_bit().unwrap(), 0);
    }

    #[test]
    fn test_read_bit_end_of_data() {
        let data = [0xFF];
        let mut reader = BitstreamReader::new(&data);

        // Read all 8 bits
        for _ in 0..8 {
            assert!(reader.read_bit().is_ok());
        }

        // Next read should fail
        assert_eq!(reader.read_bit().unwrap_err(), BitstreamError::EndOfData);
    }

    #[test]
    fn test_read_bit_empty_data() {
        let data: [u8; 0] = [];
        let mut reader = BitstreamReader::new(&data);
        assert_eq!(reader.read_bit().unwrap_err(), BitstreamError::EndOfData);
    }

    // ========================================================================
    // Read Bits Tests
    // ========================================================================

    #[test]
    fn test_read_bits_single_byte() {
        // 0xAB = 171
        let data = [0xAB];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.read_bits(8).unwrap(), 0xAB);
    }

    #[test]
    fn test_read_bits_partial() {
        // 0xF0 = 0b11110000
        let data = [0xF0];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.read_bits(4).unwrap(), 0xF); // 1111
        assert_eq!(reader.read_bits(4).unwrap(), 0x0); // 0000
    }

    #[test]
    fn test_read_bits_cross_byte_boundary() {
        // 0xAB = 10101011, 0xCD = 11001101
        // Reading 12 bits from start: 101010111100 = 0xABC
        let data = [0xAB, 0xCD];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.read_bits(12).unwrap(), 0xABC);
    }

    #[test]
    fn test_read_bits_max_32() {
        let data = [0xFF, 0xFF, 0xFF, 0xFF];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.read_bits(32).unwrap(), 0xFFFFFFFF);
    }

    #[test]
    fn test_read_bits_invalid_zero() {
        let data = [0x00];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(
            reader.read_bits(0).unwrap_err(),
            BitstreamError::InvalidBitCount(0)
        );
    }

    #[test]
    fn test_read_bits_invalid_too_large() {
        let data = [0x00];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(
            reader.read_bits(33).unwrap_err(),
            BitstreamError::InvalidBitCount(33)
        );
    }

    #[test]
    fn test_read_bits_end_of_data() {
        let data = [0xAB];
        let mut reader = BitstreamReader::new(&data);

        // Read 8 bits successfully
        assert!(reader.read_bits(8).is_ok());

        // Try to read more
        assert_eq!(reader.read_bits(1).unwrap_err(), BitstreamError::EndOfData);
    }

    // ========================================================================
    // Read Byte Tests
    // ========================================================================

    #[test]
    fn test_read_byte_aligned() {
        let data = [0xAB, 0xCD];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.read_byte().unwrap(), 0xAB);
        assert_eq!(reader.read_byte().unwrap(), 0xCD);
    }

    #[test]
    fn test_read_byte_unaligned() {
        // 0xAB = 10101011, 0xCD = 11001101
        // After reading 4 bits (1010), next 8 bits are 10111100 = 0xBC
        let data = [0xAB, 0xCD];
        let mut reader = BitstreamReader::new(&data);

        reader.read_bits(4).unwrap(); // read 1010
        assert_eq!(reader.read_byte().unwrap(), 0xBC); // 10111100
    }

    #[test]
    fn test_read_byte_end_of_data() {
        let data = [0xAB];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.read_byte().unwrap(), 0xAB);
        assert_eq!(reader.read_byte().unwrap_err(), BitstreamError::EndOfData);
    }

    // ========================================================================
    // Alignment Tests
    // ========================================================================

    #[test]
    fn test_align_to_byte_already_aligned() {
        let data = [0xAB, 0xCD];
        let mut reader = BitstreamReader::new(&data);

        assert!(reader.is_byte_aligned());
        reader.align_to_byte();
        assert!(reader.is_byte_aligned());
        assert_eq!(reader.byte_position(), 0);
    }

    #[test]
    fn test_align_to_byte_not_aligned() {
        let data = [0xAB, 0xCD];
        let mut reader = BitstreamReader::new(&data);

        reader.read_bits(3).unwrap();
        assert!(!reader.is_byte_aligned());

        reader.align_to_byte();
        assert!(reader.is_byte_aligned());
        assert_eq!(reader.byte_position(), 1);
    }

    #[test]
    fn test_is_byte_aligned() {
        let data = [0xAB];
        let mut reader = BitstreamReader::new(&data);

        assert!(reader.is_byte_aligned());
        reader.read_bit().unwrap();
        assert!(!reader.is_byte_aligned());
    }

    // ========================================================================
    // Position and State Tests
    // ========================================================================

    #[test]
    fn test_byte_position() {
        let data = [0xAB, 0xCD, 0xEF];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.byte_position(), 0);
        reader.read_byte().unwrap();
        assert_eq!(reader.byte_position(), 1);
        reader.read_byte().unwrap();
        assert_eq!(reader.byte_position(), 2);
    }

    #[test]
    fn test_bit_offset() {
        let data = [0xAB];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.bit_offset(), 0);
        reader.read_bit().unwrap();
        assert_eq!(reader.bit_offset(), 1);
        reader.read_bits(3).unwrap();
        assert_eq!(reader.bit_offset(), 4);
    }

    #[test]
    fn test_bits_remaining() {
        let data = [0xAB, 0xCD];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.bits_remaining(), 16);
        reader.read_bits(5).unwrap();
        assert_eq!(reader.bits_remaining(), 11);
        reader.read_byte().unwrap();
        assert_eq!(reader.bits_remaining(), 3);
    }

    #[test]
    fn test_bits_remaining_exhausted() {
        let data = [0xAB];
        let mut reader = BitstreamReader::new(&data);

        reader.read_byte().unwrap();
        assert_eq!(reader.bits_remaining(), 0);
    }

    #[test]
    fn test_has_bits() {
        let data = [0xAB];
        let mut reader = BitstreamReader::new(&data);

        assert!(reader.has_bits(8));
        assert!(reader.has_bits(1));
        assert!(!reader.has_bits(9));

        reader.read_bits(5).unwrap();
        assert!(reader.has_bits(3));
        assert!(!reader.has_bits(4));
    }

    #[test]
    fn test_is_exhausted() {
        let data = [0xAB];
        let mut reader = BitstreamReader::new(&data);

        assert!(!reader.is_exhausted());
        reader.read_byte().unwrap();
        assert!(reader.is_exhausted());
    }

    // ========================================================================
    // Peek Tests
    // ========================================================================

    #[test]
    fn test_peek_bit() {
        let data = [0x80]; // 10000000
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.peek_bit(), Some(1));
        assert_eq!(reader.peek_bit(), Some(1)); // Still the same
        reader.read_bit().unwrap();
        assert_eq!(reader.peek_bit(), Some(0));
    }

    #[test]
    fn test_peek_bit_exhausted() {
        let data = [0x00];
        let mut reader = BitstreamReader::new(&data);

        reader.read_byte().unwrap();
        assert_eq!(reader.peek_bit(), None);
    }

    // ========================================================================
    // Skip Tests
    // ========================================================================

    #[test]
    fn test_skip_bits() {
        let data = [0xAB, 0xCD, 0xEF];
        let mut reader = BitstreamReader::new(&data);

        reader.skip_bits(8).unwrap();
        assert_eq!(reader.byte_position(), 1);
        assert_eq!(reader.bit_offset(), 0);
    }

    #[test]
    fn test_skip_bits_partial() {
        let data = [0xAB, 0xCD];
        let mut reader = BitstreamReader::new(&data);

        reader.skip_bits(5).unwrap();
        assert_eq!(reader.byte_position(), 0);
        assert_eq!(reader.bit_offset(), 5);
    }

    #[test]
    fn test_skip_bits_cross_boundary() {
        let data = [0xAB, 0xCD, 0xEF];
        let mut reader = BitstreamReader::new(&data);

        reader.skip_bits(12).unwrap();
        assert_eq!(reader.byte_position(), 1);
        assert_eq!(reader.bit_offset(), 4);
    }

    #[test]
    fn test_skip_bits_end_of_data() {
        let data = [0xAB];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.skip_bits(9).unwrap_err(), BitstreamError::EndOfData);
    }

    #[test]
    fn test_skip_bits_exact_end() {
        let data = [0xAB];
        let mut reader = BitstreamReader::new(&data);

        reader.skip_bits(8).unwrap();
        assert!(reader.is_exhausted());
    }

    // ========================================================================
    // Data Access Tests
    // ========================================================================

    #[test]
    fn test_data_reference() {
        let data = [0xAB, 0xCD];
        let reader = BitstreamReader::new(&data);

        assert_eq!(reader.data(), &[0xAB, 0xCD]);
    }

    // ========================================================================
    // Signed Integer Tests
    // ========================================================================

    #[test]
    fn test_read_signed_bits_positive() {
        // 0b0111 = 7 as 4-bit signed
        let data = [0x70]; // 01110000
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.read_signed_bits(4).unwrap(), 7);
    }

    #[test]
    fn test_read_signed_bits_negative() {
        // 0b1111 = -1 as 4-bit signed (two's complement)
        let data = [0xF0]; // 11110000
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.read_signed_bits(4).unwrap(), -1);
    }

    #[test]
    fn test_read_signed_bits_min_value() {
        // 0b1000 = -8 as 4-bit signed
        let data = [0x80]; // 10000000
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.read_signed_bits(4).unwrap(), -8);
    }

    #[test]
    fn test_read_signed_bits_zero() {
        let data = [0x00];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.read_signed_bits(4).unwrap(), 0);
    }

    #[test]
    fn test_read_signed_bits_8_bit() {
        // -128 as 8-bit signed
        let data = [0x80];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.read_signed_bits(8).unwrap(), -128);
    }

    #[test]
    fn test_read_signed_bits_invalid() {
        let data = [0x00];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(
            reader.read_signed_bits(0).unwrap_err(),
            BitstreamError::InvalidBitCount(0)
        );
        assert_eq!(
            reader.read_signed_bits(33).unwrap_err(),
            BitstreamError::InvalidBitCount(33)
        );
    }

    // ========================================================================
    // Integration Tests
    // ========================================================================

    #[test]
    fn test_mixed_operations() {
        let data = [0xAB, 0xCD, 0xEF];
        let mut reader = BitstreamReader::new(&data);

        // Read 4 bits
        assert_eq!(reader.read_bits(4).unwrap(), 0xA);
        // Peek
        assert_eq!(reader.peek_bit(), Some(1)); // 0xB starts with 1011
                                                // Read 1 bit
        assert_eq!(reader.read_bit().unwrap(), 1);
        // Align
        reader.align_to_byte();
        assert_eq!(reader.byte_position(), 1);
        // Read byte
        assert_eq!(reader.read_byte().unwrap(), 0xCD);
        // Skip
        reader.skip_bits(4).unwrap();
        // Read remaining
        assert_eq!(reader.read_bits(4).unwrap(), 0xF);
    }

    #[test]
    fn test_sequential_byte_reads() {
        let data = [0x01, 0x02, 0x03, 0x04];
        let mut reader = BitstreamReader::new(&data);

        assert_eq!(reader.read_byte().unwrap(), 0x01);
        assert_eq!(reader.read_byte().unwrap(), 0x02);
        assert_eq!(reader.read_byte().unwrap(), 0x03);
        assert_eq!(reader.read_byte().unwrap(), 0x04);
        assert!(reader.is_exhausted());
    }
}
