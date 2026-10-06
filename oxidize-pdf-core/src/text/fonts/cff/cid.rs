//! Borrowed, non-executing CFF1 selection for PDF CIDFontType0.
use super::dict::{parse_fd_private, parse_fd_select, parse_top_dict};
use super::index::{parse_cff_index, CffIndex};
use super::types::{CffDictScanner, CffDictToken};
use crate::parser::{ParseError, ParseResult};
use std::collections::BTreeMap;

/// Validated glyph indexes and, for CID-keyed fonts, charset and Font DICT selection.
///
/// This exposes glyph program identity; it does not execute Type2 charstrings
/// or infer Unicode. CFF1 raw data and OpenType wrappers with a CFF table are accepted.
/// For a name-keyed CFF used by a PDF CIDFontType0, CIDs directly
/// index CharStrings; no FDSelect exists, so Font DICT accessors return `None`.
pub struct CidFont<'a> {
    data: &'a [u8],
    cids: BTreeMap<u16, u16>,
    charstrings: CffIndex,
    font_dicts: CffIndex,
    selection: Vec<u8>,
}

impl<'a> CidFont<'a> {
    /// Parse a single CFF1 font for a PDF CIDFontType0 without executing glyph programs.
    ///
    /// # Errors
    /// Rejects invalid headers, missing CID structures, malformed INDEX/charset
    /// or FDSelect tables, duplicate CIDs and out-of-bounds private dictionaries.
    pub fn parse(data: &'a [u8]) -> ParseResult<Self> {
        let opentype = data.starts_with(b"OTTO");
        let data = if opentype {
            crate::text::intrinsic_encoding::opentype_cff(data)
                .ok_or_else(|| invalid("invalid OpenType CFF table"))?
        } else {
            data
        };
        if data.len() < 4 || data[0] != 1 || data[2] < 4 || !(1..=4).contains(&data[3]) {
            return Err(invalid("invalid CFF1 header"));
        }
        let names = parse_cff_index(data, usize::from(data[2]))?;
        let top = parse_cff_index(data, names.end_offset())?;
        if names.count() != 1 || top.count() != 1 {
            return Err(invalid("expected one CFF font"));
        }
        let strings = parse_cff_index(data, top.end_offset())?;
        parse_cff_index(data, strings.end_offset())?;
        let dict = top
            .get_item(0, data)
            .ok_or_else(|| invalid("missing Top DICT"))?;
        let cid_keyed = validate_dict(dict)?;
        let offsets = parse_top_dict(dict);
        let offset = |value: Option<i32>| -> ParseResult<usize> {
            value
                .and_then(|v| usize::try_from(v).ok())
                .filter(|&v| v >= 4 && v < data.len())
                .ok_or_else(|| invalid("missing or invalid CFF offset"))
        };
        let charstrings = parse_cff_index(data, offset(offsets.charstrings_offset)?)?;
        let count = charstrings.count();
        if count == 0 {
            return Err(invalid("empty CFF CharStrings"));
        }
        // A CIDFontType0 may embed a name-keyed CFF program. PDF
        // codes select CIDs, which in this case directly index CharStrings;
        // the OpenType Unicode cmap and SID charset do not remap those CIDs.
        if !cid_keyed {
            if offsets.fd_array_offset.is_some() || offsets.fd_select_offset.is_some() {
                return Err(invalid("CID operators without ROS"));
            }
            if let Some((size, start)) = offsets.private_dict {
                private_range(data, size, start)?;
            }
            return Ok(Self {
                data,
                cids: (0..count).map(|gid| (gid as u16, gid as u16)).collect(),
                charstrings,
                font_dicts: CffIndex::empty(),
                selection: Vec::new(),
            });
        }
        let font_dicts = parse_cff_index(data, offset(offsets.fd_array_offset)?)?;
        if font_dicts.count() == 0 || font_dicts.count() > 256 {
            return Err(invalid("invalid CFF FDArray size"));
        }
        let selection = parse_fd_select(data, offset(offsets.fd_select_offset)?, count)?;
        if selection
            .iter()
            .any(|&fd| usize::from(fd) >= font_dicts.count())
        {
            return Err(invalid("FDSelect index outside FDArray"));
        }
        for fd in 0..font_dicts.count() {
            let dict = font_dicts
                .get_item(fd, data)
                .ok_or_else(|| invalid("missing Font DICT"))?;
            validate_dict(dict)?;
            let (size, start) =
                parse_fd_private(dict).ok_or_else(|| invalid("missing Private DICT"))?;
            private_range(data, size, start)?;
        }
        let mut cursor = offset(offsets.charset_offset)?;
        let format = byte(data, &mut cursor)?;
        if format > 2 {
            return Err(invalid("unsupported CID charset format"));
        }
        let mut cids = BTreeMap::from([(0, 0)]);
        let mut gid = 1usize;
        while gid < count {
            let first = word(data, &mut cursor)?;
            let left = match format {
                0 => 0,
                1 => u16::from(byte(data, &mut cursor)?),
                _ => word(data, &mut cursor)?,
            };
            let end = first
                .checked_add(left)
                .ok_or_else(|| invalid("CID charset range overflow"))?;
            if usize::from(left) + 1 > count - gid {
                return Err(invalid("CID charset exceeds glyph count"));
            }
            for cid in first..=end {
                if cids.insert(cid, gid as u16).is_some() {
                    return Err(invalid("duplicate CID in charset"));
                }
                gid += 1;
            }
        }
        Ok(Self {
            data,
            cids,
            charstrings,
            font_dicts,
            selection,
        })
    }

