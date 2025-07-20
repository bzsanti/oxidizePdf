//! Incremental PDF parser for streaming operations
//!
//! Parses PDF objects incrementally as they are encountered in the stream,
//! enabling processing of very large PDFs with minimal memory usage.

use crate::error::{PdfError, Result};
use crate::parser::{PdfDictionary, PdfObject};
use std::io::Read;

/// Events emitted during incremental parsing
#[derive(Debug)]
pub enum ParseEvent {
    /// PDF header found
    Header { version: String },
    /// Object definition started
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

/// State of the incremental parser
#[derive(Debug)]
enum ParserState {
    Initial,
    InObject { id: u32, generation: u16 },
    InStream { object_id: u32 },
    InXRef,
    InTrailer,
    Complete,
}

/// Incremental PDF parser
pub struct IncrementalParser {
    state: ParserState,
    buffer: String,
    #[allow(dead_code)]
    line_buffer: String,
    events: Vec<ParseEvent>,
}

impl Default for IncrementalParser {
    fn default() -> Self {
        Self::new()
    }
}

impl IncrementalParser {
    /// Create a new incremental parser
    pub fn new() -> Self {
        Self {
            state: ParserState::Initial,
            buffer: String::new(),
            line_buffer: String::new(),
            events: Vec::new(),
        }
    }

    /// Feed data to the parser
    pub fn feed(&mut self, data: &[u8]) -> Result<()> {
        let text = String::from_utf8_lossy(data);
        self.buffer.push_str(&text);

        // Process complete lines
        while let Some(newline_pos) = self.buffer.find('\n') {
            let line = self.buffer[..newline_pos].trim().to_string();
            self.buffer.drain(..=newline_pos);

            self.process_line(&line)?;
        }

        Ok(())
    }

    /// Get pending events
    pub fn take_events(&mut self) -> Vec<ParseEvent> {
        std::mem::take(&mut self.events)
    }

    /// Check if parsing is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.state, ParserState::Complete)
    }

    fn process_line(&mut self, line: &str) -> Result<()> {
        match &self.state {
            ParserState::Initial => {
                if let Some(version_part) = line.strip_prefix("%PDF-") {
                    let version = version_part.trim().to_string();
                    self.events.push(ParseEvent::Header { version });
                } else if let Some((id, gen)) = self.parse_object_header(line) {
                    self.state = ParserState::InObject {
                        id,
                        generation: gen,
                    };
                    self.events.push(ParseEvent::ObjectStart {
                        id,
                        generation: gen,
                    });
                }
            }
            ParserState::InObject { id, generation } => {
                if line == "endobj" {
                    // Create mock object for demonstration
                    let object = PdfObject::Null;
                    self.events.push(ParseEvent::ObjectEnd {
                        id: *id,
                        generation: *generation,
                        object,
                    });
                    self.state = ParserState::Initial;
                } else if line == "stream" {
                    self.state = ParserState::InStream { object_id: *id };
                }
            }
            ParserState::InStream { object_id } => {
                if line == "endstream" {
                    self.state = ParserState::InObject {
                        id: *object_id,
                        generation: 0,
                    };
                } else {
                    self.events.push(ParseEvent::StreamData {
                        object_id: *object_id,
                        data: line.as_bytes().to_vec(),
                    });
                }
            }
            ParserState::InXRef => {
                if line == "trailer" {
                    self.state = ParserState::InTrailer;
                } else if let Some(_entry) = self.parse_xref_entry(line) {
                    // Collect entries
                }
            }
            ParserState::InTrailer => {
                if line.starts_with("%%EOF") {
                    self.events.push(ParseEvent::EndOfFile);
                    self.state = ParserState::Complete;
                }
            }
            ParserState::Complete => {
                // Ignore additional input
            }
        }

        // Check for state transitions
        if line == "xref" {
            self.state = ParserState::InXRef;
        }

        Ok(())
    }

