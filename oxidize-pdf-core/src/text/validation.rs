//! Text validation and search utilities for OCR results
//!
//! This module provides functionality for validating and searching through
//! OCR-extracted text to find key elements like dates, contract terms, etc.

use regex::Regex;
use std::collections::HashMap;

/// Results from searching and validating OCR text
#[derive(Debug, Clone)]
pub struct TextValidationResult {
    /// Whether the target string was found
    pub found: bool,
    /// All matches found
    pub matches: Vec<TextMatch>,
    /// Confidence score of the overall validation
    pub confidence: f64,
    /// Additional metadata extracted
    pub metadata: HashMap<String, String>,
}

/// A specific match found in the text
#[derive(Debug, Clone)]
pub struct TextMatch {
    /// The matched text
    pub text: String,
    /// Position in the original text
    pub position: usize,
    /// Length of the match
    pub length: usize,
    /// Confidence of this specific match
    pub confidence: f64,
    /// Type of match (date, name, etc.)
    pub match_type: MatchType,
}

/// Type of text match found
#[derive(Debug, Clone, PartialEq)]
pub enum MatchType {
    Date,
    ContractNumber,
    PartyName,
    MonetaryAmount,
    Location,
    Custom(String),
}

/// Text validator for OCR results
pub struct TextValidator {
    /// Date patterns to search for
    date_patterns: Vec<Regex>,
    /// Contract-specific patterns
    contract_patterns: Vec<Regex>,
    /// Custom patterns (reserved for future use)
    #[allow(dead_code)]
    custom_patterns: HashMap<String, Regex>,
}

impl TextValidator {
    /// Create a new text validator with default patterns
    pub fn new() -> Self {
        let mut validator = Self {
            date_patterns: Vec::new(),
            contract_patterns: Vec::new(),
            custom_patterns: HashMap::new(),
        };

        validator.init_default_patterns();
        validator
    }

    /// Initialize default patterns for common contract elements
    fn init_default_patterns(&mut self) {
        // Date patterns - various formats
        let date_patterns = vec![
            // "30 September 2016", "September 30, 2016", etc.
            r"\b\d{1,2}\s+(?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{4}\b",
            // "September 30, 2016"
            r"\b(?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{1,2},?\s+\d{4}\b",
            // "30/09/2016", "09/30/2016"
            r"\b\d{1,2}[\/\-]\d{1,2}[\/\-]\d{4}\b",
            // "2016-09-30"
            r"\b\d{4}[\/\-]\d{1,2}[\/\-]\d{1,2}\b",
        ];

        for pattern in date_patterns {
            if let Ok(regex) = Regex::new(&format!("(?i){}", pattern)) {
                self.date_patterns.push(regex);
            }
        }

        // Contract-specific patterns
        let contract_patterns = vec![
            // Agreement numbers, contract numbers
            r"\b(?:Agreement|Contract)\s+(?:No\.?|Number)?\s*:?\s*([A-Z0-9\-\/]+)",
            // Party names (organizations ending with common suffixes)
            r"\b([A-Z][A-Za-z\s&,\.]+(?:LLC|Ltd|Corp|Corporation|Inc|Company|Co\.)\b)",
            // Monetary amounts
            r"\$\s*[\d,]+(?:\.\d{2})?(?:\s*(?:million|thousand|M|K))?",
        ];

        for pattern in contract_patterns {
            if let Ok(regex) = Regex::new(&format!("(?i){}", pattern)) {
                self.contract_patterns.push(regex);
            }
        }
    }

    /// Search for a specific target string in the text
    pub fn search_for_target(&self, text: &str, target: &str) -> TextValidationResult {
        let target_lower = target.to_lowercase();
        let text_lower = text.to_lowercase();

        let mut matches = Vec::new();
        let mut position = 0;

        // Find all occurrences of the target string
        while let Some(found_pos) = text_lower[position..].find(&target_lower) {
            let actual_pos = position + found_pos;
            let actual_text = &text[actual_pos..actual_pos + target.len()];

            matches.push(TextMatch {
                text: actual_text.to_string(),
                position: actual_pos,
                length: target.len(),
                confidence: calculate_string_similarity(
                    &target_lower,
                    &text_lower[actual_pos..actual_pos + target.len()],
                ),
                match_type: MatchType::Custom("target_search".to_string()),
            });

            position = actual_pos + 1;
        }

        TextValidationResult {
            found: !matches.is_empty(),
            confidence: if matches.is_empty() {
                0.0
            } else {
                matches.iter().map(|m| m.confidence).sum::<f64>() / matches.len() as f64
            },
            matches,
            metadata: HashMap::new(),
        }
    }

