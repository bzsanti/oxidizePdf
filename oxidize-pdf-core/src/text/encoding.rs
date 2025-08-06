#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextEncoding {
    StandardEncoding,
    MacRomanEncoding,
    WinAnsiEncoding,
    PdfDocEncoding,
}

impl TextEncoding {
    pub fn encode(&self, text: &str) -> Vec<u8> {
        match self {
            TextEncoding::StandardEncoding | TextEncoding::PdfDocEncoding => {
                // For now, use UTF-8 encoding
                text.bytes().collect()
            }
            TextEncoding::WinAnsiEncoding => {
                // Convert UTF-8 to Windows-1252
                let mut result = Vec::new();
                for ch in text.chars() {
                    match ch as u32 {
                        // ASCII range
                        0x00..=0x7F => result.push(ch as u8),
                        // Latin-1 Supplement that overlaps with Windows-1252
                        0xA0..=0xFF => result.push(ch as u8),
                        // Special mappings for Windows-1252
                        0x20AC => result.push(0x80), // Euro sign
                        0x201A => result.push(0x82), // Single low quotation mark
                        0x0192 => result.push(0x83), // Latin small letter f with hook
                        0x201E => result.push(0x84), // Double low quotation mark
                        0x2026 => result.push(0x85), // Horizontal ellipsis
                        0x2020 => result.push(0x86), // Dagger
                        0x2021 => result.push(0x87), // Double dagger
                        0x02C6 => result.push(0x88), // Circumflex accent
                        0x2030 => result.push(0x89), // Per mille sign
                        0x0160 => result.push(0x8A), // Latin capital letter S with caron
                        0x2039 => result.push(0x8B), // Single left angle quotation mark
                        0x0152 => result.push(0x8C), // Latin capital ligature OE
                        0x017D => result.push(0x8E), // Latin capital letter Z with caron
                        0x2018 => result.push(0x91), // Left single quotation mark
                        0x2019 => result.push(0x92), // Right single quotation mark
                        0x201C => result.push(0x93), // Left double quotation mark
                        0x201D => result.push(0x94), // Right double quotation mark
                        0x2022 => result.push(0x95), // Bullet
                        0x2013 => result.push(0x96), // En dash
                        0x2014 => result.push(0x97), // Em dash
                        0x02DC => result.push(0x98), // Small tilde
                        0x2122 => result.push(0x99), // Trade mark sign
                        0x0161 => result.push(0x9A), // Latin small letter s with caron
                        0x203A => result.push(0x9B), // Single right angle quotation mark
                        0x0153 => result.push(0x9C), // Latin small ligature oe
                        0x017E => result.push(0x9E), // Latin small letter z with caron
                        0x0178 => result.push(0x9F), // Latin capital letter Y with diaeresis
                        // Default: use question mark for unmapped characters
                        _ => result.push(b'?'),
                    }
                }
                result
            }
            TextEncoding::MacRomanEncoding => {
                // Convert UTF-8 to Mac Roman encoding
                let mut result = Vec::new();
                for ch in text.chars() {
                    match ch as u32 {
                        // ASCII range
                        0x00..=0x7F => result.push(ch as u8),
                        // Mac Roman specific mappings
                        0x00C4 => result.push(0x80), // Latin capital letter A with diaeresis
                        0x00C5 => result.push(0x81), // Latin capital letter A with ring above
                        0x00C7 => result.push(0x82), // Latin capital letter C with cedilla
                        0x00C9 => result.push(0x83), // Latin capital letter E with acute
                        0x00D1 => result.push(0x84), // Latin capital letter N with tilde
                        0x00D6 => result.push(0x85), // Latin capital letter O with diaeresis
                        0x00DC => result.push(0x86), // Latin capital letter U with diaeresis
                        0x00E1 => result.push(0x87), // Latin small letter a with acute
                        0x00E0 => result.push(0x88), // Latin small letter a with grave
                        0x00E2 => result.push(0x89), // Latin small letter a with circumflex
                        0x00E4 => result.push(0x8A), // Latin small letter a with diaeresis
                        0x00E3 => result.push(0x8B), // Latin small letter a with tilde
                        0x00E5 => result.push(0x8C), // Latin small letter a with ring above
                        0x00E7 => result.push(0x8D), // Latin small letter c with cedilla
                        0x00E9 => result.push(0x8E), // Latin small letter e with acute
                        0x00E8 => result.push(0x8F), // Latin small letter e with grave
                        0x00EA => result.push(0x90), // Latin small letter e with circumflex
                        0x00EB => result.push(0x91), // Latin small letter e with diaeresis
                        0x00ED => result.push(0x92), // Latin small letter i with acute
                        0x00EC => result.push(0x93), // Latin small letter i with grave
                        0x00EE => result.push(0x94), // Latin small letter i with circumflex
                        0x00EF => result.push(0x95), // Latin small letter i with diaeresis
                        0x00F1 => result.push(0x96), // Latin small letter n with tilde
                        0x00F3 => result.push(0x97), // Latin small letter o with acute
                        0x00F2 => result.push(0x98), // Latin small letter o with grave
                        0x00F4 => result.push(0x99), // Latin small letter o with circumflex
                        0x00F6 => result.push(0x9A), // Latin small letter o with diaeresis
                        0x00F5 => result.push(0x9B), // Latin small letter o with tilde
                        0x00FA => result.push(0x9C), // Latin small letter u with acute
                        0x00F9 => result.push(0x9D), // Latin small letter u with grave
                        0x00FB => result.push(0x9E), // Latin small letter u with circumflex
                        0x00FC => result.push(0x9F), // Latin small letter u with diaeresis
                        0x2020 => result.push(0xA0), // Dagger
                        0x00B0 => result.push(0xA1), // Degree sign
                        0x00A2 => result.push(0xA2), // Cent sign
                        0x00A3 => result.push(0xA3), // Pound sign
                        0x00A7 => result.push(0xA4), // Section sign
                        0x2022 => result.push(0xA5), // Bullet
                        0x00B6 => result.push(0xA6), // Pilcrow sign
                        0x00DF => result.push(0xA7), // Latin small letter sharp s
                        0x00AE => result.push(0xA8), // Registered sign
                        0x00A9 => result.push(0xA9), // Copyright sign
                        0x2122 => result.push(0xAA), // Trade mark sign
                        0x00B4 => result.push(0xAB), // Acute accent
                        0x00A8 => result.push(0xAC), // Diaeresis
                        0x2260 => result.push(0xAD), // Not equal to
                        0x00C6 => result.push(0xAE), // Latin capital letter AE
                        0x00D8 => result.push(0xAF), // Latin capital letter O with stroke
                        // Default: use question mark for unmapped characters
                        _ => result.push(b'?'),
                    }
                }
                result
            }
        }
    }