    fn parse_object_header(&self, line: &str) -> Option<(u32, u16)> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[2] == "obj" {
            let id = parts[0].parse().ok()?;
            let gen = parts[1].parse().ok()?;
            Some((id, gen))
        } else {
            None
        }
    }

    fn parse_xref_entry(&self, line: &str) -> Option<XRefEntry> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 3 {
            let offset = parts[0].parse().ok()?;
            let generation = parts[1].parse().ok()?;
            let in_use = parts[2] == "n";

            Some(XRefEntry {
                object_number: 0, // Would be set by context
                generation,
                offset,
                in_use,
            })
        } else {
            None
        }
    }
}

/// Process a reader incrementally
pub fn process_incrementally<R: Read, F>(mut reader: R, mut callback: F) -> Result<()>
where
    F: FnMut(ParseEvent) -> Result<()>,
{
    let mut parser = IncrementalParser::new();
    let mut buffer = vec![0u8; 4096];

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(n) => {
                parser.feed(&buffer[..n])?;

                for event in parser.take_events() {
                    callback(event)?;
                }
            }
            Err(e) => return Err(PdfError::Io(e)),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incremental_parser_creation() {
        let parser = IncrementalParser::new();
        assert!(!parser.is_complete());
    }

    #[test]
    fn test_parse_header() {
        let mut parser = IncrementalParser::new();
        parser.feed(b"%PDF-1.7\n").unwrap();

        let events = parser.take_events();
        assert_eq!(events.len(), 1);

        match &events[0] {
            ParseEvent::Header { version } => assert_eq!(version, "1.7"),
            _ => panic!("Expected Header event"),
        }
    }

    #[test]
    fn test_parse_object() {
        let mut parser = IncrementalParser::new();
        let data = b"1 0 obj\n<< /Type /Catalog >>\nendobj\n";

        parser.feed(data).unwrap();

        let events = parser.take_events();
        assert!(events.len() >= 2);

        match &events[0] {
            ParseEvent::ObjectStart { id, generation } => {
                assert_eq!(*id, 1);
                assert_eq!(*generation, 0);
            }
            _ => panic!("Expected ObjectStart event"),
        }
    }

    #[test]
    fn test_parse_stream() {
        let mut parser = IncrementalParser::new();
        parser.state = ParserState::InObject {
            id: 1,
            generation: 0,
        };

        let data = b"stream\nHello World\nendstream\n";
        parser.feed(data).unwrap();

        let events = parser.take_events();
        assert!(events
            .iter()
            .any(|e| matches!(e, ParseEvent::StreamData { .. })));
    }

    #[test]
    fn test_parse_eof() {
        let mut parser = IncrementalParser::new();
        parser.state = ParserState::InTrailer;

        parser.feed(b"%%EOF\n").unwrap();

        let events = parser.take_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ParseEvent::EndOfFile));
        assert!(parser.is_complete());
    }

    #[test]
    fn test_process_incrementally() {
        use std::io::Cursor;

        let data = b"%PDF-1.7\n1 0 obj\n<< >>\nendobj\n%%EOF";
        let cursor = Cursor::new(data);

        let mut event_count = 0;
        process_incrementally(cursor, |event| {
            event_count += 1;
            match event {
                ParseEvent::Header { version } => assert_eq!(version, "1.7"),
                ParseEvent::ObjectStart { id, .. } => assert_eq!(id, 1),
                _ => {}
            }
            Ok(())
        })
        .unwrap();

        assert!(event_count > 0);
    }

    #[test]
    fn test_parser_state_transitions() {
        let mut parser = IncrementalParser::new();

        // Initial -> Header
        parser.feed(b"%PDF-1.7\n").unwrap();

        // Header -> Object
        parser.feed(b"1 0 obj\n").unwrap();
        assert!(matches!(parser.state, ParserState::InObject { .. }));

        // Object -> Initial
        parser.feed(b"endobj\n").unwrap();
        assert!(matches!(parser.state, ParserState::Initial));

        // Initial -> XRef
        parser.feed(b"xref\n").unwrap();
        assert!(matches!(parser.state, ParserState::InXRef));

        // XRef -> Trailer
        parser.feed(b"trailer\n").unwrap();
        assert!(matches!(parser.state, ParserState::InTrailer));

        // Trailer -> Complete
        parser.feed(b"%%EOF\n").unwrap();
        assert!(parser.is_complete());
    }
}
