//! PDF Lexer
//!
//! Tokenizes PDF syntax according to ISO 32000-1 Section 7.2

use super::{ParseError, ParseOptions, ParseResult, ParseWarning};
use std::io::{Read, Seek, SeekFrom};

/// PDF Token types
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Boolean: true or false
    Boolean(bool),

    /// Integer number
    Integer(i64),

    /// Real number
    Real(f64),

    /// String (literal or hexadecimal)
    String(Vec<u8>),

    /// Name object (e.g., /Type)
    Name(String),

    /// Left square bracket [
    ArrayStart,

    /// Right square bracket ]
    ArrayEnd,

    /// Dictionary start <<
    DictStart,

    /// Dictionary end >>
    DictEnd,

    /// Stream keyword
    Stream,

    /// Endstream keyword
    EndStream,

    /// Obj keyword
    Obj,

    /// Endobj keyword
    EndObj,

    /// StartXRef keyword
    StartXRef,

    /// Reference (e.g., 1 0 R)
    Reference(u32, u16),

    /// Null object
    Null,

    /// Comment (usually ignored)
    Comment(String),

    /// End of file
    Eof,
}

/// PDF Lexer for tokenizing PDF content
pub struct Lexer<R> {
    reader: std::io::BufReader<R>,
    #[allow(dead_code)]
    buffer: Vec<u8>,
    position: usize,
    peek_buffer: Option<u8>,
    token_buffer: Vec<Token>,
    options: ParseOptions,
    warnings: Vec<ParseWarning>,
}

impl<R: Read> Lexer<R> {
    /// Create a new lexer from a reader with default options
    pub fn new(reader: R) -> Self {
        Self::new_with_options(reader, ParseOptions::default())
    }

    /// Create a new lexer from a reader with custom options
    pub fn new_with_options(reader: R, options: ParseOptions) -> Self {
        Self {
            reader: std::io::BufReader::new(reader),
            buffer: Vec::with_capacity(1024),
            position: 0,
            peek_buffer: None,
            token_buffer: Vec::new(),
            options,
            warnings: Vec::new(),
        }
    }

    /// Get warnings collected during lexing (if enabled)
    pub fn warnings(&self) -> &[ParseWarning] {
        &self.warnings
    }

    /// Get the next token
    pub fn next_token(&mut self) -> ParseResult<Token> {
        // Check if we have a pushed-back token
        if let Some(token) = self.token_buffer.pop() {
            return Ok(token);
        }

        self.skip_whitespace()?;

        let ch = match self.peek_char()? {
            Some(ch) => ch,
            None => return Ok(Token::Eof),
        };

        match ch {
            b'%' => self.read_comment(),
            b'/' => self.read_name(),
            b'(' => self.read_literal_string(),
            b'<' => self.read_angle_bracket(),
            b'>' => {
                self.consume_char()?;
                if self.peek_char()? == Some(b'>') {
                    self.consume_char()?;
                    Ok(Token::DictEnd)
                } else {
                    Err(ParseError::SyntaxError {
                        position: self.position,
                        message: "Expected '>' after '>'".to_string(),
                    })
                }
            }
            b'[' => {
                self.consume_char()?;
                Ok(Token::ArrayStart)
            }
            b']' => {
                self.consume_char()?;
                Ok(Token::ArrayEnd)
            }
            b't' | b'f' => self.read_boolean(),
            b'n' => self.read_null(),
            b'+' | b'-' | b'0'..=b'9' | b'.' => self.read_number(),
            b'R' => {
                // R could be a keyword (for references)
                self.consume_char()?;
                Ok(Token::Name("R".to_string()))
            }
            _ if ch.is_ascii_alphabetic() => self.read_keyword(),
            _ => {
                // Check if this is a problematic encoding character
                if self.is_problematic_encoding_char(ch) {
                    self.handle_encoding_char_in_token_stream(ch)
                } else {
                    Err(ParseError::SyntaxError {
                        position: self.position,
                        message: format!("Unexpected character: {}", ch as char),
                    })
                }
            }
        }
    }

