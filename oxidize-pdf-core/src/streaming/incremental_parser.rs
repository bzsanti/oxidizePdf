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

    #[test]
    fn test_incremental_parser_default() {
        let parser = IncrementalParser::default();
        assert!(!parser.is_complete());
        assert!(matches!(parser.state, ParserState::Initial));
    }

    #[test]
    fn test_parse_event_debug() {
        let events = vec![
            ParseEvent::Header {
                version: "1.7".to_string(),
            },
            ParseEvent::ObjectStart {
                id: 1,
                generation: 0,
            },
            ParseEvent::ObjectEnd {
                id: 1,
                generation: 0,
                object: PdfObject::Null,
            },
            ParseEvent::StreamData {
                object_id: 1,
                data: vec![1, 2, 3],
            },
            ParseEvent::XRef { entries: vec![] },
            ParseEvent::Trailer {
                dict: PdfDictionary::new(),
            },
            ParseEvent::EndOfFile,
        ];

        for event in events {
            let debug_str = format!("{event:?}");
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_xref_entry_debug_clone() {
        let entry = XRefEntry {
            object_number: 5,
            generation: 2,
            offset: 1024,
            in_use: true,
        };

        let debug_str = format!("{entry:?}");
        assert!(debug_str.contains("XRefEntry"));
        assert!(debug_str.contains("5"));

        let cloned = entry.clone();
        assert_eq!(cloned.object_number, entry.object_number);
        assert_eq!(cloned.generation, entry.generation);
        assert_eq!(cloned.offset, entry.offset);
        assert_eq!(cloned.in_use, entry.in_use);
    }

    #[test]
    fn test_parser_state_debug() {
        let states = vec![
            ParserState::Initial,
            ParserState::InObject {
                id: 1,
                generation: 0,
            },
            ParserState::InStream { object_id: 2 },
            ParserState::InXRef,
            ParserState::InTrailer,
            ParserState::Complete,
        ];

        for state in states {
            let debug_str = format!("{state:?}");
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_feed_empty_data() {
        let mut parser = IncrementalParser::new();
        parser.feed(b"").unwrap();

        let events = parser.take_events();
        assert!(events.is_empty());
    }

    #[test]
    fn test_feed_partial_lines() {
        let mut parser = IncrementalParser::new();

        // Feed partial line
        parser.feed(b"%PDF-").unwrap();
        let events1 = parser.take_events();
        assert!(events1.is_empty());

        // Complete the line
        parser.feed(b"1.7\n").unwrap();
        let events2 = parser.take_events();
        assert_eq!(events2.len(), 1);

        match &events2[0] {
            ParseEvent::Header { version } => assert_eq!(version, "1.7"),
            _ => panic!("Expected Header event"),
        }
    }

    #[test]
    fn test_feed_multiple_lines() {
        let mut parser = IncrementalParser::new();
        let data = b"%PDF-1.7\n1 0 obj\nendobj\n";

        parser.feed(data).unwrap();
        let events = parser.take_events();

        assert!(events.len() >= 3); // Header, ObjectStart, ObjectEnd
    }

    #[test]
    fn test_parse_object_header_valid() {
        let parser = IncrementalParser::new();

        assert_eq!(parser.parse_object_header("1 0 obj"), Some((1, 0)));
        assert_eq!(parser.parse_object_header("42 5 obj"), Some((42, 5)));
        assert_eq!(
            parser.parse_object_header("999 65535 obj"),
            Some((999, 65535))
        );
    }

    #[test]
    fn test_parse_object_header_invalid() {
        let parser = IncrementalParser::new();

        assert_eq!(parser.parse_object_header("1 0"), None);
        assert_eq!(parser.parse_object_header("1 obj"), None);
        assert_eq!(parser.parse_object_header("obj"), None);
        assert_eq!(parser.parse_object_header("not an object"), None);
        assert_eq!(parser.parse_object_header("abc 0 obj"), None);
        assert_eq!(parser.parse_object_header("1 abc obj"), None);
    }

    #[test]
    fn test_parse_xref_entry_valid() {
        let parser = IncrementalParser::new();

        let entry = parser.parse_xref_entry("0000000000 65535 f").unwrap();
        assert_eq!(entry.offset, 0);
        assert_eq!(entry.generation, 65535);
        assert!(!entry.in_use);

        let entry = parser.parse_xref_entry("0000001024 00000 n").unwrap();
        assert_eq!(entry.offset, 1024);
        assert_eq!(entry.generation, 0);
        assert!(entry.in_use);
    }

    #[test]
    fn test_parse_xref_entry_invalid() {
        let parser = IncrementalParser::new();

        assert!(parser.parse_xref_entry("invalid").is_none());
        assert!(parser.parse_xref_entry("123 456").is_none());
        assert!(parser.parse_xref_entry("abc def ghi").is_none());
        assert!(parser.parse_xref_entry("").is_none());
    }

    #[test]
    fn test_object_to_stream_transition() {
        let mut parser = IncrementalParser::new();
        parser.state = ParserState::InObject {
            id: 3,
            generation: 1,
        };

        parser.feed(b"stream\n").unwrap();
        assert!(matches!(
            parser.state,
            ParserState::InStream { object_id: 3 }
        ));
    }

    #[test]
    fn test_stream_data_collection() {
        let mut parser = IncrementalParser::new();
        parser.state = ParserState::InStream { object_id: 5 };

        parser.feed(b"line1\nline2\nendstream\n").unwrap();
        let events = parser.take_events();

        let stream_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ParseEvent::StreamData { .. }))
            .collect();

        assert_eq!(stream_events.len(), 2);

        match &stream_events[0] {
            ParseEvent::StreamData { object_id, data } => {
                assert_eq!(*object_id, 5);
                assert_eq!(data, b"line1");
            }
            _ => panic!("Expected StreamData"),
        }
    }

    #[test]
    fn test_stream_to_object_transition() {
        let mut parser = IncrementalParser::new();
        parser.state = ParserState::InStream { object_id: 7 };

        parser.feed(b"endstream\n").unwrap();
        assert!(matches!(
            parser.state,
            ParserState::InObject {
                id: 7,
                generation: 0
            }
        ));
    }

    #[test]
    fn test_xref_to_trailer_transition() {
        let mut parser = IncrementalParser::new();
        parser.state = ParserState::InXRef;

        parser.feed(b"trailer\n").unwrap();
        assert!(matches!(parser.state, ParserState::InTrailer));
    }

    #[test]
    fn test_ignore_input_after_completion() {
        let mut parser = IncrementalParser::new();
        parser.state = ParserState::Complete;

        parser.feed(b"any additional input\n").unwrap();
        let events = parser.take_events();
        assert!(events.is_empty());
    }

    #[test]
    fn test_process_incrementally_with_io_error() {
        use std::io::Error;

        struct ErrorReader;

        impl Read for ErrorReader {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Err(Error::other("Test error"))
            }
        }

        let reader = ErrorReader;
        let result = process_incrementally(reader, |_event| Ok(()));
        assert!(result.is_err());
    }

    #[test]
    fn test_process_incrementally_with_callback_error() {
        use std::io::Cursor;

        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);

        let result = process_incrementally(cursor, |_event| {
            Err(PdfError::ParseError("Callback error".to_string()))
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_process_incrementally_empty_reader() {
        use std::io::Cursor;

        let data = b"";
        let cursor = Cursor::new(data);

        let mut event_count = 0;
        process_incrementally(cursor, |_event| {
            event_count += 1;
            Ok(())
        })
        .unwrap();

        assert_eq!(event_count, 0);
    }

    #[test]
    fn test_take_events_clears_buffer() {
        let mut parser = IncrementalParser::new();
        parser.feed(b"%PDF-1.7\n").unwrap();

        assert!(!parser.events.is_empty());

        let events = parser.take_events();
        assert_eq!(events.len(), 1);
        assert!(parser.events.is_empty());

        // Subsequent call should return empty
        let events2 = parser.take_events();
        assert!(events2.is_empty());
    }

    #[test]
    fn test_parser_with_whitespace_handling() {
        let mut parser = IncrementalParser::new();

        // Test with extra whitespace
        parser.feed(b"   %PDF-1.7   \n").unwrap();
        let events = parser.take_events();

        match &events[0] {
            ParseEvent::Header { version } => assert_eq!(version, "1.7"),
            _ => panic!("Expected Header event"),
        }
    }

    #[test]
    fn test_object_parsing_with_generation() {
        let mut parser = IncrementalParser::new();
        parser.feed(b"123 456 obj\n").unwrap();

        let events = parser.take_events();
        match &events[0] {
            ParseEvent::ObjectStart { id, generation } => {
                assert_eq!(*id, 123);
                assert_eq!(*generation, 456);
            }
            _ => panic!("Expected ObjectStart event"),
        }
    }

    #[test]
    fn test_complete_pdf_parsing_sequence() {
        let mut parser = IncrementalParser::new();

        let pdf_content = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\nxref\n0 1\n0000000000 65535 f\ntrailer\n<< /Size 1 >>\n%%EOF\n";

        parser.feed(pdf_content).unwrap();
        let events = parser.take_events();

        // Should have header, object start, object end, and eof events
        assert!(events
            .iter()
            .any(|e| matches!(e, ParseEvent::Header { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, ParseEvent::ObjectStart { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, ParseEvent::ObjectEnd { .. })));
        assert!(events.iter().any(|e| matches!(e, ParseEvent::EndOfFile)));

        assert!(parser.is_complete());
    }

    #[test]
    fn test_xref_state_from_any_state() {
        let mut parser = IncrementalParser::new();

        // Start in object state
        parser.state = ParserState::InObject {
            id: 1,
            generation: 0,
        };

        // xref should transition from any state
        parser.feed(b"xref\n").unwrap();
        assert!(matches!(parser.state, ParserState::InXRef));
    }

    #[test]
    fn test_buffer_management() {
        let mut parser = IncrementalParser::new();

        // Feed data without newlines
        parser.feed(b"partial").unwrap();
        assert!(parser.buffer.contains("partial"));

        // Feed completion with newline
        parser.feed(b" line\n").unwrap();

        // Buffer should be cleared after processing the line
        assert!(!parser.buffer.contains("partial"));
    }

    #[test]
    fn test_multiple_objects_in_sequence() {
        let mut parser = IncrementalParser::new();

        let content = b"1 0 obj\n<< >>\nendobj\n2 0 obj\n<< >>\nendobj\n";
        parser.feed(content).unwrap();

        let events = parser.take_events();

        let object_starts: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                ParseEvent::ObjectStart { id, generation } => Some((*id, *generation)),
                _ => None,
            })
            .collect();

        assert_eq!(object_starts.len(), 2);
        assert_eq!(object_starts[0], (1, 0));
        assert_eq!(object_starts[1], (2, 0));
    }
}
