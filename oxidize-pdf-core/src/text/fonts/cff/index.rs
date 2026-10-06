//! CFF INDEX structure parsing and creation.

use crate::parser::{ParseError, ParseResult};

/// A parsed CFF INDEX: the byte range of each item within the original data slice.
///
/// The struct itself is `pub` so integration tests and downstream consumers
/// can use `parse_cff_index`, `build_cff_index`, and the accessor methods
/// (`count`, `get_item`, `raw_bytes`, `end_offset`), but the fields are
/// `pub(crate)` to prevent external callers from constructing instances with
/// offsets that do not correspond to any parsed data.
pub struct CffIndex {
    pub(crate) start_offset: usize,
    pub(crate) byte_length: usize,
    pub(crate) item_offsets: Vec<usize>,
}

impl CffIndex {
    /// An empty INDEX with no items. Useful as a "no local subrs" placeholder
    /// when calling the desubroutinizer for a font dict that has no Subrs.
    pub fn empty() -> Self {
        CffIndex {
            start_offset: 0,
            byte_length: 0,
            item_offsets: Vec::new(),
        }
    }

    pub fn end_offset(&self) -> usize {
        self.start_offset + self.byte_length
    }

    pub fn count(&self) -> usize {
        if self.item_offsets.is_empty() {
            0
        } else {
            self.item_offsets.len() - 1
        }
    }

    pub fn get_item<'a>(&self, index: usize, cff: &'a [u8]) -> Option<&'a [u8]> {
        if index >= self.count() {
            return None;
        }
        let start = self.item_offsets[index];
        let end = self.item_offsets[index + 1];
        if end > cff.len() || start > end {
            return None;
        }
        Some(&cff[start..end])
    }

    pub fn raw_bytes<'a>(&self, cff: &'a [u8]) -> &'a [u8] {
        let end = self.start_offset + self.byte_length;
        cff.get(self.start_offset..end.min(cff.len()))
            .unwrap_or(&[])
    }
}

pub fn parse_cff_index(cff: &[u8], pos: usize) -> ParseResult<CffIndex> {
    let invalid = || ParseError::SyntaxError {
        position: pos,
        message: "invalid or truncated CFF INDEX".into(),
    };
    let data = cff.get(pos..).ok_or_else(invalid)?;
    let header = data.get(..2).ok_or_else(invalid)?;
    let count = usize::from(u16::from_be_bytes([header[0], header[1]]));
    if count == 0 {
        return Ok(CffIndex {
            start_offset: pos,
            byte_length: 2,
            item_offsets: vec![],
        });
    }
    let off_size = usize::from(*data.get(2).ok_or_else(invalid)?);
    if !(1..=4).contains(&off_size) {
        return Err(invalid());
    }
    let data_base = 3 + (count + 1) * off_size;
    let payload = data.get(data_base..).ok_or_else(invalid)?;
    let mut item_offsets = Vec::with_capacity(count + 1);
    let mut previous = 1usize;
    for i in 0..=count {
        let raw = usize::try_from(read_offset(data, 3 + i * off_size, off_size)?)
            .map_err(|_| invalid())?;
        if raw < previous || (i == 0 && raw != 1) || raw - 1 > payload.len() {
            return Err(invalid());
        }
        // Both additions are bounded by the validated subslices above.
        item_offsets.push(pos + data_base + raw - 1);
        previous = raw;
    }
    let byte_length = data_base + previous - 1;
    Ok(CffIndex {
        start_offset: pos,
        byte_length,
        item_offsets,
    })
}

pub(crate) fn read_offset(data: &[u8], pos: usize, off_size: usize) -> ParseResult<u32> {
    if pos + off_size > data.len() {
        return Err(ParseError::SyntaxError {
            position: pos,
            message: "read_offset: out of bounds".to_string(),
        });
    }
    let val = match off_size {
        1 => data[pos] as u32,
        2 => u16::from_be_bytes([data[pos], data[pos + 1]]) as u32,
        3 => ((data[pos] as u32) << 16) | ((data[pos + 1] as u32) << 8) | (data[pos + 2] as u32),
        4 => u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]),
        _ => {
            return Err(ParseError::SyntaxError {
                position: pos,
                message: format!("read_offset: invalid off_size {}", off_size),
            })
        }
    };
    Ok(val)
}

pub fn usize_to_cff_offset(val: usize) -> ParseResult<i32> {
    i32::try_from(val).map_err(|_| ParseError::SyntaxError {
        position: 0,
        message: format!("CFF offset {} exceeds i32 range", val),
    })
}

pub fn build_cff_index(items: &[&[u8]]) -> Vec<u8> {
    let count = items.len();
    let mut result = Vec::new();
    result.extend_from_slice(&(count as u16).to_be_bytes());
    if count == 0 {
        return result;
    }
    let total_data: usize = items.iter().map(|i| i.len()).sum();
    let max_offset = total_data + 1;
    let off_size: u8 = if max_offset <= 0xFF {
        1
    } else if max_offset <= 0xFFFF {
        2
    } else if max_offset <= 0xFF_FFFF {
        3
    } else {
        4
    };
    result.push(off_size);
    let mut current: u32 = 1;
    for item in items.iter() {
        write_offset(&mut result, current, off_size);
        current += item.len() as u32;
    }
    write_offset(&mut result, current, off_size);
    for item in items {
        result.extend_from_slice(item);
    }
    result
}

fn write_offset(out: &mut Vec<u8>, value: u32, off_size: u8) {
    match off_size {
        1 => out.push(value as u8),
        2 => out.extend_from_slice(&(value as u16).to_be_bytes()),
        3 => {
            out.push((value >> 16) as u8);
            out.push((value >> 8) as u8);
            out.push(value as u8);
        }
        4 => out.extend_from_slice(&value.to_be_bytes()),
        _ => unreachable!("write_offset called with invalid off_size {}", off_size),
    }
}
