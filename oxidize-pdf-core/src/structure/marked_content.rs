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
/// # Thread Safety
///
/// `MarkedContent` is not thread-safe by design. Each instance should be used
/// within a single thread. For concurrent PDF generation, create separate
/// `MarkedContent` instances per thread.
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
use lazy_static::lazy_static;
use regex::Regex;
use std::fmt::Write;

// Constants for validation and limits
const MAX_TAG_LENGTH: usize = 127; // PDF name object limit
const MAX_OPERATIONS_SIZE: usize = 10 * 1024 * 1024; // 10MB limit
const MAX_NESTING_DEPTH: usize = 100; // Reasonable nesting limit

lazy_static! {
    /// Valid PDF name pattern: alphanumeric, underscore, hyphen
    static ref VALID_TAG_PATTERN: Regex = Regex::new(r"^[A-Za-z0-9_-]+$").unwrap();
}

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

/// Common marked content properties
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkedContentProperty {
    /// Marked Content ID (links to structure tree)
    MCID(u32),
    /// Language specification
    Lang(String),
    /// Actual text (for accessibility)
    ActualText(String),
    /// Alternate description
    Alt(String),
    /// Expansion of abbreviation
    E(String),
}

impl MarkedContentProperty {
    /// Returns the PDF dictionary key for this property
    fn key(&self) -> &str {
        match self {
            MarkedContentProperty::MCID(_) => "MCID",
            MarkedContentProperty::Lang(_) => "Lang",
            MarkedContentProperty::ActualText(_) => "ActualText",
            MarkedContentProperty::Alt(_) => "Alt",
            MarkedContentProperty::E(_) => "E",
        }
    }

    /// Returns the PDF dictionary value for this property
    fn value(&self) -> String {
        match self {
            MarkedContentProperty::MCID(id) => id.to_string(),
            MarkedContentProperty::Lang(s)
            | MarkedContentProperty::ActualText(s)
            | MarkedContentProperty::Alt(s)
            | MarkedContentProperty::E(s) => format!(
                "({})",
                s.replace('\\', "\\\\")
                    .replace('(', "\\(")
                    .replace(')', "\\)")
            ),
        }
    }
}

/// Validates a marked content tag name
///
/// Tags must be valid PDF name objects: alphanumeric, underscore, or hyphen.
/// Maximum length is 127 characters.
fn validate_tag(tag: &str) -> Result<()> {
    if tag.is_empty() {
        return Err(PdfError::InvalidOperation(
            "Marked content tag cannot be empty".to_string(),
        ));
    }

    if tag.len() > MAX_TAG_LENGTH {
        return Err(PdfError::InvalidOperation(format!(
            "Marked content tag too long: {} characters (max {})",
            tag.len(),
            MAX_TAG_LENGTH
        )));
    }

    if !VALID_TAG_PATTERN.is_match(tag) {
        return Err(PdfError::InvalidOperation(format!(
            "Invalid marked content tag '{}': must contain only alphanumeric, underscore, or hyphen",
            tag
        )));
    }

    Ok(())
}

impl MarkedContent {
    /// Creates a new marked content builder
    pub fn new() -> Self {
        Self {
            operations: String::new(),
            tag_stack: Vec::new(),
        }
    }

    /// Checks if size limit has been exceeded
    fn check_size_limit(&self) -> Result<()> {
        if self.operations.len() > MAX_OPERATIONS_SIZE {
            return Err(PdfError::InvalidOperation(format!(
                "Marked content operations exceed size limit: {} bytes (max {})",
                self.operations.len(),
                MAX_OPERATIONS_SIZE
            )));
        }
        Ok(())
    }

    /// Checks if nesting depth limit has been exceeded
    fn check_nesting_limit(&self) -> Result<()> {
        if self.tag_stack.len() >= MAX_NESTING_DEPTH {
            return Err(PdfError::InvalidOperation(format!(
                "Marked content nesting too deep: {} levels (max {})",
                self.tag_stack.len(),
                MAX_NESTING_DEPTH
            )));
        }
        Ok(())
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
        validate_tag(tag)?;
        self.check_nesting_limit()?;
        self.check_size_limit()?;

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
        validate_tag(tag)?;
        self.check_nesting_limit()?;
        self.check_size_limit()?;

        // BDC operator with inline dictionary containing MCID
        writeln!(&mut self.operations, "/{tag} << /MCID {mcid} >> BDC")
            .map_err(|e| PdfError::Internal(format!("Failed to write BDC operator: {e}")))?;

        self.tag_stack.push(tag.to_string());
        Ok(self)
    }