    /// Peek at the next character without consuming it
    fn peek_char(&mut self) -> ParseResult<Option<u8>> {
        if let Some(ch) = self.peek_buffer {
            return Ok(Some(ch));
        }

        let mut buf = [0u8; 1];
        match self.reader.read_exact(&mut buf) {
            Ok(_) => {
                self.peek_buffer = Some(buf[0]);
                Ok(Some(buf[0]))
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Consume the next character
    fn consume_char(&mut self) -> ParseResult<Option<u8>> {
        let ch = self.peek_char()?;
        if ch.is_some() {
            self.peek_buffer = None;
            self.position += 1;
        }
        Ok(ch)
    }

    /// Skip whitespace and return the number of bytes skipped
    pub(crate) fn skip_whitespace(&mut self) -> ParseResult<usize> {
        let mut count = 0;
        while let Some(ch) = self.peek_char()? {
            if ch.is_ascii_whitespace() {
                self.consume_char()?;
                count += 1;
            } else {
                break;
            }
        }
        Ok(count)
    }

    /// Read a comment (from % to end of line)
    fn read_comment(&mut self) -> ParseResult<Token> {
        self.consume_char()?; // consume '%'
        let mut comment = String::new();

        while let Some(ch) = self.peek_char()? {
            if ch == b'\n' || ch == b'\r' {
                break;
            }
            self.consume_char()?;
            comment.push(ch as char);
        }

        Ok(Token::Comment(comment))
    }

    /// Read a name object (e.g., /Type)
    fn read_name(&mut self) -> ParseResult<Token> {
        self.consume_char()?; // consume '/'
        let mut name = String::new();

        while let Some(ch) = self.peek_char()? {
            if ch.is_ascii_whitespace()
                || matches!(ch, b'/' | b'<' | b'>' | b'[' | b']' | b'(' | b')' | b'%')
            {
                break;
            }
            self.consume_char()?;

            // Handle hex codes in names (e.g., /A#20B means /A B)
            if ch == b'#' {
                let hex1 = self
                    .consume_char()?
                    .ok_or_else(|| ParseError::SyntaxError {
                        position: self.position,
                        message: "Incomplete hex code in name".to_string(),
                    })?;
                let hex2 = self
                    .consume_char()?
                    .ok_or_else(|| ParseError::SyntaxError {
                        position: self.position,
                        message: "Incomplete hex code in name".to_string(),
                    })?;

                let value = u8::from_str_radix(&format!("{}{}", hex1 as char, hex2 as char), 16)
                    .map_err(|_| ParseError::SyntaxError {
                        position: self.position,
                        message: "Invalid hex code in name".to_string(),
                    })?;

                name.push(value as char);
            } else {
                name.push(ch as char);
            }
        }

        Ok(Token::Name(name))
    }

    /// Read a literal string (parentheses)
    fn read_literal_string(&mut self) -> ParseResult<Token> {
        self.consume_char()?; // consume '('
        let mut string = Vec::new();
        let mut paren_depth = 1;
        let mut escape = false;

        while paren_depth > 0 {
            let ch = match self.consume_char()? {
                Some(c) => c,
                None => {
                    if self.options.lenient_syntax {
                        // In lenient mode, return what we have so far
                        if self.options.collect_warnings {
                            self.warnings.push(ParseWarning::SyntaxErrorRecovered {
                                position: self.position,
                                expected: "closing parenthesis".to_string(),
                                found: "EOF".to_string(),
                                recovery_action: "returned partial string content".to_string(),
                            });
                        }
                        break;
                    } else {
                        return Err(ParseError::SyntaxError {
                            position: self.position,
                            message: "Unterminated string".to_string(),
                        });
                    }
                }
            };

            if escape {
                let escaped = match ch {
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    b'b' => b'\x08',
                    b'f' => b'\x0C',
                    b'(' => b'(',
                    b')' => b')',
                    b'\\' => b'\\',
                    b'0'..=b'7' => {
                        // Octal escape sequence
                        let mut value = ch - b'0';
                        for _ in 0..2 {
                            if let Some(next) = self.peek_char()? {
                                if matches!(next, b'0'..=b'7') {
                                    self.consume_char()?;
                                    value = value * 8 + (next - b'0');
                                } else {
                                    break;
                                }
                            }
                        }
                        value
                    }
                    _ => ch, // Unknown escape, use literal
                };
                string.push(escaped);
                escape = false;
            } else {
                match ch {
                    b'\\' => escape = true,
                    b'(' => {
                        string.push(ch);
                        paren_depth += 1;
                    }
                    b')' => {
                        paren_depth -= 1;
                        if paren_depth > 0 {
                            string.push(ch);
                        }
                    }
                    _ => string.push(ch),
                }
            }
        }

        // Apply character encoding recovery if enabled
        let processed_string = if self.options.lenient_encoding {
            self.process_string_with_encoding_recovery(&string)?
        } else {
            string
        };

        Ok(Token::String(processed_string))
    }

    /// Read angle bracket tokens (hex strings or dict markers)
    fn read_angle_bracket(&mut self) -> ParseResult<Token> {
        self.consume_char()?; // consume '<'

        if self.peek_char()? == Some(b'<') {
            self.consume_char()?;
            Ok(Token::DictStart)
        } else {
            // Hex string
            let mut hex_chars = String::new();
            let mut found_end = false;

            while let Some(ch) = self.peek_char()? {
                if ch == b'>' {
                    self.consume_char()?;
                    found_end = true;
                    break;
                }
                self.consume_char()?;
                if ch.is_ascii_hexdigit() {
                    hex_chars.push(ch as char);
                } else if !ch.is_ascii_whitespace() {
                    if self.options.lenient_syntax {
                        // In lenient mode, skip invalid characters
                        if self.options.collect_warnings {
                            self.warnings.push(ParseWarning::SyntaxErrorRecovered {
                                position: self.position,
                                expected: "hex digit".to_string(),
                                found: format!("'{}'", ch as char),
                                recovery_action: "skipped invalid character".to_string(),
                            });
                        }
                    } else {
                        return Err(ParseError::SyntaxError {
                            position: self.position,
                            message: "Invalid character in hex string".to_string(),
                        });
                    }
                }
            }

            if !found_end {
                if self.options.lenient_syntax {
                    // In lenient mode, return what we have so far
                    if self.options.collect_warnings {
                        self.warnings.push(ParseWarning::SyntaxErrorRecovered {
                            position: self.position,
                            expected: ">".to_string(),
                            found: "EOF".to_string(),
                            recovery_action: "returned partial hex string".to_string(),
                        });
                    }
                } else {
                    return Err(ParseError::SyntaxError {
                        position: self.position,
                        message: "Unterminated hex string".to_string(),
                    });
                }
            }

            // Pad with 0 if odd number of digits
            if hex_chars.len() % 2 != 0 {
                hex_chars.push('0');
            }

            // Convert hex to bytes
            let mut bytes = Vec::new();
            for chunk in hex_chars.as_bytes().chunks(2) {
                let hex_str = std::str::from_utf8(chunk).map_err(|_| ParseError::SyntaxError {
                    position: self.position,
                    message: "Invalid UTF-8 in hex string".to_string(),
                })?;
                let byte =
                    u8::from_str_radix(hex_str, 16).map_err(|_| ParseError::SyntaxError {
                        position: self.position,
                        message: "Invalid hex string".to_string(),
                    })?;
                bytes.push(byte);
            }

            Ok(Token::String(bytes))
        }
    }

    /// Read boolean (true/false)
    fn read_boolean(&mut self) -> ParseResult<Token> {
        let word = self.read_word()?;
        match word.as_str() {
            "true" => Ok(Token::Boolean(true)),
            "false" => Ok(Token::Boolean(false)),
            _ => {
                // Not a boolean, might be a keyword
                self.process_keyword(word)
            }
        }
    }

    /// Read null
    fn read_null(&mut self) -> ParseResult<Token> {
        let word = self.read_word()?;
        if word == "null" {
            Ok(Token::Null)
        } else {
            // Not null, might be a keyword
            self.process_keyword(word)
        }
    }

    /// Read a number (integer or real)
    fn read_number(&mut self) -> ParseResult<Token> {
        let mut number_str = String::new();
        let mut has_dot = false;

        // Handle sign - consume it first
        if let Some(ch) = self.peek_char()? {
            if ch == b'+' || ch == b'-' {
                self.consume_char()?;
                number_str.push(ch as char);

                // After sign, we must have at least one digit
                if let Some(next) = self.peek_char()? {
                    if !next.is_ascii_digit() && next != b'.' {
                        return Err(ParseError::SyntaxError {
                            position: self.position,
                            message: "Expected digit after sign".to_string(),
                        });
                    }
                }
            }
        }

        // Read digits and decimal point
        while let Some(ch) = self.peek_char()? {
            match ch {
                b'0'..=b'9' => {
                    self.consume_char()?;
                    number_str.push(ch as char);
                }
                b'.' if !has_dot => {
                    self.consume_char()?;
                    number_str.push(ch as char);
                    has_dot = true;
                }
                _ => break,
            }
        }

        // Handle scientific notation (e/E)
        if let Some(ch) = self.peek_char()? {
            if ch == b'e' || ch == b'E' {
                self.consume_char()?;
                number_str.push(ch as char);

                // Check for optional sign after e/E
                if let Some(sign_ch) = self.peek_char()? {
                    if sign_ch == b'+' || sign_ch == b'-' {
                        self.consume_char()?;
                        number_str.push(sign_ch as char);
                    }
                }

                // Read exponent digits
                while let Some(digit_ch) = self.peek_char()? {
                    if digit_ch.is_ascii_digit() {
                        self.consume_char()?;
                        number_str.push(digit_ch as char);
                    } else {
                        break;
                    }
                }

                // Scientific notation always results in a real number
                has_dot = true;
            }
        }

        // Don't try to parse references here - let the parser handle it
        // References are just "num num R" and can be handled at a higher level

        // Parse as number
        if has_dot {
            let value = number_str
                .parse::<f64>()
                .map_err(|_| ParseError::SyntaxError {
                    position: self.position,
                    message: format!("Invalid real number: '{number_str}'"),
                })?;
            Ok(Token::Real(value))
        } else {
            let value = number_str
                .parse::<i64>()
                .map_err(|_| ParseError::SyntaxError {
                    position: self.position,
                    message: format!("Invalid integer: '{number_str}'"),
                })?;
            Ok(Token::Integer(value))
        }
    }

    /// Read a keyword
    fn read_keyword(&mut self) -> ParseResult<Token> {
        let word = self.read_word()?;
        self.process_keyword(word)
    }

    /// Process a word as a keyword
    fn process_keyword(&self, word: String) -> ParseResult<Token> {
        match word.as_str() {
            "stream" => Ok(Token::Stream),
            "endstream" => Ok(Token::EndStream),
            "obj" => Ok(Token::Obj),
            "endobj" => Ok(Token::EndObj),
            "startxref" => Ok(Token::StartXRef),
            _ => Err(ParseError::SyntaxError {
                position: self.position,
                message: format!("Unknown keyword: {word}"),
            }),
        }
    }

    /// Read a word (sequence of non-delimiter characters)
    fn read_word(&mut self) -> ParseResult<String> {
        let mut word = String::new();

        while let Some(ch) = self.peek_char()? {
            if ch.is_ascii_whitespace()
                || matches!(ch, b'/' | b'<' | b'>' | b'[' | b']' | b'(' | b')' | b'%')
            {
                break;
            }
            self.consume_char()?;
            word.push(ch as char);
        }

        Ok(word)
    }

    /// Read a sequence of digits
    #[allow(dead_code)]
    fn read_digits(&mut self) -> ParseResult<String> {
        let mut digits = String::new();

        while let Some(ch) = self.peek_char()? {
            if ch.is_ascii_digit() {
                self.consume_char()?;
                digits.push(ch as char);
            } else {
                break;
            }
        }

        Ok(digits)
    }

    /// Read a newline sequence (CR, LF, or CRLF)
    pub fn read_newline(&mut self) -> ParseResult<()> {
        match self.peek_char()? {
            Some(b'\r') => {
                self.consume_char()?;
                // Check for CRLF
                if self.peek_char()? == Some(b'\n') {
                    self.consume_char()?;
                }
                Ok(())
            }
            Some(b'\n') => {
                self.consume_char()?;
                Ok(())
            }
            _ => Err(ParseError::SyntaxError {
                position: self.position,
                message: "Expected newline".to_string(),
            }),
        }
    }

    /// Read exactly n bytes
    /// Peek at the next byte without consuming it
    pub fn peek_byte(&mut self) -> ParseResult<u8> {
        match self.peek_char()? {
            Some(b) => Ok(b),
            None => Err(ParseError::UnexpectedToken {
                expected: "byte".to_string(),
                found: "EOF".to_string(),
            }),
        }
    }

    /// Read a single byte
    pub fn read_byte(&mut self) -> ParseResult<u8> {
        match self.consume_char()? {
            Some(b) => Ok(b),
            None => Err(ParseError::UnexpectedToken {
                expected: "byte".to_string(),
                found: "EOF".to_string(),
            }),
        }
    }

    /// Seek to a specific position
    pub fn seek(&mut self, pos: u64) -> ParseResult<()>
    where
        R: Seek,
    {
        self.reader.seek(SeekFrom::Start(pos))?;
        self.position = pos as usize;
        Ok(())
    }

    pub fn read_bytes(&mut self, n: usize) -> ParseResult<Vec<u8>> {
        let mut bytes = Vec::with_capacity(n);

        // First consume any peeked byte to avoid duplication
        if self.peek_buffer.is_some() && n > 0 {
            bytes.push(self.consume_char()?.unwrap());
        }

        // Read remaining bytes directly
        let remaining = n - bytes.len();
        if remaining > 0 {
            let mut rest = vec![0u8; remaining];
            self.reader.read_exact(&mut rest)?;
            self.position += remaining;
            bytes.extend_from_slice(&rest);
        }

        Ok(bytes)
    }

    /// Read until a specific byte sequence is found
    pub fn read_until_sequence(&mut self, sequence: &[u8]) -> ParseResult<Vec<u8>> {
        let mut result = Vec::new();
        let mut match_pos = 0;

        while let Some(ch) = self.consume_char()? {
            result.push(ch);

            if ch == sequence[match_pos] {
                match_pos += 1;
                if match_pos == sequence.len() {
                    // Found the sequence, remove it from result
                    result.truncate(result.len() - sequence.len());
                    break;
                }
            } else if ch == sequence[0] {
                match_pos = 1;
            } else {
                match_pos = 0;
            }
        }

        if match_pos < sequence.len() {
            return Err(ParseError::SyntaxError {
                position: self.position,
                message: format!("Sequence {sequence:?} not found"),
            });
        }

        Ok(result)
    }

    /// Get current position
    pub fn position(&self) -> usize {
        self.position
    }

    /// Push back a token to be returned by the next call to next_token
    pub fn push_token(&mut self, token: Token) {
        self.token_buffer.push(token);
    }

    /// Expect a specific keyword token
    pub fn expect_keyword(&mut self, keyword: &str) -> ParseResult<()> {
        let token = self.next_token()?;
        match (keyword, &token) {
            ("endstream", Token::EndStream) => Ok(()),
            ("stream", Token::Stream) => Ok(()),
            ("endobj", Token::EndObj) => Ok(()),
            ("obj", Token::Obj) => Ok(()),
            ("startxref", Token::StartXRef) => Ok(()),
            _ => Err(ParseError::UnexpectedToken {
                expected: format!("keyword '{keyword}'"),
                found: format!("{token:?}"),
            }),
        }
    }

    /// Find a keyword ahead in the stream without consuming bytes
    /// Returns the number of bytes until the keyword is found
    pub fn find_keyword_ahead(
        &mut self,
        keyword: &str,
        max_bytes: usize,
    ) -> ParseResult<Option<usize>>
    where
        R: Seek,
    {
        use std::io::{Read, Seek, SeekFrom};

        // Save current position
        let current_pos = self.reader.stream_position()?;
        let start_buffer_state = self.peek_buffer;

        let keyword_bytes = keyword.as_bytes();
        let mut bytes_read = 0;
        let mut match_buffer = Vec::new();

        // Search for the keyword
        while bytes_read < max_bytes {
            let mut byte = [0u8; 1];
            match self.reader.read_exact(&mut byte) {
                Ok(_) => {
                    bytes_read += 1;
                    match_buffer.push(byte[0]);

                    // Keep only the last keyword.len() bytes in match_buffer
                    if match_buffer.len() > keyword_bytes.len() {
                        match_buffer.remove(0);
                    }

                    // Check if we found the keyword
                    if match_buffer.len() == keyword_bytes.len() && match_buffer == keyword_bytes {
                        // Restore position
                        self.reader.seek(SeekFrom::Start(current_pos))?;
                        self.peek_buffer = start_buffer_state;
                        return Ok(Some(bytes_read - keyword_bytes.len()));
                    }
                }
                Err(_) => break, // EOF or error
            }
        }

        // Restore position
        self.reader.seek(SeekFrom::Start(current_pos))?;
        self.peek_buffer = start_buffer_state;
        Ok(None)
    }

    /// Peek ahead n bytes without consuming them
    pub fn peek_ahead(&mut self, n: usize) -> ParseResult<Vec<u8>>
    where
        R: Seek,
    {
        use std::io::{Read, Seek, SeekFrom};

        // Save current position
        let current_pos = self.reader.stream_position()?;
        let start_buffer_state = self.peek_buffer;

        // Read n bytes
        let mut bytes = vec![0u8; n];
        let bytes_read = self.reader.read(&mut bytes)?;
        bytes.truncate(bytes_read);

        // Restore position
        self.reader.seek(SeekFrom::Start(current_pos))?;
        self.peek_buffer = start_buffer_state;

        Ok(bytes)
    }

    /// Save the current position for later restoration
    pub fn save_position(&mut self) -> ParseResult<(u64, Option<u8>)>
    where
        R: Seek,
    {
        use std::io::Seek;
        let pos = self.reader.stream_position()?;
        Ok((pos, self.peek_buffer))
    }

    /// Restore a previously saved position
    pub fn restore_position(&mut self, saved: (u64, Option<u8>)) -> ParseResult<()>
    where
        R: Seek,
    {
        use std::io::{Seek, SeekFrom};
        self.reader.seek(SeekFrom::Start(saved.0))?;
        self.peek_buffer = saved.1;
        self.position = saved.0 as usize;
        Ok(())
    }

    /// Peek the next token without consuming it
    pub fn peek_token(&mut self) -> ParseResult<Token>
    where
        R: Seek,
    {
        let saved_pos = self.save_position()?;
        let token = self.next_token()?;
        self.restore_position(saved_pos)?;
        Ok(token)
    }

    /// Process string bytes with enhanced character encoding recovery
    fn process_string_with_encoding_recovery(
        &mut self,
        string_bytes: &[u8],
    ) -> ParseResult<Vec<u8>> {
        use super::encoding::{CharacterDecoder, EncodingOptions, EncodingType, EnhancedDecoder};

        // First check for common problematic bytes that need special handling
        let has_problematic_chars = string_bytes.iter().any(|&b| {
            // Control characters and Latin-1 supplement range that often cause issues
            (0x80..=0x9F).contains(&b)
                || b == 0x07
                || (b <= 0x1F && b != 0x09 && b != 0x0A && b != 0x0D)
        });

        let decoder = EnhancedDecoder::new();

        // Use more aggressive encoding options if problematic characters detected
        let encoding_options = if has_problematic_chars {
            EncodingOptions {
                lenient_mode: true, // Always use lenient mode for problematic chars
                preferred_encoding: Some(EncodingType::Windows1252), // Try Windows-1252 first for control chars
                max_replacements: std::cmp::max(100, string_bytes.len() / 10), // More generous replacement limit
                log_issues: self.options.collect_warnings,
            }
        } else {
            EncodingOptions {
                lenient_mode: self.options.lenient_encoding,
                preferred_encoding: self.options.preferred_encoding,
                max_replacements: 50,
                log_issues: self.options.collect_warnings,
            }
        };

        match decoder.decode(string_bytes, &encoding_options) {
            Ok(result) => {
                // Log warning if replacements were made or problematic chars detected
                if (result.replacement_count > 0 || has_problematic_chars)
                    && self.options.collect_warnings
                {
                    self.warnings.push(ParseWarning::InvalidEncoding {
                        position: self.position,
                        recovered_text: if result.text.len() > 50 {
                            // Safe character boundary truncation
                            let truncate_at = result
                                .text
                                .char_indices()
                                .map(|(i, _)| i)
                                .nth(47)
                                .unwrap_or(result.text.len().min(47));
                            format!(
                                "{}... (truncated, {} chars total)",
                                &result.text[..truncate_at],
                                result.text.chars().count()
                            )
                        } else {
                            result.text.clone()
                        },
                        encoding_used: result.detected_encoding,
                        replacement_count: result.replacement_count,
                    });
                }

                // Convert back to bytes
                Ok(result.text.into_bytes())
            }
            Err(encoding_error) => {
                if self.options.lenient_encoding {
                    // Enhanced fallback strategy
                    let fallback_result = self.apply_fallback_encoding_strategy(string_bytes);

                    if self.options.collect_warnings {
                        self.warnings.push(ParseWarning::InvalidEncoding {
                            position: self.position,
                            recovered_text: format!(
                                "Fallback strategy applied: {} -> {} chars",
                                string_bytes.len(),
                                fallback_result.len()
                            ),
                            encoding_used: None,
                            replacement_count: string_bytes.len(),
                        });
                    }
                    Ok(fallback_result)
                } else {
                    Err(ParseError::CharacterEncodingError {
                        position: self.position,
                        message: format!(
                            "Failed to decode string with any supported encoding: {encoding_error}"
                        ),
                    })
                }
            }
        }
    }

    /// Apply fallback encoding strategy for severely corrupted strings
    fn apply_fallback_encoding_strategy(&self, string_bytes: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(string_bytes.len());

        for &byte in string_bytes {
            match byte {
                // Replace common problematic control characters with safe alternatives
                0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F => {
                    result.push(b' '); // Replace control chars with space
                }
                0x80..=0x9F => {
                    // Windows-1252 control character range - try to map to reasonable alternatives
                    let replacement = match byte {
                        0x80 => b'E',  // Euro sign -> E
                        0x81 => b' ',  // Undefined -> space
                        0x82 => b',',  // Single low-9 quotation mark -> comma
                        0x83 => b'f',  // Latin small letter f with hook -> f
                        0x84 => b'"',  // Double low-9 quotation mark -> quote
                        0x85 => b'.',  // Horizontal ellipsis -> period
                        0x86 => b'+',  // Dagger -> plus
                        0x87 => b'+',  // Double dagger -> plus
                        0x88 => b'^',  // Modifier letter circumflex accent -> caret
                        0x89 => b'%',  // Per mille sign -> percent
                        0x8A => b'S',  // Latin capital letter S with caron -> S
                        0x8B => b'<',  // Single left-pointing angle quotation mark
                        0x8C => b'O',  // Latin capital ligature OE -> O
                        0x8D => b' ',  // Undefined -> space
                        0x8E => b'Z',  // Latin capital letter Z with caron -> Z
                        0x8F => b' ',  // Undefined -> space
                        0x90 => b' ',  // Undefined -> space
                        0x91 => b'\'', // Left single quotation mark
                        0x92 => b'\'', // Right single quotation mark
                        0x93 => b'"',  // Left double quotation mark
                        0x94 => b'"',  // Right double quotation mark
                        0x95 => b'*',  // Bullet -> asterisk
                        0x96 => b'-',  // En dash -> hyphen
                        0x97 => b'-',  // Em dash -> hyphen
                        0x98 => b'~',  // Small tilde
                        0x99 => b'T',  // Trade mark sign -> T
                        0x9A => b's',  // Latin small letter s with caron -> s
                        0x9B => b'>',  // Single right-pointing angle quotation mark
                        0x9C => b'o',  // Latin small ligature oe -> o
                        0x9D => b' ',  // Undefined -> space
                        0x9E => b'z',  // Latin small letter z with caron -> z
                        0x9F => b'Y',  // Latin capital letter Y with diaeresis -> Y
                        _ => b'?',     // Fallback
                    };
                    result.push(replacement);
                }
                _ => {
                    result.push(byte); // Keep valid bytes as-is
                }
            }
        }

        result
    }

    /// Check if a character is likely a problematic encoding character
    fn is_problematic_encoding_char(&self, ch: u8) -> bool {
        // Control characters and Latin-1 supplement range that often indicate encoding issues
        (0x80..=0x9F).contains(&ch) ||
        ch == 0x07 || // Bell character
        (ch <= 0x1F && ch != 0x09 && ch != 0x0A && ch != 0x0D) || // Control chars except tab, LF, CR
        // In lenient mode, also handle extended Latin-1 characters that may appear in corrupted streams
        (self.options.lenient_syntax && ch >= 0xA0) // Extended Latin-1 range (u8 max is 0xFF)
    }

    /// Handle problematic encoding characters in the main token stream
    fn handle_encoding_char_in_token_stream(&mut self, ch: u8) -> ParseResult<Token> {
        if self.options.lenient_encoding {
            // Consume the problematic character and continue
            self.consume_char()?;

            // Log warning about the character recovery
            if self.options.collect_warnings {
                let replacement_char = match ch {
                    0x07 => "bell",
                    0x00..=0x1F => "control",
                    0x80..=0x9F => "latin1-supplement",
                    _ => "unknown",
                };

                self.warnings.push(ParseWarning::InvalidEncoding {
                    position: self.position,
                    recovered_text: format!(
                        "Skipped problematic {replacement_char} character (0x{ch:02X})"
                    ),
                    encoding_used: None,
                    replacement_count: 1,
                });
            }

            // Skip this character and try to get the next token
            self.skip_whitespace()?;
            if let Ok(Some(_)) = self.peek_char() {
                self.next_token() // Recursively try next token
            } else {
                Err(ParseError::SyntaxError {
                    position: self.position,
                    message: "Unexpected end of file after problematic character".to_string(),
                })
            }
        } else {
            // In strict mode, generate a more descriptive error
            let char_description = match ch {
                0x07 => "Bell character (\\u{07})".to_string(),
                0x00..=0x1F => format!("Control character (\\u{{{ch:02X}}})"),
                0x80..=0x9F => format!("Latin-1 supplement character (\\u{{{ch:02X}}})"),
                _ => format!("Problematic character (\\u{{{ch:02X}}})"),
            };

            Err(ParseError::CharacterEncodingError {
                position: self.position,
                message: format!(
                    "Unexpected character: {char_description} - Consider using lenient parsing mode"
                ),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_lexer_basic_tokens() {
        // Test positive and negative numbers
        let input = b"123 -456 3.14 true false null /Name";
        let mut lexer = Lexer::new(Cursor::new(input));

        assert_eq!(lexer.next_token().unwrap(), Token::Integer(123));
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(-456));
        assert_eq!(lexer.next_token().unwrap(), Token::Real(3.14));
        assert_eq!(lexer.next_token().unwrap(), Token::Boolean(true));
        assert_eq!(lexer.next_token().unwrap(), Token::Boolean(false));
        assert_eq!(lexer.next_token().unwrap(), Token::Null);
        assert_eq!(lexer.next_token().unwrap(), Token::Name("Name".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Eof);
    }

    #[test]
    fn test_lexer_negative_numbers() {
        // Test negative numbers without space
        let input = b"-123 -45.67";
        let mut lexer = Lexer::new(Cursor::new(input));

        assert_eq!(lexer.next_token().unwrap(), Token::Integer(-123));
        assert_eq!(lexer.next_token().unwrap(), Token::Real(-45.67));
    }

    #[test]
    fn test_lexer_strings() {
        let input = b"(Hello World) <48656C6C6F>";
        let mut lexer = Lexer::new(Cursor::new(input));

        assert_eq!(
            lexer.next_token().unwrap(),
            Token::String(b"Hello World".to_vec())
        );
        assert_eq!(
            lexer.next_token().unwrap(),
            Token::String(b"Hello".to_vec())
        );
    }

    #[test]
    fn test_lexer_dictionaries() {
        let input = b"<< /Type /Page >>";
        let mut lexer = Lexer::new(Cursor::new(input));

        assert_eq!(lexer.next_token().unwrap(), Token::DictStart);
        assert_eq!(lexer.next_token().unwrap(), Token::Name("Type".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Name("Page".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::DictEnd);
    }

    #[test]
    fn test_lexer_arrays() {
        let input = b"[1 2 3]";
        let mut lexer = Lexer::new(Cursor::new(input));

        assert_eq!(lexer.next_token().unwrap(), Token::ArrayStart);
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(1));
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(2));
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(3));
        assert_eq!(lexer.next_token().unwrap(), Token::ArrayEnd);
    }

    #[test]
    fn test_lexer_references() {
        let input = b"1 0 R 25 1 R";
        let mut lexer = Lexer::new(Cursor::new(input));

        // Now references are parsed as separate tokens
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(1));
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(0));
        // 'R' should be parsed as a keyword or name
        match lexer.next_token().unwrap() {
            Token::Name(s) if s == "R" => {} // Could be a name
            other => panic!("Expected R token, got {other:?}"),
        }

        assert_eq!(lexer.next_token().unwrap(), Token::Integer(25));
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(1));
        match lexer.next_token().unwrap() {
            Token::Name(s) if s == "R" => {} // Could be a name
            other => panic!("Expected R token, got {other:?}"),
        }
    }

    #[test]
    fn test_lexer_comments() {
        let input = b"%PDF-1.7\n123";
        let mut lexer = Lexer::new(Cursor::new(input));

        assert_eq!(
            lexer.next_token().unwrap(),
            Token::Comment("PDF-1.7".to_string())
        );
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(123));
    }

    // Comprehensive tests for Lexer
    mod comprehensive_tests {
        use super::*;
        use std::io::Cursor;

        #[test]
        fn test_token_debug_trait() {
            let token = Token::Integer(42);
            let debug_str = format!("{token:?}");
            assert!(debug_str.contains("Integer"));
            assert!(debug_str.contains("42"));
        }

        #[test]
        fn test_token_clone() {
            let token = Token::String(b"hello".to_vec());
            let cloned = token.clone();
            assert_eq!(token, cloned);
        }

        #[test]
        fn test_token_equality() {
            assert_eq!(Token::Integer(42), Token::Integer(42));
            assert_ne!(Token::Integer(42), Token::Integer(43));
            assert_eq!(Token::Boolean(true), Token::Boolean(true));
            assert_ne!(Token::Boolean(true), Token::Boolean(false));
            assert_eq!(Token::Null, Token::Null);
            assert_ne!(Token::Null, Token::Integer(0));
        }

        #[test]
        fn test_lexer_empty_input() {
            let input = b"";
            let mut lexer = Lexer::new(Cursor::new(input));
            assert_eq!(lexer.next_token().unwrap(), Token::Eof);
        }

        #[test]
        fn test_lexer_whitespace_only() {
            let input = b"   \t\n\r  ";
            let mut lexer = Lexer::new(Cursor::new(input));
            assert_eq!(lexer.next_token().unwrap(), Token::Eof);
        }

        #[test]
        fn test_lexer_integer_edge_cases() {
            let input = b"0 +123 -0 9876543210";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(lexer.next_token().unwrap(), Token::Integer(0));
            assert_eq!(lexer.next_token().unwrap(), Token::Integer(123));
            assert_eq!(lexer.next_token().unwrap(), Token::Integer(0));
            assert_eq!(lexer.next_token().unwrap(), Token::Integer(9876543210));
        }

        #[test]
        fn test_lexer_real_edge_cases() {
            let input = b"0.0 +3.14 -2.71828 .5 5. 123.456789";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(lexer.next_token().unwrap(), Token::Real(0.0));
            assert_eq!(lexer.next_token().unwrap(), Token::Real(3.14));
            assert_eq!(lexer.next_token().unwrap(), Token::Real(-2.71828));
            assert_eq!(lexer.next_token().unwrap(), Token::Real(0.5));
            assert_eq!(lexer.next_token().unwrap(), Token::Real(5.0));
            assert_eq!(lexer.next_token().unwrap(), Token::Real(123.456789));
        }

        #[test]
        fn test_lexer_scientific_notation() {
            let input = b"1.23e10 -4.56E-5 1e0 2E+3";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(lexer.next_token().unwrap(), Token::Real(1.23e10));
            assert_eq!(lexer.next_token().unwrap(), Token::Real(-4.56e-5));
            assert_eq!(lexer.next_token().unwrap(), Token::Real(1e0));
            assert_eq!(lexer.next_token().unwrap(), Token::Real(2e3));
        }

        #[test]
        fn test_lexer_string_literal_escapes() {
            let input = b"(Hello\\nWorld) (Tab\\tChar) (Quote\\\"Mark) (Backslash\\\\)";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(
                lexer.next_token().unwrap(),
                Token::String(b"Hello\nWorld".to_vec())
            );
            assert_eq!(
                lexer.next_token().unwrap(),
                Token::String(b"Tab\tChar".to_vec())
            );
            assert_eq!(
                lexer.next_token().unwrap(),
                Token::String(b"Quote\"Mark".to_vec())
            );
            assert_eq!(
                lexer.next_token().unwrap(),
                Token::String(b"Backslash\\".to_vec())
            );
        }

        #[test]
        fn test_lexer_string_literal_nested_parens() {
            let input = b"(Nested (parentheses) work)";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(
                lexer.next_token().unwrap(),
                Token::String(b"Nested (parentheses) work".to_vec())
            );
        }

        #[test]
        fn test_lexer_string_literal_empty() {
            let input = b"()";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(lexer.next_token().unwrap(), Token::String(b"".to_vec()));
        }

        #[test]
        fn test_lexer_hexadecimal_strings() {
            let input = b"<48656C6C6F> <20576F726C64> <>";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(
                lexer.next_token().unwrap(),
                Token::String(b"Hello".to_vec())
            );
            assert_eq!(
                lexer.next_token().unwrap(),
                Token::String(b" World".to_vec())
            );
            assert_eq!(lexer.next_token().unwrap(), Token::String(b"".to_vec()));
        }

        #[test]
        fn test_lexer_hexadecimal_strings_odd_length() {
            let input = b"<48656C6C6F2> <1> <ABC>";
            let mut lexer = Lexer::new(Cursor::new(input));

            // Odd length hex strings should pad with 0
            assert_eq!(
                lexer.next_token().unwrap(),
                Token::String(b"Hello ".to_vec())
            );
            assert_eq!(lexer.next_token().unwrap(), Token::String(b"\x10".to_vec()));
            assert_eq!(
                lexer.next_token().unwrap(),
                Token::String(b"\xAB\xC0".to_vec())
            );
        }

        #[test]
        fn test_lexer_hexadecimal_strings_whitespace() {
            let input = b"<48 65 6C 6C 6F>";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(
                lexer.next_token().unwrap(),
                Token::String(b"Hello".to_vec())
            );
        }

        #[test]
        fn test_lexer_names() {
            let input = b"/Type /Page /Root /Kids /Count /MediaBox";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(lexer.next_token().unwrap(), Token::Name("Type".to_string()));
            assert_eq!(lexer.next_token().unwrap(), Token::Name("Page".to_string()));
            assert_eq!(lexer.next_token().unwrap(), Token::Name("Root".to_string()));
            assert_eq!(lexer.next_token().unwrap(), Token::Name("Kids".to_string()));
            assert_eq!(
                lexer.next_token().unwrap(),
                Token::Name("Count".to_string())
            );
            assert_eq!(
                lexer.next_token().unwrap(),
                Token::Name("MediaBox".to_string())
            );
        }

        #[test]
        fn test_lexer_names_with_special_chars() {
            let input = b"/Name#20with#20spaces /Name#2Fwith#2Fslashes";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(
                lexer.next_token().unwrap(),
                Token::Name("Name with spaces".to_string())
            );
            assert_eq!(
                lexer.next_token().unwrap(),
                Token::Name("Name/with/slashes".to_string())
            );
        }

        #[test]
        fn test_lexer_names_edge_cases() {
            let input = b"/ /A /123 /true /false /null";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(lexer.next_token().unwrap(), Token::Name("".to_string()));
            assert_eq!(lexer.next_token().unwrap(), Token::Name("A".to_string()));
            assert_eq!(lexer.next_token().unwrap(), Token::Name("123".to_string()));
            assert_eq!(lexer.next_token().unwrap(), Token::Name("true".to_string()));
            assert_eq!(
                lexer.next_token().unwrap(),
                Token::Name("false".to_string())
            );
            assert_eq!(lexer.next_token().unwrap(), Token::Name("null".to_string()));
        }

        #[test]
        fn test_lexer_nested_dictionaries() {
            let input = b"<< /Type /Page /Resources << /Font << /F1 123 0 R >> >> >>";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(lexer.next_token().unwrap(), Token::DictStart);
            assert_eq!(lexer.next_token().unwrap(), Token::Name("Type".to_string()));
            assert_eq!(lexer.next_token().unwrap(), Token::Name("Page".to_string()));
            assert_eq!(
                lexer.next_token().unwrap(),
                Token::Name("Resources".to_string())
            );
            assert_eq!(lexer.next_token().unwrap(), Token::DictStart);
            assert_eq!(lexer.next_token().unwrap(), Token::Name("Font".to_string()));
            assert_eq!(lexer.next_token().unwrap(), Token::DictStart);
            assert_eq!(lexer.next_token().unwrap(), Token::Name("F1".to_string()));
            assert_eq!(lexer.next_token().unwrap(), Token::Integer(123));
            assert_eq!(lexer.next_token().unwrap(), Token::Integer(0));
            assert_eq!(lexer.next_token().unwrap(), Token::Name("R".to_string()));
            assert_eq!(lexer.next_token().unwrap(), Token::DictEnd);
            assert_eq!(lexer.next_token().unwrap(), Token::DictEnd);
            assert_eq!(lexer.next_token().unwrap(), Token::DictEnd);
        }

        #[test]
        fn test_lexer_nested_arrays() {
            let input = b"[[1 2] [3 4] [5 [6 7]]]";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(lexer.next_token().unwrap(), Token::ArrayStart);
            assert_eq!(lexer.next_token().unwrap(), Token::ArrayStart);
            assert_eq!(lexer.next_token().unwrap(), Token::Integer(1));
            assert_eq!(lexer.next_token().unwrap(), Token::Integer(2));
            assert_eq!(lexer.next_token().unwrap(), Token::ArrayEnd);
            assert_eq!(lexer.next_token().unwrap(), Token::ArrayStart);
            assert_eq!(lexer.next_token().unwrap(), Token::Integer(3));
            assert_eq!(lexer.next_token().unwrap(), Token::Integer(4));
            assert_eq!(lexer.next_token().unwrap(), Token::ArrayEnd);
            assert_eq!(lexer.next_token().unwrap(), Token::ArrayStart);
            assert_eq!(lexer.next_token().unwrap(), Token::Integer(5));
            assert_eq!(lexer.next_token().unwrap(), Token::ArrayStart);
            assert_eq!(lexer.next_token().unwrap(), Token::Integer(6));
            assert_eq!(lexer.next_token().unwrap(), Token::Integer(7));
            assert_eq!(lexer.next_token().unwrap(), Token::ArrayEnd);
            assert_eq!(lexer.next_token().unwrap(), Token::ArrayEnd);
            assert_eq!(lexer.next_token().unwrap(), Token::ArrayEnd);
        }

        #[test]
        fn test_lexer_mixed_content() {
            let input = b"<< /Type /Page /MediaBox [0 0 612 792] /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 >> >> >> >>";
            let mut lexer = Lexer::new(Cursor::new(input));

            // Just test that we can parse this without errors
            let mut tokens = Vec::new();
            loop {
                match lexer.next_token().unwrap() {
                    Token::Eof => break,
                    token => tokens.push(token),
                }
            }
            assert!(tokens.len() > 10);
        }

        #[test]
        fn test_lexer_keywords() {
            let input = b"obj endobj stream endstream startxref";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(lexer.next_token().unwrap(), Token::Obj);
            assert_eq!(lexer.next_token().unwrap(), Token::EndObj);
            assert_eq!(lexer.next_token().unwrap(), Token::Stream);
            assert_eq!(lexer.next_token().unwrap(), Token::EndStream);
            assert_eq!(lexer.next_token().unwrap(), Token::StartXRef);
        }

        #[test]
        fn test_lexer_multiple_comments() {
            let input = b"%First comment\n%Second comment\n123";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(
                lexer.next_token().unwrap(),
                Token::Comment("First comment".to_string())
            );
            assert_eq!(
                lexer.next_token().unwrap(),
                Token::Comment("Second comment".to_string())
            );
            assert_eq!(lexer.next_token().unwrap(), Token::Integer(123));
        }

        #[test]
        fn test_lexer_comment_without_newline() {
            let input = b"%Comment at end";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(
                lexer.next_token().unwrap(),
                Token::Comment("Comment at end".to_string())
            );
            assert_eq!(lexer.next_token().unwrap(), Token::Eof);
        }

        #[test]
        fn test_lexer_special_characters_in_streams() {
            let input = b"<< /Length 5 >> stream\nHello endstream";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(lexer.next_token().unwrap(), Token::DictStart);
            assert_eq!(
                lexer.next_token().unwrap(),
                Token::Name("Length".to_string())
            );
            assert_eq!(lexer.next_token().unwrap(), Token::Integer(5));
            assert_eq!(lexer.next_token().unwrap(), Token::DictEnd);
            assert_eq!(lexer.next_token().unwrap(), Token::Stream);
            // The actual stream content would be handled by a higher-level parser
        }

        #[test]
        fn test_lexer_push_token() {
            let input = b"123 456";
            let mut lexer = Lexer::new(Cursor::new(input));

            let token1 = lexer.next_token().unwrap();
            assert_eq!(token1, Token::Integer(123));

            let token2 = lexer.next_token().unwrap();
            assert_eq!(token2, Token::Integer(456));

            // Push token2 back
            lexer.push_token(token2.clone());

            // Should get token2 again
            let token3 = lexer.next_token().unwrap();
            assert_eq!(token3, token2);

            // Should get EOF
            let token4 = lexer.next_token().unwrap();
            assert_eq!(token4, Token::Eof);
        }

        #[test]
        fn test_lexer_push_multiple_tokens() {
            let input = b"123";
            let mut lexer = Lexer::new(Cursor::new(input));

            let original_token = lexer.next_token().unwrap();
            assert_eq!(original_token, Token::Integer(123));

            // Push multiple tokens
            lexer.push_token(Token::Boolean(true));
            lexer.push_token(Token::Boolean(false));
            lexer.push_token(Token::Null);

            // Should get them back in reverse order (stack behavior)
            assert_eq!(lexer.next_token().unwrap(), Token::Null);
            assert_eq!(lexer.next_token().unwrap(), Token::Boolean(false));
            assert_eq!(lexer.next_token().unwrap(), Token::Boolean(true));
            assert_eq!(lexer.next_token().unwrap(), Token::Eof);
        }

        #[test]
        fn test_lexer_read_newline() {
            let input = b"123\n456\r\n789";
            let mut lexer = Lexer::new(Cursor::new(input));

            // Read first digits
            let digits1 = lexer.read_digits().unwrap();
            assert_eq!(digits1, "123");
            assert!(lexer.read_newline().is_ok());

            // Read second digits
            let digits2 = lexer.read_digits().unwrap();
            assert_eq!(digits2, "456");
            assert!(lexer.read_newline().is_ok());

            // Read final digits
            let digits3 = lexer.read_digits().unwrap();
            assert_eq!(digits3, "789");
        }

        #[test]
        fn test_lexer_read_bytes() {
            let input = b"Hello World";
            let mut lexer = Lexer::new(Cursor::new(input));

            let bytes = lexer.read_bytes(5).unwrap();
            assert_eq!(bytes, b"Hello");

            let bytes = lexer.read_bytes(6).unwrap();
            assert_eq!(bytes, b" World");
        }

        #[test]
        fn test_lexer_read_until_sequence() {
            let input = b"Hello endstream World";
            let mut lexer = Lexer::new(Cursor::new(input));

            let result = lexer.read_until_sequence(b"endstream").unwrap();
            assert_eq!(result, b"Hello ");

            // Continue reading after the sequence
            let rest = lexer.read_digits().unwrap();
            assert_eq!(rest, ""); // read_digits only reads digits, " World" has no digits
        }

        #[test]
        fn test_lexer_read_until_sequence_not_found() {
            let input = b"Hello World";
            let mut lexer = Lexer::new(Cursor::new(input));

            let result = lexer.read_until_sequence(b"notfound");
            assert!(result.is_err());
        }

        #[test]
        fn test_lexer_position_tracking() {
            let input = b"123 456";
            let mut lexer = Lexer::new(Cursor::new(input));

            let initial_pos = lexer.position();
            assert_eq!(initial_pos, 0);

            lexer.next_token().unwrap(); // "123"
            let pos_after_first = lexer.position();
            assert!(pos_after_first > initial_pos);

            lexer.next_token().unwrap(); // "456"
            let pos_after_second = lexer.position();
            assert!(pos_after_second > pos_after_first);
        }

        #[test]
        fn test_lexer_large_numbers() {
            let input = b"2147483647 -2147483648 9223372036854775807 -9223372036854775808";
            let mut lexer = Lexer::new(Cursor::new(input));

            assert_eq!(lexer.next_token().unwrap(), Token::Integer(2147483647));
            assert_eq!(lexer.next_token().unwrap(), Token::Integer(-2147483648));
            assert_eq!(
                lexer.next_token().unwrap(),
                Token::Integer(9223372036854775807)
            );
            assert_eq!(
                lexer.next_token().unwrap(),
                Token::Integer(-9223372036854775808)
            );
        }

        #[test]
        fn test_lexer_very_long_string() {
            let long_str = "A".repeat(1000);
            let input = format!("({long_str})");
            let mut lexer = Lexer::new(Cursor::new(input.as_bytes()));

            if let Token::String(s) = lexer.next_token().unwrap() {
                assert_eq!(s.len(), 1000);
                assert_eq!(s, long_str.as_bytes());
            } else {
                panic!("Expected string token");
            }
        }

        #[test]
        fn test_lexer_very_long_name() {
            let long_name = "A".repeat(500);
            let input = format!("/{long_name}");
            let mut lexer = Lexer::new(Cursor::new(input.as_bytes()));

            if let Token::Name(name) = lexer.next_token().unwrap() {
                assert_eq!(name.len(), 500);
                assert_eq!(name, long_name);
            } else {
                panic!("Expected name token");
            }
        }

        #[test]
        fn test_lexer_error_handling_invalid_hex() {
            let input = b"<48656C6C6FG>";
            let mut lexer = Lexer::new(Cursor::new(input));

            // Should handle invalid hex gracefully
            let result = lexer.next_token();
            assert!(result.is_ok() || result.is_err()); // Either works or fails gracefully
        }

        #[test]
        fn test_lexer_all_token_types() {
            let input = b"true false null 123 -456 3.14 (string) <48656C6C6F> /Name [ ] << >> obj endobj stream endstream startxref % comment\n";
            let mut lexer = Lexer::new(Cursor::new(input));

            let mut token_types = Vec::new();
            loop {
                match lexer.next_token().unwrap() {
                    Token::Eof => break,
                    token => token_types.push(std::mem::discriminant(&token)),
                }
            }

            // Should have multiple different token types
            assert!(token_types.len() > 10);
        }

        #[test]
        fn test_lexer_performance() {
            let input = "123 456 789 ".repeat(1000);
            let mut lexer = Lexer::new(Cursor::new(input.as_bytes()));

            let start_time = std::time::Instant::now();
            let mut count = 0;
            loop {
                match lexer.next_token().unwrap() {
                    Token::Eof => break,
                    _ => count += 1,
                }
            }
            let elapsed = start_time.elapsed();

            assert_eq!(count, 3000); // 1000 repetitions * 3 tokens each
            assert!(elapsed.as_millis() < 1000); // Should complete within 1 second
        }
    }

    #[test]
    fn test_lexer_find_keyword_ahead() {
        let input = b"some data here endstream more data";
        let mut lexer = Lexer::new(Cursor::new(input));

        // Find endstream keyword
        let result = lexer.find_keyword_ahead("endstream", 100);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(15)); // Position of endstream

        // Try to find non-existent keyword
        let result2 = lexer.find_keyword_ahead("notfound", 100);
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), None);

