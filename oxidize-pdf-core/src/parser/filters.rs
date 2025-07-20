//! PDF Stream Filters
//!
//! Handles decompression and decoding of PDF streams according to ISO 32000-1 Section 7.4

use super::objects::{PdfDictionary, PdfObject};
use super::{ParseError, ParseResult};

#[cfg(feature = "compression")]
use flate2::read::ZlibDecoder;
use std::io::Read;

/// Supported PDF filters
#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    /// ASCII hex decode
    ASCIIHexDecode,

    /// ASCII 85 decode
    ASCII85Decode,

    /// LZW decode
    LZWDecode,

    /// Flate decode (zlib/deflate compression)
    FlateDecode,

    /// Run length decode
    RunLengthDecode,

    /// CCITT fax decode
    CCITTFaxDecode,

    /// JBIG2 decode
    JBIG2Decode,

    /// DCT decode (JPEG)
    DCTDecode,

    /// JPX decode (JPEG 2000)
    JPXDecode,

    /// Crypt filter
    Crypt,
}

impl Filter {
    /// Parse filter from name
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "ASCIIHexDecode" => Some(Filter::ASCIIHexDecode),
            "ASCII85Decode" => Some(Filter::ASCII85Decode),
            "LZWDecode" => Some(Filter::LZWDecode),
            "FlateDecode" => Some(Filter::FlateDecode),
            "RunLengthDecode" => Some(Filter::RunLengthDecode),
            "CCITTFaxDecode" => Some(Filter::CCITTFaxDecode),
            "JBIG2Decode" => Some(Filter::JBIG2Decode),
            "DCTDecode" => Some(Filter::DCTDecode),
            "JPXDecode" => Some(Filter::JPXDecode),
            "Crypt" => Some(Filter::Crypt),
            _ => None,
        }
    }
}

/// Decode stream data according to specified filters
pub fn decode_stream(data: &[u8], dict: &PdfDictionary) -> ParseResult<Vec<u8>> {
    // Get filter(s) from dictionary
    let filters = match dict.get("Filter") {
        Some(PdfObject::Name(name)) => vec![name.as_str()],
        Some(PdfObject::Array(array)) => {
            let mut filter_names = Vec::new();
            for obj in &array.0 {
                if let PdfObject::Name(name) = obj {
                    filter_names.push(name.as_str());
                } else {
                    return Err(ParseError::SyntaxError {
                        position: 0,
                        message: "Invalid filter in array".to_string(),
                    });
                }
            }
            filter_names
        }
        None => {
            // No filter, return data as-is
            return Ok(data.to_vec());
        }
        _ => {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "Invalid Filter type".to_string(),
            });
        }
    };

    // Apply filters in order
    let mut result = data.to_vec();
    for filter_name in filters {
        let filter = Filter::from_name(filter_name).ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: format!("Unknown filter: {filter_name}"),
        })?;

        result = apply_filter(&result, filter)?;
    }

    Ok(result)
}

/// Apply a single filter to data
fn apply_filter(data: &[u8], filter: Filter) -> ParseResult<Vec<u8>> {
    match filter {
        Filter::FlateDecode => decode_flate(data),
        Filter::ASCIIHexDecode => decode_ascii_hex(data),
        Filter::ASCII85Decode => decode_ascii85(data),
        _ => Err(ParseError::SyntaxError {
            position: 0,
            message: format!("Filter {filter:?} not yet implemented"),
        }),
    }
}

/// Decode FlateDecode (zlib/deflate) compressed data
#[cfg(feature = "compression")]
fn decode_flate(data: &[u8]) -> ParseResult<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut result = Vec::new();
    decoder
        .read_to_end(&mut result)
        .map_err(|e| ParseError::StreamDecodeError(format!("Flate decode error: {e}")))?;
    Ok(result)
}

#[cfg(not(feature = "compression"))]
fn decode_flate(_data: &[u8]) -> ParseResult<Vec<u8>> {
    Err(ParseError::StreamDecodeError(
        "FlateDecode requires 'compression' feature".to_string(),
    ))
}

