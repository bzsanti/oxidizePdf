use super::*;

pub(super) struct PreparedStream {
    pub(super) operations: Vec<ContentOperation>,
    pub(super) resources: Option<PdfDictionary>,
    pub(super) structure_actual_text: StructureActualText,
}

pub(super) enum FontScope {
    Inherited,
    Independent,
}

impl TextExtractor {
    /// Page forms and appearances share decoding, diagnostics and resource lookup.
    pub(super) fn prepare_stream<R: Read + Seek>(
        &mut self,
        document: &PdfDocument<R>,
        stream: crate::parser::PdfStream,
        location: RecoveryLocation,
    ) -> ParseResult<Option<PreparedStream>> {
        let Some(data) = self.decode_content_for_recovery(document, &stream, location)? else {
            return Ok(None);
        };
        let Ok(operations) = ContentParser::parse_content(&data) else {
            return Ok(None);
        };
        let resources = stream
            .dict
            .get("Resources")
            .and_then(|res| resolve_annotation_dictionary(document, res));
        Ok(Some(PreparedStream {
            operations,
            resources,
            structure_actual_text: resolve_structure_actual_text(&stream.dict, document),
        }))
    }

    /// Restore every name-local font cache on success and error. Object-identity
    /// caching remains document-local and may be reused by independent sources.
    pub(super) fn with_font_scope<R: Read + Seek, T>(
        &mut self,
        document: &PdfDocument<R>,
        resources: Option<&PdfDictionary>,
        scope: FontScope,
        run: impl FnOnce(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<T> {
        let fonts = std::mem::take(&mut self.font_cache);
        let widths = std::mem::take(&mut self.type0_implicit_space_widths);
        if matches!(scope, FontScope::Inherited) {
            self.font_cache = fonts.clone();
            self.type0_implicit_space_widths = widths.clone();
        }
        let result = (|| {
            if let Some(resources) = resources {
                self.cache_fonts_from_resources(resources, document)?;
            }
            run(self)
        })();
        self.font_cache = fonts;
        self.type0_implicit_space_widths = widths;
        result
    }
}
