//! Value validators for invoice field extraction
//!
//! This module provides validation functions that calculate confidence adjustments
//! based on the format and content of extracted field values.

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// ISO 8601 date format: YYYY-MM-DD
    static ref ISO_DATE_PATTERN: Regex = Regex::new(r"^(\d{4})-(\d{2})-(\d{2})$")
        .expect("ISO date pattern is hardcoded and valid");

    /// European/US date format: DD/MM/YYYY or MM/DD/YYYY
    static ref SLASH_DATE_PATTERN: Regex = Regex::new(r"^(\d{1,2})[/-](\d{1,2})[/-](\d{2,4})$")
        .expect("Slash date pattern is hardcoded and valid");
}

/// Validate a date string and return confidence adjustment
///
/// Supported formats:
/// - ISO 8601: "2025-01-20"
/// - European: "20/01/2025", "20-01-2025"
/// - US: "01/20/2025", "01-20-2025"
///
/// # Returns
/// - `+0.20`: Valid date format with reasonable values
/// - `+0.10`: Valid format but edge case (e.g., day=31 in February)
/// - `-0.50`: Invalid format or impossible date (e.g., month=13)
///
/// # Examples
/// ```
/// use oxidize_pdf::text::invoice::validators::validate_date;
///
/// assert_eq!(validate_date("20/01/2025"), 0.20);  // Valid
/// assert_eq!(validate_date("99/99/9999"), -0.50); // Invalid
/// assert_eq!(validate_date("not-a-date"), -0.50); // Invalid
/// ```
pub fn validate_date(value: &str) -> f64 {
    if let Some(caps) = ISO_DATE_PATTERN.captures(value) {
        let year: i32 = caps[1].parse().unwrap_or(0);
        let month: i32 = caps[2].parse().unwrap_or(0);
        let day: i32 = caps[3].parse().unwrap_or(0);

        return validate_date_components(year, month, day);
    }

    if let Some(caps) = SLASH_DATE_PATTERN.captures(value) {
        let part1: i32 = caps[1].parse().unwrap_or(0);
        let part2: i32 = caps[2].parse().unwrap_or(0);
        let year: i32 = caps[3].parse().unwrap_or(0);

        // Normalize 2-digit years (25 -> 2025)
        let year = if year < 100 { 2000 + year } else { year };

        // Try both DD/MM/YYYY and MM/DD/YYYY interpretations
        // Prefer interpretation with valid day/month ranges
        if part1 >= 1 && part1 <= 31 && part2 >= 1 && part2 <= 12 {
            // Could be DD/MM/YYYY (European)
            return validate_date_components(year, part2, part1);
        } else if part2 >= 1 && part2 <= 31 && part1 >= 1 && part1 <= 12 {
            // Could be MM/DD/YYYY (US)
            return validate_date_components(year, part1, part2);
        } else {
            // Invalid ranges
            return -0.50;
        }
    }

    // No valid date format matched
    -0.50
}

