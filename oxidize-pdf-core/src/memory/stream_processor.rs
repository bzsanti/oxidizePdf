//! Stream processing for memory-efficient PDF operations
//!
//! Uses the seekable PDF reader and materializes content/text one page at a time.
//! Reader caches and caller-owned values are not covered by max_stream_size.

use crate::error::{PdfError, Result};
use crate::parser::content::{ContentOperation, ContentParser};
use crate::parser::PdfObject;
use crate::streaming::PageStreamer;
use std::io::{BufReader, Read, Seek, Write};

/// Options for streaming operations
#[derive(Debug, Clone)]
pub struct StreamingOptions {
    /// Buffer size for reading
    pub buffer_size: usize,
    /// Maximum combined decoded page content (also the individual stream limit)
    pub max_stream_size: usize,
    /// Reserved for resource enumeration. Page interpretation still resolves needed resources.
    pub skip_images: bool,
    /// Reserved for resource enumeration. Text extraction still resolves its fonts.
    pub skip_fonts: bool,
}

impl Default for StreamingOptions {
    fn default() -> Self {
        Self {
            buffer_size: 64 * 1024,            // 64KB
            max_stream_size: 10 * 1024 * 1024, // 10MB
            skip_images: false,
            skip_fonts: false,
        }
    }
}

/// Stream processor for incremental PDF processing
pub struct StreamProcessor<R: Read + Seek> {
    streamer: PageStreamer<BufReader<R>>,
}

impl<R: Read + Seek> StreamProcessor<R> {
    /// Create a page-at-a-time processor. Input is validated on processing.
    pub fn new(reader: R, options: StreamingOptions) -> Self {
        Self {
            streamer: PageStreamer::with_content_limit(
                BufReader::with_capacity(options.buffer_size.max(1), reader),
                options.max_stream_size,
            ),
        }
    }
    /// Emit start, source header, source pages and end. Stop terminates immediately;
    /// callback/parser errors propagate without a success End event. Objects and
    /// resources are not separately enumerated by this page-oriented adapter.
    pub fn process_with<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(ProcessingEvent) -> Result<ProcessingAction>,
    {
        if callback(ProcessingEvent::Start)? == ProcessingAction::Stop {
            return Ok(());
        }
        let version = self.streamer.version()?;
        if callback(ProcessingEvent::Header { version })? == ProcessingAction::Stop {
            return Ok(());
        }
        while let Some(page) = self.streamer.next()? {
            let mut content = Vec::new();
            page.process_content(|chunk| {
                content.extend_from_slice(chunk);
                Ok(())
            })?;
            let data = PageData {
                number: page.number(),
                width: page.width() as f32,
                height: page.height() as f32,
                text: Some(page.extract_text_streaming()?),
                operations: ContentParser::parse_strict(&content)?,
            };
            if callback(ProcessingEvent::Page(data))? == ProcessingAction::Stop {
                return Ok(());
            }
        }
        callback(ProcessingEvent::End)?;
        Ok(())
    }

    /// Process pages incrementally
    pub fn process_pages<F>(&mut self, mut page_callback: F) -> Result<()>
    where
        F: FnMut(u32, PageData) -> Result<ProcessingAction>,
    {
        self.process_with(|event| match event {
            ProcessingEvent::Page(data) => page_callback(data.number, data),
            _ => Ok(ProcessingAction::Continue),
        })
    }

    /// Extract text incrementally
    pub fn extract_text_streaming<W: Write>(&mut self, output: &mut W) -> Result<()> {
        self.process_pages(|_index, page_data| {
            if let Some(text) = page_data.text {
                output.write_all(text.as_bytes())?;
                output.write_all(b"\n")?;
            }
            Ok(ProcessingAction::Continue)
        })
    }
}

/// Events during stream processing
#[derive(Debug)]
pub enum ProcessingEvent {
    /// Processing started
    Start,
    /// PDF header found
    Header { version: String },
    /// Object encountered
    Object { id: (u32, u16), object: PdfObject },
    /// Page encountered
    Page(PageData),
    /// Resource encountered
    Resource {
        name: String,
        resource_type: ResourceType,
    },
    /// Processing ended
    End,
}

/// Page data during streaming
#[derive(Debug)]
pub struct PageData {
    /// Page number (0-indexed)
    pub number: u32,
    /// Page width in points
    pub width: f32,
    /// Page height in points
    pub height: f32,
    /// Extracted text (if any)
    pub text: Option<String>,
    /// Content operations (if requested)
    pub operations: Vec<ContentOperation>,
}

