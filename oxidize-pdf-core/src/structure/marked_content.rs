/// Marked Content operators for Tagged PDF (ISO 32000-1 Section 14.6)
///
/// Marked content provides a way to identify portions of a content stream
/// and associate them with structure elements in the structure tree.
///
/// # Operators
///
/// - **BMC** (Begin Marked Content): Simple marked content without properties
/// - **BDC** (Begin Marked Content with Dictionary): Marked content with properties
/// - **EMC** (End Marked Content): Closes the most recent BMC or BDC
///
/// # Example
///
/// ```rust
/// use oxidize_pdf::structure::MarkedContent;
///
/// let mut mc = MarkedContent::new();
///
/// // Begin marked content with MCID for structure element
/// mc.begin_with_mcid("P", 0);
/// // ... add content (text, graphics, etc.) ...
/// mc.end();
///
/// // Get the PDF operators as string
/// let operators = mc.finish();
/// ```
use crate::error::{PdfError, Result};
use std::fmt::Write;

/// Marked content builder for Tagged PDF
#[derive(Clone, Debug)]
pub struct MarkedContent {
    operations: String,
    /// Stack of open marked content tags (for validation)
    tag_stack: Vec<String>,
}

impl Default for MarkedContent {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkedContent {
    /// Creates a new marked content builder
    pub fn new() -> Self {
        Self {
            operations: String::new(),
            tag_stack: Vec::new(),
        }
    }

    /// Begin marked content without properties (BMC operator)
    ///
    /// # Arguments
    ///
    /// * `tag` - Structure type tag (e.g., "P" for paragraph, "H1" for heading)
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::structure::MarkedContent;
    ///
    /// let mut mc = MarkedContent::new();
    /// mc.begin("P");
    /// // ... add content ...
    /// mc.end();
    /// ```
    pub fn begin(&mut self, tag: &str) -> Result<&mut Self> {
        // Validate tag name (must be a valid PDF name)
        if tag.is_empty() || tag.contains(|c: char| c.is_whitespace() || "()<>[]{}/%".contains(c))
        {
            return Err(PdfError::InvalidOperation(format!(
                "Invalid marked content tag: '{tag}'"
            )));
        }

        writeln!(&mut self.operations, "/{tag} BMC")
            .map_err(|e| PdfError::Internal(format!("Failed to write BMC operator: {e}")))?;

        self.tag_stack.push(tag.to_string());
        Ok(self)
    }

    /// Begin marked content with properties dictionary (BDC operator)
    ///
    /// This is the primary method for Tagged PDF, as it allows specifying
    /// the MCID (Marked Content ID) that links content to structure elements.
    ///
    /// # Arguments
    ///
    /// * `tag` - Structure type tag (e.g., "P", "H1", "Figure")
    /// * `mcid` - Marked Content ID linking to structure tree
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::structure::MarkedContent;
    ///
    /// let mut mc = MarkedContent::new();
    /// mc.begin_with_mcid("P", 0)?;
    /// // ... add paragraph content ...
    /// mc.end()?;
    /// # Ok::<(), oxidize_pdf::PdfError>(())
    /// ```
    pub fn begin_with_mcid(&mut self, tag: &str, mcid: u32) -> Result<&mut Self> {
        // Validate tag name
        if tag.is_empty() || tag.contains(|c: char| c.is_whitespace() || "()<>[]{}/%".contains(c))
        {
            return Err(PdfError::InvalidOperation(format!(
                "Invalid marked content tag: '{tag}'"
            )));
        }

        // BDC operator with inline dictionary containing MCID
        writeln!(&mut self.operations, "/{tag} << /MCID {mcid} >> BDC").map_err(|e| {
            PdfError::Internal(format!("Failed to write BDC operator: {e}"))
        })?;

        self.tag_stack.push(tag.to_string());
        Ok(self)
    }

    /// Begin marked content with custom properties dictionary
    ///
    /// Allows specifying additional properties beyond MCID.
    ///
    /// # Arguments
    ///
    /// * `tag` - Structure type tag
    /// * `properties` - Dictionary entries as key-value pairs
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::structure::MarkedContent;
    ///
    /// let mut mc = MarkedContent::new();
    /// let props = vec![("MCID", "0"), ("Lang", "(en-US)")];
    /// mc.begin_with_properties("P", &props)?;
    /// # Ok::<(), oxidize_pdf::PdfError>(())
    /// ```
    pub fn begin_with_properties(
        &mut self,
        tag: &str,
        properties: &[(&str, &str)],
    ) -> Result<&mut Self> {
        // Validate tag name
        if tag.is_empty() || tag.contains(|c: char| c.is_whitespace() || "()<>[]{}/%".contains(c))
        {
            return Err(PdfError::InvalidOperation(format!(
                "Invalid marked content tag: '{tag}'"
            )));
        }

        // Build properties dictionary
        write!(&mut self.operations, "/{tag} <<").map_err(|e| {
            PdfError::Internal(format!("Failed to write BDC operator start: {e}"))
        })?;

        for (key, value) in properties {
            write!(&mut self.operations, " /{key} {value}").map_err(|e| {
                PdfError::Internal(format!("Failed to write property {key}: {e}"))
            })?;
        }

        writeln!(&mut self.operations, " >> BDC").map_err(|e| {
            PdfError::Internal(format!("Failed to write BDC operator end: {e}"))
        })?;

        self.tag_stack.push(tag.to_string());
        Ok(self)
    }