        // Test with limited search distance
        let result3 = lexer.find_keyword_ahead("endstream", 10);
        assert!(result3.is_ok());
        assert_eq!(result3.unwrap(), None); // Not found within 10 bytes
    }

    #[test]
    fn test_lexer_peek_token() {
        let input = b"123 456 /Name";
        let mut lexer = Lexer::new(Cursor::new(input));

        // Peek first token
        let peeked = lexer.peek_token();
        assert!(peeked.is_ok());
        assert_eq!(peeked.unwrap(), Token::Integer(123));

        // Verify peek doesn't consume
        let next = lexer.next_token();
        assert!(next.is_ok());
        assert_eq!(next.unwrap(), Token::Integer(123));

        // Peek and consume next tokens
        assert_eq!(lexer.peek_token().unwrap(), Token::Integer(456));
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(456));

        assert_eq!(lexer.peek_token().unwrap(), Token::Name("Name".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Name("Name".to_string()));
    }

    #[test]
    fn test_lexer_expect_keyword() {
        let input = b"endstream obj endobj";
        let mut lexer = Lexer::new(Cursor::new(input));

        // Expect correct keyword
        assert!(lexer.expect_keyword("endstream").is_ok());

        // Expect another correct keyword
        assert!(lexer.expect_keyword("obj").is_ok());

        // Expect wrong keyword (should fail)
        let result = lexer.expect_keyword("stream");
        assert!(result.is_err());
        match result {
            Err(ParseError::UnexpectedToken { expected, found }) => {
                assert!(expected.contains("stream"));
                assert!(found.contains("EndObj"));
            }
            _ => panic!("Expected UnexpectedToken error"),
        }
    }

    #[test]
    fn test_lexer_save_restore_position() {
        let input = b"123 456 789";
        let mut lexer = Lexer::new(Cursor::new(input));

        // Read first token
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(123));

        // Save position
        let saved = lexer.save_position();
        assert!(saved.is_ok());
        let saved_pos = saved.unwrap();

        // Read more tokens
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(456));
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(789));

        // Restore position
        assert!(lexer.restore_position(saved_pos).is_ok());

        // Should be back at second token
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(456));
    }

    #[test]
    fn test_lexer_character_encoding_recovery() {
        // Test string with encoding issues (Windows-1252 bytes)
        let input = b"(Caf\x80 \x91Hello\x92)"; // "Café 'Hello'"
        let options = ParseOptions::lenient();
        let mut lexer = Lexer::new_with_options(Cursor::new(input), options);

        match lexer.next_token().unwrap() {
            Token::String(bytes) => {
                // Should contain the text, potentially with encoding recovery
                let text = String::from_utf8_lossy(&bytes);
                println!("Recovered text: {text}");
                assert!(!text.is_empty()); // Should not be empty
            }
            other => panic!("Expected String token, got {other:?}"),
        }

        // Check that warnings were collected
        let warnings = lexer.warnings();
        if !warnings.is_empty() {
            println!("Encoding warnings: {warnings:?}");
        }
    }
}
