//! PDF Header Parser
//!
//! Parses PDF header and version according to ISO 32000-1 Section 7.5.2

use super::{ParseError, ParseResult};
use std::io::{BufRead, BufReader, Read};

/// PDF Version information
#[derive(Debug, Clone, PartialEq)]
pub struct PdfVersion {
    pub major: u8,
    pub minor: u8,
}

impl PdfVersion {
    /// Create a new PDF version
    pub fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }

    /// Check if this version is supported
    pub fn is_supported(&self) -> bool {
        // We support PDF 1.0 through 2.0
        matches!((self.major, self.minor), (1, 0..=7) | (2, 0))
    }
}

impl std::fmt::Display for PdfVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// PDF Header information
#[derive(Debug, Clone)]
pub struct PdfHeader {
    pub version: PdfVersion,
    pub has_binary_marker: bool,
}

impl PdfHeader {
    /// Parse PDF header from a reader
    pub fn parse<R: Read>(reader: R) -> ParseResult<Self> {
        let mut buf_reader = BufReader::new(reader);
        let mut header = Self::parse_version_line(&mut buf_reader)?;

        // Check for binary marker (recommended for PDF 1.2+)
        header.has_binary_marker = Self::check_binary_marker(&mut buf_reader)?;

        Ok(header)
    }