/// Decode ASCIIHexDecode data
fn decode_ascii_hex(data: &[u8]) -> ParseResult<Vec<u8>> {
    let mut result = Vec::new();
    let mut chars = data.iter().filter(|&&b| !b.is_ascii_whitespace());

    loop {
        let high = match chars.next() {
            Some(&b'>') => break, // End marker
            Some(&ch) => ch,
            None => break,
        };

        let low = match chars.next() {
            Some(&b'>') => {
                // Odd number of digits, pad with 0
                b'0'
            }
            Some(&ch) => ch,
            None => b'0', // Pad with 0
        };

        let high_val = hex_digit_value(high).ok_or_else(|| {
            ParseError::StreamDecodeError(format!("Invalid hex digit: {}", high as char))
        })?;
        let low_val = hex_digit_value(low).ok_or_else(|| {
            ParseError::StreamDecodeError(format!("Invalid hex digit: {}", low as char))
        })?;

        result.push((high_val << 4) | low_val);

        if low == b'>' {
            break;
        }
    }

    Ok(result)
}

/// Get value of hex digit
fn hex_digit_value(ch: u8) -> Option<u8> {
    match ch {
        b'0'..=b'9' => Some(ch - b'0'),
        b'A'..=b'F' => Some(ch - b'A' + 10),
        b'a'..=b'f' => Some(ch - b'a' + 10),
        _ => None,
    }
}