    /// End marked content (EMC operator)
    ///
    /// Closes the most recently opened marked content section.
    ///
    /// # Errors
    ///
    /// Returns an error if there are no open marked content sections.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::structure::MarkedContent;
    ///
    /// let mut mc = MarkedContent::new();
    /// mc.begin("P")?;
    /// mc.end()?; // Closes the "P" section
    /// # Ok::<(), oxidize_pdf::PdfError>(())
    /// ```
    pub fn end(&mut self) -> Result<&mut Self> {
        if self.tag_stack.is_empty() {
            return Err(PdfError::InvalidStructure(
                "Cannot end marked content: no open sections".to_string(),
            ));
        }

        self.tag_stack.pop();
        writeln!(&mut self.operations, "EMC")
            .map_err(|e| PdfError::Internal(format!("Failed to write EMC operator: {e}")))?;

        Ok(self)
    }

    /// Returns true if there are open marked content sections
    pub fn has_open_sections(&self) -> bool {
        !self.tag_stack.is_empty()
    }

    /// Returns the number of open marked content sections
    pub fn open_section_count(&self) -> usize {
        self.tag_stack.len()
    }

    /// Get the current tag stack (for debugging/validation)
    pub fn tag_stack(&self) -> &[String] {
        &self.tag_stack
    }

    /// Finishes marked content generation and returns the PDF operators
    ///
    /// # Errors
    ///
    /// Returns an error if there are still open marked content sections.
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::structure::MarkedContent;
    ///
    /// let mut mc = MarkedContent::new();
    /// mc.begin_with_mcid("P", 0)?;
    /// mc.end()?;
    /// let operators = mc.finish()?;
    /// assert!(operators.contains("BMC") || operators.contains("BDC"));
    /// assert!(operators.contains("EMC"));
    /// # Ok::<(), oxidize_pdf::PdfError>(())
    /// ```
    pub fn finish(self) -> Result<String> {
        if !self.tag_stack.is_empty() {
            return Err(PdfError::InvalidStructure(format!(
                "Cannot finish marked content: {} open section(s) remaining: {:?}",
                self.tag_stack.len(),
                self.tag_stack
            )));
        }

        Ok(self.operations)
    }

    /// Returns the operations string without consuming self
    ///
    /// Unlike `finish()`, this does not validate that all sections are closed.
    /// Useful for incremental content generation.
    pub fn operations(&self) -> &str {
        &self.operations
    }

    /// Clears all operations and resets the tag stack
    ///
    /// Useful for reusing the builder for multiple content sections.
    pub fn reset(&mut self) {
        self.operations.clear();
        self.tag_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bmc_operator() {
        let mut mc = MarkedContent::new();
        mc.begin("P").unwrap();
        mc.end().unwrap();

        let ops = mc.finish().unwrap();
        assert!(ops.contains("/P BMC"));
        assert!(ops.contains("EMC"));
    }

    #[test]
    fn test_bdc_with_mcid() {
        let mut mc = MarkedContent::new();
        mc.begin_with_mcid("P", 42).unwrap();
        mc.end().unwrap();

        let ops = mc.finish().unwrap();
        assert!(ops.contains("/P << /MCID 42 >> BDC"));
        assert!(ops.contains("EMC"));
    }

    #[test]
    fn test_bdc_with_properties() {
        let mut mc = MarkedContent::new();
        let props = vec![("MCID", "0"), ("Lang", "(en-US)")];
        mc.begin_with_properties("P", &props).unwrap();
        mc.end().unwrap();

        let ops = mc.finish().unwrap();
        assert!(ops.contains("/P << /MCID 0 /Lang (en-US) >> BDC"));
    }

    #[test]
    fn test_nested_marked_content() {
        let mut mc = MarkedContent::new();
        mc.begin_with_mcid("Div", 0).unwrap();
        mc.begin_with_mcid("P", 1).unwrap();
        mc.end().unwrap(); // Close P
        mc.end().unwrap(); // Close Div

        let ops = mc.finish().unwrap();
        assert_eq!(ops.matches("BDC").count(), 2);
        assert_eq!(ops.matches("EMC").count(), 2);
    }

    #[test]
    fn test_invalid_tag_name() {
        let mut mc = MarkedContent::new();
        let result = mc.begin("Invalid Tag");
        assert!(result.is_err());

        let result = mc.begin("Tag<>");
        assert!(result.is_err());
    }

    #[test]
    fn test_end_without_begin() {
        let mut mc = MarkedContent::new();
        let result = mc.end();
        assert!(result.is_err());
    }

    #[test]
    fn test_finish_with_open_sections() {
        let mut mc = MarkedContent::new();
        mc.begin("P").unwrap();
        // Don't call end()

        let result = mc.finish();
        assert!(result.is_err());
    }

    #[test]
    fn test_tag_stack() {
        let mut mc = MarkedContent::new();
        assert_eq!(mc.open_section_count(), 0);
        assert!(!mc.has_open_sections());

        mc.begin("Div").unwrap();
        assert_eq!(mc.open_section_count(), 1);
        assert!(mc.has_open_sections());

        mc.begin("P").unwrap();
        assert_eq!(mc.open_section_count(), 2);

        mc.end().unwrap();
        assert_eq!(mc.open_section_count(), 1);

        mc.end().unwrap();
        assert_eq!(mc.open_section_count(), 0);
        assert!(!mc.has_open_sections());
    }

    #[test]
    fn test_reset() {
        let mut mc = MarkedContent::new();
        mc.begin("P").unwrap();
        mc.end().unwrap();

        mc.reset();
        assert_eq!(mc.operations().len(), 0);
        assert_eq!(mc.open_section_count(), 0);
    }
}
