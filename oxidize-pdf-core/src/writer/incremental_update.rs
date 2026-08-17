//! Shared primitives for appending one ISO 32000 incremental revision.

use crate::error::{PdfError, Result};
use crate::parser::objects::{PdfDictionary, PdfName, PdfObject, PdfString};
use crate::parser::PdfReader;
use std::io::{Cursor, Read, Seek};

pub(super) struct IncrementalUpdate<'a> {
    base: &'a [u8],
    previous_xref: u64,
    root: (u32, u16),
    original_size: u32,
    next_id: u32,
    first_id: Option<Vec<u8>>,
    replacements: Vec<(u32, u16, PdfObject)>,
}

impl<'a> IncrementalUpdate<'a> {
    pub(super) fn from_reader<R: Read + Seek>(
        base: &'a [u8],
        reader: &PdfReader<R>,
    ) -> Result<Self> {
        let trailer = reader.trailer();
        let root = trailer
            .root()
            .map_err(|e| PdfError::InvalidStructure(format!("base /Root: {e}")))?;
        let size = trailer
            .size()
            .map_err(|e| PdfError::InvalidStructure(format!("base /Size: {e}")))?;
        let first_id = match trailer.id() {
            Some(PdfObject::Array(array)) => array
                .0
                .first()
                .and_then(PdfObject::as_string)
                .map(|value| value.as_bytes().to_vec()),
            _ => None,
        };
        Ok(Self {
            base,
            previous_xref: trailer.xref_offset,
            root,
            original_size: size,
            next_id: size,
            first_id,
            replacements: Vec::new(),
        })
    }

    pub(super) fn from_base(base: &'a [u8]) -> Result<Self> {
        let reader = PdfReader::new(Cursor::new(base))
            .map_err(|e| PdfError::InvalidStructure(format!("parse base PDF: {e}")))?;
        if reader.is_encrypted() {
            return Err(PdfError::PermissionDenied(
                "incremental updates are not supported on encrypted PDFs".to_string(),
            ));
        }
        Self::from_reader(base, &reader)
    }

    pub(super) fn allocate_id(&mut self) -> Result<(u32, u16)> {
        let id = (self.next_id, 0);
        self.next_id = self.next_id.checked_add(1).ok_or_else(|| {
            PdfError::InvalidStructure("PDF object number space is exhausted".to_string())
        })?;
        Ok(id)
    }

    pub(super) fn replace(&mut self, id: (u32, u16), object: PdfObject) -> Result<()> {
        if self
            .replacements
            .iter()
            .any(|(number, generation, _)| (*number, *generation) == id)
        {
            return Err(PdfError::InvalidStructure(format!(
                "object {} {} is rewritten more than once",
                id.0, id.1
            )));
        }
        self.replacements.push((id.0, id.1, object));
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<Vec<u8>> {
        self.replacements
            .sort_by_key(|(number, generation, _)| (*number, *generation));
        let mut out = Vec::with_capacity(self.base.len() + self.replacements.len() * 256 + 512);
        out.extend_from_slice(self.base);
        if !out.ends_with(b"\n") && !out.ends_with(b"\r") {
            out.push(b'\n');
        }

        let mut changed = Vec::with_capacity(self.replacements.len());
        for (number, generation, object) in &self.replacements {
            let offset = out.len() as u64;
            write_indirect_object(&mut out, *number, *generation, object)?;
            changed.push((*number, *generation, offset));
        }

        let xref_position = out.len() as u64;
        out.extend_from_slice(&partial_xref(&changed));
        let size = self.next_id.max(self.original_size);
        let id_pair = self.first_id.map(|first| {
            let mut material = first.clone();
            material.extend_from_slice(&out[self.base.len()..]);
            material.extend_from_slice(&xref_position.to_le_bytes());
            (first, md5::compute(material).0.to_vec())
        });
        write_trailer(
            &mut out,
            self.previous_xref,
            self.root,
            size,
            xref_position,
            id_pair,
        );
        Ok(out)
    }
}

fn write_indirect_object(
    out: &mut Vec<u8>,
    number: u32,
    generation: u16,
    object: &PdfObject,
) -> Result<()> {
    out.extend_from_slice(format!("{number} {generation} obj\n").as_bytes());
    write_object(out, object)?;
    out.extend_from_slice(b"\nendobj\n");
    Ok(())
}

pub(super) fn write_object(out: &mut Vec<u8>, object: &PdfObject) -> Result<()> {
    match object {
        PdfObject::Null => out.extend_from_slice(b"null"),
        PdfObject::Boolean(value) => out.extend_from_slice(if *value { b"true" } else { b"false" }),
        PdfObject::Integer(value) => out.extend_from_slice(value.to_string().as_bytes()),
        PdfObject::Real(value) => out.extend_from_slice(format_real(*value).as_bytes()),
        PdfObject::String(value) => write_string(out, value),
        PdfObject::Name(value) => write_name(out, value),
        PdfObject::Reference(number, generation) => {
            out.extend_from_slice(format!("{number} {generation} R").as_bytes())
        }
        PdfObject::Array(array) => {
            out.push(b'[');
            for (index, value) in array.0.iter().enumerate() {
                if index != 0 {
                    out.push(b' ');
                }
                write_object(out, value)?;
            }
            out.push(b']');
        }
        PdfObject::Dictionary(dictionary) => write_dictionary(out, dictionary)?,
        PdfObject::Stream(_) => {
            return Err(PdfError::InvalidStructure(
                "generic incremental object writer does not accept streams".to_string(),
            ))
        }
    }
    Ok(())
}

pub(super) fn write_dictionary(out: &mut Vec<u8>, dictionary: &PdfDictionary) -> Result<()> {
    out.extend_from_slice(b"<< ");
    let mut keys: Vec<_> = dictionary.0.keys().collect();
    keys.sort_by(|left, right| left.0.cmp(&right.0));
    for key in keys {
        write_name(out, key);
        out.push(b' ');
        write_object(out, &dictionary.0[key])?;
        out.push(b' ');
    }
    out.extend_from_slice(b">>");
    Ok(())
}

fn write_name(out: &mut Vec<u8>, name: &PdfName) {
    out.push(b'/');
    for byte in name.0.bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'+' | b'-' | b'.' | b'_' | b'@' | b'$' | b':' | b';' | b'*' | b'?'
            )
        {
            out.push(byte);
        } else {
            out.extend_from_slice(format!("#{byte:02X}").as_bytes());
        }
    }
}