    pub fn decode(&self, data: &[u8]) -> String {
        match self {
            TextEncoding::StandardEncoding | TextEncoding::PdfDocEncoding => {
                // For now, assume UTF-8
                String::from_utf8_lossy(data).to_string()
            }
            TextEncoding::WinAnsiEncoding => {
                // Decode Windows-1252 to UTF-8
                let mut result = String::new();
                for &byte in data {
                    let ch = match byte {
                        // ASCII range
                        0x00..=0x7F => byte as char,
                        // Windows-1252 specific mappings
                        0x80 => '\u{20AC}', // Euro sign
                        0x82 => '\u{201A}', // Single low quotation mark
                        0x83 => '\u{0192}', // Latin small letter f with hook
                        0x84 => '\u{201E}', // Double low quotation mark
                        0x85 => '\u{2026}', // Horizontal ellipsis
                        0x86 => '\u{2020}', // Dagger
                        0x87 => '\u{2021}', // Double dagger
                        0x88 => '\u{02C6}', // Circumflex accent
                        0x89 => '\u{2030}', // Per mille sign
                        0x8A => '\u{0160}', // Latin capital letter S with caron
                        0x8B => '\u{2039}', // Single left angle quotation mark
                        0x8C => '\u{0152}', // Latin capital ligature OE
                        0x8E => '\u{017D}', // Latin capital letter Z with caron
                        0x91 => '\u{2018}', // Left single quotation mark
                        0x92 => '\u{2019}', // Right single quotation mark
                        0x93 => '\u{201C}', // Left double quotation mark
                        0x94 => '\u{201D}', // Right double quotation mark
                        0x95 => '\u{2022}', // Bullet
                        0x96 => '\u{2013}', // En dash
                        0x97 => '\u{2014}', // Em dash
                        0x98 => '\u{02DC}', // Small tilde
                        0x99 => '\u{2122}', // Trade mark sign
                        0x9A => '\u{0161}', // Latin small letter s with caron
                        0x9B => '\u{203A}', // Single right angle quotation mark
                        0x9C => '\u{0153}', // Latin small ligature oe
                        0x9E => '\u{017E}', // Latin small letter z with caron
                        0x9F => '\u{0178}', // Latin capital letter Y with diaeresis
                        // Latin-1 range that overlaps with Windows-1252
                        0xA0..=0xFF => char::from_u32(byte as u32).unwrap_or('?'),
                        // Undefined bytes
                        _ => '?',
                    };
                    result.push(ch);
                }
                result
            }
            TextEncoding::MacRomanEncoding => {
                // Decode Mac Roman to UTF-8
                let mut result = String::new();
                for &byte in data {
                    let ch = match byte {
                        // ASCII range
                        0x00..=0x7F => byte as char,
                        // Mac Roman specific mappings
                        0x80 => '\u{00C4}', // Latin capital letter A with diaeresis
                        0x81 => '\u{00C5}', // Latin capital letter A with ring above
                        0x82 => '\u{00C7}', // Latin capital letter C with cedilla
                        0x83 => '\u{00C9}', // Latin capital letter E with acute
                        0x84 => '\u{00D1}', // Latin capital letter N with tilde
                        0x85 => '\u{00D6}', // Latin capital letter O with diaeresis
                        0x86 => '\u{00DC}', // Latin capital letter U with diaeresis
                        0x87 => '\u{00E1}', // Latin small letter a with acute
                        0x88 => '\u{00E0}', // Latin small letter a with grave
                        0x89 => '\u{00E2}', // Latin small letter a with circumflex
                        0x8A => '\u{00E4}', // Latin small letter a with diaeresis
                        0x8B => '\u{00E3}', // Latin small letter a with tilde
                        0x8C => '\u{00E5}', // Latin small letter a with ring above
                        0x8D => '\u{00E7}', // Latin small letter c with cedilla
                        0x8E => '\u{00E9}', // Latin small letter e with acute
                        0x8F => '\u{00E8}', // Latin small letter e with grave
                        0x90 => '\u{00EA}', // Latin small letter e with circumflex
                        0x91 => '\u{00EB}', // Latin small letter e with diaeresis
                        0x92 => '\u{00ED}', // Latin small letter i with acute
                        0x93 => '\u{00EC}', // Latin small letter i with grave
                        0x94 => '\u{00EE}', // Latin small letter i with circumflex
                        0x95 => '\u{00EF}', // Latin small letter i with diaeresis
                        0x96 => '\u{00F1}', // Latin small letter n with tilde
                        0x97 => '\u{00F3}', // Latin small letter o with acute
                        0x98 => '\u{00F2}', // Latin small letter o with grave
                        0x99 => '\u{00F4}', // Latin small letter o with circumflex
                        0x9A => '\u{00F6}', // Latin small letter o with diaeresis
                        0x9B => '\u{00F5}', // Latin small letter o with tilde
                        0x9C => '\u{00FA}', // Latin small letter u with acute
                        0x9D => '\u{00F9}', // Latin small letter u with grave
                        0x9E => '\u{00FB}', // Latin small letter u with circumflex
                        0x9F => '\u{00FC}', // Latin small letter u with diaeresis
                        0xA0 => '\u{2020}', // Dagger
                        0xA1 => '\u{00B0}', // Degree sign
                        0xA2 => '\u{00A2}', // Cent sign
                        0xA3 => '\u{00A3}', // Pound sign
                        0xA4 => '\u{00A7}', // Section sign
                        0xA5 => '\u{2022}', // Bullet
                        0xA6 => '\u{00B6}', // Pilcrow sign
                        0xA7 => '\u{00DF}', // Latin small letter sharp s
                        0xA8 => '\u{00AE}', // Registered sign
                        0xA9 => '\u{00A9}', // Copyright sign
                        0xAA => '\u{2122}', // Trade mark sign
                        0xAB => '\u{00B4}', // Acute accent
                        0xAC => '\u{00A8}', // Diaeresis
                        0xAD => '\u{2260}', // Not equal to
                        0xAE => '\u{00C6}', // Latin capital letter AE
                        0xAF => '\u{00D8}', // Latin capital letter O with stroke
                        0xB0 => '\u{221E}', // Infinity
                        0xB1 => '\u{00B1}', // Plus-minus sign
                        0xB2 => '\u{2264}', // Less-than or equal to
                        0xB3 => '\u{2265}', // Greater-than or equal to
                        0xB4 => '\u{00A5}', // Yen sign
                        0xB5 => '\u{00B5}', // Micro sign
                        0xB6 => '\u{2202}', // Partial differential
                        0xB7 => '\u{2211}', // N-ary summation
                        0xB8 => '\u{220F}', // N-ary product
                        0xB9 => '\u{03C0}', // Greek small letter pi
                        0xBA => '\u{222B}', // Integral
                        0xBB => '\u{00AA}', // Feminine ordinal indicator
                        0xBC => '\u{00BA}', // Masculine ordinal indicator
                        0xBD => '\u{03A9}', // Greek capital letter omega
                        0xBE => '\u{00E6}', // Latin small letter ae
                        0xBF => '\u{00F8}', // Latin small letter o with stroke
                        0xC0 => '\u{00BF}', // Inverted question mark
                        0xC1 => '\u{00A1}', // Inverted exclamation mark
                        0xC2 => '\u{00AC}', // Not sign
                        0xC3 => '\u{221A}', // Square root
                        0xC4 => '\u{0192}', // Latin small letter f with hook
                        0xC5 => '\u{2248}', // Almost equal to
                        0xC6 => '\u{2206}', // Increment
                        0xC7 => '\u{00AB}', // Left-pointing double angle quotation mark
                        0xC8 => '\u{00BB}', // Right-pointing double angle quotation mark
                        0xC9 => '\u{2026}', // Horizontal ellipsis
                        0xCA => '\u{00A0}', // No-break space
                        0xCB => '\u{00C0}', // Latin capital letter A with grave
                        0xCC => '\u{00C3}', // Latin capital letter A with tilde
                        0xCD => '\u{00D5}', // Latin capital letter O with tilde
                        0xCE => '\u{0152}', // Latin capital ligature OE
                        0xCF => '\u{0153}', // Latin small ligature oe
                        0xD0 => '\u{2013}', // En dash
                        0xD1 => '\u{2014}', // Em dash
                        0xD2 => '\u{201C}', // Left double quotation mark
                        0xD3 => '\u{201D}', // Right double quotation mark
                        0xD4 => '\u{2018}', // Left single quotation mark
                        0xD5 => '\u{2019}', // Right single quotation mark
                        0xD6 => '\u{00F7}', // Division sign
                        0xD7 => '\u{25CA}', // Lozenge
                        0xD8 => '\u{00FF}', // Latin small letter y with diaeresis
                        0xD9 => '\u{0178}', // Latin capital letter Y with diaeresis
                        0xDA => '\u{2044}', // Fraction slash
                        0xDB => '\u{20AC}', // Euro sign
                        0xDC => '\u{2039}', // Single left-pointing angle quotation mark
                        0xDD => '\u{203A}', // Single right-pointing angle quotation mark
                        0xDE => '\u{FB01}', // Latin small ligature fi
                        0xDF => '\u{FB02}', // Latin small ligature fl
                        0xE0 => '\u{2021}', // Double dagger
                        0xE1 => '\u{00B7}', // Middle dot
                        0xE2 => '\u{201A}', // Single low-9 quotation mark
                        0xE3 => '\u{201E}', // Double low-9 quotation mark
                        0xE4 => '\u{2030}', // Per mille sign
                        0xE5 => '\u{00C2}', // Latin capital letter A with circumflex
                        0xE6 => '\u{00CA}', // Latin capital letter E with circumflex
                        0xE7 => '\u{00C1}', // Latin capital letter A with acute
                        0xE8 => '\u{00CB}', // Latin capital letter E with diaeresis
                        0xE9 => '\u{00C8}', // Latin capital letter E with grave
                        0xEA => '\u{00CD}', // Latin capital letter I with acute
                        0xEB => '\u{00CE}', // Latin capital letter I with circumflex
                        0xEC => '\u{00CF}', // Latin capital letter I with diaeresis
                        0xED => '\u{00CC}', // Latin capital letter I with grave
                        0xEE => '\u{00D3}', // Latin capital letter O with acute
                        0xEF => '\u{00D4}', // Latin capital letter O with circumflex
                        0xF0 => '\u{F8FF}', // Apple logo
                        0xF1 => '\u{00D2}', // Latin capital letter O with grave
                        0xF2 => '\u{00DA}', // Latin capital letter U with acute
                        0xF3 => '\u{00DB}', // Latin capital letter U with circumflex
                        0xF4 => '\u{00D9}', // Latin capital letter U with grave
                        0xF5 => '\u{0131}', // Latin small letter dotless i
                        0xF6 => '\u{02C6}', // Modifier letter circumflex accent
                        0xF7 => '\u{02DC}', // Small tilde
                        0xF8 => '\u{00AF}', // Macron
                        0xF9 => '\u{02D8}', // Breve
                        0xFA => '\u{02D9}', // Dot above
                        0xFB => '\u{02DA}', // Ring above
                        0xFC => '\u{00B8}', // Cedilla
                        0xFD => '\u{02DD}', // Double acute accent
                        0xFE => '\u{02DB}', // Ogonek
                        0xFF => '\u{02C7}', // Caron
                    };
                    result.push(ch);
                }
                result
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_encoding_variants() {
        let encodings = [
            TextEncoding::StandardEncoding,
            TextEncoding::MacRomanEncoding,
            TextEncoding::WinAnsiEncoding,
            TextEncoding::PdfDocEncoding,
        ];

        for encoding in &encodings {
            assert_eq!(*encoding, *encoding);
        }

        assert_ne!(
            TextEncoding::StandardEncoding,
            TextEncoding::WinAnsiEncoding
        );
    }

    #[test]
    fn test_standard_encoding_basic_ascii() {
        let encoding = TextEncoding::StandardEncoding;
        let text = "Hello World!";

        let encoded = encoding.encode(text);
        let decoded = encoding.decode(&encoded);
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_win_ansi_encoding_special_chars() {
        let encoding = TextEncoding::WinAnsiEncoding;

        // Test Euro sign
        let text = "€100";
        let encoded = encoding.encode(text);
        assert_eq!(encoded[0], 0x80);
        let decoded = encoding.decode(&encoded);
        assert_eq!(decoded, text);

        // Test other special characters
        let text2 = "Hello—World"; // Em dash
        let encoded2 = encoding.encode(text2);
        let decoded2 = encoding.decode(&encoded2);
        assert_eq!(decoded2, text2);
    }

    #[test]
    fn test_mac_roman_encoding_special_chars() {
        let encoding = TextEncoding::MacRomanEncoding;

        // Test accented characters
        let text = "café";
        let encoded = encoding.encode(text);
        assert_eq!(encoded[3], 0x8E); // é
        let decoded = encoding.decode(&encoded);
        assert_eq!(decoded, text);

        // Test Apple logo (special Mac character)
        let apple_bytes = vec![0xF0];
        let decoded_apple = encoding.decode(&apple_bytes);
        assert_eq!(decoded_apple, "\u{F8FF}");

        // Test various accented characters
        let text2 = "Zürich";
        let encoded2 = encoding.encode(text2);
        assert_eq!(encoded2[1], 0x9F); // ü
        let decoded2 = encoding.decode(&encoded2);
        assert_eq!(decoded2, text2);
    }

    #[test]
    fn test_pdf_doc_encoding() {
        let encoding = TextEncoding::PdfDocEncoding;
        let text = "PDF Document";

        let encoded = encoding.encode(text);
        let decoded = encoding.decode(&encoded);

        assert_eq!(text, decoded);
    }

    #[test]
    fn test_pdf_doc_encoding_basic_ascii() {
        let encoding = TextEncoding::PdfDocEncoding;
        let text = "Hello World!";

        let encoded = encoding.encode(text);
        let decoded = encoding.decode(&encoded);

        assert_eq!(text, decoded);
    }

    #[test]
    fn test_mac_roman_encoding_basic_ascii() {
        let encoding = TextEncoding::MacRomanEncoding;
        let text = "Hello World!";

        let encoded = encoding.encode(text);
        let decoded = encoding.decode(&encoded);

        assert_eq!(text, decoded);
    }

    #[test]
    fn test_win_ansi_encoding_basic_ascii() {
        let encoding = TextEncoding::WinAnsiEncoding;
        let text = "Hello World!";

        let encoded = encoding.encode(text);
        let decoded = encoding.decode(&encoded);

        assert_eq!(text, decoded);
    }

    #[test]
    fn test_win_ansi_encoding_special_characters() {
        let encoding = TextEncoding::WinAnsiEncoding;

        // Test Euro sign
        let euro_text = "€";
        let encoded = encoding.encode(euro_text);
        assert_eq!(encoded, vec![0x80]);
        let decoded = encoding.decode(&encoded);
        assert_eq!(decoded, euro_text);

        // Test em dash
        let dash_text = "—";
        let encoded = encoding.encode(dash_text);
        assert_eq!(encoded, vec![0x97]);
        let decoded = encoding.decode(&encoded);
        assert_eq!(decoded, dash_text);

        // Test single low quotation mark
        let quote_text = "‚";
        let encoded = encoding.encode(quote_text);
        assert_eq!(encoded, vec![0x82]);
        let decoded = encoding.decode(&encoded);
        assert_eq!(decoded, quote_text);
    }

    #[test]
    fn test_win_ansi_encoding_latin_supplement() {
        let encoding = TextEncoding::WinAnsiEncoding;
        let text = "café";

        let encoded = encoding.encode(text);
        let decoded = encoding.decode(&encoded);

        assert_eq!(text, decoded);
    }

    #[test]
    fn test_win_ansi_encoding_unmapped_character() {
        let encoding = TextEncoding::WinAnsiEncoding;

        // Use a character that's not in Windows-1252
        let text = "❤"; // Heart emoji
        let encoded = encoding.encode(text);
        assert_eq!(encoded, vec![b'?']); // Should be replaced with ?

        let decoded = encoding.decode(&encoded);
        assert_eq!(decoded, "?");
    }

    #[test]
    fn test_win_ansi_encoding_round_trip_special_chars() {
        let encoding = TextEncoding::WinAnsiEncoding;

        let special_chars = [
            ("€", 0x80),        // Euro sign
            ("‚", 0x82),        // Single low quotation mark
            ("ƒ", 0x83),        // Latin small letter f with hook
            ("„", 0x84),        // Double low quotation mark
            ("…", 0x85),        // Horizontal ellipsis
            ("†", 0x86),        // Dagger
            ("‡", 0x87),        // Double dagger
            ("‰", 0x89),        // Per mille sign
            ("\u{2018}", 0x91), // Left single quotation mark
            ("\u{2019}", 0x92), // Right single quotation mark
            ("\u{201C}", 0x93), // Left double quotation mark
            ("\u{201D}", 0x94), // Right double quotation mark
            ("•", 0x95),        // Bullet
            ("–", 0x96),        // En dash
            ("—", 0x97),        // Em dash
            ("™", 0x99),        // Trade mark sign
        ];

        for (text, expected_byte) in &special_chars {
            let encoded = encoding.encode(text);
            assert_eq!(encoded, vec![*expected_byte], "Failed for character {text}");

            let decoded = encoding.decode(&encoded);
            assert_eq!(decoded, *text, "Round trip failed for character {text}");
        }
    }

    #[test]
    fn test_encoding_equality() {
        assert_eq!(
            TextEncoding::StandardEncoding,
            TextEncoding::StandardEncoding
        );
        assert_eq!(TextEncoding::WinAnsiEncoding, TextEncoding::WinAnsiEncoding);

        assert_ne!(
            TextEncoding::StandardEncoding,
            TextEncoding::WinAnsiEncoding
        );
        assert_ne!(TextEncoding::MacRomanEncoding, TextEncoding::PdfDocEncoding);
    }

    #[test]
    fn test_encoding_debug() {
        let encoding = TextEncoding::WinAnsiEncoding;
        let debug_str = format!("{encoding:?}");
        assert_eq!(debug_str, "WinAnsiEncoding");
    }

    #[test]
    fn test_encoding_clone() {
        let encoding1 = TextEncoding::PdfDocEncoding;
        let encoding2 = encoding1;
        assert_eq!(encoding1, encoding2);
    }

    #[test]
    fn test_encoding_copy() {
        let encoding1 = TextEncoding::StandardEncoding;
        let encoding2 = encoding1; // Copy semantics
        assert_eq!(encoding1, encoding2);

        // Both variables should still be usable
        assert_eq!(encoding1, TextEncoding::StandardEncoding);
        assert_eq!(encoding2, TextEncoding::StandardEncoding);
    }

    #[test]
    fn test_empty_string_encoding() {
        for encoding in &[
            TextEncoding::StandardEncoding,
            TextEncoding::MacRomanEncoding,
            TextEncoding::WinAnsiEncoding,
            TextEncoding::PdfDocEncoding,
        ] {
            let encoded = encoding.encode("");
            assert!(encoded.is_empty());

            let decoded = encoding.decode(&[]);
            assert!(decoded.is_empty());
        }
    }

    #[test]
    fn test_win_ansi_decode_undefined_bytes() {
        let encoding = TextEncoding::WinAnsiEncoding;

        // Test some undefined bytes in Windows-1252 (0x81, 0x8D, 0x8F, 0x90, 0x9D)
        let undefined_bytes = [0x81, 0x8D, 0x8F, 0x90, 0x9D];

        for &byte in &undefined_bytes {
            let decoded = encoding.decode(&[byte]);
            assert_eq!(
                decoded, "?",
                "Undefined byte 0x{byte:02X} should decode to '?'"
            );
        }
    }

    #[test]
    fn test_win_ansi_ascii_range() {
        let encoding = TextEncoding::WinAnsiEncoding;

        // Test ASCII range (0x00-0x7F)
        for byte in 0x20..=0x7E {
            // Printable ASCII
            let text = char::from(byte).to_string();
            let encoded = encoding.encode(&text);
            assert_eq!(encoded, vec![byte]);

            let decoded = encoding.decode(&encoded);
            assert_eq!(decoded, text);
        }
    }

    #[test]
    fn test_win_ansi_latin1_overlap() {
        let encoding = TextEncoding::WinAnsiEncoding;

        // Test Latin-1 range that overlaps with Windows-1252 (0xA0-0xFF)
        let test_chars = "¡¢£¤¥¦§¨©ª«¬®¯°±²³´µ¶·¸¹º»¼½¾¿ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕÖ×ØÙÚÛÜÝÞßàáâãäåæçèéêëìíîïðñòóôõö÷øùúûüýþÿ";

        let encoded = encoding.encode(test_chars);
        let decoded = encoding.decode(&encoded);

        assert_eq!(decoded, test_chars);
    }
}
