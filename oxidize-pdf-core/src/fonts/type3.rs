//! Renderer-neutral resolution of embedded Type 3 glyph programs.

use crate::parser::content::{ContentOperation, ContentParser};
use crate::parser::document::PdfDocument;
use crate::parser::objects::{PdfDictionary, PdfObject};
use crate::parser::{ParseError, ParseResult};
use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Seek};

/// Maximum decoded size accepted for one glyph program.
pub const MAX_TYPE3_GLYPH_STREAM_SIZE: usize = 8 * 1024 * 1024;

/// A resolved Type 3 font suitable for use by a downstream renderer.
#[derive(Debug, Clone)]
pub struct Type3Font {
    /// Optional PDF font name (`/Name` or `/BaseFont`).
    pub name: Option<String>,
    /// Glyph-space to text-space transformation.
    pub font_matrix: [f64; 6],
    /// Font bounding box in glyph space.
    pub font_bbox: [f64; 4],
    /// Resolved private resources used by glyph programs.
    pub resources: Option<PdfDictionary>,
    glyphs: BTreeMap<u8, Type3Glyph>,
}

/// One resolved Type 3 character procedure.
#[derive(Debug, Clone)]
pub struct Type3Glyph {
    /// Character code used by page text-showing operators.
    pub code: u8,
    /// Name selected through the font encoding.
    pub name: String,
    /// Advance declared by `/Widths` (glyph-space units).
    pub width: f64,
    /// Displacement declared by the `d0` or `d1` operator.
    pub procedure_width: (f64, f64),
    /// Glyph bounding box declared by `d1`, when present.
    pub bbox: Option<[f64; 4]>,
    /// Parsed, renderer-neutral glyph drawing operations. The leading `d0` or
    /// `d1` operator is represented by [`Self::procedure_width`] and
    /// [`Self::bbox`] rather than duplicated here.
    pub operations: Vec<ContentOperation>,
}

impl Type3Font {
    /// Resolve a direct or indirect Type 3 font dictionary and its character
    /// procedures. Glyph names are not interpreted as Unicode.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid Type 3 dictionary, unsupported encoding,
    /// malformed or oversized character procedure, or an unresolved required
    /// indirect object. Codes without a name or `CharProc` are omitted so
    /// [`Self::glyph`] returns `None` and downstream fallback remains possible.
    pub fn resolve<R: Read + Seek>(
        font_object: &PdfObject,
        document: &PdfDocument<R>,
    ) -> ParseResult<Self> {
        let font_object = document.resolve(font_object)?;
        let font = expect_dict(&font_object, "Type 3 font")?;
        if font
            .get("Subtype")
            .and_then(PdfObject::as_name)
            .map(|n| n.0.as_str())
            != Some("Type3")
        {
            return Err(syntax("font subtype is not Type3"));
        }

        let font_matrix = number_array::<6>(font, "FontMatrix")?;
        let font_bbox = number_array::<4>(font, "FontBBox")?;
        let first = integer(font, "FirstChar")?;
        let last = integer(font, "LastChar")?;
        if !(0..=255).contains(&first) || !(0..=255).contains(&last) || first > last {
            return Err(syntax("invalid Type 3 FirstChar/LastChar range"));
        }

        let widths_object = resolve_required(document, font, "Widths")?;
        let widths = widths_object
            .as_array()
            .ok_or_else(|| syntax("Type 3 Widths must be an array"))?;
        let expected = (last - first + 1) as usize;
        if widths.0.len() < expected {
            return Err(syntax("Type 3 Widths is shorter than the character range"));
        }

        let encoding_object = resolve_required(document, font, "Encoding")?;
        let names = encoding_names(&encoding_object)?;
        let charprocs_object = resolve_required(document, font, "CharProcs")?;
        let charprocs = expect_dict(&charprocs_object, "Type 3 CharProcs")?;

        let mut glyphs = BTreeMap::new();
        for code in first..=last {
            let code = code as u8;
            let Some(name) = names.get(&code) else {
                continue;
            };
            let Some(proc_object) = charprocs.get(name) else {
                continue;
            };
            let proc_object = document.resolve(proc_object)?;
            let stream = proc_object
                .as_stream()
                .ok_or_else(|| syntax("Type 3 CharProc must be a stream"))?;
            let data = stream
                .decode_with_limit(&document.options(), MAX_TYPE3_GLYPH_STREAM_SIZE)
                .map_err(|error| {
                    syntax(&format!(
                        "failed to decode Type 3 CharProc /{name} (code {code}): {error}"
                    ))
                })?;
            let parsed = ContentParser::parse_type3_charproc(&data).map_err(|error| {
                syntax(&format!(
                    "invalid Type 3 CharProc /{name} (code {code}): {error}"
                ))
            })?;
            let width = widths.0[(i64::from(code) - first) as usize]
                .as_real()
                .ok_or_else(|| syntax("Type 3 width must be numeric"))?;
            glyphs.insert(
                code,
                Type3Glyph {
                    code,
                    name: name.clone(),
                    width,
                    procedure_width: parsed.width,
                    bbox: parsed.bbox,
                    operations: parsed.operations,
                },
            );
        }

        let resources = match font.get("Resources") {
            Some(value) => {
                Some(expect_dict(&document.resolve(value)?, "Type 3 Resources")?.clone())
            }
            None => None,
        };
        let name = font
            .get("Name")
            .or_else(|| font.get("BaseFont"))
            .and_then(PdfObject::as_name)
            .map(|value| value.0.clone());
        Ok(Self {
            name,
            font_matrix,
            font_bbox,
            resources,
            glyphs,
        })
    }

