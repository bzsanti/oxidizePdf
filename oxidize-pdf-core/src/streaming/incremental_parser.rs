//! Incremental, byte-preserving parsing of standalone PDF objects and classic xref tables.
//! Streams require a direct integer Length. Indirect lengths, encrypted input and
//! compressed object/xref streams require the seekable PdfReader instead.
//! Call `finish` at physical EOF to detect truncation. Events are syntactic input
//! records, not proof of document validity or selection of its latest revision.

use crate::error::{PdfError, Result};
use crate::parser::lexer::{Lexer, Token};
use crate::parser::{ParseOptions, PdfDictionary, PdfObject};
use std::io::{Cursor, Read};

/// Events emitted during incremental parsing
#[derive(Debug)]
pub enum ParseEvent {
    /// PDF header found
    Header { version: String },
    /// Object definition identified (emitted after the full object is validated)
    ObjectStart { id: u32, generation: u16 },
    /// Object definition completed
    ObjectEnd {
        id: u32,
        generation: u16,
        object: PdfObject,
    },
    /// Stream data chunk
    StreamData { object_id: u32, data: Vec<u8> },
    /// Cross-reference table found
    XRef { entries: Vec<XRefEntry> },
    /// Trailer dictionary found
    Trailer { dict: PdfDictionary },
    /// End of file marker
    EndOfFile,
}

/// Cross-reference table entry
#[derive(Debug, Clone)]
pub struct XRefEntry {
    pub object_number: u32,
    pub generation: u16,
    pub offset: u64,
    pub in_use: bool,
}

const MAX_BUFFER: usize = 16 * 1024 * 1024;
const MAX_DEPTH: usize = 128;

#[derive(Debug)]
enum Scan {
    Normal,
    Word(usize),
    Literal { depth: usize, escaped: bool },
    Hex,
    Comment(usize),
    StreamEol(usize),
    StreamBytes(usize),
}

