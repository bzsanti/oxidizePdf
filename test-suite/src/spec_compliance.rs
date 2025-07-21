//! PDF Specification Compliance Testing
//!
//! This module provides traits and implementations for testing PDF compliance
//! with various versions of the PDF specification (ISO 32000).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use oxidize_pdf::parser::reader::PDFLines;

/// Result of a specification test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Whether the test passed
    pub passed: bool,
    /// Test name
    pub test_name: String,
    /// Error or warning messages
    pub messages: Vec<String>,
    /// Additional details
    pub details: HashMap<String, String>,
}

impl TestResult {
    /// Create a passing test result
    pub fn pass(test_name: &str) -> Self {
        Self {
            passed: true,
            test_name: test_name.to_string(),
            messages: Vec::new(),
            details: HashMap::new(),
        }
    }

    /// Create a failing test result
    pub fn fail(test_name: &str, message: &str) -> Self {
        Self {
            passed: false,
            test_name: test_name.to_string(),
            messages: vec![message.to_string()],
            details: HashMap::new(),
        }
    }

    /// Add a message
    pub fn add_message(&mut self, message: &str) {
        self.messages.push(message.to_string());
    }

    /// Add a detail
    pub fn add_detail(&mut self, key: &str, value: &str) {
        self.details.insert(key.to_string(), value.to_string());
    }
}

/// Trait for PDF specification compliance testing
pub trait SpecificationTest {
    /// Test PDF header compliance
    fn test_header_compliance(&self, pdf: &[u8]) -> TestResult;

    /// Test cross-reference table compliance
    fn test_xref_compliance(&self, pdf: &[u8]) -> TestResult;

    /// Test object compliance
    fn test_object_compliance(&self, pdf: &[u8]) -> TestResult;

    /// Test content stream compliance
    fn test_content_stream_compliance(&self, pdf: &[u8]) -> TestResult;

    /// Test trailer compliance
    fn test_trailer_compliance(&self, pdf: &[u8]) -> TestResult;

    /// Test overall structure compliance
    fn test_structure_compliance(&self, pdf: &[u8]) -> TestResult;

    /// Run all compliance tests
    fn test_all(&self, pdf: &[u8]) -> Vec<TestResult> {
        vec![
            self.test_header_compliance(pdf),
            self.test_xref_compliance(pdf),
            self.test_object_compliance(pdf),
            self.test_content_stream_compliance(pdf),
            self.test_trailer_compliance(pdf),
            self.test_structure_compliance(pdf),
        ]
    }
}

/// PDF 1.7 (ISO 32000-1:2008) compliance tester
pub struct Pdf17ComplianceTester;

impl SpecificationTest for Pdf17ComplianceTester {
    fn test_header_compliance(&self, pdf: &[u8]) -> TestResult {
        let mut result = TestResult::pass("PDF 1.7 Header Compliance");

        // Check for %PDF-x.x header
        if pdf.len() < 8 {
            return TestResult::fail(
                "PDF 1.7 Header Compliance",
                "File too small to contain valid header",
            );
        }

        let header = &pdf[0..8];
        if !header.starts_with(b"%PDF-") {
            return TestResult::fail("PDF 1.7 Header Compliance", "Missing %PDF- header");
        }

        // Extract version
        let version_str = std::str::from_utf8(&header[5..8]).unwrap_or("");
        if let Ok(version) = version_str.trim().parse::<f32>() {
            result.add_detail("version", &format!("{version:.1}"));

            // Check if version is valid for PDF 1.7
            if version > 1.7 {
                result.passed = false;
                result.add_message(&format!("PDF version {version} is newer than 1.7"));
            }
        } else {
            return TestResult::fail(
                "PDF 1.7 Header Compliance",
                "Invalid version number in header",
            );
        }

        // Check for binary marker (recommended)
        let mut found_binary_marker = false;
        for &byte in pdf.iter().take(pdf.len().min(1024)).skip(8) {
            if byte > 127 {
                found_binary_marker = true;
                break;
            }
        }

        if !found_binary_marker {
            result
                .add_message("Warning: No binary marker found in header (recommended for PDF 1.7)");
        }

        result
    }

