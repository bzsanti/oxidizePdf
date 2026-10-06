//! Bounded, non-executing readers for embedded Type1/CFF intrinsic encodings.
//! Computed PostScript remains an unsupported recovery path.
use super::cmap::{tokenize_cmap, Token};
use super::font_program_names::{
    EXPERT_CHARSET, EXPERT_ENCODING, EXPERT_SUBSET_CHARSET, STANDARD_ENCODING, STANDARD_STRINGS,
};
use super::fonts::cff::index::parse_cff_index;
use super::fonts::cff::types::{CffDictScanner, CffDictToken};
use std::collections::HashMap;

fn keyword(token: &Token, expected: &str) -> bool {
    matches!(token,Token::Keyword(value) if value==expected)
}
fn standard_names() -> HashMap<u8, String> {
    STANDARD_ENCODING
        .iter()
        .enumerate()
        .map(|(code, sid)| (code as u8, STANDARD_STRINGS[usize::from(*sid)].to_owned()))
        .collect()
}

/// Read only static StandardEncoding or the conventional 256-array/dup form.
/// Braces are lexical delimiters; no PostScript procedure is executed.
pub(super) fn type1(cleartext: &[u8]) -> Option<HashMap<u8, String>> {
    if cleartext.len() > 512 * 1024 {
        return None;
    }
    let content = std::str::from_utf8(cleartext)
        .ok()?
        .replace('{', " { ")
        .replace('}', " } ")
        .replace("<<", " __dict_begin__ ")
        .replace(">>", " __dict_end__ ");
    let tokens = tokenize_cmap(&content);
    let mut procedures = 0usize;
    let mut dictionaries = 0usize;
    let mut begins = 0usize;
    let mut start = None;
    for (index, token) in tokens.iter().enumerate() {
        if keyword(token, "{") {
            procedures += 1;
        } else if keyword(token, "}") {
            procedures = procedures.checked_sub(1)?;
        } else if procedures == 0 {
            if keyword(token, "__dict_begin__") {
                dictionaries += 1;
            } else if keyword(token, "__dict_end__") {
                dictionaries = dictionaries.checked_sub(1)?;
            } else if dictionaries == 0 {
                if keyword(token, "begin") {
                    begins += 1;
                } else if keyword(token, "end") {
                    begins = begins.checked_sub(1)?;
                } else if begins <= 1 && matches!(token, Token::Name(n) if n == "Encoding") {
                    start = Some(index + 1);
                    break;
                }
            }
        }
    }
    let start = start?;
    let end = tokens[start..].iter().position(|t| keyword(t, "def"))? + start;
    let tokens = &tokens[start..end];
    let mut i;
    let mut names;
    if tokens
        .first()
        .is_some_and(|t| keyword(t, "StandardEncoding"))
    {
        names = standard_names();
        i = 1;
        if matches!(tokens.get(i), Some(Token::Integer(256)))
            && tokens.get(i + 1).is_some_and(|t| keyword(t, "array"))
            && tokens.get(i + 2).is_some_and(|t| keyword(t, "copy"))
        {
            i += 3;
        }
    } else {
        // Require the actual initializer, not merely an arbitrary procedure
        // containing /Encoding or a dup/name/put sequence in unrelated data.
        if !matches!(tokens.first(), Some(Token::Integer(256)))
            || !tokens.get(1).is_some_and(|t| keyword(t, "array"))
        {
            return None;
        }
        let init = tokens.get(2..13)?;
        if !matches!(
            (&init[0], &init[1], &init[2]),
            (Token::Integer(0), Token::Integer(1), Token::Integer(255))
        ) || !keyword(&init[3], "{")
            || !matches!(init[4], Token::Integer(1))
            || !keyword(&init[5], "index")
            || !keyword(&init[6], "exch")
            || !matches!(&init[7], Token::Name(n) if n == ".notdef")
            || !keyword(&init[8], "put")
            || !keyword(&init[9], "}")
            || !keyword(&init[10], "for")
        {
            return None;
        }
        names = (0..=255).map(|code| (code, ".notdef".to_owned())).collect();
        i = 13;
    }
    while i < tokens.len() {
        if keyword(&tokens[i], "readonly") && i + 1 == tokens.len() {
            break;
        }
        let sequence = tokens.get(i..i + 4)?;
        if !keyword(&sequence[0], "dup") || !keyword(&sequence[3], "put") {
            return None;
        }
        let (Token::Integer(code), Token::Name(name)) = (&sequence[1], &sequence[2]) else {
            return None;
        };
        if name.len() > 127 {
            return None;
        }
        names.insert(u8::try_from(*code).ok()?, name.clone());
        i += 4;
    }
    Some(names)
}

