//! CFF primitive types: DICT token scanner and binary reader utilities.
//!
//! These items are extracted from `cff_subsetter` so they can be shared
//! across multiple CFF-related modules without circular dependencies.

use crate::parser::{ParseError, ParseResult};

// =============================================================================
// CFF DICT token scanner
// =============================================================================

/// A token produced by scanning a CFF DICT byte sequence.
///
/// CFF DICTs consist of interleaved operands and operators per CFF spec §4.
/// The scanner yields one token per call to `next()`.
#[derive(Debug, PartialEq, Clone)]
pub enum CffDictToken {
    /// Integer operand value decoded from the CFF encoding.
    /// Real numbers (byte 30) emit `Operand(0)` as a placeholder since they
    /// are not used for any offset calculation.
    Operand(i32),
    /// Single-byte operator (bytes 0..=27 except 12, 28, 29, 30).
    Operator(u8),
    /// Two-byte escaped operator: first byte was 12, second byte is stored here.
    EscapedOperator(u8),
}

/// Iterator over CFF DICT tokens.
///
/// Implements the full CFF integer/real operand encoding per CFF spec §4:
///
/// | Byte range | Encoding                                      |
/// |-----------|-----------------------------------------------|
/// | 32–246    | 1-byte integer: `value = b − 139`            |
/// | 247–250   | 2-byte positive: `(b0−247)×256 + b1 + 108`  |
/// | 251–254   | 2-byte negative: `−(b0−251)×256 − b1 − 108` |
/// | 28        | 2-byte signed big-endian i16                  |
/// | 29        | 4-byte signed big-endian i32                  |
/// | 30        | Real number (nibble pairs until 0xF): `Operand(0)` |
/// | 12        | Escaped operator; reads one more byte         |
/// | 0–27 (excl. 12, 28, 29, 30) | Single-byte operator    |
pub struct CffDictScanner<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> CffDictScanner<'a> {
    /// Create a new scanner over the given CFF DICT data.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Current byte position within the data slice.
    pub fn position(&self) -> usize {
        self.pos
    }
}

impl<'a> Iterator for CffDictScanner<'a> {
    type Item = CffDictToken;

    fn next(&mut self) -> Option<CffDictToken> {
        loop {
            if self.pos >= self.data.len() {
                return None;
            }
            let b = self.data[self.pos];

            match b {
                28 => {
                    // 2-byte signed integer (big-endian i16)
                    if self.pos + 2 >= self.data.len() {
                        return None; // truncated — stop cleanly
                    }
                    let v = i16::from_be_bytes([self.data[self.pos + 1], self.data[self.pos + 2]])
                        as i32;
                    self.pos += 3;
                    return Some(CffDictToken::Operand(v));
                }
                29 => {
                    // 4-byte signed integer (big-endian i32)
                    if self.pos + 4 >= self.data.len() {
                        return None; // truncated — stop cleanly
                    }
                    let v = i32::from_be_bytes([
                        self.data[self.pos + 1],
                        self.data[self.pos + 2],
                        self.data[self.pos + 3],
                        self.data[self.pos + 4],
                    ]);
                    self.pos += 5;
                    return Some(CffDictToken::Operand(v));
                }
                30 => {
                    // Real number — skip nibble pairs until 0xF terminator
                    // Emit Operand(0) as a placeholder (real values not used for offsets).
                    self.pos += 1;
                    while self.pos < self.data.len() {
                        let nibble_byte = self.data[self.pos];
                        self.pos += 1;
                        if nibble_byte & 0x0F == 0x0F || nibble_byte >> 4 == 0x0F {
                            break;
                        }
                    }
                    return Some(CffDictToken::Operand(0));
                }
                32..=246 => {
                    // 1-byte integer: value = b − 139
                    let v = b as i32 - 139;
                    self.pos += 1;
                    return Some(CffDictToken::Operand(v));
                }
                247..=250 => {
                    // 2-byte positive integer
                    if self.pos + 1 >= self.data.len() {
                        return None; // truncated — stop cleanly
                    }
                    let w = self.data[self.pos + 1] as i32;
                    let v = (b as i32 - 247) * 256 + w + 108;
                    self.pos += 2;
                    return Some(CffDictToken::Operand(v));
                }
                251..=254 => {
                    // 2-byte negative integer
                    if self.pos + 1 >= self.data.len() {
                        return None; // truncated — stop cleanly
                    }
                    let w = self.data[self.pos + 1] as i32;
                    let v = -(b as i32 - 251) * 256 - w - 108;
                    self.pos += 2;
                    return Some(CffDictToken::Operand(v));
                }
                12 => {
                    // Escaped operator: read next byte
                    self.pos += 1;
                    if self.pos >= self.data.len() {
                        return None;
                    }
                    let op2 = self.data[self.pos];
                    self.pos += 1;
                    return Some(CffDictToken::EscapedOperator(op2));
                }
                _ => {
                    // Single-byte operator (bytes 0–27 excluding 12, 28, 29, 30)
                    self.pos += 1;
                    return Some(CffDictToken::Operator(b));
                }
            }
        }
    }
}

// =============================================================================
// Binary reader utilities
// =============================================================================

pub(crate) fn read_u16(data: &[u8], offset: usize) -> ParseResult<u16> {
    if offset + 2 > data.len() {
        return Err(ParseError::SyntaxError {
            position: offset,
            message: "read_u16: out of bounds".to_string(),
        });
    }
    Ok(u16::from_be_bytes([data[offset], data[offset + 1]]))
}

pub(crate) fn read_u32(data: &[u8], offset: usize) -> ParseResult<u32> {
    if offset + 4 > data.len() {
        return Err(ParseError::SyntaxError {
            position: offset,
            message: "read_u32: out of bounds".to_string(),
        });
    }
    Ok(u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

pub(crate) fn read_i16(data: &[u8], offset: usize) -> ParseResult<i16> {
    if offset + 2 > data.len() {
        return Err(ParseError::SyntaxError {
            position: offset,
            message: "read_i16: out of bounds".to_string(),
        });
    }
    Ok(i16::from_be_bytes([data[offset], data[offset + 1]]))
}

// =============================================================================
// CFF integer encoding
// =============================================================================

/// Encode an integer as the 5-byte CFF form (byte 29 + big-endian i32).
/// Using the fixed-width encoding simplifies two-pass offset calculation:
/// the byte size of Top DICT is always the same regardless of offset values.
pub(crate) fn encode_cff_int_5byte(value: i32) -> [u8; 5] {
    let bytes = value.to_be_bytes();
    [29, bytes[0], bytes[1], bytes[2], bytes[3]]
}
