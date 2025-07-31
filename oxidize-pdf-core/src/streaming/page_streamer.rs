//! Page streaming for incremental page processing
//!
//! Provides efficient streaming of PDF pages without loading the entire
//! document structure into memory.

use crate::error::Result;
use std::io::{Read, Seek};

/// A page that can be processed in streaming mode
#[derive(Debug, Clone)]
pub struct StreamingPage {
    pub(crate) number: u32,
    pub(crate) width: f64,
    pub(crate) height: f64,
    #[allow(dead_code)]
    pub(crate) content_offset: u64,
    #[allow(dead_code)]
    pub(crate) content_length: usize,
}

impl StreamingPage {
    /// Creates a new StreamingPage for testing purposes
    #[doc(hidden)]
    pub fn new_for_test(
        number: u32,
        width: f64,
        height: f64,
        content_offset: u64,
        content_length: usize,
    ) -> Self {
        Self {
            number,
            width,
            height,
            content_offset,
            content_length,
        }
    }

    /// Get the page number (0-indexed)
    pub fn number(&self) -> u32 {
        self.number
    }

    /// Get page width in points
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Get page height in points
    pub fn height(&self) -> f64 {
        self.height
    }

    /// Extract text from this page in streaming mode
    pub fn extract_text_streaming(&self) -> Result<String> {
        // In a real implementation, this would stream the content
        Ok(format!("Text from page {}", self.number + 1))
    }

    /// Process content stream in chunks
    pub fn process_content<F>(&self, mut callback: F) -> Result<()>
    where
        F: FnMut(&[u8]) -> Result<()>,
    {
        // In a real implementation, this would read content in chunks
        let mock_content = format!("BT /F1 12 Tf 100 700 Td (Page {}) Tj ET", self.number + 1);
        callback(mock_content.as_bytes())?;
        Ok(())
    }

    /// Get the media box for this page
    pub fn media_box(&self) -> [f64; 4] {
        [0.0, 0.0, self.width, self.height]
    }
}

/// Streams pages from a PDF document
pub struct PageStreamer<R: Read + Seek> {
    #[allow(dead_code)]
    reader: R,
    current_page: u32,
    total_pages: Option<u32>,
    #[allow(dead_code)]
    buffer: Vec<u8>,
}

impl<R: Read + Seek> PageStreamer<R> {
    /// Create a new page streamer
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            current_page: 0,
            total_pages: None,
            buffer: Vec::with_capacity(4096),
        }
    }

    /// Get the next page in the stream
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<StreamingPage>> {
        // In a real implementation, this would parse the next page
        if self.current_page >= 3 {
            // Mock: only 3 pages
            return Ok(None);
        }

        let page = StreamingPage {
            number: self.current_page,
            width: 595.0,
            height: 842.0,
            content_offset: self.current_page as u64 * 1024,
            content_length: 512,
        };

        self.current_page += 1;
        Ok(Some(page))
    }

    /// Skip to a specific page
    pub fn seek_to_page(&mut self, page_num: u32) -> Result<()> {
        self.current_page = page_num;
        // In a real implementation, seek in the file
        Ok(())
    }

    /// Get total number of pages if known
    pub fn total_pages(&self) -> Option<u32> {
        self.total_pages
    }
}

/// Iterator adapter for page streaming
pub struct PageIterator<R: Read + Seek> {
    streamer: PageStreamer<R>,
}

impl<R: Read + Seek> PageIterator<R> {
    pub fn new(reader: R) -> Self {
        Self {
            streamer: PageStreamer::new(reader),
        }
    }
}

impl<R: Read + Seek> Iterator for PageIterator<R> {
    type Item = Result<StreamingPage>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.streamer.next() {
            Ok(Some(page)) => Some(Ok(page)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_streaming_page() {
        let page = StreamingPage::new_for_test(0, 612.0, 792.0, 1024, 2048);

        assert_eq!(page.number(), 0);
        assert_eq!(page.width(), 612.0);
        assert_eq!(page.height(), 792.0);

        let media_box = page.media_box();
        assert_eq!(media_box, [0.0, 0.0, 612.0, 792.0]);
    }

    #[test]
    fn test_extract_text_streaming() {
        let page = StreamingPage {
            number: 5,
            width: 595.0,
            height: 842.0,
            content_offset: 0,
            content_length: 0,
        };

        let text = page.extract_text_streaming().unwrap();
        assert!(text.contains("page 6"));
    }

    #[test]
    fn test_process_content() {
        let page = StreamingPage {
            number: 0,
            width: 595.0,
            height: 842.0,
            content_offset: 0,
            content_length: 0,
        };

        let mut chunks = Vec::new();
        page.process_content(|chunk| {
            chunks.push(chunk.to_vec());
            Ok(())
        })
        .unwrap();

        assert!(!chunks.is_empty());
        let content = String::from_utf8_lossy(&chunks[0]);
        assert!(content.contains("Page 1"));
    }

    #[test]
    fn test_page_streamer() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let mut streamer = PageStreamer::new(cursor);

        // Should get first page
        let page1 = streamer.next().unwrap();
        assert!(page1.is_some());
        assert_eq!(page1.unwrap().number(), 0);

        // Should get second page
        let page2 = streamer.next().unwrap();
        assert!(page2.is_some());
        assert_eq!(page2.unwrap().number(), 1);
    }

    #[test]
    fn test_page_streamer_seek() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let mut streamer = PageStreamer::new(cursor);

        // Seek to page 2
        streamer.seek_to_page(2).unwrap();

        let page = streamer.next().unwrap();
        assert!(page.is_some());
        assert_eq!(page.unwrap().number(), 2);
    }

