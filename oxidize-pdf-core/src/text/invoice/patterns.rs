//! Pattern matching for invoice fields
//!
//! This module contains regex patterns and matching logic for extracting
//! structured data from invoice text.

use super::error::{ExtractionError, Result};
use super::types::Language;
use regex::Regex;

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
        // Invoice number patterns
        // Matches: "Factura N° 2025-001", "Factura Nº: 12345", "Núm. Factura: INV-001"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::InvoiceNumber,
            r"(?:Factura|FACTURA|Fac\.?)\s+(?:N[úuº°]?\.?|Número)\s*:?\s*([A-Z0-9][A-Z0-9\-/]*)",
            0.9,
            Some(Language::Spanish),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "factura".to_string(),
                "número".to_string(),
                "nº".to_string(),
            ]));
        }

        // Invoice date patterns
        // Matches: "Fecha: 15/03/2025", "Fecha de emisión: 15-03-2025"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::InvoiceDate,
            r"(?:Fecha(?:\s+de\s+emisión)?|FECHA):?\s*(\d{1,2}[-/]\d{1,2}[-/]\d{2,4})",
            0.85,
            Some(Language::Spanish),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "fecha".to_string(),
                "emisión".to_string(),
            ]));
        }

        // Due date patterns
        // Matches: "Vencimiento: 15/04/2025"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::DueDate,
            r"(?:Vencimiento|Fecha\s+de\s+vencimiento|VENCIMIENTO):?\s*(\d{1,2}[-/]\d{1,2}[-/]\d{2,4})",
            0.85,
            Some(Language::Spanish),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "vencimiento".to_string(),
            ]));
        }

        // Total amount patterns
        // Matches: "Total: 1.234,56 €", "TOTAL: € 1.234,56", "Importe Total: 1234.56"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::TotalAmount,
            r"(?:Total|TOTAL|Importe\s+Total):?\s*€?\s*([0-9]{1,3}(?:[.,][0-9]{3})*[.,][0-9]{2})\s*€?",
            0.9,
            Some(Language::Spanish),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "total".to_string(),
                "importe".to_string(),
            ]));
        }

        // Tax amount (IVA) patterns
        // Matches: "IVA (21%): 123,45 €"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::TaxAmount,
            r"(?:IVA|I\.V\.A\.|Impuesto).*?:?\s*€?\s*([0-9]{1,3}(?:[.,][0-9]{3})*[.,][0-9]{2})\s*€?",
            0.85,
            Some(Language::Spanish),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "iva".to_string(),
                "impuesto".to_string(),
            ]));
        }

        // Net amount patterns
        // Matches: "Base Imponible: 500,00 €"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::NetAmount,
            r"(?:Base\s+Imponible|Base):?\s*€?\s*([0-9]{1,3}(?:[.,][0-9]{3})*[.,][0-9]{2})\s*€?",
            0.85,
            Some(Language::Spanish),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "base".to_string(),
                "imponible".to_string(),
            ]));
        }

        // VAT number patterns (Spanish CIF/NIF)
        // Matches: "CIF: A12345678", "NIF: 12345678Z"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::VatNumber,
            r"(?:CIF|NIF|N\.I\.F\.|C\.I\.F\.):?\s*([A-Z]?[0-9]{8}[A-Z0-9])",
            0.9,
            Some(Language::Spanish),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "cif".to_string(),
                "nif".to_string(),
            ]));
        }

        // Currency pattern
        // Matches: "€", "EUR"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::Currency,
            r"(€|EUR)",
            0.7,
            Some(Language::Spanish),
        ) {
            self.add_pattern(pattern);
        }
    }

    /// Load English invoice patterns
    fn load_english_patterns(&mut self) {
        // Invoice number patterns
        // Matches: "Invoice #12345", "Invoice No: INV-001", "Invoice Number: 2025-001"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::InvoiceNumber,
            r"(?:Invoice|INVOICE)\s+(?:#|No\.?|Number)\s*:?\s*([A-Z0-9][A-Z0-9\-/]*)",
            0.9,
            Some(Language::English),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "invoice".to_string(),
                "number".to_string(),
                "no".to_string(),
            ]));
        }

        // Invoice date patterns
        // Matches: "Date: 10/20/2025", "Invoice Date: 20-10-2025"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::InvoiceDate,
            r"(?:(?:Invoice\s+)?Date|DATE):?\s*(\d{1,2}[-/]\d{1,2}[-/]\d{2,4})",
            0.85,
            Some(Language::English),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "date".to_string(),
                "invoice".to_string(),
            ]));
        }

        // Due date patterns
        // Matches: "Due Date: 11/20/2025", "Payment Due: 20-11-2025"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::DueDate,
            r"(?:Due\s+Date|Payment\s+Due|DUE\s+DATE):?\s*(\d{1,2}[-/]\d{1,2}[-/]\d{2,4})",
            0.85,
            Some(Language::English),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "due".to_string(),
                "payment".to_string(),
            ]));
        }

        // Total amount patterns
        // Matches: "Total: $1,234.56", "Total: £1,234.56", "Amount Due: 1234.56"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::TotalAmount,
            r"(?:Total|TOTAL|Amount\s+Due):?\s*[$£]?\s*([0-9]{1,3}(?:,[0-9]{3})*\.[0-9]{2})\s*[$£]?",
            0.9,
            Some(Language::English),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "total".to_string(),
                "amount".to_string(),
                "due".to_string(),
            ]));
        }

        // Tax amount (VAT) patterns
        // Matches: "VAT: 123.45", "Tax: 123.45", "VAT (20%): 123.45"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::TaxAmount,
            r"(?:VAT|Tax|V\.A\.T\.).*?:?\s*[$£]?\s*([0-9]{1,3}(?:,[0-9]{3})*\.[0-9]{2})\s*[$£]?",
            0.85,
            Some(Language::English),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "vat".to_string(),
                "tax".to_string(),
            ]));
        }

        // Net amount patterns
        // Matches: "Subtotal: 500.00", "Net Amount: 500.00"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::NetAmount,
            r"(?:Subtotal|Net\s+Amount|SUBTOTAL):?\s*[$£]?\s*([0-9]{1,3}(?:,[0-9]{3})*\.[0-9]{2})\s*[$£]?",
            0.85,
            Some(Language::English),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "subtotal".to_string(),
                "net".to_string(),
            ]));
        }

        // VAT number patterns
        // Matches: "VAT No: GB123456789", "VAT Reg: 123456789"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::VatNumber,
            r"(?:VAT\s+(?:No\.?|Reg\.?|Registration)|V\.A\.T\.\s+No\.?):?\s*([A-Z]{2}[0-9]{9,12}|[0-9]{9,12})",
            0.9,
            Some(Language::English),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "vat".to_string(),
                "registration".to_string(),
            ]));
        }

        // Currency pattern
        // Matches: "$", "£", "USD", "GBP"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::Currency,
            r"([$£]|USD|GBP)",
            0.7,
            Some(Language::English),
        ) {
            self.add_pattern(pattern);
        }
    }

    /// Load German invoice patterns
    fn load_german_patterns(&mut self) {
        // Invoice number patterns
        // Matches: "Rechnungsnummer: 2025-001", "Rechnung Nr. 12345", "Re.-Nr.: INV-001"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::InvoiceNumber,
            r"(?:Rechnungsnummer|Rechnung\s+Nr\.?|Re\.-Nr\.?):?\s*([A-Z0-9][A-Z0-9\-/]*)",
            0.9,
            Some(Language::German),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "rechnung".to_string(),
                "rechnungsnummer".to_string(),
                "nummer".to_string(),
            ]));
        }

        // Invoice date patterns
        // Matches: "Datum: 20.10.2025", "Rechnungsdatum: 20-10-2025"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::InvoiceDate,
            r"(?:(?:Rechnungs)?datum|DATUM):?\s*(\d{1,2}[.\-]\d{1,2}[.\-]\d{2,4})",
            0.85,
            Some(Language::German),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "datum".to_string(),
                "rechnungsdatum".to_string(),
            ]));
        }

        // Due date patterns
        // Matches: "Fälligkeitsdatum: 20.11.2025", "Zahlbar bis: 20.11.2025"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::DueDate,
            r"(?:Fälligkeitsdatum|Zahlbar\s+bis|FÄLLIGKEITSDATUM):?\s*(\d{1,2}[.\-]\d{1,2}[.\-]\d{2,4})",
            0.85,
            Some(Language::German),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "fälligkeitsdatum".to_string(),
                "zahlbar".to_string(),
            ]));
        }

        // Total amount patterns
        // Matches: "Gesamtbetrag: 1.234,56 €", "Betrag: 1.234,56€", "Summe: 1234,56"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::TotalAmount,
            r"(?:Gesamtbetrag|Betrag|Summe|GESAMTBETRAG):?\s*€?\s*([0-9]{1,3}(?:\.[0-9]{3})*,[0-9]{2})\s*€?",
            0.9,
            Some(Language::German),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "gesamtbetrag".to_string(),
                "betrag".to_string(),
                "summe".to_string(),
            ]));
        }

        // Tax amount (MwSt/USt) patterns
        // Matches: "MwSt: 123,45 €", "Umsatzsteuer (19%): 123,45", "USt: 123,45"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::TaxAmount,
            r"(?:MwSt\.?|Umsatzsteuer|USt\.?).*?:?\s*€?\s*([0-9]{1,3}(?:\.[0-9]{3})*,[0-9]{2})\s*€?",
            0.85,
            Some(Language::German),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "mwst".to_string(),
                "umsatzsteuer".to_string(),
                "ust".to_string(),
            ]));
        }

        // Net amount patterns
        // Matches: "Nettobetrag: 500,00 €", "Zwischensumme: 500,00"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::NetAmount,
            r"(?:Nettobetrag|Zwischensumme|NETTOBETRAG):?\s*€?\s*([0-9]{1,3}(?:\.[0-9]{3})*,[0-9]{2})\s*€?",
            0.85,
            Some(Language::German),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "nettobetrag".to_string(),
                "zwischensumme".to_string(),
            ]));
        }

        // VAT number patterns (German USt-IdNr or Steuernummer)
        // Matches: "USt-IdNr: DE123456789", "Steuernummer: 123/456/78901"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::VatNumber,
            r"(?:USt-IdNr\.?|Steuernummer):?\s*(DE[0-9]{9}|[0-9]{2,3}/[0-9]{3}/[0-9]{4,5})",
            0.9,
            Some(Language::German),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "ust-idnr".to_string(),
                "steuernummer".to_string(),
            ]));
        }

        // Currency pattern
        // Matches: "€", "EUR"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::Currency,
            r"(€|EUR)",
            0.7,
            Some(Language::German),
        ) {
            self.add_pattern(pattern);
        }
    }

    /// Load Italian invoice patterns
    fn load_italian_patterns(&mut self) {
        // Invoice number patterns
        // Matches: "Fattura N. 2025-001", "Numero Fattura: 12345", "N. Fatt.: INV-001"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::InvoiceNumber,
            r"(?:Fattura\s+N\.?|Numero\s+Fattura|N\.\s+Fatt\.?|FATTURA\s+N\.?):?\s*([A-Z0-9][A-Z0-9\-/]*)",
            0.9,
            Some(Language::Italian),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "fattura".to_string(),
                "numero".to_string(),
            ]));
        }

        // Invoice date patterns
        // Matches: "Data: 20/10/2025", "Data Fattura: 20-10-2025"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::InvoiceDate,
            r"(?:(?:Data\s+)?Fattura|Data|DATA):?\s*(\d{1,2}[-/]\d{1,2}[-/]\d{2,4})",
            0.85,
            Some(Language::Italian),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "data".to_string(),
                "fattura".to_string(),
            ]));
        }

        // Due date patterns
        // Matches: "Scadenza: 20/11/2025", "Data Scadenza: 20-11-2025"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::DueDate,
            r"(?:(?:Data\s+)?Scadenza|SCADENZA):?\s*(\d{1,2}[-/]\d{1,2}[-/]\d{2,4})",
            0.85,
            Some(Language::Italian),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "scadenza".to_string(),
            ]));
        }

        // Total amount patterns
        // Matches: "Totale: 1.234,56 €", "Importo Totale: €1.234,56", "Totale Fattura: 1234,56"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::TotalAmount,
            r"(?:Totale(?:\s+Fattura)?|Importo\s+Totale|TOTALE):?\s*€?\s*([0-9]{1,3}(?:\.[0-9]{3})*,[0-9]{2})\s*€?",
            0.9,
            Some(Language::Italian),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "totale".to_string(),
                "importo".to_string(),
            ]));
        }

        // Tax amount (IVA) patterns
        // Matches: "IVA: 123,45 €", "IVA (22%): 123,45", "Imposta: 123,45"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::TaxAmount,
            r"(?:IVA|I\.V\.A\.|Imposta).*?:?\s*€?\s*([0-9]{1,3}(?:\.[0-9]{3})*,[0-9]{2})\s*€?",
            0.85,
            Some(Language::Italian),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "iva".to_string(),
                "imposta".to_string(),
            ]));
        }

        // Net amount patterns
        // Matches: "Imponibile: 500,00 €", "Subtotale: 500,00"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::NetAmount,
            r"(?:Imponibile|Subtotale|IMPONIBILE):?\s*€?\s*([0-9]{1,3}(?:\.[0-9]{3})*,[0-9]{2})\s*€?",
            0.85,
            Some(Language::Italian),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "imponibile".to_string(),
                "subtotale".to_string(),
            ]));
        }

        // VAT number patterns (Italian P.IVA)
        // Matches: "P.IVA: IT12345678901", "Partita IVA: 12345678901"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::VatNumber,
            r"(?:P\.IVA|P\.\s*IVA|Partita\s+IVA):?\s*(IT[0-9]{11}|[0-9]{11})",
            0.9,
            Some(Language::Italian),
        ) {
            self.add_pattern(pattern.with_hints(vec![
                "p.iva".to_string(),
                "partita".to_string(),
            ]));
        }

        // Currency pattern
        // Matches: "€", "EUR"
        if let Ok(pattern) = FieldPattern::new(
            InvoiceFieldType::Currency,
            r"(€|EUR)",
            0.7,
            Some(Language::Italian),
        ) {
            self.add_pattern(pattern);
        }
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
        let pattern = FieldPattern::new(InvoiceFieldType::InvoiceNumber, r"INV-(\d+)", 0.9, None)
            .expect("Hardcoded regex pattern should be valid");

        assert_eq!(pattern.matches("INV-12345"), Some("12345".to_string()));
        assert_eq!(pattern.matches("Invoice INV-999"), Some("999".to_string()));
        assert_eq!(pattern.matches("No match here"), None);
    }
}
