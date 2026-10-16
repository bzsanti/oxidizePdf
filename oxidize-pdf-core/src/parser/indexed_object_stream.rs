//! Internal, decoded object-stream index. Unlike the public eager ObjectStream,
//! this representation never parses unrelated objects (notably sparse owners).
use super::lexer::{Lexer, Token};
use super::objects::{PdfObject, PdfStream};
use super::{ParseError, ParseOptions, ParseResult};
use std::collections::HashMap;
use std::io::Cursor;
use std::ops::Range;

pub(super) struct IndexedObjectStream {
    data: Vec<u8>,
    ranges: HashMap<u32, Range<usize>>,
}

fn invalid(message: &str) -> ParseError {
    ParseError::SyntaxError {
        position: 0,
        message: message.to_owned(),
    }
}

impl IndexedObjectStream {
    pub(super) fn parse(stream: &PdfStream, options: &ParseOptions) -> ParseResult<Self> {
        let integer = |key| -> ParseResult<usize> {
            stream
                .dict
                .get(key)
                .and_then(PdfObject::as_integer)
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| invalid("Invalid object-stream N or First"))
        };
        let n = integer("N")?;
        let first = integer("First")?;
        let data = stream.decode(options)?;
        if first > data.len() || n > first {
            return Err(invalid("Object-stream header exceeds decoded data"));
        }
        let mut cursor = Cursor::new(&data[..first]);
        let mut lexer = Lexer::new_with_options(&mut cursor, options.clone());
        let mut offsets = Vec::new();
        for _ in 0..n {
            let number = match lexer.next_token()? {
                Token::Integer(v) => u32::try_from(v).ok().filter(|v| *v > 0),
                _ => None,
            }
            .ok_or_else(|| invalid("Invalid object-stream object number"))?;
            let offset = match lexer.next_token()? {
                Token::Integer(v) => usize::try_from(v).ok(),
                _ => None,
            }
            .and_then(|v| first.checked_add(v))
            .filter(|v| *v < data.len())
            .ok_or_else(|| invalid("Invalid object-stream offset"))?;
            offsets.push((number, offset));
        }
        // Use physical boundaries even if header entries are not ordered.
        offsets.sort_unstable_by_key(|(_, offset)| *offset);
        let mut ranges = HashMap::new();
        for (index, &(number, start)) in offsets.iter().enumerate() {
            let end = offsets
                .get(index + 1)
                .map_or(data.len(), |(_, offset)| *offset);
            if start >= end || ranges.insert(number, start..end).is_some() {
                return Err(invalid("Duplicate object-stream object or offset"));
            }
        }
        Ok(Self { data, ranges })
    }

    pub(super) fn decoded_len(&self) -> usize {
        self.data.len()
    }

    fn bytes(&self, number: u32) -> ParseResult<&[u8]> {
        let range = self
            .ranges
            .get(&number)
            .ok_or_else(|| invalid("Object not found in object stream"))?;
        Ok(&self.data[range.clone()])
    }

    pub(super) fn object(&self, number: u32, options: &ParseOptions) -> ParseResult<PdfObject> {
        let mut cursor = Cursor::new(self.bytes(number)?);
        let mut lexer = Lexer::new_with_options(&mut cursor, options.clone());
        PdfObject::parse_with_options(&mut lexer, options)
    }

    pub(super) fn sparse_array(
        &self,
        number: u32,
        options: &ParseOptions,
    ) -> ParseResult<HashMap<usize, PdfObject>> {
        let mut cursor = Cursor::new(self.bytes(number)?);
        let mut lexer = Lexer::new_with_options(&mut cursor, options.clone());
        PdfObject::parse_sparse_array_with_options(&mut lexer, options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::objects::PdfDictionary;

    fn stream(bodies: &[&str]) -> PdfStream {
        let mut header = String::new();
        let mut data = String::new();
        for (index, body) in bodies.iter().enumerate() {
            header.push_str(&format!("{} {} ", index + 1, data.len()));
            data.push_str(body);
            data.push('\n');
        }
        let mut dict = PdfDictionary::new();
        dict.insert("N".to_string(), PdfObject::Integer(bodies.len() as i64));
        dict.insert("First".to_string(), PdfObject::Integer(header.len() as i64));
        PdfStream {
            dict,
            data: format!("{header}{data}").into_bytes(),
        }
    }

    #[test]
    fn sparse_indices_match_dense_objects_without_null_storage() {
        let options = ParseOptions::default();
        let indexed = IndexedObjectStream::parse(
            &stream(&[
                "[null % padding\n 4 0 R null << /ActualText (x) >> null]",
                "false",
            ]),
            &options,
        )
        .unwrap();
        let sparse = indexed.sparse_array(1, &options).unwrap();
        let dense = indexed.object(1, &options).unwrap();
        let array = dense.as_array().unwrap();
        assert_eq!(sparse.len(), 2);
        assert_eq!(sparse[&1], PdfObject::Reference(4, 0));
        assert_eq!(sparse[&3], array.0[3]);
        assert!(!sparse.contains_key(&0));
        assert!(!sparse.contains_key(&2));
        assert!(!sparse.contains_key(&4));
        assert_eq!(
            indexed.object(2, &options).unwrap(),
            PdfObject::Boolean(false)
        );
    }

    #[test]
    fn large_null_prefix_stores_only_the_live_owner() {
        let input = format!("[{}7 0 R]", "null ".repeat(150_000));
        let options = ParseOptions::default();
        let indexed = IndexedObjectStream::parse(&stream(&[&input, "[broken"]), &options).unwrap();
        let sparse = indexed.sparse_array(1, &options).unwrap();
        assert_eq!(sparse.len(), 1);
        assert_eq!(sparse[&150_000], PdfObject::Reference(7, 0));
        assert!(indexed.object(2, &options).is_err());
        assert!(indexed.object(3, &options).is_err());
    }

    #[test]
    fn malformed_header_and_cross_object_array_are_rejected() {
        let options = ParseOptions::default();
        for (key, value) in [
            ("N", -1),
            ("N", i64::MAX),
            ("First", -1),
            ("First", i64::MAX),
        ] {
            let mut source = stream(&["true"]);
            source
                .dict
                .insert(key.to_string(), PdfObject::Integer(value));
            assert!(
                IndexedObjectStream::parse(&source, &options).is_err(),
                "{key}={value}"
            );
        }
        let source = stream(&["[null", "true]"]);
        let indexed = IndexedObjectStream::parse(&source, &options).unwrap();
        assert!(indexed.sparse_array(1, &options).is_err());
        assert!(indexed.sparse_array(2, &options).is_err());
        for header in ["1 0 1 5 ", "1 0 2 0 ", "1 0 2 9 ", "1 0 2 -1"] {
            let mut source = stream(&["true", "false"]);
            source
                .dict
                .insert("First".to_string(), PdfObject::Integer(header.len() as i64));
            source.data = format!("{header}true false").into_bytes();
            // Offset 9 points at the final 'e', still within the byte range.
            // It must fail when that object is requested, not index creation.
            if header == "1 0 2 9 " {
                let indexed = IndexedObjectStream::parse(&source, &options).unwrap();
                assert!(indexed.object(2, &options).is_err());
            } else {
                assert!(
                    IndexedObjectStream::parse(&source, &options).is_err(),
                    "{header}"
                );
            }
        }
    }
}