    /// Return the resolved glyph for a character code, or `None` when its
    /// encoding is undefined or the font provides no corresponding `CharProc`.
    pub fn glyph(&self, code: u8) -> Option<&Type3Glyph> {
        self.glyphs.get(&code)
    }

    /// Iterate over all resolved glyphs.
    pub fn glyphs(&self) -> impl Iterator<Item = &Type3Glyph> {
        self.glyphs.values()
    }

    /// Resolve one entry from the font's private resource dictionary.
    ///
    /// # Errors
    ///
    /// Returns an error when the category is not a dictionary or an indirect
    /// resource cannot be resolved.
    pub fn resolve_resource<R: Read + Seek>(
        &self,
        category: &str,
        name: &str,
        document: &PdfDocument<R>,
    ) -> ParseResult<Option<PdfObject>> {
        let Some(resources) = &self.resources else {
            return Ok(None);
        };
        let Some(category_object) = resources.get(category) else {
            return Ok(None);
        };
        let category_object = document.resolve(category_object)?;
        let category_dict = expect_dict(&category_object, "Type 3 resource category")?;
        category_dict
            .get(name)
            .map(|value| document.resolve(value))
            .transpose()
    }
}

fn resolve_required<R: Read + Seek>(
    document: &PdfDocument<R>,
    dict: &PdfDictionary,
    key: &str,
) -> ParseResult<PdfObject> {
    document.resolve(
        dict.get(key)
            .ok_or_else(|| ParseError::MissingKey(key.into()))?,
    )
}

fn expect_dict<'a>(object: &'a PdfObject, description: &str) -> ParseResult<&'a PdfDictionary> {
    object
        .as_dict()
        .ok_or_else(|| syntax(&format!("{description} must be a dictionary")))
}

fn integer(dict: &PdfDictionary, key: &str) -> ParseResult<i64> {
    dict.get(key)
        .and_then(PdfObject::as_integer)
        .ok_or_else(|| ParseError::MissingKey(key.into()))
}

fn number_array<const N: usize>(dict: &PdfDictionary, key: &str) -> ParseResult<[f64; N]> {
    let array = dict
        .get(key)
        .and_then(PdfObject::as_array)
        .ok_or_else(|| ParseError::MissingKey(key.into()))?;
    if array.0.len() != N {
        return Err(syntax(&format!("{key} must contain {N} numbers")));
    }
    let mut result = [0.0; N];
    for (target, object) in result.iter_mut().zip(&array.0) {
        *target = object
            .as_real()
            .ok_or_else(|| syntax(&format!("{key} must contain only numbers")))?;
    }
    Ok(result)
}

fn encoding_names(encoding: &PdfObject) -> ParseResult<HashMap<u8, String>> {
    match encoding {
        PdfObject::Name(name) => predefined_encoding(name.as_str()),
        PdfObject::Dictionary(dict) => {
            let mut names = match dict.get("BaseEncoding").and_then(PdfObject::as_name) {
                Some(name) => predefined_encoding(name.as_str())?,
                None => predefined_encoding("StandardEncoding")?,
            };
            apply_encoding_differences(dict, &mut names)?;
            Ok(names)
        }
        _ => Err(syntax("Type 3 Encoding must be a name or dictionary")),
    }
}