/// Resource types
#[derive(Debug, Clone)]
pub enum ResourceType {
    Font,
    Image,
    ColorSpace,
    Pattern,
    XObject,
}

/// Action to take after processing an event
#[derive(Debug, PartialEq)]
pub enum ProcessingAction {
    /// Continue processing
    Continue,
    /// Skip this item
    Skip,
    /// Stop processing
    Stop,
}

/// Stream-based content processor for individual content streams
pub struct ContentStreamProcessor {
    buffer: Vec<u8>,
    options: StreamingOptions,
}

impl ContentStreamProcessor {
    /// Create a new content stream processor
    pub fn new(options: StreamingOptions) -> Self {
        Self {
            buffer: Vec::with_capacity(options.buffer_size),
            options,
        }
    }

    /// Process a content stream incrementally
    pub fn process_stream<R: Read, F>(&mut self, mut reader: R, mut callback: F) -> Result<()>
    where
        F: FnMut(&ContentOperation) -> Result<ProcessingAction>,
    {
        self.buffer.clear();
        reader.read_to_end(&mut self.buffer)?;

        if self.buffer.len() > self.options.max_stream_size {
            return Err(PdfError::ContentStreamTooLarge(self.buffer.len()));
        }

        let operations =
            ContentParser::parse(&self.buffer).map_err(|e| PdfError::ParseError(e.to_string()))?;

        for op in operations {
            match callback(&op)? {
                ProcessingAction::Continue => {}
                ProcessingAction::Skip => continue,
                ProcessingAction::Stop => break,
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_streaming_options_default() {
        let options = StreamingOptions::default();
        assert_eq!(options.buffer_size, 64 * 1024);
        assert_eq!(options.max_stream_size, 10 * 1024 * 1024);
        assert!(!options.skip_images);
        assert!(!options.skip_fonts);
    }

    #[test]
    fn test_stream_processor_creation() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let options = StreamingOptions::default();
        let _processor = StreamProcessor::new(cursor, options);
    }

    #[test]
    fn test_processing_action() {
        assert_eq!(ProcessingAction::Continue, ProcessingAction::Continue);
        assert_eq!(ProcessingAction::Skip, ProcessingAction::Skip);
        assert_eq!(ProcessingAction::Stop, ProcessingAction::Stop);
        assert_ne!(ProcessingAction::Continue, ProcessingAction::Stop);
    }

    #[test]
    fn test_content_stream_processor() {
        let options = StreamingOptions::default();
        let mut processor = ContentStreamProcessor::new(options);

        // Test with simple content
        let content = b"BT /F1 12 Tf 100 700 Td (Hello) Tj ET";
        let cursor = Cursor::new(content);

        let mut op_count = 0;
        processor
            .process_stream(cursor, |op| {
                op_count += 1;
                match op {
                    ContentOperation::BeginText => assert_eq!(op_count, 1),
                    ContentOperation::EndText => assert_eq!(op_count, 5),
                    _ => {}
                }
                Ok(ProcessingAction::Continue)
            })
            .unwrap();

        assert!(op_count > 0);
    }

    #[test]
    fn test_streaming_options_custom() {
        let options = StreamingOptions {
            buffer_size: 1024,
            max_stream_size: 2048,
            skip_images: true,
            skip_fonts: true,
        };

        assert_eq!(options.buffer_size, 1024);
        assert_eq!(options.max_stream_size, 2048);
        assert!(options.skip_images);
        assert!(options.skip_fonts);
    }

    #[test]
    fn test_streaming_options_debug_clone() {
        let options = StreamingOptions {
            buffer_size: 512,
            max_stream_size: 1024,
            skip_images: false,
            skip_fonts: true,
        };

        let debug_str = format!("{options:?}");
        assert!(debug_str.contains("StreamingOptions"));
        assert!(debug_str.contains("512"));
        assert!(debug_str.contains("1024"));

        let cloned = options;
        assert_eq!(cloned.buffer_size, 512);
        assert_eq!(cloned.max_stream_size, 1024);
        assert!(!cloned.skip_images);
        assert!(cloned.skip_fonts);
    }

    #[test]
    fn test_processing_event_debug() {
        let events = vec![
            ProcessingEvent::Start,
            ProcessingEvent::Header {
                version: "1.7".to_string(),
            },
            ProcessingEvent::Object {
                id: (1, 0),
                object: PdfObject::Null,
            },
            ProcessingEvent::Page(PageData {
                number: 0,
                width: 595.0,
                height: 842.0,
                text: Some("test".to_string()),
                operations: vec![],
            }),
            ProcessingEvent::Resource {
                name: "Font1".to_string(),
                resource_type: ResourceType::Font,
            },
            ProcessingEvent::End,
        ];

        for event in events {
            let debug_str = format!("{event:?}");
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_page_data_debug() {
        let page_data = PageData {
            number: 5,
            width: 612.0,
            height: 792.0,
            text: Some("Page content".to_string()),
            operations: vec![ContentOperation::BeginText],
        };

        let debug_str = format!("{page_data:?}");
        assert!(debug_str.contains("PageData"));
        assert!(debug_str.contains("5"));
        assert!(debug_str.contains("612.0"));
        assert!(debug_str.contains("Page content"));
    }

    #[test]
    fn test_resource_type_debug_clone() {
        let resource_types = vec![
            ResourceType::Font,
            ResourceType::Image,
            ResourceType::ColorSpace,
            ResourceType::Pattern,
            ResourceType::XObject,
        ];

        for resource_type in resource_types {
            let debug_str = format!("{resource_type:?}");
            assert!(!debug_str.is_empty());

            let cloned = resource_type.clone();
            let cloned_debug = format!("{cloned:?}");
            assert_eq!(debug_str, cloned_debug);
        }
    }

    #[test]
    fn test_processing_action_debug_partial_eq() {
        let action = ProcessingAction::Continue;

        let debug_str = format!("{action:?}");
        assert!(debug_str.contains("Continue"));

        assert_eq!(ProcessingAction::Continue, ProcessingAction::Continue);
        assert_eq!(ProcessingAction::Skip, ProcessingAction::Skip);
        assert_eq!(ProcessingAction::Stop, ProcessingAction::Stop);

        assert_ne!(ProcessingAction::Continue, ProcessingAction::Skip);
        assert_ne!(ProcessingAction::Skip, ProcessingAction::Stop);
        assert_ne!(ProcessingAction::Stop, ProcessingAction::Continue);
    }

    #[test]
    fn test_stream_processor_invalid_header() {
        let data = b"Not a PDF\n";
        let cursor = Cursor::new(data);
        let options = StreamingOptions::default();
        let mut processor = StreamProcessor::new(cursor, options);

        let result = processor.process_with(|_event| Ok(ProcessingAction::Continue));

        assert!(result.is_err());
        match result {
            Err(PdfError::InvalidHeader) => {}
            _ => panic!("Expected InvalidHeader error"),
        }
    }

    #[test]
    fn test_content_stream_processor_creation() {
        let options = StreamingOptions {
            buffer_size: 2048,
            max_stream_size: 4096,
            skip_images: true,
            skip_fonts: false,
        };

        let processor = ContentStreamProcessor::new(options.clone());

        assert_eq!(processor.buffer.capacity(), options.buffer_size);
        assert_eq!(processor.options.buffer_size, 2048);
        assert_eq!(processor.options.max_stream_size, 4096);
        assert!(processor.options.skip_images);
        assert!(!processor.options.skip_fonts);
    }

    #[test]
    fn test_content_stream_processor_empty_stream() {
        let options = StreamingOptions::default();
        let mut processor = ContentStreamProcessor::new(options);

        let content = b"";
        let cursor = Cursor::new(content);

        let mut op_count = 0;
        processor
            .process_stream(cursor, |_op| {
                op_count += 1;
                Ok(ProcessingAction::Continue)
            })
            .unwrap();

        assert_eq!(op_count, 0);
    }

    #[test]
    fn test_content_stream_processor_large_stream_error() {
        let options = StreamingOptions {
            buffer_size: 1024,
            max_stream_size: 10, // Very small limit
            skip_images: false,
            skip_fonts: false,
        };

        let mut processor = ContentStreamProcessor::new(options);

        // Create content larger than max_stream_size
        let content = b"BT /F1 12 Tf 100 700 Td (This is a long content stream) Tj ET";
        let cursor = Cursor::new(content);

        let result = processor.process_stream(cursor, |_op| Ok(ProcessingAction::Continue));

        assert!(result.is_err());
        match result {
            Err(PdfError::ContentStreamTooLarge(size)) => {
                assert_eq!(size, content.len());
            }
            _ => panic!("Expected ContentStreamTooLarge error"),
        }
    }

    #[test]
    fn test_content_stream_processor_skip_action() {
        let options = StreamingOptions::default();
        let mut processor = ContentStreamProcessor::new(options);

        let content = b"BT /F1 12 Tf 100 700 Td (Hello) Tj 50 0 Td (World) Tj ET";
        let cursor = Cursor::new(content);

        let mut processed_count = 0;
        let mut skipped_count = 0;

        processor
            .process_stream(cursor, |op| match op {
                ContentOperation::ShowText(_) => {
                    skipped_count += 1;
                    Ok(ProcessingAction::Skip)
                }
                _ => {
                    processed_count += 1;
                    Ok(ProcessingAction::Continue)
                }
            })
            .unwrap();

        assert!(processed_count > 0);
        assert!(skipped_count > 0);
    }

    #[test]
    fn test_content_stream_processor_stop_action() {
        let options = StreamingOptions::default();
        let mut processor = ContentStreamProcessor::new(options);

        let content = b"BT /F1 12 Tf 100 700 Td (Hello) Tj 50 0 Td (World) Tj ET";
        let cursor = Cursor::new(content);

        let mut op_count = 0;

        processor
            .process_stream(cursor, |_op| {
                op_count += 1;
                if op_count >= 3 {
                    Ok(ProcessingAction::Stop)
                } else {
                    Ok(ProcessingAction::Continue)
                }
            })
            .unwrap();

        assert_eq!(op_count, 3);
    }

    #[test]
    fn test_content_stream_processor_invalid_content() {
        let options = StreamingOptions::default();
        let mut processor = ContentStreamProcessor::new(options);

        let content = b"Invalid PDF content that cannot be parsed";
        let cursor = Cursor::new(content);

        let result = processor.process_stream(cursor, |_op| Ok(ProcessingAction::Continue));

        // Should handle parse errors gracefully
        match result {
            Ok(_) => {}                        // If parser is lenient and returns empty operations
            Err(PdfError::ParseError(_)) => {} // If parser returns error
            _ => panic!("Unexpected error type"),
        }
    }

    #[test]
    fn test_content_stream_processor_callback_error() {
        let options = StreamingOptions::default();
        let mut processor = ContentStreamProcessor::new(options);

        let content = b"BT /F1 12 Tf ET";
        let cursor = Cursor::new(content);

        let result = processor.process_stream(cursor, |_op| {
            Err(PdfError::ParseError("Test error".to_string()))
        });

        assert!(result.is_err());
        match result {
            Err(PdfError::ParseError(msg)) => {
                assert_eq!(msg, "Test error");
            }
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_page_data_with_operations() {
        let page_data = PageData {
            number: 0,
            width: 595.0,
            height: 842.0,
            text: Some("Test page".to_string()),
            operations: vec![ContentOperation::BeginText, ContentOperation::EndText],
        };

        assert_eq!(page_data.number, 0);
        assert_eq!(page_data.width, 595.0);
        assert_eq!(page_data.height, 842.0);
        assert_eq!(page_data.text, Some("Test page".to_string()));
        assert_eq!(page_data.operations.len(), 2);
    }

    #[test]
    fn test_page_data_without_text() {
        let page_data = PageData {
            number: 1,
            width: 612.0,
            height: 792.0,
            text: None,
            operations: vec![],
        };

        assert_eq!(page_data.number, 1);
        assert_eq!(page_data.text, None);
        assert!(page_data.operations.is_empty());
    }

    #[test]
    fn test_stream_processor_io_error() {
        use std::io::Error;

        struct ErrorReader;
        impl Read for ErrorReader {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Err(Error::other("IO Error"))
            }
        }
        impl Seek for ErrorReader {
            fn seek(&mut self, _pos: std::io::SeekFrom) -> std::io::Result<u64> {
                Ok(0)
            }
        }

        let options = StreamingOptions::default();
        let mut processor = StreamProcessor::new(ErrorReader, options);

        let result = processor.process_with(|_event| Ok(ProcessingAction::Continue));
        assert!(result.is_err());
    }

    #[test]
    fn test_content_stream_processor_buffer_reuse() {
        let options = StreamingOptions::default();
        let mut processor = ContentStreamProcessor::new(options);

        // Process first stream
        let content1 = b"BT (First) Tj ET";
        let cursor1 = Cursor::new(content1);

        let mut first_ops = Vec::new();
        processor
            .process_stream(cursor1, |op| {
                first_ops.push(format!("{op:?}"));
                Ok(ProcessingAction::Continue)
            })
            .unwrap();

        // Process second stream - buffer should be cleared and reused
        let content2 = b"BT (Second) Tj ET";
        let cursor2 = Cursor::new(content2);

        let mut second_ops = Vec::new();
        processor
            .process_stream(cursor2, |op| {
                second_ops.push(format!("{op:?}"));
                Ok(ProcessingAction::Continue)
            })
            .unwrap();

        assert!(!first_ops.is_empty());
        assert!(!second_ops.is_empty());
        // Operations should be different for different content
        assert_ne!(first_ops, second_ops);
    }
}