    /// Parse the PDF version line
    fn parse_version_line<R: BufRead>(reader: &mut R) -> ParseResult<Self> {
        // Read the first line more flexibly - some PDFs might have non-standard formatting
        let mut line_bytes = Vec::new();
        let mut consecutive_nulls = 0;

        loop {
            let mut byte = [0u8; 1];
            match reader.read_exact(&mut byte) {
                Ok(_) => {
                    // Track consecutive null bytes - if we see too many, likely not a PDF
                    if byte[0] == 0 {
                        consecutive_nulls += 1;
                        if consecutive_nulls > 10 {
                            return Err(ParseError::InvalidHeader);
                        }
                    } else {
                        consecutive_nulls = 0;
                    }

                    if byte[0] == b'\n' || byte[0] == b'\r' {
                        // Handle CRLF more robustly
                        if byte[0] == b'\r' {
                            // Peek at next byte
                            let mut next_byte = [0u8; 1];
                            if reader.read_exact(&mut next_byte).is_ok() && next_byte[0] != b'\n' {
                                // Not CRLF, put back the byte (can't seek, so store it)
                                line_bytes.push(byte[0]);
                            }
                        }
                        break;
                    }
                    line_bytes.push(byte[0]);
                    // Be more generous with line length - some PDFs have longer headers
                    if line_bytes.len() > 200 {
                        return Err(ParseError::InvalidHeader);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    if line_bytes.is_empty() {
                        return Err(ParseError::InvalidHeader);
                    }
                    break;
                }
                Err(e) => return Err(e.into()),
            }
        }

        // Convert to string for parsing - be more lenient with encoding
        let line = String::from_utf8_lossy(&line_bytes).into_owned();

        // Look for PDF header anywhere in the first line (some files have leading garbage)
        let (pdf_start, pdf_prefix_len) = if let Some(pos) = line.find("%PDF-") {
            (pos, 5) // "%PDF-" is 5 characters
        } else {
            // Try case-insensitive match
            let lower_line = line.to_lowercase();
            if let Some(pos) = lower_line.find("%pdf-") {
                (pos, 5) // "%pdf-" is also 5 characters
            } else {
                return Err(ParseError::InvalidHeader);
            }
        };

        // Extract the PDF header part
        let pdf_line = &line[pdf_start..];
        if pdf_line.len() < 7 {
            // Not enough characters for "%PDF-XY" (minimum format)
            return Err(ParseError::InvalidHeader);
        }

        // Extract version (trim any trailing whitespace/newlines)
        let version_part = &pdf_line[pdf_prefix_len..]; // Skip "%PDF-" or "%pdf-"

        // Extract version more flexibly - look for digits and dots up to the first non-version character
        let mut version_chars = String::new();
        for ch in version_part.chars() {
            if ch.is_ascii_digit() || ch == '.' {
                version_chars.push(ch);
            } else if ch.is_whitespace() && !version_chars.is_empty() {
                // Allow whitespace within version, but clean it up
                continue;
            } else if !version_chars.is_empty() {
                // Found non-version character after version started, stop here
                break;
            }
            // Skip leading non-version characters
        }

        let version_str = version_chars.trim();

        // Handle various version formats
        let (major, minor) = if version_str.contains('.') {
            // Standard format with dot: "1.4", "2.0", etc.
            let parts: Vec<&str> = version_str.split('.').collect();
            if parts.len() != 2 {
                return Err(ParseError::InvalidHeader);
            }

            let major = parts[0]
                .trim()
                .parse::<u8>()
                .map_err(|_| ParseError::InvalidHeader)?;
            let minor = parts[1]
                .trim()
                .parse::<u8>()
                .map_err(|_| ParseError::InvalidHeader)?;

            (major, minor)
        } else {
            // Try parsing without dot (some malformed PDFs like "%PDF-14")
            let clean_version = version_str
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>();

            if clean_version.len() >= 2 {
                let major_str = &clean_version[0..1];
                let minor_str = &clean_version[1..2];

                let major = major_str
                    .parse::<u8>()
                    .map_err(|_| ParseError::InvalidHeader)?;
                let minor = minor_str
                    .parse::<u8>()
                    .map_err(|_| ParseError::InvalidHeader)?;

                (major, minor)
            } else {
                return Err(ParseError::InvalidHeader);
            }
        };

        let version = PdfVersion::new(major, minor);

        if !version.is_supported() {
            return Err(ParseError::UnsupportedVersion(version.to_string()));
        }

        Ok(PdfHeader {
            version,
            has_binary_marker: false,
        })
    }

    /// Check for binary marker comment
    fn check_binary_marker<R: BufRead>(reader: &mut R) -> ParseResult<bool> {
        let mut buffer = Vec::new();

        // Read bytes until we find a newline or EOF
        loop {
            let mut byte = [0u8; 1];
            match reader.read_exact(&mut byte) {
                Ok(_) => {
                    buffer.push(byte[0]);
                    if byte[0] == b'\n' || byte[0] == b'\r' {
                        break;
                    }
                    // Limit line length to prevent excessive memory usage
                    if buffer.len() > 1024 {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(e) => return Err(e.into()),
            }
        }

        if buffer.is_empty() {
            return Ok(false);
        }

        // Binary marker should be a comment with at least 4 binary characters
        if buffer.first() == Some(&b'%') {
            let binary_count = buffer
                .iter()
                .skip(1) // Skip the %
                .filter(|&&b| b >= 128)
                .count();

            Ok(binary_count >= 4)
        } else {
            // Not a comment, probably start of document content
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_pdf_header_basic() {
        let input = b"%PDF-1.7\n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert_eq!(header.version.major, 1);
        assert_eq!(header.version.minor, 7);
        assert!(!header.has_binary_marker);
    }

    #[test]
    fn test_parse_pdf_header_with_binary_marker() {
        let input = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert_eq!(header.version.major, 1);
        assert_eq!(header.version.minor, 4);
        assert!(header.has_binary_marker);
    }

    #[test]
    fn test_parse_pdf_20() {
        let input = b"%PDF-2.0\n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert_eq!(header.version.major, 2);
        assert_eq!(header.version.minor, 0);
    }

    #[test]
    fn test_invalid_header() {
        let input = b"Not a PDF\n";
        let result = PdfHeader::parse(Cursor::new(input));

        assert!(matches!(result, Err(ParseError::InvalidHeader)));
    }

    #[test]
    fn test_unsupported_version() {
        let input = b"%PDF-3.0\n";
        let result = PdfHeader::parse(Cursor::new(input));

        assert!(matches!(result, Err(ParseError::UnsupportedVersion(_))));
    }

    #[test]
    fn test_pdf_version_new() {
        let version = PdfVersion::new(1, 5);
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 5);
    }

    #[test]
    fn test_pdf_version_display() {
        let version = PdfVersion::new(1, 7);
        assert_eq!(version.to_string(), "1.7");
        assert_eq!(format!("{version}"), "1.7");
    }

    #[test]
    fn test_pdf_version_is_supported() {
        // Supported versions
        assert!(PdfVersion::new(1, 0).is_supported());
        assert!(PdfVersion::new(1, 1).is_supported());
        assert!(PdfVersion::new(1, 4).is_supported());
        assert!(PdfVersion::new(1, 7).is_supported());
        assert!(PdfVersion::new(2, 0).is_supported());

        // Unsupported versions
        assert!(!PdfVersion::new(0, 9).is_supported());
        assert!(!PdfVersion::new(1, 8).is_supported());
        assert!(!PdfVersion::new(2, 1).is_supported());
        assert!(!PdfVersion::new(3, 0).is_supported());
    }

    #[test]
    fn test_pdf_version_equality() {
        let v1 = PdfVersion::new(1, 5);
        let v2 = PdfVersion::new(1, 5);
        let v3 = PdfVersion::new(1, 6);

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_header_with_crlf() {
        let input = b"%PDF-1.6\r\n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert_eq!(header.version.major, 1);
        assert_eq!(header.version.minor, 6);
    }

    #[test]
    fn test_header_with_cr_only() {
        let input = b"%PDF-1.3\r";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert_eq!(header.version.major, 1);
        assert_eq!(header.version.minor, 3);
    }

    #[test]
    fn test_header_with_extra_whitespace() {
        let input = b"%PDF-1.5   \n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert_eq!(header.version.major, 1);
        assert_eq!(header.version.minor, 5);
    }

    #[test]
    fn test_header_no_newline() {
        let input = b"%PDF-1.2";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert_eq!(header.version.major, 1);
        assert_eq!(header.version.minor, 2);
    }

    #[test]
    fn test_malformed_version_single_digit() {
        let input = b"%PDF-1\n";
        let result = PdfHeader::parse(Cursor::new(input));

        assert!(matches!(result, Err(ParseError::InvalidHeader)));
    }

    #[test]
    fn test_malformed_version_too_many_parts() {
        let input = b"%PDF-1.4.2\n";
        let result = PdfHeader::parse(Cursor::new(input));

        assert!(matches!(result, Err(ParseError::InvalidHeader)));
    }

    #[test]
    fn test_malformed_version_non_numeric() {
        let input = b"%PDF-1.x\n";
        let result = PdfHeader::parse(Cursor::new(input));

        assert!(matches!(result, Err(ParseError::InvalidHeader)));
    }

    #[test]
    fn test_empty_input() {
        let input = b"";
        let result = PdfHeader::parse(Cursor::new(input));

        assert!(matches!(result, Err(ParseError::InvalidHeader)));
    }

    #[test]
    fn test_header_too_long() {
        // Create a header line that's over 100 characters
        let long_header = format!("%PDF-1.0{}", "x".repeat(200));
        let result = PdfHeader::parse(Cursor::new(long_header.as_bytes()));

        assert!(matches!(result, Err(ParseError::InvalidHeader)));
    }

    #[test]
    fn test_binary_marker_insufficient_bytes() {
        let input = b"%PDF-1.4\n%\xE2\xE3\n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert!(!header.has_binary_marker); // Only 2 binary bytes, need 4+
    }

    #[test]
    fn test_binary_marker_exact_threshold() {
        let input = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert!(header.has_binary_marker); // Exactly 4 binary bytes
    }

    #[test]
    fn test_binary_marker_more_than_threshold() {
        let input = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\x80\x81\n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert!(header.has_binary_marker); // More than 4 binary bytes
    }

    #[test]
    fn test_binary_marker_no_comment() {
        let input = b"%PDF-1.4\n1 0 obj\n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert!(!header.has_binary_marker); // No % comment
    }

    #[test]
    fn test_binary_marker_ascii_only() {
        let input = b"%PDF-1.4\n%This is a comment\n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert!(!header.has_binary_marker); // ASCII only comment
    }

    #[test]
    fn test_binary_marker_mixed_content() {
        let input = b"%PDF-1.4\n%Some text \xE2\xE3\xCF\xD3 more text\n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert!(header.has_binary_marker); // Mixed content with sufficient binary
    }

    #[test]
    fn test_binary_marker_very_long_line() {
        let mut long_line = b"%PDF-1.4\n%".to_vec();
        // Add enough binary characters to exceed the limit
        for _ in 0..2000 {
            long_line.push(0x80);
        }
        long_line.push(b'\n');

        let header = PdfHeader::parse(Cursor::new(long_line)).unwrap();

        assert!(header.has_binary_marker); // Should still detect binary marker
    }

    #[test]
    fn test_version_all_supported_ranges() {
        let supported_versions = vec![
            (1, 0),
            (1, 1),
            (1, 2),
            (1, 3),
            (1, 4),
            (1, 5),
            (1, 6),
            (1, 7),
            (2, 0),
        ];

        for (major, minor) in supported_versions {
            let input = format!("%PDF-{major}.{minor}\n");
            let header = PdfHeader::parse(Cursor::new(input.as_bytes())).unwrap();

            assert_eq!(header.version.major, major);
            assert_eq!(header.version.minor, minor);
            assert!(header.version.is_supported());
        }
    }

    #[test]
    fn test_clone_and_debug() {
        let version = PdfVersion::new(1, 4);
        let cloned_version = version.clone();

        assert_eq!(version, cloned_version);
        assert_eq!(format!("{version:?}"), "PdfVersion { major: 1, minor: 4 }");

        let header = PdfHeader {
            version: version.clone(),
            has_binary_marker: true,
        };
        let cloned_header = header.clone();

        assert_eq!(header.version, cloned_header.version);
        assert_eq!(header.has_binary_marker, cloned_header.has_binary_marker);
    }

    // Enhanced flexibility tests for improved header validation

    #[test]
    fn test_header_with_leading_garbage() {
        let input = b"junk%PDF-1.4\n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert_eq!(header.version.major, 1);
        assert_eq!(header.version.minor, 4);
    }

    #[test]
    fn test_header_case_insensitive() {
        let input = b"%pdf-1.5\n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert_eq!(header.version.major, 1);
        assert_eq!(header.version.minor, 5);
    }

    #[test]
    fn test_header_version_without_dot() {
        let input = b"%PDF-14\n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert_eq!(header.version.major, 1);
        assert_eq!(header.version.minor, 4);
    }

    #[test]
    fn test_header_longer_line_limit() {
        // Create a header line that's longer than the old 100 char limit but under 200
        let mut long_header = b"%PDF-1.7".to_vec();
        long_header.extend(vec![b' '; 150]); // Add 150 spaces
        long_header.push(b'\n');

        let header = PdfHeader::parse(Cursor::new(long_header)).unwrap();
        assert_eq!(header.version.major, 1);
        assert_eq!(header.version.minor, 7);
    }

    #[test]
    fn test_header_with_multiple_spaces() {
        let input = b"%PDF-  1  .  7  \n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert_eq!(header.version.major, 1);
        assert_eq!(header.version.minor, 7);
    }

    #[test]
    fn test_header_null_byte_protection() {
        // A few null bytes should be tolerated
        let input = b"\0\0%PDF-1.6\n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert_eq!(header.version.major, 1);
        assert_eq!(header.version.minor, 6);
    }

    #[test]
    fn test_header_too_many_nulls() {
        // Too many consecutive null bytes should fail
        let mut input = vec![0u8; 15]; // 15 null bytes
        input.extend_from_slice(b"%PDF-1.4\n");

        let result = PdfHeader::parse(Cursor::new(input));
        assert!(matches!(result, Err(ParseError::InvalidHeader)));
    }

    #[test]
    fn test_header_minimal_length() {
        let input = b"%PDF-1.0";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert_eq!(header.version.major, 1);
        assert_eq!(header.version.minor, 0);
    }

    #[test]
    fn test_header_too_short() {
        let input = b"%PDF-1";
        let result = PdfHeader::parse(Cursor::new(input));
        assert!(matches!(result, Err(ParseError::InvalidHeader)));
    }

    #[test]
    fn test_header_version_extraction_edge_cases() {
        // Test various whitespace and formatting scenarios
        let test_cases = vec![("prefix%PDF-1.7\n", (1, 7))];

        for (input, expected) in test_cases {
            let header = PdfHeader::parse(Cursor::new(input.as_bytes())).unwrap();
            assert_eq!(header.version.major, expected.0);
            assert_eq!(header.version.minor, expected.1);
        }
    }

    #[test]
    fn test_header_with_extra_text() {
        // Test header with additional text after version
        let input = b"%PDF-1.4   extra text\n";
        let header = PdfHeader::parse(Cursor::new(input)).unwrap();

        assert_eq!(header.version.major, 1);
        assert_eq!(header.version.minor, 4);
    }
}