    /// Map a CID to its actual CharStrings index; absent CIDs return `None`.
    pub fn glyph_id(&self, cid: u16) -> Option<u16> {
        self.cids.get(&cid).copied()
    }
    /// Iterate over the validated CID → GID mapping, including CID0/GID0.
    pub fn mappings(&self) -> impl Iterator<Item = (u16, u16)> + '_ {
        self.cids.iter().map(|(&c, &g)| (c, g))
    }
    /// Return the Font DICT selected by FDSelect for this GID.
    pub fn font_dict_index(&self, gid: u16) -> Option<u8> {
        self.selection.get(usize::from(gid)).copied()
    }
    /// Borrow the original Type2 program bytes for a GID without executing them.
    pub fn charstring(&self, gid: u16) -> Option<&'a [u8]> {
        self.charstrings.get_item(usize::from(gid), self.data)
    }
    /// Borrow a Font DICT's bytes; indexes are those returned by FDSelect.
    pub fn font_dict(&self, index: u8) -> Option<&'a [u8]> {
        self.font_dicts.get_item(usize::from(index), self.data)
    }
}
fn invalid(message: &str) -> ParseError {
    ParseError::SyntaxError {
        position: 0,
        message: message.into(),
    }
}
fn byte(data: &[u8], cursor: &mut usize) -> ParseResult<u8> {
    let value = *data
        .get(*cursor)
        .ok_or_else(|| invalid("truncated CID charset"))?;
    *cursor += 1;
    Ok(value)
}
fn word(data: &[u8], cursor: &mut usize) -> ParseResult<u16> {
    Ok(u16::from_be_bytes([
        byte(data, cursor)?,
        byte(data, cursor)?,
    ]))
}

// Validate token completion and bound operands before the shared DICT readers
// collect them. Offset/ROS operators cannot accept real or extra operands.
fn validate_dict(data: &[u8]) -> ParseResult<bool> {
    let mut scanner = CffDictScanner::new(data);
    let mut end = 0;
    let mut operands = 0;
    let mut has_real = false;
    let mut ros = false;
    loop {
        let start = scanner.position();
        let Some(token) = scanner.next() else {
            break;
        };
        end = scanner.position();
        if matches!(token, CffDictToken::Operand(_)) {
            operands += 1;
            has_real |= data[start] == 30;
            if operands > 48 {
                return Err(invalid("CFF DICT operand limit exceeded"));
            }
            // CffDictScanner tolerates unterminated reals; metadata must not.
            if data[start] == 30
                && !data[start + 1..end]
                    .iter()
                    .any(|b| b & 15 == 15 || b >> 4 == 15)
            {
                return Err(invalid("unterminated CFF real operand"));
            }
            continue;
        }
        let expected = match token {
            CffDictToken::Operator(15 | 17) | CffDictToken::EscapedOperator(36 | 37) => Some(1),
            CffDictToken::Operator(18) => Some(2),
            CffDictToken::EscapedOperator(30) => {
                ros = true;
                Some(3)
            }
            _ => None,
        };
        if expected.is_some_and(|count| count != operands || has_real) {
            return Err(invalid("invalid CFF metadata operands"));
        }
        operands = 0;
        has_real = false;
    }
    if end != data.len() || operands != 0 {
        return Err(invalid("truncated CFF DICT"));
    }
    Ok(ros)
}

fn private_range(data: &[u8], size: i32, start: i32) -> ParseResult<()> {
    let start = usize::try_from(start).map_err(|_| invalid("invalid Private DICT offset"))?;
    let size = usize::try_from(size).map_err(|_| invalid("invalid Private DICT size"))?;
    let end = start
        .checked_add(size)
        .ok_or_else(|| invalid("Private DICT overflow"))?;
    data.get(start..end)
        .ok_or_else(|| invalid("Private DICT outside font"))?;
    Ok(())
}