fn apply_encoding_differences(
    encoding: &PdfDictionary,
    result: &mut HashMap<u8, String>,
) -> ParseResult<()> {
    let Some(differences) = encoding.get("Differences").and_then(PdfObject::as_array) else {
        return Ok(());
    };
    let mut code: Option<i64> = None;
    for object in &differences.0 {
        if let Some(value) = object.as_integer() {
            if !(0..=255).contains(&value) {
                return Err(syntax("Encoding Differences code is outside 0..=255"));
            }
            code = Some(value);
        } else if let Some(name) = object.as_name() {
            let value =
                code.ok_or_else(|| syntax("Encoding Differences name has no starting code"))?;
            result.insert(value as u8, name.0.clone());
            code = value.checked_add(1);
        } else {
            return Err(syntax("invalid object in Encoding Differences"));
        }
    }
    Ok(())
}

fn predefined_encoding(name: &str) -> ParseResult<HashMap<u8, String>> {
    if !matches!(
        name,
        "StandardEncoding" | "WinAnsiEncoding" | "MacRomanEncoding"
    ) {
        return Err(syntax(&format!("unsupported Type 3 encoding /{name}")));
    }
    let mut names = HashMap::new();
    for code in 32u8..=126 {
        names.insert(code, ascii_glyph_name(code).to_string());
    }
    match name {
        "StandardEncoding" => {
            names.insert(39, "quoteright".into());
            names.insert(96, "quoteleft".into());
            for &(code, glyph) in STANDARD_ENCODING_EXTENDED {
                names.insert(code, glyph.into());
            }
        }
        "WinAnsiEncoding" => {
            for &(code, glyph) in WIN_ANSI_EXTENDED {
                names.insert(code, glyph.into());
            }
        }
        "MacRomanEncoding" => {
            for (offset, glyph) in MAC_ROMAN_EXTENDED.iter().enumerate() {
                names.insert(0x80 + offset as u8, (*glyph).into());
            }
        }
        _ => unreachable!(),
    }
    Ok(names)
}

const STANDARD_ENCODING_EXTENDED: &[(u8, &str)] = &[
    (161, "exclamdown"),
    (162, "cent"),
    (163, "sterling"),
    (164, "fraction"),
    (165, "yen"),
    (166, "florin"),
    (167, "section"),
    (168, "currency"),
    (169, "quotesingle"),
    (170, "quotedblleft"),
    (171, "guillemotleft"),
    (172, "guilsinglleft"),
    (173, "guilsinglright"),
    (174, "fi"),
    (175, "fl"),
    (177, "endash"),
    (178, "dagger"),
    (179, "daggerdbl"),
    (180, "periodcentered"),
    (182, "paragraph"),
    (183, "bullet"),
    (184, "quotesinglbase"),
    (185, "quotedblbase"),
    (186, "quotedblright"),
    (187, "guillemotright"),
    (188, "ellipsis"),
    (189, "perthousand"),
    (191, "questiondown"),
    (193, "grave"),
    (194, "acute"),
    (195, "circumflex"),
    (196, "tilde"),
    (197, "macron"),
    (198, "breve"),
    (199, "dotaccent"),
    (200, "dieresis"),
    (202, "ring"),
    (203, "cedilla"),
    (205, "hungarumlaut"),
    (206, "ogonek"),
    (207, "caron"),
    (208, "emdash"),
    (225, "AE"),
    (227, "ordfeminine"),
    (232, "Lslash"),
    (233, "Oslash"),
    (234, "OE"),
    (235, "ordmasculine"),
    (241, "ae"),
    (245, "dotlessi"),
    (248, "lslash"),
    (249, "oslash"),
    (250, "oe"),
    (251, "germandbls"),
];