    fn test_xref_compliance(&self, pdf: &[u8]) -> TestResult {
        let mut result = TestResult::pass("PDF 1.7 Cross-Reference Table Compliance");

        // Find startxref
        let pdf_str = String::from_utf8_lossy(pdf);
        let startxref_pos = pdf_str.rfind("startxref");

        if startxref_pos.is_none() {
            return TestResult::fail(
                "PDF 1.7 Cross-Reference Table Compliance",
                "Missing startxref keyword",
            );
        }

        // Find xref table
        if let Some(xref_pos) = pdf_str.find("xref") {
            result.add_detail("xref_type", "table");

            // Basic xref table format validation
            let xref_section = &pdf_str[xref_pos..];
            let lines: Vec<&str> = xref_section.pdf_lines().collect();

            if lines.len() < 2 {
                return TestResult::fail(
                    "PDF 1.7 Cross-Reference Table Compliance",
                    "Invalid xref table format",
                );
            }

            // Check subsection header format
            if !lines[0].starts_with("xref") {
                result.passed = false;
                result.add_message("Invalid xref table header");
            }

            // Validate entries format (nnnnnnnnnn ggggg n/f)
            let entry_regex = regex::Regex::new(r"^\d{10} \d{5} [nf] $").unwrap();
            let mut valid_entries = 0;
            let mut invalid_entries = 0;

            for line in lines.iter().skip(2) {
                if line.starts_with("trailer") {
                    break;
                }
                if line.trim().is_empty() {
                    continue;
                }

                if entry_regex.is_match(line) {
                    valid_entries += 1;
                } else if line.chars().any(|c| c.is_numeric()) {
                    invalid_entries += 1;
                }
            }

            result.add_detail("valid_entries", &valid_entries.to_string());
            if invalid_entries > 0 {
                result.passed = false;
                result.add_message(&format!("{invalid_entries} invalid xref entries found"));
            }
        } else {
            // Check for cross-reference stream (PDF 1.5+)
            if pdf_str.contains("/Type /XRef") || pdf_str.contains("/Type/XRef") {
                result.add_detail("xref_type", "stream");
                result.add_message("Uses cross-reference stream (PDF 1.5+ feature)");
            } else {
                return TestResult::fail(
                    "PDF 1.7 Cross-Reference Table Compliance",
                    "No cross-reference table found",
                );
            }
        }

        result
    }

    fn test_object_compliance(&self, pdf: &[u8]) -> TestResult {
        let mut result = TestResult::pass("PDF 1.7 Object Compliance");

        let pdf_str = String::from_utf8_lossy(pdf);

        // Count different object patterns
        let obj_regex = regex::Regex::new(r"(\d+) (\d+) obj").unwrap();
        let mut objects = HashMap::new();

        for cap in obj_regex.captures_iter(&pdf_str) {
            let obj_num = cap[1].parse::<u32>().unwrap_or(0);
            let gen_num = cap[2].parse::<u32>().unwrap_or(0);
            objects.insert((obj_num, gen_num), true);
        }

        result.add_detail("object_count", &objects.len().to_string());

        // Check for required objects
        let mut has_catalog = false;
        let mut has_pages = false;

        // Look for catalog
        if pdf_str.contains("/Type /Catalog") || pdf_str.contains("/Type/Catalog") {
            has_catalog = true;
        }

        // Look for pages
        if pdf_str.contains("/Type /Pages") || pdf_str.contains("/Type/Pages") {
            has_pages = true;
        }

        if !has_catalog {
            result.passed = false;
            result.add_message("Missing required Catalog object");
        }

        if !has_pages {
            result.passed = false;
            result.add_message("Missing required Pages object");
        }

        // Check for balanced obj/endobj
        let obj_count = pdf_str.matches(" obj").count();
        let endobj_count = pdf_str.matches("endobj").count();

        if obj_count != endobj_count {
            result.passed = false;
            result.add_message(&format!(
                "Unbalanced obj/endobj: {obj_count} obj, {endobj_count} endobj"
            ));
        }

        result
    }

    fn test_content_stream_compliance(&self, pdf: &[u8]) -> TestResult {
        let mut result = TestResult::pass("PDF 1.7 Content Stream Compliance");

        let pdf_str = String::from_utf8_lossy(pdf);

        // Count streams
        let stream_count =
            pdf_str.matches("stream\n").count() + pdf_str.matches("stream\r\n").count();
        let endstream_count = pdf_str.matches("endstream").count();

        result.add_detail("stream_count", &stream_count.to_string());

        if stream_count != endstream_count {
            result.passed = false;
            result.add_message(&format!(
                "Unbalanced stream/endstream: {stream_count} stream, {endstream_count} endstream"
            ));
        }

        // Check for common content stream operators
        let operators = vec!["BT", "ET", "Tj", "Tf", "q", "Q", "cm", "re", "f", "S"];
        let mut found_operators = Vec::new();

        for op in operators {
            if pdf_str.contains(&format!(" {op}")) || pdf_str.contains(&format!("\n{op}")) {
                found_operators.push(op);
            }
        }

        if !found_operators.is_empty() {
            result.add_detail("content_operators", &found_operators.join(", "));
        }

        result
    }