/// Decode ASCII85Decode data
fn decode_ascii85(data: &[u8]) -> ParseResult<Vec<u8>> {
    let mut result = Vec::new();
    let mut chars = data.iter().filter(|&&b| !b.is_ascii_whitespace());
    let mut group = Vec::with_capacity(5);

    // Skip optional <~ prefix
    let mut ch = match chars.next() {
        Some(&b'<') => {
            if chars.next() == Some(&b'~') {
                // Skip the prefix and get next char
                chars.next()
            } else {
                // Not a valid prefix, treat '<' as data
                Some(&b'<')
            }
        }
        other => other,
    };

    while let Some(&c) = ch {
        match c {
            b'~' => {
                // Check for end marker ~>
                if chars.next() == Some(&b'>') {
                    break;
                } else {
                    return Err(ParseError::StreamDecodeError(
                        "Invalid ASCII85 end marker".to_string(),
                    ));
                }
            }
            b'z' if group.is_empty() => {
                // Special case: 'z' represents four zero bytes
                result.extend_from_slice(&[0, 0, 0, 0]);
            }
            b'!'..=b'u' => {
                group.push(c);
                if group.len() == 5 {
                    // Decode complete group
                    let value = group
                        .iter()
                        .enumerate()
                        .map(|(i, &ch)| (ch - b'!') as u32 * 85u32.pow(4 - i as u32))
                        .sum::<u32>();

                    result.push((value >> 24) as u8);
                    result.push((value >> 16) as u8);
                    result.push((value >> 8) as u8);
                    result.push(value as u8);

                    group.clear();
                }
            }
            _ => {
                return Err(ParseError::StreamDecodeError(format!(
                    "Invalid ASCII85 character: {}",
                    c as char
                )));
            }
        }
        ch = chars.next();
    }

    // Handle incomplete final group
    if !group.is_empty() {
        // Save original length to know how many bytes to output
        let original_len = group.len();

        // Pad with 'u' (84)
        while group.len() < 5 {
            group.push(b'u');
        }

        let value = group
            .iter()
            .enumerate()
            .map(|(i, &ch)| (ch - b'!') as u32 * 85u32.pow(4 - i as u32))
            .sum::<u32>();

        // Only output the number of bytes that were actually encoded
        let output_bytes = original_len - 1;
        for i in 0..output_bytes {
            result.push((value >> (24 - 8 * i)) as u8);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::objects::{PdfArray, PdfDictionary, PdfName, PdfObject};

    #[test]
    fn test_ascii_hex_decode() {
        let data = b"48656C6C6F>";
        let result = decode_ascii_hex(data).unwrap();
        assert_eq!(result, b"Hello");

        let data = b"48 65 6C 6C 6F>"; // With spaces
        let result = decode_ascii_hex(data).unwrap();
        assert_eq!(result, b"Hello");

        let data = b"48656C6C6>"; // Odd number of digits
        let result = decode_ascii_hex(data).unwrap();
        assert_eq!(result, b"Hell`");
    }

    #[test]
    fn test_ascii85_decode() {
        let data = b"87cURD]j7BEbo80~>";
        let result = decode_ascii85(data).unwrap();
        assert_eq!(result, b"Hello world!");

        let data = b"z~>"; // Special case for zeros
        let result = decode_ascii85(data).unwrap();
        assert_eq!(result, &[0, 0, 0, 0]);
    }

    #[test]
    fn test_filter_from_name() {
        assert_eq!(
            Filter::from_name("ASCIIHexDecode"),
            Some(Filter::ASCIIHexDecode)
        );
        assert_eq!(
            Filter::from_name("ASCII85Decode"),
            Some(Filter::ASCII85Decode)
        );
        assert_eq!(Filter::from_name("LZWDecode"), Some(Filter::LZWDecode));
        assert_eq!(Filter::from_name("FlateDecode"), Some(Filter::FlateDecode));
        assert_eq!(
            Filter::from_name("RunLengthDecode"),
            Some(Filter::RunLengthDecode)
        );
        assert_eq!(
            Filter::from_name("CCITTFaxDecode"),
            Some(Filter::CCITTFaxDecode)
        );
        assert_eq!(Filter::from_name("JBIG2Decode"), Some(Filter::JBIG2Decode));
        assert_eq!(Filter::from_name("DCTDecode"), Some(Filter::DCTDecode));
        assert_eq!(Filter::from_name("JPXDecode"), Some(Filter::JPXDecode));
        assert_eq!(Filter::from_name("Crypt"), Some(Filter::Crypt));
        assert_eq!(Filter::from_name("UnknownFilter"), None);
    }

    #[test]
    fn test_filter_equality() {
        assert_eq!(Filter::ASCIIHexDecode, Filter::ASCIIHexDecode);
        assert_ne!(Filter::ASCIIHexDecode, Filter::ASCII85Decode);
        assert_ne!(Filter::FlateDecode, Filter::LZWDecode);
    }

    #[test]
    fn test_filter_clone() {
        let filter = Filter::FlateDecode;
        let cloned = filter.clone();
        assert_eq!(filter, cloned);
    }

    #[test]
    fn test_decode_stream_no_filter() {
        let data = b"Hello, world!";
        let dict = PdfDictionary::new();

        let result = decode_stream(data, &dict).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_decode_stream_single_filter() {
        let data = b"48656C6C6F>";
        let mut dict = PdfDictionary::new();
        dict.insert(
            "Filter".to_string(),
            PdfObject::Name(PdfName("ASCIIHexDecode".to_string())),
        );

        let result = decode_stream(data, &dict).unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_decode_stream_invalid_filter() {
        let data = b"test data";
        let mut dict = PdfDictionary::new();
        dict.insert(
            "Filter".to_string(),
            PdfObject::Name(PdfName("UnknownFilter".to_string())),
        );

        let result = decode_stream(data, &dict);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_stream_filter_array() {
        let data = b"48656C6C6F>";
        let mut dict = PdfDictionary::new();
        let filters = vec![PdfObject::Name(PdfName("ASCIIHexDecode".to_string()))];
        dict.insert("Filter".to_string(), PdfObject::Array(PdfArray(filters)));

        let result = decode_stream(data, &dict).unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_decode_stream_invalid_filter_type() {
        let data = b"test data";
        let mut dict = PdfDictionary::new();
        dict.insert("Filter".to_string(), PdfObject::Integer(42)); // Invalid type

        let result = decode_stream(data, &dict);
        assert!(result.is_err());
    }

    #[test]
    fn test_ascii_hex_decode_empty() {
        let data = b">";
        let result = decode_ascii_hex(data).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_ascii_hex_decode_invalid() {
        let data = b"GG>"; // Invalid hex
        let result = decode_ascii_hex(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_ascii_hex_decode_no_terminator() {
        let data = b"48656C6C6F"; // Missing '>'
        let result = decode_ascii_hex(data).unwrap();
        assert_eq!(result, b"Hello"); // Should work without terminator
    }

    #[test]
    fn test_ascii85_decode_empty() {
        let data = b"~>";
        let result = decode_ascii85(data).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_ascii85_decode_invalid() {
        let data = b"invalid~>";
        let result = decode_ascii85(data);
        assert!(result.is_err());
    }

    #[cfg(feature = "compression")]
    #[test]
    fn test_flate_decode() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let original = b"Hello, compressed world!";
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let result = decode_flate(&compressed).unwrap();
        assert_eq!(result, original);
    }

    #[cfg(not(feature = "compression"))]
    #[test]
    fn test_flate_decode_not_supported() {
        let data = b"compressed data";
        let result = decode_flate(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_filter() {
        let data = b"48656C6C6F>";
        let result = apply_filter(data, Filter::ASCIIHexDecode).unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_apply_filter_unsupported() {
        let data = b"test data";
        let unsupported_filters = vec![
            Filter::LZWDecode,
            Filter::RunLengthDecode,
            Filter::CCITTFaxDecode,
            Filter::JBIG2Decode,
            Filter::DCTDecode,
            Filter::JPXDecode,
            Filter::Crypt,
        ];

        for filter in unsupported_filters {
            let result = apply_filter(data, filter);
            assert!(result.is_err());
        }
    }
}
