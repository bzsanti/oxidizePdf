//! Pattern matching for invoice fields
//!
//! This module contains regex patterns and matching logic for extracting
//! structured data from invoice text.

use super::error::{ExtractionError, Result};
use super::types::{InvoiceField, Language};
use regex::Regex;
use std::collections::HashMap;

/// A pattern for matching invoice fields
#[derive(Debug, Clone)]
pub struct FieldPattern {
    /// Type of field this pattern matches
    pub field_type: InvoiceFieldType,

    /// Compiled regex pattern
    pub regex: Regex,

    /// Base confidence score (0.0 to 1.0)
    pub confidence_base: f64,

    /// Language this pattern is specific to (None = all languages)
    pub language: Option<Language>,

    /// Context hints - words that increase confidence when found nearby
    pub context_hints: Vec<String>,
}

/// Field type identifier (without data)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvoiceFieldType {
    InvoiceNumber,
    InvoiceDate,
    DueDate,
    TotalAmount,
    TaxAmount,
    NetAmount,
    VatNumber,
    SupplierName,
    CustomerName,
    Currency,
    ArticleNumber,
    LineItemDescription,
    LineItemQuantity,
    LineItemUnitPrice,
}

impl FieldPattern {
    /// Create a new field pattern
    pub fn new(
        field_type: InvoiceFieldType,
        pattern: &str,
        confidence_base: f64,
        language: Option<Language>,
    ) -> Result<Self> {
        let regex = Regex::new(pattern)
            .map_err(|e| ExtractionError::RegexError(format!("{}: {}", pattern, e)))?;

        Ok(Self {
            field_type,
            regex,
            confidence_base,
            language,
            context_hints: Vec::new(),
        })
    }

    /// Add context hints to this pattern
    pub fn with_hints(mut self, hints: Vec<String>) -> Self {
        self.context_hints = hints;
        self
    }

    /// Check if this pattern matches the given text
    pub fn matches(&self, text: &str) -> Option<String> {
        self.regex
            .captures(text)
            .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
    }
}

/// Library of patterns for invoice field extraction
pub struct PatternLibrary {
    patterns: Vec<FieldPattern>,
}

impl PatternLibrary {
    /// Create a new empty pattern library
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    /// Create a pattern library for a specific language
    pub fn with_language(lang: Language) -> Self {
        let mut lib = Self::new();
        lib.load_patterns_for_language(lang);
        lib
    }

    /// Add a pattern to the library
    pub fn add_pattern(&mut self, pattern: FieldPattern) {
        self.patterns.push(pattern);
    }

    /// Match text against all patterns
    pub fn match_text(&self, text: &str) -> Vec<(InvoiceFieldType, String, f64)> {
        let mut matches = Vec::new();

        for pattern in &self.patterns {
            if let Some(matched_value) = pattern.matches(text) {
                matches.push((
                    pattern.field_type,
                    matched_value,
                    pattern.confidence_base,
                ));
            }
        }

        matches
    }

    /// Load patterns for a specific language
    fn load_patterns_for_language(&mut self, lang: Language) {
        match lang {
            Language::Spanish => self.load_spanish_patterns(),
            Language::English => self.load_english_patterns(),
            Language::German => self.load_german_patterns(),
            Language::Italian => self.load_italian_patterns(),
        }
    }

    /// Load Spanish invoice patterns
    fn load_spanish_patterns(&mut self) {
        // TODO: Implement Spanish patterns
    }

    /// Load English invoice patterns
    fn load_english_patterns(&mut self) {
        // TODO: Implement English patterns
    }

    /// Load German invoice patterns
    fn load_german_patterns(&mut self) {
        // TODO: Implement German patterns
    }

    /// Load Italian invoice patterns
    fn load_italian_patterns(&mut self) {
        // TODO: Implement Italian patterns
    }
}

impl Default for PatternLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_library_new() {
        let lib = PatternLibrary::new();
        assert_eq!(lib.patterns.len(), 0);
    }

    #[test]
    fn test_field_pattern_creation() {
        let pattern =
            FieldPattern::new(InvoiceFieldType::InvoiceNumber, r"INV-(\d+)", 0.9, None);
        assert!(pattern.is_ok());
    }

    #[test]
    fn test_field_pattern_invalid_regex() {
        let pattern = FieldPattern::new(InvoiceFieldType::InvoiceNumber, r"[invalid(", 0.9, None);
        assert!(pattern.is_err());
    }

    #[test]
    fn test_pattern_matches() {
        let pattern =
            FieldPattern::new(InvoiceFieldType::InvoiceNumber, r"INV-(\d+)", 0.9, None).unwrap();

        assert_eq!(pattern.matches("INV-12345"), Some("12345".to_string()));
        assert_eq!(pattern.matches("Invoice INV-999"), Some("999".to_string()));
        assert_eq!(pattern.matches("No match here"), None);
    }
}