fn byte(data: &[u8], position: &mut usize) -> Option<u8> {
    let value = *data.get(*position)?;
    *position += 1;
    Some(value)
}
fn word(data: &[u8], position: &mut usize) -> Option<u16> {
    Some(u16::from_be_bytes([
        byte(data, position)?,
        byte(data, position)?,
    ]))
}

// Validate INDEX offsets before calling the shared parser: zero, descending,
// or out-of-file offsets must never reach its unchecked length arithmetic.
fn checked_index(data: &[u8], start: usize) -> Option<super::fonts::cff::index::CffIndex> {
    let mut cursor = start;
    let count = usize::from(word(data, &mut cursor)?);
    if count > 0 {
        let width = usize::from(byte(data, &mut cursor)?);
        if !(1..=4).contains(&width) {
            return None;
        }
        let base = cursor.checked_add((count + 1).checked_mul(width)?)?;
        let available = data.len().checked_sub(base)?;
        let mut previous = 1usize;
        for index in 0..=count {
            let mut offset = 0usize;
            for _ in 0..width {
                offset = offset
                    .checked_mul(256)?
                    .checked_add(usize::from(byte(data, &mut cursor)?))?;
            }
            if offset < previous
                || (index == 0 && offset != 1)
                || offset.checked_sub(1)? > available
            {
                return None;
            }
            previous = offset;
        }
    }
    parse_cff_index(data, start).ok()
}

/// CFF1 custom Encoding formats0/1 and supplements, plus StandardEncoding.
/// Charset formats0/1/2 and all three predefined charsets are supported.
pub(super) fn cff(data: &[u8]) -> Option<HashMap<u8, String>> {
    if data.first() != Some(&1) || data.len() > 8 * 1024 * 1024 {
        return None;
    }
    let header = usize::from(*data.get(2)?);
    if header < 4 {
        return None;
    }
    let names = checked_index(data, header)?;
    if names.count() != 1 {
        return None;
    }
    let top = checked_index(data, names.end_offset())?;
    if top.count() != 1 {
        return None;
    }
    let dictionary = top.get_item(0, data)?;
    let strings = checked_index(data, top.end_offset())?;
    let (mut charset, mut encoding, mut charstrings) = (0usize, 0usize, None);
    let mut operand = None;
    for token in CffDictScanner::new(dictionary) {
        match token {
            CffDictToken::Operand(value) => operand = Some(value),
            CffDictToken::Operator(operator) => {
                match operator {
                    15 => charset = usize::try_from(operand?).ok()?,
                    16 => encoding = usize::try_from(operand?).ok()?,
                    17 => charstrings = Some(usize::try_from(operand?).ok()?),
                    _ => {}
                }
                operand = None;
            }
            CffDictToken::EscapedOperator(30) => return None, // CID-keyed: not a simple-font Encoding.
            CffDictToken::EscapedOperator(_) => operand = None,
        }
    }
    let glyphs = checked_index(data, charstrings?)?.count();
    if glyphs == 0 {
        return None;
    }
    let sids = match charset {
        0 => {
            if glyphs > 229 {
                return None;
            }
            (0..glyphs).map(|gid| gid as u16).collect::<Vec<_>>()
        }
        1 => EXPERT_CHARSET.get(..glyphs)?.to_vec(),
        2 => EXPERT_SUBSET_CHARSET.get(..glyphs)?.to_vec(),
        _ => {
            let mut position = charset;
            let format = byte(data, &mut position)?;
            let mut sids = vec![0];
            while sids.len() < glyphs {
                let first = word(data, &mut position)?;
                let left = match format {
                    0 => 0,
                    1 => u16::from(byte(data, &mut position)?),
                    2 => word(data, &mut position)?,
                    _ => return None,
                };
                if sids.len() + usize::from(left) + 1 > glyphs {
                    return None;
                }
                let last = first.checked_add(left)?;
                sids.extend(first..=last);
            }
            sids
        }
    };
    if sids.iter().any(|sid| *sid > 64999) {
        return None;
    }
    let mut code_sids = [0u16; 256];
    match encoding {
        0 | 1 => {
            let table = if encoding == 0 {
                &STANDARD_ENCODING
            } else {
                &EXPERT_ENCODING
            };
            for (code, sid) in table.iter().enumerate() {
                if sids.contains(sid) {
                    code_sids[code] = *sid;
                }
            }
        }
        _ => {
            let mut position = encoding;
            let format = byte(data, &mut position)?;
            let count = usize::from(byte(data, &mut position)?);
            let mut gid = 1;
            match format & 0x7f {
                0 => {
                    for _ in 0..count {
                        let code = byte(data, &mut position)?;
                        code_sids[usize::from(code)] = *sids.get(gid)?;
                        gid += 1;
                    }
                }
                1 => {
                    for _ in 0..count {
                        let first = byte(data, &mut position)?;
                        let last = first.checked_add(byte(data, &mut position)?)?;
                        for code in first..=last {
                            code_sids[usize::from(code)] = *sids.get(gid)?;
                            gid += 1;
                        }
                    }
                }
                _ => return None,
            }
            if format & 0x80 != 0 {
                let count = byte(data, &mut position)?;
                for _ in 0..count {
                    let code = byte(data, &mut position)?;
                    let sid = word(data, &mut position)?;
                    if !sids.contains(&sid) {
                        return None;
                    }
                    code_sids[usize::from(code)] = sid;
                }
            }
        }
    }
    let mut result = HashMap::new();
    for (code, sid) in code_sids.into_iter().enumerate() {
        let name = if usize::from(sid) < STANDARD_STRINGS.len() {
            STANDARD_STRINGS[usize::from(sid)]
        } else {
            std::str::from_utf8(strings.get_item(usize::from(sid) - STANDARD_STRINGS.len(), data)?)
                .ok()?
        };
        if name.len() > 127 {
            return None;
        }
        result.insert(code as u8, name.to_owned());
    }
    Some(result)
}

