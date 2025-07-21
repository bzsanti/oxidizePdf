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

    // Get decode parameters
    let decode_params = dict.get("DecodeParms");

    // Apply filters in order
    let mut result = data.to_vec();
    for (i, filter_name) in filters.iter().enumerate() {
        let filter = Filter::from_name(filter_name).ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: format!("Unknown filter: {filter_name}"),
        })?;

        // Get decode parameters for this filter
        let filter_params = get_filter_params(decode_params, i);

        result = apply_filter_with_params(&result, filter, filter_params)?;
    }

    Ok(result)
}

/// Apply a single filter to data (legacy function, use apply_filter_with_params)
#[allow(dead_code)]
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

    // PNG Predictor Tests for Compressed XRef Streams

    #[test]
    fn test_apply_filter_with_params_no_predictor() {
        let data = b"48656C6C6F>";
        let dict = PdfDictionary::new();

        let result = apply_filter_with_params(data, Filter::ASCIIHexDecode, Some(&dict)).unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_apply_predictor_none() {
        let data = vec![1, 2, 3, 4];
        let dict = PdfDictionary::new();

        let result = apply_predictor(&data, 1, &dict).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_apply_predictor_unknown() {
        let data = vec![1, 2, 3, 4];
        let dict = PdfDictionary::new();

        // Unknown predictor should return data as-is
        let result = apply_predictor(&data, 99, &dict).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_png_predictor_sub_filter() {
        // Test PNG Sub filter (predictor 1)
        let data = vec![1, 5, 10]; // bytes_per_pixel = 1
        let result = apply_png_sub_filter(&data, 1);
        assert_eq!(result, vec![1, 6, 16]); // 1, 1+5=6, 5+10=15->16 (wrapping)
    }

    #[test]
    fn test_png_predictor_up_filter() {
        // Test PNG Up filter (predictor 2)
        let data = vec![1, 2, 3];
        let prev_row = vec![5, 10, 15];
        let result = apply_png_up_filter(&data, Some(&prev_row));
        assert_eq!(result, vec![6, 12, 18]); // 1+5=6, 2+10=12, 3+15=18
    }

    #[test]
    fn test_png_predictor_up_filter_no_prev() {
        // Test PNG Up filter with no previous row
        let data = vec![1, 2, 3];
        let result = apply_png_up_filter(&data, None);
        assert_eq!(result, vec![1, 2, 3]); // No change when no previous row
    }

    #[test]
    fn test_png_predictor_average_filter() {
        // Test PNG Average filter (predictor 3)
        let data = vec![2, 4]; // bytes_per_pixel = 1
        let prev_row = vec![6, 8];
        let result = apply_png_average_filter(&data, Some(&prev_row), 1);
        // First byte: left=0, up=6, avg=3, result=2+3=5
        // Second byte: left=5, up=8, avg=6, result=4+6=10
        assert_eq!(result, vec![5, 10]);
    }

    #[test]
    fn test_png_predictor_paeth_filter() {
        // Test PNG Paeth filter (predictor 4)
        let data = vec![1, 2]; // bytes_per_pixel = 1
        let prev_row = vec![3, 4];
        let result = apply_png_paeth_filter(&data, Some(&prev_row), 1);
        // Complex Paeth predictor calculation
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_paeth_predictor_algorithm() {
        // Test the Paeth predictor algorithm directly
        // For (1, 2, 0): p = 1 + 2 - 0 = 3; pa = |3-1| = 2, pb = |3-2| = 1, pc = |3-0| = 3
        // pb <= pa and pb <= pc, so result is up = 2
        assert_eq!(paeth_predictor(1, 2, 0), 2);

        // For (5, 2, 3): p = 5 + 2 - 3 = 4; pa = |4-5| = 1, pb = |4-2| = 2, pc = |4-3| = 1
        // pa <= pb and pa <= pc (tie with pc), so result is left = 5
        assert_eq!(paeth_predictor(5, 2, 3), 5);

        // For (5, 8, 3): p = 5 + 8 - 3 = 10; pa = |10-5| = 5, pb = |10-8| = 2, pc = |10-3| = 7
        // pb <= pa and pb <= pc, so result is up = 8
        assert_eq!(paeth_predictor(5, 8, 3), 8);
    }

    #[test]
    fn test_apply_png_predictor_invalid_data() {
        let mut params = PdfDictionary::new();
        params.insert("Columns".to_string(), PdfObject::Integer(3));

        // Data length not multiple of row size (3+1=4)
        let data = vec![0, 1, 2, 3, 4, 5]; // 6 bytes, not multiple of 4
        let result = apply_png_predictor(&data, 10, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_png_predictor_valid_simple() {
        let mut params = PdfDictionary::new();
        params.insert("Columns".to_string(), PdfObject::Integer(2));
        params.insert("BitsPerComponent".to_string(), PdfObject::Integer(8));
        params.insert("Colors".to_string(), PdfObject::Integer(1));

        // Row size = 2 columns + 1 predictor byte = 3
        let data = vec![
            0, 1, 2, // Row 1: predictor=0 (None), data=[1,2]
            0, 3, 4, // Row 2: predictor=0 (None), data=[3,4]
        ];

        let result = apply_png_predictor(&data, 10, &params).unwrap();
        assert_eq!(result, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_apply_png_predictor_with_sub_filter() {
        let mut params = PdfDictionary::new();
        params.insert("Columns".to_string(), PdfObject::Integer(3));
        params.insert("BitsPerComponent".to_string(), PdfObject::Integer(8));
        params.insert("Colors".to_string(), PdfObject::Integer(1));

        // Row size = 3 columns + 1 predictor byte = 4
        let data = vec![
            1, 1, 2, 3, // Row 1: predictor=1 (Sub), data=[1,2,3] -> [1,3,6]
        ];

        let result = apply_png_predictor(&data, 10, &params).unwrap();
        assert_eq!(result, vec![1, 3, 6]); // Sub filter: 1, 1+2=3, 2+3=5->6
    }

    #[test]
    fn test_apply_png_predictor_invalid_filter_type() {
        let mut params = PdfDictionary::new();
        params.insert("Columns".to_string(), PdfObject::Integer(2));

        // Invalid predictor byte (5 is not defined)
        let data = vec![5, 1, 2];
        let result = apply_png_predictor(&data, 10, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_filter_params_dict() {
        let mut dict = PdfDictionary::new();
        dict.insert("Predictor".to_string(), PdfObject::Integer(12));
        let obj = PdfObject::Dictionary(dict);

        let result = get_filter_params(Some(&obj), 0);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().get("Predictor"),
            Some(&PdfObject::Integer(12))
        );
    }

    #[test]
    fn test_get_filter_params_array() {
        let mut inner_dict = PdfDictionary::new();
        inner_dict.insert("Predictor".to_string(), PdfObject::Integer(15));

        let array = vec![PdfObject::Dictionary(inner_dict)];
        let obj = PdfObject::Array(crate::parser::objects::PdfArray(array));

        let result = get_filter_params(Some(&obj), 0);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().get("Predictor"),
            Some(&PdfObject::Integer(15))
        );
    }

    #[test]
    fn test_get_filter_params_none() {
        let result = get_filter_params(None, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_compressed_xref_integration() {
        // Integration test: FlateDecode + PNG Predictor
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        #[cfg(feature = "compression")]
        {
            // Create test data with PNG predictor applied
            let original_data = vec![
                0, 1, 2, // Row 1: predictor=0 (None), data=[1,2]
                0, 3, 4, // Row 2: predictor=0 (None), data=[3,4]
            ];

            // Compress the data
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&original_data).unwrap();
            let compressed = encoder.finish().unwrap();

            // Create decode parameters
            let mut decode_params = PdfDictionary::new();
            decode_params.insert("Predictor".to_string(), PdfObject::Integer(12)); // PNG Optimum
            decode_params.insert("Columns".to_string(), PdfObject::Integer(2));
            decode_params.insert("BitsPerComponent".to_string(), PdfObject::Integer(8));
            decode_params.insert("Colors".to_string(), PdfObject::Integer(1));

            // Apply filter with parameters
            let result =
                apply_filter_with_params(&compressed, Filter::FlateDecode, Some(&decode_params))
                    .unwrap();
            assert_eq!(result, vec![1, 2, 3, 4]);
        }
    }
}

/// Apply a single filter to data with parameters (enhanced version)
fn apply_filter_with_params(
    data: &[u8],
    filter: Filter,
    params: Option<&PdfDictionary>,
) -> ParseResult<Vec<u8>> {
    let result = match filter {
        Filter::FlateDecode => decode_flate(data)?,
        Filter::ASCIIHexDecode => decode_ascii_hex(data)?,
        Filter::ASCII85Decode => decode_ascii85(data)?,
        _ => {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: format!("Filter {filter:?} not yet implemented"),
            });
        }
    };

    // Apply predictor if specified in decode parameters
    if let Some(params_dict) = params {
        if let Some(predictor_obj) = params_dict.get("Predictor") {
            if let Some(predictor) = predictor_obj.as_integer() {
                return apply_predictor(&result, predictor as u32, params_dict);
            }
        }
    }

    Ok(result)
}

/// Get filter parameters for a specific filter index
fn get_filter_params(decode_params: Option<&PdfObject>, _index: usize) -> Option<&PdfDictionary> {
    match decode_params {
        Some(PdfObject::Dictionary(dict)) => Some(dict),
        Some(PdfObject::Array(array)) => {
            // For multiple filters, each can have its own decode params
            // For now, use the first one
            array.0.first().and_then(|obj| obj.as_dict())
        }
        _ => None,
    }
}

/// Apply predictor function to decoded data
fn apply_predictor(data: &[u8], predictor: u32, params: &PdfDictionary) -> ParseResult<Vec<u8>> {
    match predictor {
        1 => {
            // No prediction
            Ok(data.to_vec())
        }
        10..=15 => {
            // PNG predictor functions
            apply_png_predictor(data, predictor, params)
        }
        _ => {
            // Unknown predictor - return data as-is with warning
            #[cfg(debug_assertions)]
            eprintln!(
                "Warning: Unknown predictor {}, returning data as-is",
                predictor
            );
            Ok(data.to_vec())
        }
    }
}

/// Apply PNG predictor functions (values 10-15)
fn apply_png_predictor(
    data: &[u8],
    _predictor: u32,
    params: &PdfDictionary,
) -> ParseResult<Vec<u8>> {
    // Get columns (width of a row in bytes)
    let columns = params
        .get("Columns")
        .and_then(|obj| obj.as_integer())
        .unwrap_or(1) as usize;

    // Get BitsPerComponent (defaults to 8)
    let bpc = params
        .get("BitsPerComponent")
        .and_then(|obj| obj.as_integer())
        .unwrap_or(8) as usize;

    // Get Colors (number of color components, defaults to 1)
    let colors = params
        .get("Colors")
        .and_then(|obj| obj.as_integer())
        .unwrap_or(1) as usize;

    // Calculate bytes per pixel
    let bytes_per_pixel = (bpc * colors).div_ceil(8);

    // Calculate row size (columns + 1 for predictor byte)
    let row_size = columns + 1;

    if data.len() % row_size != 0 {
        return Err(ParseError::StreamDecodeError(
            "PNG predictor: data length not multiple of row size".to_string(),
        ));
    }

    let num_rows = data.len() / row_size;
    let mut result = Vec::with_capacity(columns * num_rows);

    for row in 0..num_rows {
        let row_start = row * row_size;
        let predictor_byte = data[row_start];
        let row_data = &data[row_start + 1..row_start + row_size];

        // Apply PNG filter based on predictor byte
        let filtered_row = match predictor_byte {
            0 => {
                // None filter - no prediction
                row_data.to_vec()
            }
            1 => {
                // Sub filter - each byte is prediction from byte to the left
                apply_png_sub_filter(row_data, bytes_per_pixel)
            }
            2 => {
                // Up filter - each byte is prediction from byte above
                let prev_row = if row > 0 {
                    Some(&result[(row - 1) * columns..row * columns])
                } else {
                    None
                };
                apply_png_up_filter(row_data, prev_row)
            }
            3 => {
                // Average filter
                let prev_row = if row > 0 {
                    Some(&result[(row - 1) * columns..row * columns])
                } else {
                    None
                };
                apply_png_average_filter(row_data, prev_row, bytes_per_pixel)
            }
            4 => {
                // Paeth filter
                let prev_row = if row > 0 {
                    Some(&result[(row - 1) * columns..row * columns])
                } else {
                    None
                };
                apply_png_paeth_filter(row_data, prev_row, bytes_per_pixel)
            }
            _ => {
                return Err(ParseError::StreamDecodeError(format!(
                    "PNG predictor: unknown filter type {}",
                    predictor_byte
                )));
            }
        };

        result.extend_from_slice(&filtered_row);
    }

    Ok(result)
}

/// Apply PNG Sub filter (predictor 1)
fn apply_png_sub_filter(data: &[u8], bytes_per_pixel: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());

    for (i, &byte) in data.iter().enumerate() {
        if i < bytes_per_pixel {
            result.push(byte);
        } else {
            result.push(byte.wrapping_add(result[i - bytes_per_pixel]));
        }
    }

    result
}

/// Apply PNG Up filter (predictor 2)
fn apply_png_up_filter(data: &[u8], prev_row: Option<&[u8]>) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());

    for (i, &byte) in data.iter().enumerate() {
        let up_byte = prev_row.and_then(|row| row.get(i)).unwrap_or(&0);
        result.push(byte.wrapping_add(*up_byte));
    }

    result
}

/// Apply PNG Average filter (predictor 3)
fn apply_png_average_filter(
    data: &[u8],
    prev_row: Option<&[u8]>,
    bytes_per_pixel: usize,
) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());

    for (i, &byte) in data.iter().enumerate() {
        let left_byte = if i < bytes_per_pixel {
            0
        } else {
            result[i - bytes_per_pixel]
        };
        let up_byte = prev_row.and_then(|row| row.get(i)).unwrap_or(&0);
        let average = ((left_byte as u16 + *up_byte as u16) / 2) as u8;
        result.push(byte.wrapping_add(average));
    }

    result
}

/// Apply PNG Paeth filter (predictor 4)
fn apply_png_paeth_filter(data: &[u8], prev_row: Option<&[u8]>, bytes_per_pixel: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());

    for (i, &byte) in data.iter().enumerate() {
        let left_byte = if i < bytes_per_pixel {
            0
        } else {
            result[i - bytes_per_pixel]
        };
        let up_byte = prev_row.and_then(|row| row.get(i)).unwrap_or(&0);
        let up_left_byte = if i < bytes_per_pixel {
            0
        } else {
            *prev_row
                .and_then(|row| row.get(i - bytes_per_pixel))
                .unwrap_or(&0)
        };

        let paeth = paeth_predictor(left_byte, *up_byte, up_left_byte);
        result.push(byte.wrapping_add(paeth));
    }

    result
}

/// Paeth predictor algorithm
fn paeth_predictor(left: u8, up: u8, up_left: u8) -> u8 {
    let p = left as i16 + up as i16 - up_left as i16;
    let pa = (p - left as i16).abs();
    let pb = (p - up as i16).abs();
    let pc = (p - up_left as i16).abs();

    if pa <= pb && pa <= pc {
        left
    } else if pb <= pc {
        up
    } else {
        up_left
    }
}