    /// Perform comprehensive validation of OCR text
    pub fn validate_contract_text(&self, text: &str) -> TextValidationResult {
        let mut all_matches = Vec::new();
        let mut metadata = HashMap::new();

        // Search for dates
        for pattern in &self.date_patterns {
            for mat in pattern.find_iter(text) {
                all_matches.push(TextMatch {
                    text: mat.as_str().to_string(),
                    position: mat.start(),
                    length: mat.len(),
                    confidence: 0.9, // High confidence for regex matches
                    match_type: MatchType::Date,
                });
            }
        }

        // Search for contract elements
        for pattern in &self.contract_patterns {
            for mat in pattern.find_iter(text) {
                let match_text = mat.as_str().to_string();
                let match_type = if match_text.contains("$") {
                    MatchType::MonetaryAmount
                } else if match_text.to_lowercase().contains("agreement")
                    || match_text.to_lowercase().contains("contract")
                {
                    MatchType::ContractNumber
                } else {
                    MatchType::PartyName
                };

                all_matches.push(TextMatch {
                    text: match_text,
                    position: mat.start(),
                    length: mat.len(),
                    confidence: 0.8,
                    match_type,
                });
            }
        }

        // Calculate overall confidence
        let confidence = if all_matches.is_empty() {
            0.0
        } else {
            all_matches.iter().map(|m| m.confidence).sum::<f64>() / all_matches.len() as f64
        };

        // Add metadata
        metadata.insert("total_matches".to_string(), all_matches.len().to_string());
        metadata.insert("text_length".to_string(), text.len().to_string());

        let date_matches = all_matches
            .iter()
            .filter(|m| m.match_type == MatchType::Date)
            .count();
        metadata.insert("date_matches".to_string(), date_matches.to_string());

        TextValidationResult {
            found: !all_matches.is_empty(),
            confidence,
            matches: all_matches,
            metadata,
        }
    }

    /// Extract key information from contract text
    pub fn extract_key_info(&self, text: &str) -> HashMap<String, Vec<String>> {
        let mut extracted = HashMap::new();

        // Extract dates
        let mut dates = Vec::new();
        for pattern in &self.date_patterns {
            for mat in pattern.find_iter(text) {
                dates.push(mat.as_str().to_string());
            }
        }
        if !dates.is_empty() {
            extracted.insert("dates".to_string(), dates);
        }

        // Extract monetary amounts
        let money_regex =
            Regex::new(r"\$\s*[\d,]+(?:\.\d{2})?(?:\s*(?:million|thousand|M|K))?").unwrap();
        let mut amounts = Vec::new();
        for mat in money_regex.find_iter(text) {
            amounts.push(mat.as_str().to_string());
        }
        if !amounts.is_empty() {
            extracted.insert("monetary_amounts".to_string(), amounts);
        }

        // Extract potential party names (capitalized words followed by organization suffixes)
        let org_regex =
            Regex::new(r"\b([A-Z][A-Za-z\s&,\.]+(?:LLC|Ltd|Corp|Corporation|Inc|Company|Co\.)\b)")
                .unwrap();
        let mut organizations = Vec::new();
        for mat in org_regex.find_iter(text) {
            organizations.push(mat.as_str().to_string());
        }
        if !organizations.is_empty() {
            extracted.insert("organizations".to_string(), organizations);
        }

        extracted
    }
}

impl Default for TextValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate similarity between two strings (0.0 to 1.0)
fn calculate_string_similarity(s1: &str, s2: &str) -> f64 {
    if s1 == s2 {
        return 1.0;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    if s1_chars.is_empty() || s2_chars.is_empty() {
        return 0.0;
    }

    // Simple character-based similarity
    let max_len = s1_chars.len().max(s2_chars.len());
    let min_len = s1_chars.len().min(s2_chars.len());

    let mut matches = 0;
    for i in 0..min_len {
        if s1_chars[i] == s2_chars[i] {
            matches += 1;
        }
    }

    matches as f64 / max_len as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_validation() {
        let validator = TextValidator::new();
        let text =
            "This agreement was signed on 30 September 2016 and expires on December 31, 2020.";

        let result = validator.validate_contract_text(text);
        assert!(result.found);

        // Should find at least the dates
        let date_matches: Vec<_> = result
            .matches
            .iter()
            .filter(|m| m.match_type == MatchType::Date)
            .collect();
        assert!(!date_matches.is_empty());
    }

    #[test]
    fn test_target_search() {
        let validator = TextValidator::new();
        let text = "The contract was executed on 30 September 2016 by both parties.";

        let result = validator.search_for_target(text, "30 September 2016");
        assert!(result.found);
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].text, "30 September 2016");
    }

    #[test]
    fn test_key_info_extraction() {
        let validator = TextValidator::new();
        let text =
            "Agreement between ABC Corp and XYZ LLC for $1,000,000 signed on 30 September 2016.";

        let extracted = validator.extract_key_info(text);

        assert!(extracted.contains_key("dates"));
        assert!(extracted.contains_key("monetary_amounts"));
        assert!(extracted.contains_key("organizations"));
    }
}