const WIN_ANSI_EXTENDED: &[(u8, &str)] = &[
    (128, "Euro"),
    (130, "quotesinglbase"),
    (131, "florin"),
    (132, "quotedblbase"),
    (133, "ellipsis"),
    (134, "dagger"),
    (135, "daggerdbl"),
    (136, "circumflex"),
    (137, "perthousand"),
    (138, "Scaron"),
    (139, "guilsinglleft"),
    (140, "OE"),
    (142, "Zcaron"),
    (145, "quoteleft"),
    (146, "quoteright"),
    (147, "quotedblleft"),
    (148, "quotedblright"),
    (149, "bullet"),
    (150, "endash"),
    (151, "emdash"),
    (152, "tilde"),
    (153, "trademark"),
    (154, "scaron"),
    (155, "guilsinglright"),
    (156, "oe"),
    (158, "zcaron"),
    (159, "Ydieresis"),
    (160, "space"),
    (161, "exclamdown"),
    (162, "cent"),
    (163, "sterling"),
    (164, "currency"),
    (165, "yen"),
    (166, "brokenbar"),
    (167, "section"),
    (168, "dieresis"),
    (169, "copyright"),
    (170, "ordfeminine"),
    (171, "guillemotleft"),
    (172, "logicalnot"),
    (173, "hyphen"),
    (174, "registered"),
    (175, "macron"),
    (176, "degree"),
    (177, "plusminus"),
    (178, "twosuperior"),
    (179, "threesuperior"),
    (180, "acute"),
    (181, "mu"),
    (182, "paragraph"),
    (183, "periodcentered"),
    (184, "cedilla"),
    (185, "onesuperior"),
    (186, "ordmasculine"),
    (187, "guillemotright"),
    (188, "onequarter"),
    (189, "onehalf"),
    (190, "threequarters"),
    (191, "questiondown"),
    (192, "Agrave"),
    (193, "Aacute"),
    (194, "Acircumflex"),
    (195, "Atilde"),
    (196, "Adieresis"),
    (197, "Aring"),
    (198, "AE"),
    (199, "Ccedilla"),
    (200, "Egrave"),
    (201, "Eacute"),
    (202, "Ecircumflex"),
    (203, "Edieresis"),
    (204, "Igrave"),
    (205, "Iacute"),
    (206, "Icircumflex"),
    (207, "Idieresis"),
    (208, "Eth"),
    (209, "Ntilde"),
    (210, "Ograve"),
    (211, "Oacute"),
    (212, "Ocircumflex"),
    (213, "Otilde"),
    (214, "Odieresis"),
    (215, "multiply"),
    (216, "Oslash"),
    (217, "Ugrave"),
    (218, "Uacute"),
    (219, "Ucircumflex"),
    (220, "Udieresis"),
    (221, "Yacute"),
    (222, "Thorn"),
    (223, "germandbls"),
    (224, "agrave"),
    (225, "aacute"),
    (226, "acircumflex"),
    (227, "atilde"),
    (228, "adieresis"),
    (229, "aring"),
    (230, "ae"),
    (231, "ccedilla"),
    (232, "egrave"),
    (233, "eacute"),
    (234, "ecircumflex"),
    (235, "edieresis"),
    (236, "igrave"),
    (237, "iacute"),
    (238, "icircumflex"),
    (239, "idieresis"),
    (240, "eth"),
    (241, "ntilde"),
    (242, "ograve"),
    (243, "oacute"),
    (244, "ocircumflex"),
    (245, "otilde"),
    (246, "odieresis"),
    (247, "divide"),
    (248, "oslash"),
    (249, "ugrave"),
    (250, "uacute"),
    (251, "ucircumflex"),
    (252, "udieresis"),
    (253, "yacute"),
    (254, "thorn"),
    (255, "ydieresis"),
];

const MAC_ROMAN_EXTENDED: [&str; 128] = [
    "Adieresis",
    "Aring",
    "Ccedilla",
    "Eacute",
    "Ntilde",
    "Odieresis",
    "Udieresis",
    "aacute",
    "agrave",
    "acircumflex",
    "adieresis",
    "atilde",
    "aring",
    "ccedilla",
    "eacute",
    "egrave",
    "ecircumflex",
    "edieresis",
    "iacute",
    "igrave",
    "icircumflex",
    "idieresis",
    "ntilde",
    "oacute",
    "ograve",
    "ocircumflex",
    "odieresis",
    "otilde",
    "uacute",
    "ugrave",
    "ucircumflex",
    "udieresis",
    "dagger",
    "degree",
    "cent",
    "sterling",
    "section",
    "bullet",
    "paragraph",
    "germandbls",
    "registered",
    "copyright",
    "trademark",
    "acute",
    "dieresis",
    "notequal",
    "AE",
    "Oslash",
    "infinity",
    "plusminus",
    "lessequal",
    "greaterequal",
    "yen",
    "mu",
    "partialdiff",
    "summation",
    "product",
    "pi",
    "integral",
    "ordfeminine",
    "ordmasculine",
    "Omega",
    "ae",
    "oslash",
    "questiondown",
    "exclamdown",
    "logicalnot",
    "radical",
    "florin",
    "approxequal",
    "Delta",
    "guillemotleft",
    "guillemotright",
    "ellipsis",
    "space",
    "Agrave",
    "Atilde",
    "Otilde",
    "OE",
    "oe",
    "endash",
    "emdash",
    "quotedblleft",
    "quotedblright",
    "quoteleft",
    "quoteright",
    "divide",
    "lozenge",
    "ydieresis",
    "Ydieresis",
    "fraction",
    "currency",
    "guilsinglleft",
    "guilsinglright",
    "fi",
    "fl",
    "daggerdbl",
    "periodcentered",
    "quotesinglbase",
    "quotedblbase",
    "perthousand",
    "Acircumflex",
    "Ecircumflex",
    "Aacute",
    "Edieresis",
    "Egrave",
    "Iacute",
    "Icircumflex",
    "Idieresis",
    "Igrave",
    "Oacute",
    "Ocircumflex",
    "apple",
    "Ograve",
    "Uacute",
    "Ucircumflex",
    "Ugrave",
    "dotlessi",
    "circumflex",
    "tilde",
    "macron",
    "breve",
    "dotaccent",
    "ring",
    "cedilla",
    "hungarumlaut",
    "ogonek",
    "caron",
];