pub(super) fn opentype_cff(data: &[u8]) -> Option<&[u8]> {
    if data.get(..4)? != b"OTTO" {
        return None;
    }
    let count = usize::from(u16::from_be_bytes(data.get(4..6)?.try_into().ok()?));
    for index in 0..count {
        let entry = data.get(12 + index * 16..28 + index * 16)?;
        if &entry[..4] == b"CFF " {
            let start = usize::try_from(u32::from_be_bytes(entry[8..12].try_into().ok()?)).ok()?;
            let length =
                usize::try_from(u32::from_be_bytes(entry[12..16].try_into().ok()?)).ok()?;
            return data.get(start..start.checked_add(length)?);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::super::fonts::cff::index::build_cff_index;
    use super::*;

    fn program(charset: &[u8], encoding: &[u8], predefined: Option<u8>) -> Vec<u8> {
        let mut data = vec![1, 0, 4, 4];
        data.extend(build_cff_index(&[b"Contract"]));
        let dict_start = data.len();
        let make_dict = |charset: usize, encoding: usize, chars: usize| {
            let mut dict = Vec::new();
            for (value, operator) in [(charset, 15), (encoding, 16), (chars, 17)] {
                dict.push(29);
                dict.extend((value as i32).to_be_bytes());
                dict.push(operator);
            }
            build_cff_index(&[&dict])
        };
        let offset = dict_start + make_dict(0, 0, 0).len() + 4;
        data.extend(make_dict(
            offset,
            predefined.map_or(offset + charset.len(), usize::from),
            offset + charset.len() + encoding.len(),
        ));
        data.extend([0, 0, 0, 0]); // empty String and GlobalSubr INDEX
        data.extend(charset);
        data.extend(encoding);
        data.extend(build_cff_index(&[&[14], &[14], &[14]]));
        data
    }

    #[test]
    fn unexecuted_scopes_cannot_supply_the_intrinsic_encoding() {
        for prefix in [
            "/Unused { /Encoding StandardEncoding def } def ",
            "/Unused { { /Encoding StandardEncoding def } } def ",
            "/Unused << /Encoding StandardEncoding >> def ",
            "/Notice (/Encoding StandardEncoding def) def ",
            "% /Encoding StandardEncoding def\n",
        ] {
            let content =
                format!("{prefix}/Encoding StandardEncoding 256 array copy dup 65 /B put def");
            assert_eq!(type1(content.as_bytes()).unwrap()[&65], "B", "{prefix}");
        }
        assert!(type1(b"/Unused { /Encoding StandardEncoding def } def").is_none());
        let font = b"10 dict begin /FontInfo 1 dict begin /Encoding StandardEncoding def end def /Encoding StandardEncoding 256 array copy dup 65 /B put def end";
        assert_eq!(type1(font).unwrap()[&65], "B");
    }

    #[test]
    fn type1_static_array_and_standard_copy_preserve_boundaries() {
        let parsed = type1(b"/Encoding 256 array 0 1 255 {1 index exch /.notdef put} for dup 0 /A put dup 255 /B put readonly def").unwrap();
        assert_eq!(parsed[&0], "A");
        assert_eq!(parsed[&255], "B");
        assert_eq!(parsed[&65], ".notdef");
        let parsed = type1(b"/Encoding StandardEncoding 256 array copy dup 65 /B put def").unwrap();
        assert_eq!(parsed[&65], "B");
        assert_eq!(parsed[&66], "B");
    }

    #[test]
    fn type1_rejects_computed_procedures_and_invalid_codes() {
        for input in [
            "/Encoding StandardEncoding { dup 65 /B put } def",
            "/Encoding StandardEncoding dup -1 /B put def",
            "/Encoding StandardEncoding dup 256 /B put def",
            "/Encoding StandardEncoding 1 2 add def",
            "/Encoding 256 array 0 1 255 1 index exch /.notdef put for def",
        ] {
            assert!(type1(input.as_bytes()).is_none(), "{input}");
        }
        assert!(type1(&vec![b' '; 512 * 1024 + 1]).is_none());
    }

    #[test]
    fn cff_custom_formats_and_supplements_use_charset_sids() {
        // Charset A/B: SIDs34/35; code65 deliberately selects B via supplement.
        for encoding in [
            &[0x80, 2, 65, 66, 1, 65, 0, 35][..],
            &[0x81, 1, 65, 1, 1, 65, 0, 35][..],
        ] {
            let parsed = cff(&program(&[0, 0, 34, 0, 35], encoding, None)).unwrap();
            assert_eq!(parsed[&65], "B");
            assert_eq!(parsed[&66], "B");
            assert_eq!(parsed[&67], ".notdef");
        }
        for charset in [&[1, 0, 34, 1][..], &[2, 0, 34, 0, 1][..]] {
            let parsed = cff(&program(charset, &[0, 2, 66, 65], None)).unwrap();
            assert_eq!(parsed[&65], "B");
            assert_eq!(parsed[&66], "A");
        }
    }

    #[test]
    fn cff_expert_is_distinct_from_standard_and_macexpert() {
        // Adobe Technical Note5176: Expert code33=SID229,34=SID230.
        let parsed = cff(&program(&[0, 0, 229, 0, 230], &[], Some(1))).unwrap();
        assert_eq!(parsed[&33], "exclamsmall");
        assert_eq!(parsed[&34], "Hungarumlautsmall");
        assert_eq!(parsed[&65], ".notdef");
    }

    #[test]
    fn malformed_cff_ranges_offsets_and_truncation_do_not_panic() {
        let valid = program(&[0, 0, 34, 0, 35], &[0, 2, 65, 66], None);
        for end in 0..valid.len() {
            assert!(cff(&valid[..end]).is_none(), "prefix {end}");
        }
        for encoding in [
            &[1, 1, 255, 1][..],
            &[0, 3, 1, 2, 3][..],
            &[0x80, 0, 1, 65, 1, 255][..],
        ] {
            assert!(cff(&program(&[0, 0, 34, 0, 35], encoding, None)).is_none());
        }
        for offsets in [[0, 0], [2, 1], [1, 255]] {
            let mut invalid = valid.clone();
            invalid[7..9].copy_from_slice(&offsets);
            assert!(cff(&invalid).is_none());
        }
    }

    #[test]
    fn opentype_directory_bounds_and_cff_table_selection() {
        let mut data = b"OTTO\0\x01\0\0\0\0\0\0CFF \0\0\0\0\0\0\0\x1c\0\0\0\x03abc".to_vec();
        assert_eq!(opentype_cff(&data), Some(&b"abc"[..]));
        data[24..28].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(opentype_cff(&data).is_none());
        assert!(opentype_cff(&data[..20]).is_none());
    }
}
