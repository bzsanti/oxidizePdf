//! Page-at-a-time access through the PDF reader. Parsing requires seekable input.
//! Content and text are materialized for one page; this is not a constant-memory
//! parser. Reader object caches and pages retained by callers consume memory too.

use crate::error::{PdfError, Result};
use crate::parser::{PdfDocument, PdfObject, PdfReader};
use std::io::{Read, Seek};
use std::sync::Arc;

/// A source page with decoded content and extracted text.
#[derive(Debug, Clone)]
pub struct StreamingPage {
    number: u32,
    media_box: [f64; 4],
    content: Option<Arc<Vec<u8>>>,
    text: Option<Arc<String>>,
}

impl StreamingPage {
    /// Construct geometry without source content, for tests. Content access fails.
    #[doc(hidden)]
    pub fn new_for_test(
        number: u32,
        width: f64,
        height: f64,
        _content_offset: u64,
        _content_length: usize,
    ) -> Self {
        Self {
            number,
            media_box: [0., 0., width, height],
            content: None,
            text: None,
        }
    }
    /// Page number (zero based).
    pub fn number(&self) -> u32 {
        self.number
    }
    /// Width in PDF points.
    pub fn width(&self) -> f64 {
        self.media_box[2] - self.media_box[0]
    }
    /// Height in PDF points.
    pub fn height(&self) -> f64 {
        self.media_box[3] - self.media_box[1]
    }
    /// Extracted source text. Test-only geometry has no source and returns an error.
    pub fn extract_text_streaming(&self) -> Result<String> {
        self.text
            .as_ref()
            .map(|t| t.as_ref().clone())
            .ok_or_else(|| PdfError::InvalidOperation("Page has no source text".into()))
    }
    /// Deliver decoded source content in chunks. Callback errors stop delivery.
    pub fn process_content<F>(&self, mut callback: F) -> Result<()>
    where
        F: FnMut(&[u8]) -> Result<()>,
    {
        let content = self
            .content
            .as_ref()
            .ok_or_else(|| PdfError::InvalidOperation("Page has no source content".into()))?;
        for chunk in content.chunks(4096) {
            callback(chunk)?;
        }
        Ok(())
    }
    /// Original media box, including its origin.
    pub fn media_box(&self) -> [f64; 4] {
        self.media_box
    }
    pub(crate) fn retained_bytes(&self) -> usize {
        self.content.as_ref().map_or(0, |v| v.len()) + self.text.as_ref().map_or(0, |v| v.len())
    }
}

/// Reads source pages lazily; initialization errors are returned by `next`/`seek_to_page`.
pub struct PageStreamer<R: Read + Seek> {
    reader: Option<R>,
    document: Option<PdfDocument<R>>,
    current_page: u32,
    total_pages: Option<u32>,
    failed: bool,
    max_content_size: usize,
}
// SAFETY: the document's RefCells are private and accessed only by methods taking
// &mut self. No reference into that document escapes: returned pages/text own their
// data. The sole shared accessor, total_pages, reads only a separate Option<u32>.
// Sharing &PageStreamer therefore cannot access those RefCells concurrently. Keep
// this exclusive-access invariant when adding methods. R: Sync preserves the
// original public auto-trait bound, including readers which are Sync but not Send.
unsafe impl<R: Read + Seek + Sync> Sync for PageStreamer<R> {}

// Shared access cannot mutate reader/document state; preserve the former bound.
impl<R: Read + Seek + std::panic::RefUnwindSafe> std::panic::RefUnwindSafe for PageStreamer<R> {}