fn ascii_glyph_name(code: u8) -> &'static str {
    const DIGITS: [&str; 10] = [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
    ];
    match code {
        b'A'..=b'Z' | b'a'..=b'z' => match code {
            b'A' => "A",
            b'B' => "B",
            b'C' => "C",
            b'D' => "D",
            b'E' => "E",
            b'F' => "F",
            b'G' => "G",
            b'H' => "H",
            b'I' => "I",
            b'J' => "J",
            b'K' => "K",
            b'L' => "L",
            b'M' => "M",
            b'N' => "N",
            b'O' => "O",
            b'P' => "P",
            b'Q' => "Q",
            b'R' => "R",
            b'S' => "S",
            b'T' => "T",
            b'U' => "U",
            b'V' => "V",
            b'W' => "W",
            b'X' => "X",
            b'Y' => "Y",
            b'Z' => "Z",
            b'a' => "a",
            b'b' => "b",
            b'c' => "c",
            b'd' => "d",
            b'e' => "e",
            b'f' => "f",
            b'g' => "g",
            b'h' => "h",
            b'i' => "i",
            b'j' => "j",
            b'k' => "k",
            b'l' => "l",
            b'm' => "m",
            b'n' => "n",
            b'o' => "o",
            b'p' => "p",
            b'q' => "q",
            b'r' => "r",
            b's' => "s",
            b't' => "t",
            b'u' => "u",
            b'v' => "v",
            b'w' => "w",
            b'x' => "x",
            b'y' => "y",
            _ => "z",
        },
        b'0'..=b'9' => DIGITS[(code - b'0') as usize],
        b' ' => "space",
        b'!' => "exclam",
        b'\"' => "quotedbl",
        b'#' => "numbersign",
        b'$' => "dollar",
        b'%' => "percent",
        b'&' => "ampersand",
        b'\'' => "quotesingle",
        b'(' => "parenleft",
        b')' => "parenright",
        b'*' => "asterisk",
        b'+' => "plus",
        b',' => "comma",
        b'-' => "hyphen",
        b'.' => "period",
        b'/' => "slash",
        b':' => "colon",
        b';' => "semicolon",
        b'<' => "less",
        b'=' => "equal",
        b'>' => "greater",
        b'?' => "question",
        b'@' => "at",
        b'[' => "bracketleft",
        b'\\' => "backslash",
        b']' => "bracketright",
        b'^' => "asciicircum",
        b'_' => "underscore",
        b'`' => "grave",
        b'{' => "braceleft",
        b'|' => "bar",
        b'}' => "braceright",
        b'~' => "asciitilde",
        _ => ".notdef",
    }
}

fn syntax(message: &str) -> ParseError {
    ParseError::SyntaxError {
        position: 0,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::objects::{PdfArray, PdfName};

    #[test]
    fn named_standard_encoding_maps_ascii_glyph_names() {
        let names = encoding_names(&PdfObject::Name(PdfName("StandardEncoding".into())))
            .expect("known encoding");
        assert_eq!(names.get(&65).map(String::as_str), Some("A"));
        assert_eq!(names.get(&48).map(String::as_str), Some("zero"));
    }

    #[test]
    fn differences_override_base_encoding() {
        let mut encoding = PdfDictionary::new();
        encoding.insert(
            "BaseEncoding".into(),
            PdfObject::Name(PdfName("WinAnsiEncoding".into())),
        );
        encoding.insert(
            "Differences".into(),
            PdfObject::Array(PdfArray(vec![
                PdfObject::Integer(65),
                PdfObject::Name(PdfName("OpaqueA".into())),
            ])),
        );
        let names = encoding_names(&PdfObject::Dictionary(encoding)).expect("valid encoding");
        assert_eq!(names.get(&65).map(String::as_str), Some("OpaqueA"));
        assert_eq!(names.get(&66).map(String::as_str), Some("B"));
    }
}
