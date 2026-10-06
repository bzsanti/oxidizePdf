use super::*;
use crate::parser::filters::FilterRecovery;
use crate::parser::ParseError;

/// Location of recovered or omitted content within the requested page.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryLocation {
    /// Zero-based index in the page's Contents sequence.
    PageContents(usize),
    /// Resource name and object identity of an invoked Form XObject.
    FormXObject { name: String, object: (u32, u16) },
}
/// A recovery action taken while extracting this page.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextRecoveryAction {
    /// Bytes decoded without full integrity/completion verification.
    Recovered(FilterRecovery),
    /// No bytes recovered; the stream was skipped and its error retained.
    Omitted { error: String },
}
/// Observable limitation attached to recovered text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRecoveryDiagnostic {
    /// Stream/form in the requested page.
    pub location: RecoveryLocation,
    /// Recovery or omission and its underlying error.
    pub action: TextRecoveryAction,
}
/// Page text plus explicit Flate recovery/omission diagnostics.
/// Empty diagnostics mean no Flate recovery was needed, not full PDF validation.
#[derive(Debug)]
pub struct RecoveredText {
    /// Zero-based requested page index.
    pub page_index: u32,
    /// Extracted content, potentially incomplete or unverified.
    pub text: ExtractedText,
    /// Every Flate stream recovered or omitted during extraction.
    pub diagnostics: Vec<TextRecoveryDiagnostic>,
}
pub(super) struct RecoveryContext {
    pub(super) max_stream_bytes: usize,
    pub(super) diagnostics: Vec<TextRecoveryDiagnostic>,
}
impl TextExtractor {
    /// Extract one page with explicit Flate recovery and a decoded content/Form cap.
    /// Resource-limit and predictor errors remain errors. Inspect diagnostics even
    /// when the text is nonempty: recovery does not certify integrity/completeness.
    ///
    /// `max_stream_bytes` limits decoded bytes per content stream or Form, including
    /// PDF operators. It is independent of [`ExtractionOptions::max_extracted_bytes`],
    /// which limits emitted text. This explicit API works with strict or lenient
    /// parser options; lenient parsing alone does not enable Flate recovery in
    /// [`Self::extract_from_page`]. Missing checksums are reported as unverified,
    /// truncated payloads as incomplete, and undecodable streams as omitted.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use oxidize_pdf::parser::{ParseOptions, PdfReader};
    /// use oxidize_pdf::text::TextExtractor;
    /// use std::fs::File;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let reader = PdfReader::new_with_options(
    ///     File::open("document.pdf")?, ParseOptions::lenient(),
    /// )?;
    /// let document = reader.into_document();
    /// let mut extractor = TextExtractor::new();
    /// for page in 0..document.page_count()? {
    ///     let recovered = extractor.extract_from_page_with_recovery(
    ///         &document, page, 8 * 1024 * 1024,
    ///     )?;
    ///     // Keep diagnostics alongside the text: recovery may have omitted content.
    ///     for diagnostic in &recovered.diagnostics {
    ///         eprintln!("Page {}: {:?}", recovered.page_index + 1, diagnostic);
    ///     }
    ///     println!("{}", recovered.text.text);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_from_page_with_recovery<R: Read + Seek>(
        &mut self,
        document: &PdfDocument<R>,
        page_index: u32,
        max_stream_bytes: usize,
    ) -> ParseResult<RecoveredText> {
        self.recovery = Some(RecoveryContext {
            max_stream_bytes,
            diagnostics: Vec::new(),
        });
        let result = self.extract_page_impl(document, page_index);
        // Restore strict behavior on both success and error, including nested Forms.
        let context = self
            .recovery
            .take()
            .expect("recovery context exists for this call");
        result.map(|text| RecoveredText {
            page_index,
            text,
            diagnostics: context.diagnostics,
        })
    }
}

impl TextExtractor {
    pub(super) fn decode_content_for_recovery<R: Read + Seek>(
        &mut self,
        document: &PdfDocument<R>,
        stream: &crate::parser::PdfStream,
        location: RecoveryLocation,
    ) -> ParseResult<Option<Vec<u8>>> {
        use crate::parser::filters::StreamRecoveryErrorKind;
        let Some(context) = self.recovery.as_mut() else {
            return document.decode_stream(stream).map(Some);
        };
        match document.decode_stream_with_recovery(stream, context.max_stream_bytes) {
            Ok(decoded) => {
                context
                    .diagnostics
                    .extend(decoded.diagnostics.into_iter().map(|diagnostic| {
                        TextRecoveryDiagnostic {
                            location: location.clone(),
                            action: TextRecoveryAction::Recovered(diagnostic),
                        }
                    }));
                Ok(Some(decoded.data))
            }
            Err(error) if error.kind == StreamRecoveryErrorKind::InvalidFlate => {
                context.diagnostics.push(TextRecoveryDiagnostic {
                    location,
                    action: TextRecoveryAction::Omitted {
                        error: error.source.to_string(),
                    },
                });
                Ok(None)
            }
            Err(error) => Err(error.source),
        }
    }
    pub(super) fn recovery_content_groups<R: Read + Seek>(
        &mut self,
        document: &PdfDocument<R>,
        page: &crate::parser::ParsedPage,
    ) -> ParseResult<Vec<Vec<Vec<u8>>>> {
        use crate::parser::PdfObject;
        let Some(contents) = page.dict.get("Contents") else {
            return Ok(Vec::new());
        };
        let resolved = document.resolve(contents)?;
        let items = match resolved {
            PdfObject::Stream(s) => vec![PdfObject::Stream(s)],
            PdfObject::Array(a) => a.0,
            _ => {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: "Contents must be a stream or array of streams".into(),
                })
            }
        };
        let mut groups = Vec::new();
        let mut current = Vec::new();
        for (index, item) in items.into_iter().enumerate() {
            let resolved = document.resolve(&item)?;
            if let PdfObject::Stream(stream) = resolved {
                let before = self.recovery.as_ref().unwrap().diagnostics.len();
                let data = self.decode_content_for_recovery(
                    document,
                    &stream,
                    RecoveryLocation::PageContents(index),
                )?;
                if self.recovery.as_ref().unwrap().diagnostics.len() != before {
                    // Never join operands across omitted/incomplete content.
                    // Adjacent verified streams still share their parser state.
                    if !current.is_empty() {
                        groups.push(std::mem::take(&mut current));
                    }
                    if let Some(data) = data {
                        groups.push(vec![data]);
                    }
                } else if let Some(data) = data {
                    current.push(data);
                }
            }
        }
        if !current.is_empty() {
            groups.push(current);
        }
        Ok(groups)
    }
}