impl<R: Read + Seek> PageStreamer<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: Some(reader),
            document: None,
            current_page: 0,
            total_pages: None,
            failed: false,
            max_content_size: 10 * 1024 * 1024,
        }
    }
    pub(crate) fn with_content_limit(reader: R, limit: usize) -> Self {
        Self {
            max_content_size: limit,
            ..Self::new(reader)
        }
    }
    pub(crate) fn initialize(&mut self) -> Result<()> {
        if self.failed {
            return Err(PdfError::ParseError(
                "Page streamer failed previously".into(),
            ));
        }
        if let Some(reader) = self.reader.take() {
            // Do not let a failed initialization turn a subsequent call into EOF.
            self.failed = true;
            let document = PdfReader::new(reader)
                .map_err(|error| match error {
                    crate::parser::ParseError::InvalidHeader => PdfError::InvalidHeader,
                    crate::parser::ParseError::Io(error) => PdfError::Io(error),
                    other => other.into(),
                })?
                .into_document();
            self.total_pages = Some(document.page_count()?);
            self.document = Some(document);
            self.failed = false;
        }
        Ok(())
    }
    pub(crate) fn version(&mut self) -> Result<String> {
        self.initialize()?;
        Ok(self
            .document
            .as_mut()
            .ok_or_else(|| PdfError::ParseError("Missing document".into()))?
            .version()?)
    }
    /// Read the next actual source page. Failures leave the cursor at that page.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<StreamingPage>> {
        self.initialize()?;
        if self.current_page >= self.total_pages.unwrap_or(0) {
            return Ok(None);
        }
        let document = self
            .document
            .as_mut()
            .ok_or_else(|| PdfError::ParseError("Missing document".into()))?;
        let page = document.get_page(self.current_page)?;
        let mut content = Vec::new();
        if let Some(contents) = page.dict.get("Contents") {
            let resolved = document.resolve(contents)?;
            let objects = match resolved {
                PdfObject::Array(a) => a.0,
                other => vec![other],
            };
            for (index, object) in objects.iter().enumerate() {
                if index > 0 {
                    if content.len() >= self.max_content_size {
                        return Err(PdfError::ContentStreamTooLarge(
                            content.len().saturating_add(1),
                        ));
                    }
                    content.push(b'\n');
                }
                let object = document.resolve(object)?;
                let stream = object.as_stream().ok_or_else(|| {
                    PdfError::ParseError("Page Contents entry is not a stream".into())
                })?;
                let data = document
                    .decode_stream_with_limit(stream, self.max_content_size - content.len())?;
                if data.len() > self.max_content_size - content.len() {
                    return Err(PdfError::ContentStreamTooLarge(data.len()));
                }
                content.extend_from_slice(&data);
            }
        }
        let text = document.extract_text_from_page(self.current_page)?.text;
        let result = StreamingPage {
            number: self.current_page,
            media_box: page.media_box,
            content: Some(Arc::new(content)),
            text: Some(Arc::new(text)),
        };
        self.current_page += 1;
        Ok(Some(result))
    }
    /// Seek to a zero-based page. The position immediately after the last page is allowed.
    pub fn seek_to_page(&mut self, page_num: u32) -> Result<()> {
        self.initialize()?;
        if page_num > self.total_pages.unwrap_or(0) {
            return Err(PdfError::ParseError("Page index out of range".into()));
        }
        self.current_page = page_num;
        Ok(())
    }
    pub fn total_pages(&self) -> Option<u32> {
        self.total_pages
    }
}

/// Iterator which emits a parsing error once, then terminates.
pub struct PageIterator<R: Read + Seek> {
    streamer: PageStreamer<R>,
    done: bool,
}
impl<R: Read + Seek> PageIterator<R> {
    pub fn new(reader: R) -> Self {
        Self {
            streamer: PageStreamer::new(reader),
            done: false,
        }
    }
}
impl<R: Read + Seek> Iterator for PageIterator<R> {
    type Item = Result<StreamingPage>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match self.streamer.next() {
            Ok(Some(page)) => Some(Ok(page)),
            Ok(None) => {
                self.done = true;
                None
            }
            Err(e) => {
                self.done = true;
                Some(Err(e))
            }
        }
    }
}