fn write_string(out: &mut Vec<u8>, value: &PdfString) {
    out.push(b'<');
    for byte in value.as_bytes() {
        out.extend_from_slice(format!("{byte:02X}").as_bytes());
    }
    out.push(b'>');
}

pub(super) fn format_real(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let shortest = value.to_string();
    let Some(exponent_index) = shortest.find(['e', 'E']) else {
        return if shortest == "-0" {
            "0".to_string()
        } else {
            shortest
        };
    };

    let exponent: i32 = shortest[exponent_index + 1..]
        .parse()
        .expect("f64 formatting always emits a valid exponent");
    let mantissa = &shortest[..exponent_index];
    let negative = mantissa.starts_with('-');
    let digits: String = mantissa
        .trim_start_matches('-')
        .chars()
        .filter(|character| *character != '.')
        .collect();
    let decimal_position = mantissa
        .trim_start_matches('-')
        .find('.')
        .map_or(digits.len() as i32, |position| position as i32)
        + exponent;
    let sign = if negative { "-" } else { "" };

    if decimal_position <= 0 {
        format!(
            "{sign}0.{}{digits}",
            "0".repeat((-decimal_position) as usize)
        )
    } else if decimal_position as usize >= digits.len() {
        format!(
            "{sign}{digits}{}",
            "0".repeat(decimal_position as usize - digits.len())
        )
    } else {
        let split = decimal_position as usize;
        format!("{sign}{}.{}", &digits[..split], &digits[split..])
    }
}

fn partial_xref(changed: &[(u32, u16, u64)]) -> Vec<u8> {
    let mut out = b"xref\n".to_vec();
    let mut index = 0;
    while index < changed.len() {
        let start = changed[index].0;
        let mut end = index;
        while end + 1 < changed.len() && changed[end + 1].0 == changed[end].0 + 1 {
            end += 1;
        }
        out.extend_from_slice(format!("{start} {}\n", end - index + 1).as_bytes());
        for (_, generation, offset) in &changed[index..=end] {
            out.extend_from_slice(format!("{offset:010} {generation:05} n \n").as_bytes());
        }
        index = end + 1;
    }
    out
}

fn write_trailer(
    out: &mut Vec<u8>,
    previous_xref: u64,
    root: (u32, u16),
    size: u32,
    xref_position: u64,
    id_pair: Option<(Vec<u8>, Vec<u8>)>,
) {
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {size} /Root {} {} R /Prev {previous_xref} ",
            root.0, root.1
        )
        .as_bytes(),
    );
    if let Some((first, second)) = id_pair {
        let hex = |bytes: &[u8]| {
            bytes
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<String>()
        };
        out.extend_from_slice(format!("/ID [<{}> <{}>] ", hex(&first), hex(&second)).as_bytes());
    }
    out.extend_from_slice(format!(">>\nstartxref\n{xref_position}\n%%EOF\n").as_bytes());
}