/// Validate date components (year, month, day)
fn validate_date_components(year: i32, month: i32, day: i32) -> f64 {
    // Check reasonable year range (1900-2100)
    if year < 1900 || year > 2100 {
        return -0.50;
    }

    // Check month range
    if month < 1 || month > 12 {
        return -0.50;
    }

    // Check day range
    if day < 1 || day > 31 {
        return -0.50;
    }

    // Check day validity for specific months
    let max_days = match month {
        2 => {
            // February: check leap year
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30, // April, June, September, November
        _ => 31,              // Other months
    };

    if day > max_days {
        // Edge case: valid format but impossible date (e.g., Feb 30)
        return 0.10;
    }

    // Valid date
    0.20
}

/// Check if a year is a leap year
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Validate an amount (monetary value) and return confidence adjustment
///
/// # Returns
/// - `+0.20`: Valid positive amount with 2 decimal places
/// - `+0.10`: Valid amount with non-standard decimals (0, 1, or 3+ places)
/// - `-0.30`: Negative amount (suspicious in most invoice contexts)
/// - `-0.20`: Zero amount (suspicious, may indicate missing data)
///
/// # Examples
/// ```
/// use oxidize_pdf::text::invoice::validators::validate_amount;
///
/// assert_eq!(validate_amount("1,234.56"), 0.20);   // Valid
/// assert_eq!(validate_amount("-123.45"), -0.30);   // Negative
/// assert_eq!(validate_amount("0.00"), -0.20);      // Zero
/// ```
pub fn validate_amount(value: &str) -> f64 {
    // Remove thousand separators and parse
    let cleaned = value.replace(",", "").replace(".", "");

    // Check if numeric
    if !cleaned
        .chars()
        .all(|c| c.is_ascii_digit() || c == '-' || c == '.')
    {
        return -0.50;
    }

    // Parse as float
    let amount_str = value.replace(",", "");
    let amount: f64 = match amount_str.parse() {
        Ok(a) => a,
        Err(_) => return -0.50,
    };

    // Check for negative (suspicious in invoices)
    if amount < 0.0 {
        return -0.30;
    }

    // Check for zero (may indicate missing data)
    if amount == 0.0 {
        return -0.20;
    }

    // Check decimal places
    if let Some(dot_pos) = amount_str.find('.') {
        let decimals = amount_str.len() - dot_pos - 1;

        if decimals == 2 {
            // Standard format (1234.56)
            return 0.20;
        } else {
            // Non-standard decimals (acceptable but less common)
            return 0.10;
        }
    }

    // No decimals (e.g., "1234") - acceptable but less precise
    0.10
}

/// Validate an invoice number and return confidence adjustment
///
/// # Returns
/// - `+0.10`: Alphanumeric with separators (e.g., "INV-2025-001")
/// - `+0.05`: Pure numeric (e.g., "12345")
/// - `-0.30`: Empty or too short (< 2 chars)
///
/// # Examples
/// ```
/// use oxidize_pdf::text::invoice::validators::validate_invoice_number;
///
/// assert_eq!(validate_invoice_number("INV-2025-001"), 0.10);  // Strong
/// assert_eq!(validate_invoice_number("12345"), 0.05);         // Weak
/// assert_eq!(validate_invoice_number("1"), -0.30);            // Too short
/// ```
pub fn validate_invoice_number(value: &str) -> f64 {
    // Empty or too short
    if value.len() < 2 {
        return -0.30;
    }

    // Check for alphanumeric with separators (strong format)
    let has_letters = value.chars().any(|c| c.is_alphabetic());
    let has_separators = value.contains('-') || value.contains('/') || value.contains('_');

    if has_letters && has_separators {
        // Strong format: "INV-2025-001", "FAC/2025/123"
        return 0.10;
    }

    if has_letters {
        // Medium format: "INV2025001"
        return 0.08;
    }

    // Pure numeric (weaker but acceptable)
    let all_numeric = value.chars().all(|c| c.is_ascii_digit());
    if all_numeric {
        return 0.05;
    }

    // Contains special chars but no standard separator pattern
    0.0
}

/// Validate a VAT number format (basic validation)
///
/// # Returns
/// - `+0.15`: Valid format for known country patterns
/// - `+0.05`: Numeric format (less specific)
/// - `-0.20`: Invalid format
///
/// # Examples
/// ```
/// use oxidize_pdf::text::invoice::validators::validate_vat_number;
///
/// assert_eq!(validate_vat_number("GB272052232"), 0.15);    // UK VAT
/// assert_eq!(validate_vat_number("A12345678"), 0.15);      // Spanish CIF
/// assert_eq!(validate_vat_number("123456789"), 0.05);      // Generic
/// ```
pub fn validate_vat_number(value: &str) -> f64 {
    if value.is_empty() {
        return -0.20;
    }

    // UK VAT: GB followed by 9 or 12 digits
    if value.starts_with("GB") && value.len() >= 11 {
        let numeric_part = &value[2..];
        if numeric_part.chars().all(|c| c.is_ascii_digit()) {
            return 0.15;
        }
    }

    // Spanish CIF/NIF: Letter + 8 digits + optional letter/digit
    if value.len() >= 9 {
        if let Some(first_char) = value.chars().next() {
            if first_char.is_alphabetic() {
                let middle = &value[1..9];
                if middle.chars().all(|c| c.is_ascii_digit()) {
                    return 0.15;
                }
            }
        }
    }

    // Italian P.IVA: IT + 11 digits
    if value.starts_with("IT") && value.len() == 13 {
        let numeric_part = &value[2..];
        if numeric_part.chars().all(|c| c.is_ascii_digit()) {
            return 0.15;
        }
    }

    // German USt-IdNr: DE + 9 digits
    if value.starts_with("DE") && value.len() == 11 {
        let numeric_part = &value[2..];
        if numeric_part.chars().all(|c| c.is_ascii_digit()) {
            return 0.15;
        }
    }

    // Generic numeric VAT (less specific)
    if value.chars().all(|c| c.is_ascii_digit()) && value.len() >= 8 {
        return 0.05;
    }

    // Unknown format
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_date_iso8601() {
        assert_eq!(validate_date("2025-01-20"), 0.20);
        assert_eq!(validate_date("2025-12-31"), 0.20);
        assert_eq!(validate_date("2025-02-29"), 0.10); // Not a leap year
        assert_eq!(validate_date("2024-02-29"), 0.20); // Leap year
    }

    #[test]
    fn test_validate_date_invalid() {
        assert_eq!(validate_date("2025-13-01"), -0.50); // Month > 12
        assert_eq!(validate_date("2025-00-15"), -0.50); // Month = 0
        assert_eq!(validate_date("2025-01-32"), -0.50); // Day > 31
        assert_eq!(validate_date("99/99/9999"), -0.50); // Invalid
        assert_eq!(validate_date("not-a-date"), -0.50); // Not a date
    }

    #[test]
    fn test_validate_date_slash_format() {
        assert_eq!(validate_date("20/01/2025"), 0.20); // European
        assert_eq!(validate_date("01/20/2025"), 0.20); // US
        assert_eq!(validate_date("20/01/25"), 0.20); // 2-digit year
    }

    #[test]
    fn test_validate_amount_valid() {
        assert_eq!(validate_amount("1234.56"), 0.20);
        assert_eq!(validate_amount("1,234.56"), 0.20);
        assert_eq!(validate_amount("0.01"), 0.20);
    }

    #[test]
    fn test_validate_amount_invalid() {
        assert_eq!(validate_amount("-123.45"), -0.30); // Negative
        assert_eq!(validate_amount("0.00"), -0.20); // Zero
    }

    #[test]
    fn test_validate_amount_non_standard() {
        assert_eq!(validate_amount("1234"), 0.10); // No decimals
        assert_eq!(validate_amount("1234.5"), 0.10); // 1 decimal
        assert_eq!(validate_amount("1234.567"), 0.10); // 3 decimals
    }

    #[test]
    fn test_validate_invoice_number() {
        assert_eq!(validate_invoice_number("INV-2025-001"), 0.10);
        assert_eq!(validate_invoice_number("FAC/2025/123"), 0.10);
        assert_eq!(validate_invoice_number("12345"), 0.05);
        assert_eq!(validate_invoice_number("1"), -0.30);
        assert_eq!(validate_invoice_number(""), -0.30);
    }

    #[test]
    fn test_validate_vat_number() {
        assert_eq!(validate_vat_number("GB272052232"), 0.15); // UK
        assert_eq!(validate_vat_number("A12345678Z"), 0.15); // Spanish
        assert_eq!(validate_vat_number("IT12345678901"), 0.15); // Italian
        assert_eq!(validate_vat_number("DE123456789"), 0.15); // German
        assert_eq!(validate_vat_number("123456789"), 0.05); // Generic
        assert_eq!(validate_vat_number(""), -0.20); // Empty
    }

    #[test]
    fn test_leap_year() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2025));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
    }
}