    fn test_trailer_compliance(&self, pdf: &[u8]) -> TestResult {
        let mut result = TestResult::pass("PDF 1.7 Trailer Compliance");

        let pdf_str = String::from_utf8_lossy(pdf);

        // Check for trailer
        if !pdf_str.contains("trailer") {
            // Could be using cross-reference stream
            if pdf_str.contains("/Type /XRef") || pdf_str.contains("/Type/XRef") {
                result.add_message("Uses cross-reference stream instead of traditional trailer");
                return result;
            }
            return TestResult::fail("PDF 1.7 Trailer Compliance", "Missing trailer dictionary");
        }

        // Check for required trailer entries
        let trailer_pos = pdf_str.rfind("trailer").unwrap();
        let trailer_section = &pdf_str[trailer_pos..];

        // Check for /Size
        if !trailer_section.contains("/Size") {
            result.passed = false;
            result.add_message("Missing required /Size entry in trailer");
        }

        // Check for /Root
        if !trailer_section.contains("/Root") {
            result.passed = false;
            result.add_message("Missing required /Root entry in trailer");
        }

        // Check for %%EOF
        if !pdf_str.trim().ends_with("%%EOF") {
            result.add_message("Warning: Missing or misplaced %%EOF marker");
        }

        result
    }

    fn test_structure_compliance(&self, pdf: &[u8]) -> TestResult {
        let mut result = TestResult::pass("PDF 1.7 Overall Structure Compliance");

        // Test basic structure order
        let pdf_str = String::from_utf8_lossy(pdf);

        let header_pos = pdf_str.find("%PDF-");
        let body_pos = pdf_str.find(" obj");
        let xref_pos = pdf_str
            .find("xref\n")
            .or_else(|| pdf_str.find("/Type /XRef"));
        let trailer_pos = pdf_str.rfind("trailer");
        let startxref_pos = pdf_str.rfind("startxref");
        let eof_pos = pdf_str.rfind("%%EOF");

        // Check order
        if let (Some(h), Some(b)) = (header_pos, body_pos) {
            if h > b {
                result.passed = false;
                result.add_message("Invalid structure: header appears after body");
            }
        }

        if let (Some(b), Some(x)) = (body_pos, xref_pos) {
            if b > x {
                result.passed = false;
                result.add_message("Invalid structure: body appears after xref");
            }
        }

        if trailer_pos.is_some() && xref_pos.is_some() {
            if let (Some(x), Some(t)) = (xref_pos, trailer_pos) {
                if x > t {
                    result.passed = false;
                    result.add_message("Invalid structure: xref appears after trailer");
                }
            }
        }

        if let (Some(s), Some(e)) = (startxref_pos, eof_pos) {
            if s > e {
                result.passed = false;
                result.add_message("Invalid structure: startxref appears after %%EOF");
            }
        }

        // Check file size
        result.add_detail("file_size", &pdf.len().to_string());

        result
    }
}

/// PDF 2.0 (ISO 32000-2:2020) compliance tester
pub struct Pdf20ComplianceTester;

impl SpecificationTest for Pdf20ComplianceTester {
    fn test_header_compliance(&self, pdf: &[u8]) -> TestResult {
        let mut result = TestResult::pass("PDF 2.0 Header Compliance");

        // Check for %PDF-x.x header
        if pdf.len() < 8 {
            return TestResult::fail(
                "PDF 2.0 Header Compliance",
                "File too small to contain valid header",
            );
        }

        let header = &pdf[0..8];
        if !header.starts_with(b"%PDF-") {
            return TestResult::fail("PDF 2.0 Header Compliance", "Missing %PDF- header");
        }

        // Extract version
        let version_str = std::str::from_utf8(&header[5..8]).unwrap_or("");
        if let Ok(version) = version_str.trim().parse::<f32>() {
            result.add_detail("version", &format!("{version:.1}"));

            // Check if version is valid for PDF 2.0
            if version == 2.0 {
                result.add_detail("pdf_2_0", "true");
            } else if version < 2.0 {
                result.add_message(&format!("PDF version {version} is older than 2.0"));
            }
        } else {
            return TestResult::fail(
                "PDF 2.0 Header Compliance",
                "Invalid version number in header",
            );
        }

        // Check for binary marker (recommended)
        let mut found_binary_marker = false;
        for &byte in pdf.iter().take(pdf.len().min(1024)).skip(8) {
            if byte > 127 {
                found_binary_marker = true;
                break;
            }
        }

        if found_binary_marker {
            result.add_detail("binary_marker", "present");
        } else {
            result.add_message("Binary marker not found (recommended for PDF 2.0)");
        }

        result
    }