    /// Begin marked content with typed properties (BDC operator)
    ///
    /// This is a type-safe alternative to `begin_with_properties` that uses
    /// an enum for common marked content properties.
    ///
    /// # Arguments
    ///
    /// * `tag` - Structure type tag
    /// * `properties` - Typed properties (MCID, Lang, ActualText, etc.)
    ///
    /// # Example
    ///
    /// ```
    /// use oxidize_pdf::structure::{MarkedContent, MarkedContentProperty};
    ///
    /// let mut mc = MarkedContent::new();
    /// let props = vec![
    ///     MarkedContentProperty::MCID(0),
    ///     MarkedContentProperty::Lang("en-US".to_string()),
    /// ];
    /// mc.begin_with_typed_properties("P", &props)?;
    /// # Ok::<(), oxidize_pdf::PdfError>(())
    /// ```
    pub fn begin_with_typed_properties(
        &mut self,
        tag: &str,
        properties: &[MarkedContentProperty],
    ) -> Result<&mut Self> {
        validate_tag(tag)?;
        self.check_nesting_limit()?;
        self.check_size_limit()?;

        // Build properties dictionary
        write!(&mut self.operations, "/{tag} <<")
            .map_err(|e| PdfError::Internal(format!("Failed to write BDC operator start: {e}")))?;

        for prop in properties {
            write!(&mut self.operations, " /{} {}", prop.key(), prop.value()).map_err(|e| {
                PdfError::Internal(format!("Failed to write property {}: {e}", prop.key()))
            })?;
        }

        writeln!(&mut self.operations, " >> BDC")
            .map_err(|e| PdfError::Internal(format!("Failed to write BDC operator end: {e}")))?;

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
        validate_tag(tag)?;
        self.check_nesting_limit()?;
        self.check_size_limit()?;

        // Build properties dictionary
        write!(&mut self.operations, "/{tag} <<")
            .map_err(|e| PdfError::Internal(format!("Failed to write BDC operator start: {e}")))?;

        for (key, value) in properties {
            write!(&mut self.operations, " /{key} {value}")
                .map_err(|e| PdfError::Internal(format!("Failed to write property {key}: {e}")))?;
        }

        writeln!(&mut self.operations, " >> BDC")
            .map_err(|e| PdfError::Internal(format!("Failed to write BDC operator end: {e}")))?;

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

    // NEW TESTS for quality improvements

    #[test]
    fn test_deep_nesting() {
        // Test 20-level nesting
        let mut mc = MarkedContent::new();

        // Open 20 levels
        for i in 0..20 {
            mc.begin(&format!("Level{}", i)).unwrap();
        }

        assert_eq!(mc.open_section_count(), 20);

        // Close all 20 levels
        for _ in 0..20 {
            mc.end().unwrap();
        }

        assert_eq!(mc.open_section_count(), 0);
        let ops = mc.finish().unwrap();
        assert_eq!(ops.matches("BMC").count(), 20);
        assert_eq!(ops.matches("EMC").count(), 20);
    }

    #[test]
    fn test_nesting_limit_exceeded() {
        let mut mc = MarkedContent::new();

        // Try to exceed the 100-level limit
        for i in 0..100 {
            mc.begin(&format!("L{}", i)).unwrap();
        }

        // 101st level should fail
        let result = mc.begin("TooDeep");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nesting too deep"));
    }

    #[test]
    fn test_tag_validation_alphanumeric() {
        let mut mc = MarkedContent::new();

        // Valid tags
        assert!(mc.begin("P").is_ok());
        mc.end().unwrap();

        assert!(mc.begin("H1").is_ok());
        mc.end().unwrap();

        assert!(mc.begin("My_Tag").is_ok());
        mc.end().unwrap();

        assert!(mc.begin("Tag-123").is_ok());
        mc.end().unwrap();
    }

    #[test]
    fn test_tag_validation_invalid_chars() {
        let mut mc = MarkedContent::new();

        // Invalid: contains special characters
        assert!(mc.begin("Tag@Value").is_err());
        assert!(mc.begin("Tag#123").is_err());
        assert!(mc.begin("Tag$Name").is_err());
        assert!(mc.begin("Tag%").is_err());
        assert!(mc.begin("Tag/Path").is_err());
    }

    #[test]
    fn test_tag_length_limit() {
        let mut mc = MarkedContent::new();

        // 127 characters (at limit)
        let max_tag = "A".repeat(127);
        assert!(mc.begin(&max_tag).is_ok());
        mc.end().unwrap();

        // 128 characters (over limit)
        let over_tag = "A".repeat(128);
        assert!(mc.begin(&over_tag).is_err());
    }

    #[test]
    fn test_typed_properties() {
        let mut mc = MarkedContent::new();

        let props = vec![
            MarkedContentProperty::MCID(42),
            MarkedContentProperty::Lang("en-US".to_string()),
            MarkedContentProperty::ActualText("Hello".to_string()),
        ];

        mc.begin_with_typed_properties("P", &props).unwrap();
        mc.end().unwrap();

        let ops = mc.finish().unwrap();
        assert!(ops.contains("/MCID 42"));
        assert!(ops.contains("/Lang (en-US)"));
        assert!(ops.contains("/ActualText (Hello)"));
    }

    #[test]
    fn test_property_string_escaping() {
        let mut mc = MarkedContent::new();

        let props = vec![
            MarkedContentProperty::ActualText("Text with (parens)".to_string()),
            MarkedContentProperty::Alt("Text\\with\\backslashes".to_string()),
        ];

        mc.begin_with_typed_properties("P", &props).unwrap();
        mc.end().unwrap();

        let ops = mc.finish().unwrap();
        // Verify escaping
        assert!(ops.contains("\\(") && ops.contains("\\)"));
        assert!(ops.contains("\\\\"));
    }

    #[test]
    fn test_size_limit() {
        let mut mc = MarkedContent::new();

        // Generate content until we hit the size limit
        // We'll add a large tag to make each operation substantial
        let large_tag = "T".repeat(100); // 100-char tag

        let mut iteration_count = 0;
        let mut hit_limit = false;

        // Try to add many operations
        for i in 0..200_000 {
            iteration_count = i;

            // Use unique large tags to grow the buffer
            let tag = format!("{}{}", large_tag, i);

            match mc.begin(&tag) {
                Ok(_) => {
                    // Successfully added, now end it
                    if mc.end().is_err() {
                        hit_limit = true;
                        break;
                    }
                }
                Err(_) => {
                    // Hit the limit
                    hit_limit = true;
                    break;
                }
            }
        }

        // Verify we eventually hit the size limit
        assert!(
            hit_limit,
            "Expected to hit size limit but completed {} iterations (ops size: {} bytes)",
            iteration_count,
            mc.operations().len()
        );

        // Verify operations size is at or near the limit
        let ops_size = mc.operations().len();
        assert!(
            ops_size > 9_000_000,
            "Operations size {} should be near 10MB limit after hitting size check",
            ops_size
        );
    }

    #[test]
    fn test_property_enum_equality() {
        let prop1 = MarkedContentProperty::MCID(42);
        let prop2 = MarkedContentProperty::MCID(42);
        let prop3 = MarkedContentProperty::MCID(43);

        assert_eq!(prop1, prop2);
        assert_ne!(prop1, prop3);

        let prop4 = MarkedContentProperty::Lang("en".to_string());
        let prop5 = MarkedContentProperty::Lang("en".to_string());
        assert_eq!(prop4, prop5);
    }

    #[test]
    fn test_empty_tag() {
        let mut mc = MarkedContent::new();
        let result = mc.begin("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_tag_function() {
        // Test the validation function directly
        assert!(validate_tag("P").is_ok());
        assert!(validate_tag("H1").is_ok());
        assert!(validate_tag("My-Tag_123").is_ok());

        assert!(validate_tag("").is_err());
        assert!(validate_tag("Tag with spaces").is_err());
        assert!(validate_tag("Tag<>").is_err());
        assert!(validate_tag(&"A".repeat(128)).is_err());
    }
}