    #[test]
    fn test_page_iterator() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let iterator = PageIterator::new(cursor);

        let mut pages = Vec::new();
        for result in iterator {
            pages.push(result.unwrap());
        }

        assert_eq!(pages.len(), 3); // Mock returns 3 pages
        assert_eq!(pages[0].number(), 0);
        assert_eq!(pages[1].number(), 1);
        assert_eq!(pages[2].number(), 2);
    }

    #[test]
    fn test_page_iterator_for_loop() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let iterator = PageIterator::new(cursor);

        let mut count = 0;
        for page_result in iterator {
            let page = page_result.unwrap();
            assert_eq!(page.number(), count);
            count += 1;
        }

        assert_eq!(count, 3);
    }

    #[test]
    fn test_streaming_page_debug_clone() {
        let page = StreamingPage {
            number: 1,
            width: 500.0,
            height: 600.0,
            content_offset: 2048,
            content_length: 1024,
        };

        let debug_str = format!("{:?}", page);
        assert!(debug_str.contains("StreamingPage"));
        assert!(debug_str.contains("1"));

        let cloned = page.clone();
        assert_eq!(cloned.number, page.number);
        assert_eq!(cloned.width, page.width);
        assert_eq!(cloned.height, page.height);
        assert_eq!(cloned.content_offset, page.content_offset);
        assert_eq!(cloned.content_length, page.content_length);
    }

    #[test]
    fn test_streaming_page_new_for_test() {
        let page = StreamingPage::new_for_test(5, 200.0, 300.0, 4096, 512);

        assert_eq!(page.number(), 5);
        assert_eq!(page.width(), 200.0);
        assert_eq!(page.height(), 300.0);
        assert_eq!(page.content_offset, 4096);
        assert_eq!(page.content_length, 512);
    }

    #[test]
    fn test_streaming_page_media_box_various_sizes() {
        let test_cases = vec![
            (100.0, 100.0, [0.0, 0.0, 100.0, 100.0]),
            (612.0, 792.0, [0.0, 0.0, 612.0, 792.0]),
            (841.89, 1190.55, [0.0, 0.0, 841.89, 1190.55]),
        ];

        for (width, height, expected) in test_cases {
            let page = StreamingPage::new_for_test(0, width, height, 0, 0);
            assert_eq!(page.media_box(), expected);
        }
    }

    #[test]
    fn test_streaming_page_extract_text_different_pages() {
        for page_num in 0..5 {
            let page = StreamingPage {
                number: page_num,
                width: 595.0,
                height: 842.0,
                content_offset: 0,
                content_length: 0,
            };

            let text = page.extract_text_streaming().unwrap();
            assert!(text.contains(&format!("page {}", page_num + 1)));
        }
    }

    #[test]
    fn test_streaming_page_process_content_callback_error() {
        let page = StreamingPage::new_for_test(0, 595.0, 842.0, 0, 0);

        let result = page.process_content(|_chunk| {
            Err(crate::error::PdfError::ParseError(
                "Callback error".to_string(),
            ))
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_streaming_page_process_content_multiple_calls() {
        let page = StreamingPage::new_for_test(3, 595.0, 842.0, 0, 0);

        let mut call_count = 0;
        page.process_content(|chunk| {
            call_count += 1;
            assert!(!chunk.is_empty());
            let content = String::from_utf8_lossy(chunk);
            assert!(content.contains("Page 4")); // page number + 1
            Ok(())
        })
        .unwrap();

        assert_eq!(call_count, 1);
    }

    #[test]
    fn test_page_streamer_creation() {
        let data = b"test data";
        let cursor = Cursor::new(data);
        let streamer = PageStreamer::new(cursor);

        assert_eq!(streamer.current_page, 0);
        assert_eq!(streamer.total_pages, None);
        assert_eq!(streamer.buffer.capacity(), 4096);
    }

    #[test]
    fn test_page_streamer_total_pages() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let streamer = PageStreamer::new(cursor);

        assert_eq!(streamer.total_pages(), None);
    }

    #[test]
    fn test_page_streamer_seek_beyond_pages() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let mut streamer = PageStreamer::new(cursor);

        // Seek to page beyond available pages
        streamer.seek_to_page(10).unwrap();

        let page = streamer.next().unwrap();
        assert!(page.is_none()); // Should return None for out-of-range
    }

    #[test]
    fn test_page_streamer_exhaustion() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let mut streamer = PageStreamer::new(cursor);

        // Exhaust all pages
        let _ = streamer.next().unwrap(); // Page 0
        let _ = streamer.next().unwrap(); // Page 1
        let _ = streamer.next().unwrap(); // Page 2

        // Should return None after exhaustion
        let page = streamer.next().unwrap();
        assert!(page.is_none());

        // Subsequent calls should also return None
        let page = streamer.next().unwrap();
        assert!(page.is_none());
    }

    #[test]
    fn test_page_streamer_page_properties() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let mut streamer = PageStreamer::new(cursor);

        for expected_page_num in 0..3 {
            let page = streamer.next().unwrap().unwrap();

            assert_eq!(page.number(), expected_page_num);
            assert_eq!(page.width(), 595.0); // A4 width
            assert_eq!(page.height(), 842.0); // A4 height
            assert_eq!(page.content_offset, expected_page_num as u64 * 1024);
            assert_eq!(page.content_length, 512);
        }
    }

    #[test]
    fn test_page_iterator_creation() {
        let data = b"test";
        let cursor = Cursor::new(data);
        let iterator = PageIterator::new(cursor);

        assert_eq!(iterator.streamer.current_page, 0);
    }

    #[test]
    fn test_page_iterator_collect() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let iterator = PageIterator::new(cursor);

        let pages: Result<Vec<_>> = iterator.collect();
        let pages = pages.unwrap();

        assert_eq!(pages.len(), 3);
        for (i, page) in pages.iter().enumerate() {
            assert_eq!(page.number(), i as u32);
        }
    }

    #[test]
    fn test_page_iterator_take() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let iterator = PageIterator::new(cursor);

        let first_two: Vec<_> = iterator.take(2).collect();
        assert_eq!(first_two.len(), 2);

        let page0 = &first_two[0].as_ref().unwrap();
        let page1 = &first_two[1].as_ref().unwrap();

        assert_eq!(page0.number(), 0);
        assert_eq!(page1.number(), 1);
    }

    #[test]
    fn test_page_iterator_skip() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let iterator = PageIterator::new(cursor);

        let last_page: Vec<_> = iterator.skip(2).collect();
        assert_eq!(last_page.len(), 1);

        let page = &last_page[0].as_ref().unwrap();
        assert_eq!(page.number(), 2);
    }

    #[test]
    fn test_page_iterator_enumerate() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let iterator = PageIterator::new(cursor);

        for (index, page_result) in iterator.enumerate() {
            let page = page_result.unwrap();
            assert_eq!(page.number(), index as u32);
        }
    }

    #[test]
    fn test_page_streamer_seek_to_zero() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let mut streamer = PageStreamer::new(cursor);

        // Move forward
        let _ = streamer.next().unwrap(); // Page 0
        let _ = streamer.next().unwrap(); // Page 1

        // Seek back to beginning
        streamer.seek_to_page(0).unwrap();

        let page = streamer.next().unwrap().unwrap();
        assert_eq!(page.number(), 0);
    }

    #[test]
    fn test_page_streamer_seek_middle() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let mut streamer = PageStreamer::new(cursor);

        // Seek to middle page
        streamer.seek_to_page(1).unwrap();

        let page = streamer.next().unwrap().unwrap();
        assert_eq!(page.number(), 1);

        // Next call should return page 2
        let page = streamer.next().unwrap().unwrap();
        assert_eq!(page.number(), 2);
    }

    #[test]
    fn test_streaming_page_zero_dimensions() {
        let page = StreamingPage::new_for_test(0, 0.0, 0.0, 0, 0);

        assert_eq!(page.width(), 0.0);
        assert_eq!(page.height(), 0.0);
        assert_eq!(page.media_box(), [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_streaming_page_large_dimensions() {
        let page = StreamingPage::new_for_test(0, 10000.0, 20000.0, 0, 0);

        assert_eq!(page.width(), 10000.0);
        assert_eq!(page.height(), 20000.0);
        assert_eq!(page.media_box(), [0.0, 0.0, 10000.0, 20000.0]);
    }

    #[test]
    fn test_page_iterator_empty_after_exhaustion() {
        let data = b"%PDF-1.7\n";
        let cursor = Cursor::new(data);
        let mut iterator = PageIterator::new(cursor);

        // Consume all pages
        while let Some(_) = iterator.next() {}

        // Iterator should be exhausted
        assert!(iterator.next().is_none());
    }

    #[test]
    fn test_streaming_page_content_callback_data() {
        let page = StreamingPage::new_for_test(7, 595.0, 842.0, 0, 0);

        let mut collected_data = Vec::new();
        page.process_content(|chunk| {
            collected_data.extend_from_slice(chunk);
            Ok(())
        })
        .unwrap();

        let content = String::from_utf8_lossy(&collected_data);
        assert!(content.contains("BT"));
        assert!(content.contains("Tf"));
        assert!(content.contains("Td"));
        assert!(content.contains("Tj"));
        assert!(content.contains("ET"));
        assert!(content.contains("Page 8")); // page 7 + 1
    }
}