/// Incremental syntax reader with a 16 MiB pending-record limit and depth 128.
/// Unsupported structures fail explicitly; no placeholder objects are emitted.
pub struct IncrementalParser {
    buffer: Vec<u8>,
    pos: usize,
    scan: Scan,
    depth: usize,
    header: bool,
    tail: bool,
    complete: bool,
    failed: bool,
    events: Vec<ParseEvent>,
}
impl Default for IncrementalParser {
    fn default() -> Self {
        Self::new()
    }
}
impl IncrementalParser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            pos: 0,
            scan: Scan::Normal,
            depth: 0,
            header: false,
            tail: false,
            complete: false,
            failed: false,
            events: Vec::new(),
        }
    }
    /// Feed raw bytes. Events can be drained after each feed. An error is terminal.
    /// ObjectStart/StreamData/ObjectEnd are emitted together for each complete object.
    pub fn feed(&mut self, data: &[u8]) -> Result<()> {
        if self.failed {
            return Err(invalid("Parser failed previously"));
        }
        let result = self.feed_inner(data);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn feed_inner(&mut self, data: &[u8]) -> Result<()> {
        let mut remaining = data;
        while !remaining.is_empty() {
            let available = MAX_BUFFER - self.buffer.len();
            if available == 0 {
                return Err(invalid("Incremental record exceeds 16 MiB"));
            }
            let count = remaining.len().min(4096).min(available);
            self.buffer.extend_from_slice(&remaining[..count]);
            remaining = &remaining[count..];
            self.advance()?;
        }
        Ok(())
    }
    /// Validate physical EOF. A missing header, unfinished object or absent EOF fails.
    pub fn finish(&mut self) -> Result<()> {
        if self.failed {
            return Err(invalid("Parser failed previously"));
        }
        if !self.complete {
            self.failed = true;
            return Err(invalid("Truncated PDF: missing complete trailer and EOF"));
        }
        Ok(())
    }
    pub fn take_events(&mut self) -> Vec<ParseEvent> {
        std::mem::take(&mut self.events)
    }
    pub fn is_complete(&self) -> bool {
        self.complete && !self.failed
    }

    fn advance(&mut self) -> Result<()> {
        if !self.header {
            let Some(end) = self.buffer.iter().position(|b| matches!(b, b'\r' | b'\n')) else {
                if self.buffer.len() > 32 {
                    return Err(PdfError::InvalidHeader);
                }
                return Ok(());
            };
            let raw = &self.buffer[..end];
            if raw.len() != 8
                || !raw.starts_with(b"%PDF-")
                || !raw[5].is_ascii_digit()
                || raw[6] != b'.'
                || !raw[7].is_ascii_digit()
            {
                return Err(PdfError::InvalidHeader);
            }
            let version =
                String::from_utf8(raw[5..].to_vec()).map_err(|_| PdfError::InvalidHeader)?;
            self.events.push(ParseEvent::Header { version });
            self.buffer.drain(..=end);
            self.header = true;
        }
        while self.pos < self.buffer.len() {
            if self.complete {
                if !white(self.buffer[self.pos]) {
                    return Err(invalid("Additional PDF revisions/data require PdfReader"));
                }
                self.pos += 1;
                continue;
            }
            let byte = self.buffer[self.pos];
            match self.scan {
                Scan::Normal => match byte {
                    b'%' => {
                        self.scan = Scan::Comment(self.pos);
                        self.pos += 1;
                    }
                    b'(' => {
                        self.scan = Scan::Literal {
                            depth: 1,
                            escaped: false,
                        };
                        self.pos += 1;
                    }
                    b'<' => {
                        if self.pos + 1 == self.buffer.len() {
                            break;
                        }
                        if self.buffer[self.pos + 1] == b'<' {
                            self.open()?;
                            self.pos += 2;
                        } else {
                            self.scan = Scan::Hex;
                            self.pos += 1;
                        }
                    }
                    b'>' => {
                        if self.pos + 1 == self.buffer.len() {
                            break;
                        }
                        if self.buffer[self.pos + 1] != b'>' {
                            return Err(invalid("Unexpected >"));
                        }
                        self.close()?;
                        self.pos += 2;
                    }
                    b'[' => {
                        self.open()?;
                        self.pos += 1;
                    }
                    b']' => {
                        self.close()?;
                        self.pos += 1;
                    }
                    b')' => return Err(invalid("Unexpected )")),
                    b if white(b) => {
                        self.pos += 1;
                    }
                    _ => {
                        self.scan = Scan::Word(self.pos);
                        self.pos += 1;
                    }
                },
                Scan::Word(start) => {
                    // Slash belongs to a name only at the start, otherwise it delimits a token.
                    if delimiter(byte) {
                        let word = &self.buffer[start..self.pos];
                        self.scan = Scan::Normal;
                        if word == b"xref" && self.depth == 0 {
                            self.tail = true;
                        } else if word == b"stream" {
                            if self.depth != 0 || self.tail {
                                return Err(invalid("Streams must be top-level indirect objects"));
                            }
                            let length = self.stream_length(start)?;
                            self.scan = Scan::StreamEol(length);
                        } else if word == b"endobj" && self.depth == 0 && !self.tail {
                            self.emit_object(self.pos)?;
                            self.buffer.drain(..self.pos);
                            self.pos = 0;
                        }
                    } else {
                        self.pos += 1;
                    }
                }
                Scan::Literal { depth, escaped } => {
                    self.pos += 1;
                    if escaped {
                        self.scan = Scan::Literal {
                            depth,
                            escaped: false,
                        };
                    } else if byte == b'\\' {
                        self.scan = Scan::Literal {
                            depth,
                            escaped: true,
                        };
                    } else if byte == b'(' {
                        if depth >= MAX_DEPTH {
                            return Err(invalid("Literal nesting exceeds 128"));
                        }
                        self.scan = Scan::Literal {
                            depth: depth + 1,
                            escaped: false,
                        };
                    } else if byte == b')' {
                        self.scan = if depth == 1 {
                            Scan::Normal
                        } else {
                            Scan::Literal {
                                depth: depth - 1,
                                escaped: false,
                            }
                        };
                    }
                }
                Scan::Hex => {
                    self.pos += 1;
                    if byte == b'>' {
                        self.scan = Scan::Normal;
                    }
                }
                Scan::Comment(start) => {
                    if matches!(byte, b'\r' | b'\n') {
                        self.scan = Scan::Normal;
                    } else {
                        self.pos += 1;
                        if self.tail && self.depth == 0 && &self.buffer[start..self.pos] == b"%%EOF"
                        {
                            self.emit_tail(start)?;
                            self.events.push(ParseEvent::EndOfFile);
                            self.complete = true;
                            self.scan = Scan::Normal;
                        }
                    }
                }
                Scan::StreamEol(length) => {
                    let skip = match byte {
                        b'\n' => 1,
                        b'\r' => {
                            if self.pos + 1 == self.buffer.len() {
                                break;
                            }
                            if self.buffer[self.pos + 1] == b'\n' {
                                2
                            } else {
                                1
                            }
                        }
                        _ => return Err(invalid("Stream keyword must be followed by EOL")),
                    };
                    self.pos += skip;
                    self.scan = Scan::StreamBytes(length);
                }
                Scan::StreamBytes(left) => {
                    let take = left.min(self.buffer.len() - self.pos);
                    self.pos += take;
                    self.scan = if take == left {
                        Scan::Normal
                    } else {
                        Scan::StreamBytes(left - take)
                    };
                }
            }
        }
        // Avoid retaining arbitrary trailing whitespace after EOF.
        if self.complete {
            self.buffer.clear();
            self.pos = 0;
        }
        Ok(())
    }
    fn open(&mut self) -> Result<()> {
        if self.depth >= MAX_DEPTH {
            return Err(invalid("Object nesting exceeds 128"));
        }
        self.depth += 1;
        Ok(())
    }
    fn close(&mut self) -> Result<()> {
        self.depth = self
            .depth
            .checked_sub(1)
            .ok_or_else(|| invalid("Unbalanced object delimiters"))?;
        Ok(())
    }
    fn stream_length(&self, end: usize) -> Result<usize> {
        let mut lexer =
            Lexer::new_with_options(Cursor::new(&self.buffer[..end]), ParseOptions::strict());
        object_header(&mut lexer)?;
        // This slice ends before stream, so dictionary parsing cannot reserve Length bytes.
        let object = PdfObject::parse_with_options(&mut lexer, &ParseOptions::strict())?;
        let dict = object
            .as_dict()
            .ok_or_else(|| invalid("Stream requires dictionary"))?;
        if matches!(dict.get_type(), Some("ObjStm" | "XRef")) {
            return Err(invalid("Compressed objects/xref require PdfReader"));
        }
        let length=dict.get("Length").and_then(PdfObject::as_integer).ok_or_else(||invalid("Incremental streams require direct integer Length; use PdfReader for indirect lengths"))?;
        let length = usize::try_from(length).map_err(|_| invalid("Invalid stream Length"))?;
        if length > MAX_BUFFER.saturating_sub(end) {
            return Err(invalid("Incremental stream exceeds 16 MiB"));
        }
        Ok(length)
    }
    fn emit_object(&mut self, end: usize) -> Result<()> {
        let mut lexer =
            Lexer::new_with_options(Cursor::new(&self.buffer[..end]), ParseOptions::strict());
        let (id, generation) = object_header(&mut lexer)?;
        let object = PdfObject::parse_with_options(&mut lexer, &ParseOptions::strict())?;
        if significant(&mut lexer)? != Token::EndObj {
            return Err(invalid("Missing endobj"));
        }
        self.events.push(ParseEvent::ObjectStart { id, generation });
        if let Some(stream) = object.as_stream() {
            self.events.push(ParseEvent::StreamData {
                object_id: id,
                data: stream.raw_data().to_vec(),
            });
        }
        self.events.push(ParseEvent::ObjectEnd {
            id,
            generation,
            object,
        });
        Ok(())
    }
    fn emit_tail(&mut self, end: usize) -> Result<()> {
        let mut offset = 0;
        let raw = &self.buffer[..end];
        if ascii_token(raw, &mut offset)? != b"xref" {
            return Err(invalid("Expected classic xref"));
        }
        let mut entries = Vec::new();
        loop {
            let token = ascii_token(raw, &mut offset)?;
            if token == b"trailer" {
                break;
            }
            let start = ascii_integer(token)?;
            let count = ascii_integer(ascii_token(raw, &mut offset)?)?;
            if count > raw.len() as u64 {
                return Err(invalid("Invalid xref subsection length"));
            }
            for i in 0..count {
                let entry_offset = ascii_integer(ascii_token(raw, &mut offset)?)?;
                let generation = u16::try_from(ascii_integer(ascii_token(raw, &mut offset)?)?)
                    .map_err(|_| invalid("Invalid xref generation"))?;
                let in_use = match ascii_token(raw, &mut offset)? {
                    b"n" => true,
                    b"f" => false,
                    _ => return Err(invalid("Invalid xref entry")),
                };
                let object_number = u32::try_from(
                    start
                        .checked_add(i)
                        .ok_or_else(|| invalid("Invalid object number"))?,
                )
                .map_err(|_| invalid("Invalid object number"))?;
                entries.push(XRefEntry {
                    object_number,
                    generation,
                    offset: entry_offset,
                    in_use,
                });
            }
        }
        let mut lexer =
            Lexer::new_with_options(Cursor::new(&raw[offset..]), ParseOptions::strict());
        let object = PdfObject::parse_with_options(&mut lexer, &ParseOptions::strict())?;
        let dict = object
            .as_dict()
            .ok_or_else(|| invalid("Trailer must be dictionary"))?
            .clone();
        if dict.contains_key("Encrypt") || dict.contains_key("XRefStm") || dict.contains_key("Prev")
        {
            return Err(invalid(
                "Encryption, hybrid xref and incremental revisions require PdfReader",
            ));
        }
        if significant(&mut lexer)? != Token::StartXRef {
            return Err(invalid("Missing startxref"));
        }
        integer(significant(&mut lexer)?)?;
        if significant(&mut lexer)? != Token::Eof {
            return Err(invalid("Unexpected bytes before EOF"));
        }
        self.events.push(ParseEvent::XRef { entries });
        self.events.push(ParseEvent::Trailer { dict });
        Ok(())
    }
}
fn ascii_token<'a>(bytes: &'a [u8], offset: &mut usize) -> Result<&'a [u8]> {
    loop {
        while *offset < bytes.len() && white(bytes[*offset]) {
            *offset += 1;
        }
        if bytes.get(*offset) != Some(&b'%') {
            break;
        }
        while *offset < bytes.len() && !matches!(bytes[*offset], b'\r' | b'\n') {
            *offset += 1;
        }
    }
    let start = *offset;
    while *offset < bytes.len() && !delimiter(bytes[*offset]) {
        *offset += 1;
    }
    if start == *offset {
        return Err(invalid("Missing xref token"));
    }
    Ok(&bytes[start..*offset])
}
fn ascii_integer(bytes: &[u8]) -> Result<u64> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return Err(invalid("Invalid xref integer"));
    }
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| invalid("Invalid xref integer"))
}
fn invalid(message: &str) -> PdfError {
    PdfError::ParseError(message.into())
}
fn white(b: u8) -> bool {
    matches!(b, 0 | 9 | 10 | 12 | 13 | 32)
}
fn delimiter(b: u8) -> bool {
    white(b) || matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'/' | b'%')
}
fn significant<R: Read>(lexer: &mut Lexer<R>) -> crate::parser::ParseResult<Token> {
    loop {
        let token = lexer.next_token()?;
        if !matches!(token, Token::Comment(_)) {
            return Ok(token);
        }
    }
}
fn integer(token: Token) -> Result<u64> {
    match token {
        Token::Integer(n) if n >= 0 => Ok(n as u64),
        _ => Err(invalid("Expected nonnegative integer")),
    }
}
fn object_header<R: Read>(lexer: &mut Lexer<R>) -> Result<(u32, u16)> {
    let id = u32::try_from(integer(significant(lexer)?)?)
        .map_err(|_| invalid("Invalid object number"))?;
    let generation =
        u16::try_from(integer(significant(lexer)?)?).map_err(|_| invalid("Invalid generation"))?;
    if significant(lexer)? != Token::Obj {
        return Err(invalid("Expected obj"));
    }
    Ok((id, generation))
}
/// Read to physical EOF, emitting actual syntax records. Truncated input is an error.
pub fn process_incrementally<R: Read, F>(mut reader: R, mut callback: F) -> Result<()>
where
    F: FnMut(ParseEvent) -> Result<()>,
{
    let mut parser = IncrementalParser::new();
    let mut buffer = [0; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                parser.feed(&buffer[..n])?;
                for event in parser.take_events() {
                    callback(event)?;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(PdfError::Io(e)),
        }
    }
    parser.finish()?;
    for event in parser.take_events() {
        callback(event)?;
    }
    Ok(())
}