    fn test_xref_compliance(&self, pdf: &[u8]) -> TestResult {
        let mut result = Pdf17ComplianceTester.test_xref_compliance(pdf);
        result.test_name = "PDF 2.0 Cross-Reference Compliance".to_string();

        // PDF 2.0 prefers cross-reference streams
        if result.details.get("xref_type") == Some(&"table".to_string()) {
            result.add_message("Note: PDF 2.0 prefers cross-reference streams over tables");
        }

        result
    }

    fn test_object_compliance(&self, pdf: &[u8]) -> TestResult {
        let mut result = Pdf17ComplianceTester.test_object_compliance(pdf);
        result.test_name = "PDF 2.0 Object Compliance".to_string();

        // Check for PDF 2.0 specific features
        let pdf_str = String::from_utf8_lossy(pdf);

        // Check for new PDF 2.0 features
        if pdf_str.contains("/Type /DPartRoot") {
            result.add_detail("dpart_root", "true");
            result.add_message("Uses PDF 2.0 DPartRoot feature");
        }

        if pdf_str.contains("/Type /Namespaces") {
            result.add_detail("namespaces", "true");
            result.add_message("Uses PDF 2.0 Namespaces feature");
        }

        result
    }

    fn test_content_stream_compliance(&self, pdf: &[u8]) -> TestResult {
        let mut result = Pdf17ComplianceTester.test_content_stream_compliance(pdf);
        result.test_name = "PDF 2.0 Content Stream Compliance".to_string();

        // PDF 2.0 deprecates certain operators
        let pdf_str = String::from_utf8_lossy(pdf);
        let deprecated_ops = vec!["BX", "EX"];

        for op in deprecated_ops {
            if pdf_str.contains(&format!(" {op}")) || pdf_str.contains(&format!("\n{op}")) {
                result.add_message(&format!(
                    "Warning: Uses deprecated operator '{op}' in PDF 2.0"
                ));
            }
        }

        result
    }

    fn test_trailer_compliance(&self, pdf: &[u8]) -> TestResult {
        let mut result = Pdf17ComplianceTester.test_trailer_compliance(pdf);
        result.test_name = "PDF 2.0 Trailer Compliance".to_string();
        result
    }

    fn test_structure_compliance(&self, pdf: &[u8]) -> TestResult {
        let mut result = Pdf17ComplianceTester.test_structure_compliance(pdf);
        result.test_name = "PDF 2.0 Structure Compliance".to_string();
        result
    }
}

/// Run compliance tests against multiple specifications
pub fn test_compliance(pdf: &[u8]) -> HashMap<String, Vec<TestResult>> {
    let mut results = HashMap::new();

    // Test against PDF 1.7
    let pdf17_tester = Pdf17ComplianceTester;
    results.insert("PDF 1.7".to_string(), pdf17_tester.test_all(pdf));

    // Test against PDF 2.0
    let pdf20_tester = Pdf20ComplianceTester;
    results.insert("PDF 2.0".to_string(), pdf20_tester.test_all(pdf));

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_compliance() {
        let valid_pdf = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\ntest";
        let tester = Pdf17ComplianceTester;
        let result = tester.test_header_compliance(valid_pdf);
        assert!(result.passed);

        let invalid_pdf = b"Not a PDF";
        let result = tester.test_header_compliance(invalid_pdf);
        assert!(!result.passed);
    }

    #[test]
    fn test_result_creation() {
        let mut result = TestResult::pass("Test");
        assert!(result.passed);

        result.add_message("Warning");
        result.add_detail("key", "value");
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.details.get("key"), Some(&"value".to_string()));
    }
}
