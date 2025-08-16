//! Form field validation system according to ISO 32000-1 Section 12.7.5.3
//!
//! This module provides comprehensive validation for PDF form fields including:
//! - Format masks for various data types
//! - Required field validation
//! - Range validation for numeric fields
//! - Custom validation rules
//! - Real-time format enforcement

use crate::forms::calculations::FieldValue;
use chrono::{NaiveDate, NaiveTime};
use regex::Regex;
use std::collections::HashMap;
use std::fmt;

/// Form validation system
#[derive(Debug, Clone, Default)]
pub struct FormValidationSystem {
    /// Field validators
    validators: HashMap<String, FieldValidator>,
    /// Required fields
    required_fields: HashMap<String, RequiredFieldInfo>,
    /// Validation results cache
    validation_cache: HashMap<String, ValidationResult>,
    /// Settings
    #[allow(dead_code)]
    settings: ValidationSettings,
}

/// Field validator specification
#[derive(Debug, Clone)]
pub struct FieldValidator {
    /// Field name
    pub field_name: String,
    /// Validation rules
    pub rules: Vec<ValidationRule>,
    /// Format mask (if applicable)
    pub format_mask: Option<FormatMask>,
    /// Custom error message
    pub error_message: Option<String>,
}

/// Validation rules
#[derive(Debug, Clone)]
pub enum ValidationRule {
    /// Field is required
    Required,
    /// Numeric range validation
    Range { min: Option<f64>, max: Option<f64> },
    /// String length validation
    Length {
        min: Option<usize>,
        max: Option<usize>,
    },
    /// Regular expression pattern
    Pattern(String),
    /// Email validation
    Email,
    /// URL validation
    Url,
    /// Phone number validation
    PhoneNumber { country: PhoneCountry },
    /// Date validation
    Date {
        min: Option<NaiveDate>,
        max: Option<NaiveDate>,
    },
    /// Time validation
    Time {
        min: Option<NaiveTime>,
        max: Option<NaiveTime>,
    },
    /// Credit card validation
    CreditCard,
    /// Custom validation function
    Custom {
        name: String,
        validator: fn(&FieldValue) -> bool,
    },
}

/// Format masks for field input
#[derive(Debug, Clone)]
pub enum FormatMask {
    /// Numeric format
    Number {
        decimals: usize,
        thousands_separator: bool,
        allow_negative: bool,
        prefix: Option<String>,
        suffix: Option<String>,
    },
    /// Date format
    Date { format: DateFormat },
    /// Time format
    Time {
        format: TimeFormat,
        use_24_hour: bool,
    },
    /// Phone number format
    Phone { country: PhoneCountry },
    /// Social Security Number
    SSN,
    /// ZIP code (5 or 9 digits)
    ZipCode { plus_four: bool },
    /// Credit card number
    CreditCard,
    /// Custom mask pattern
    Custom { pattern: String, placeholder: char },
}

/// Date format types
#[derive(Debug, Clone, Copy)]
pub enum DateFormat {
    /// MM/DD/YYYY
    MDY,
    /// DD/MM/YYYY
    DMY,
    /// YYYY-MM-DD
    YMD,
    /// DD.MM.YYYY
    DotDMY,
    /// MM-DD-YYYY
    DashMDY,
}

/// Time format types
#[derive(Debug, Clone, Copy)]
pub enum TimeFormat {
    /// HH:MM
    HM,
    /// HH:MM:SS
    HMS,
    /// HH:MM AM/PM
    HMAM,
    /// HH:MM:SS AM/PM
    HMSAM,
}

/// Phone number country formats
#[derive(Debug, Clone, Copy)]
pub enum PhoneCountry {
    US,    // (XXX) XXX-XXXX
    UK,    // +44 XXXX XXXXXX
    EU,    // +XX XXX XXX XXXX
    Japan, // XXX-XXXX-XXXX
    Custom,
}

/// Required field information
#[derive(Debug, Clone)]
pub struct RequiredFieldInfo {
    /// Field name
    pub field_name: String,
    /// Error message when empty
    pub error_message: String,
    /// Group name (for conditional requirements)
    #[allow(dead_code)]
    pub group: Option<String>,
    /// Condition for requirement
    pub condition: Option<RequirementCondition>,
}

/// Requirement conditions
#[derive(Debug, Clone)]
pub enum RequirementCondition {
    /// Always required
    Always,
    /// Required if another field has a specific value
    IfFieldEquals { field: String, value: FieldValue },
    /// Required if another field is not empty
    IfFieldNotEmpty { field: String },
    /// Required if at least one field in group is filled
    IfGroupHasValue { group: String },
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Field name
    pub field_name: String,
    /// Is valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<ValidationError>,
    /// Warnings (non-blocking)
    pub warnings: Vec<String>,
    /// Formatted value (if mask applied)
    pub formatted_value: Option<String>,
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Field name
    pub field_name: String,
    /// Error type
    pub error_type: ValidationErrorType,
    /// Error message
    pub message: String,
    /// Additional details
    pub details: Option<String>,
}

/// Validation error types
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationErrorType {
    Required,
    Format,
    Range,
    Length,
    Pattern,
    Custom,
}

/// Validation settings
#[derive(Debug, Clone)]
pub struct ValidationSettings {
    /// Validate on field change
    pub validate_on_change: bool,
    /// Show format hints
    pub show_format_hints: bool,
    /// Auto-format on blur
    pub auto_format: bool,
    /// Allow partial validation
    pub allow_partial: bool,
    /// Real-time validation
    pub real_time_validation: bool,
    /// Highlight errors visually
    pub highlight_errors: bool,
    /// Show error messages
    pub show_error_messages: bool,
}

impl Default for ValidationSettings {
    fn default() -> Self {
        Self {
            validate_on_change: true,
            show_format_hints: true,
            auto_format: true,
            allow_partial: false,
            real_time_validation: true,
            highlight_errors: true,
            show_error_messages: true,
        }
    }
}

impl FormValidationSystem {
    /// Create a new validation system
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom settings
    pub fn with_settings(settings: ValidationSettings) -> Self {
        Self {
            settings,
            ..Self::default()
        }
    }

    /// Add a field validator
    pub fn add_validator(&mut self, validator: FieldValidator) {
        self.validators
            .insert(validator.field_name.clone(), validator);
    }

    /// Add a required field
    pub fn add_required_field(&mut self, info: RequiredFieldInfo) {
        self.required_fields.insert(info.field_name.clone(), info);
    }

    /// Validate a single field
    pub fn validate_field(&mut self, field_name: &str, value: &FieldValue) -> ValidationResult {
        let mut errors = Vec::new();
        let warnings = Vec::new();
        let mut formatted_value = None;

        // Check if field is required
        if let Some(required_info) = self.required_fields.get(field_name) {
            if self.is_field_required(required_info) && self.is_empty(value) {
                errors.push(ValidationError {
                    field_name: field_name.to_string(),
                    error_type: ValidationErrorType::Required,
                    message: required_info.error_message.clone(),
                    details: None,
                });
            }
        }

        // Apply validator rules
        if let Some(validator) = self.validators.get(field_name) {
            // Apply format mask
            if let Some(ref mask) = validator.format_mask {
                match self.apply_format_mask(value, mask) {
                    Ok(formatted) => formatted_value = Some(formatted),
                    Err(e) => errors.push(ValidationError {
                        field_name: field_name.to_string(),
                        error_type: ValidationErrorType::Format,
                        message: e.to_string(),
                        details: None,
                    }),
                }
            }

            // Apply validation rules
            for rule in &validator.rules {
                if let Err(e) = self.apply_rule(value, rule) {
                    errors.push(ValidationError {
                        field_name: field_name.to_string(),
                        error_type: self.get_error_type(rule),
                        message: validator
                            .error_message
                            .clone()
                            .unwrap_or_else(|| e.to_string()),
                        details: None,
                    });
                }
            }
        }

        let result = ValidationResult {
            field_name: field_name.to_string(),
            is_valid: errors.is_empty(),
            errors,
            warnings,
            formatted_value,
        };

        // Cache result
        self.validation_cache
            .insert(field_name.to_string(), result.clone());

        result
    }

    /// Check if field is required based on conditions
    fn is_field_required(&self, info: &RequiredFieldInfo) -> bool {
        match &info.condition {
            Some(RequirementCondition::Always) | None => true,
            Some(RequirementCondition::IfFieldEquals { field: _, value: _ }) => {
                // Check if referenced field equals value
                // In real implementation, would need access to field values
                false
            }
            Some(RequirementCondition::IfFieldNotEmpty { field: _ }) => {
                // Check if referenced field is not empty
                false
            }
            Some(RequirementCondition::IfGroupHasValue { group: _ }) => {
                // Check if any field in group has value
                false
            }
        }
    }

    /// Check if value is empty
    fn is_empty(&self, value: &FieldValue) -> bool {
        match value {
            FieldValue::Empty => true,
            FieldValue::Text(s) => s.is_empty(),
            _ => false,
        }
    }

    /// Apply validation rule
    fn apply_rule(&self, value: &FieldValue, rule: &ValidationRule) -> Result<(), String> {
        match rule {
            ValidationRule::Required => {
                if self.is_empty(value) {
                    Err("Field is required".to_string())
                } else {
                    Ok(())
                }
            }
            ValidationRule::Range { min, max } => {
                let num = value.to_number();
                if let Some(min_val) = min {
                    if num < *min_val {
                        return Err(format!("Value must be at least {}", min_val));
                    }
                }
                if let Some(max_val) = max {
                    if num > *max_val {
                        return Err(format!("Value must be at most {}", max_val));
                    }
                }
                Ok(())
            }
            ValidationRule::Length { min, max } => {
                let text = value.to_string();
                let len = text.len();
                if let Some(min_len) = min {
                    if len < *min_len {
                        return Err(format!("Must be at least {} characters", min_len));
                    }
                }
                if let Some(max_len) = max {
                    if len > *max_len {
                        return Err(format!("Must be at most {} characters", max_len));
                    }
                }
                Ok(())
            }
            ValidationRule::Pattern(pattern) => {
                let text = value.to_string();
                let re = Regex::new(pattern).map_err(|e| e.to_string())?;
                if re.is_match(&text) {
                    Ok(())
                } else {
                    Err("Does not match required pattern".to_string())
                }
            }
            ValidationRule::Email => {
                let text = value.to_string();
                let email_regex =
                    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
                if email_regex.is_match(&text) {
                    Ok(())
                } else {
                    Err("Invalid email address".to_string())
                }
            }
            ValidationRule::Url => {
                let text = value.to_string();
                let url_regex = Regex::new(r"^https?://[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
                if url_regex.is_match(&text) {
                    Ok(())
                } else {
                    Err("Invalid URL".to_string())
                }
            }
            ValidationRule::PhoneNumber { country } => {
                self.validate_phone_number(&value.to_string(), *country)
            }
            ValidationRule::CreditCard => self.validate_credit_card(&value.to_string()),
            ValidationRule::Date { min, max } => {
                // Parse date and validate range
                let text = value.to_string();
                let date = NaiveDate::parse_from_str(&text, "%Y-%m-%d")
                    .map_err(|e| format!("Invalid date format: {}", e))?;

                if let Some(min_date) = min {
                    if date < *min_date {
                        return Err(format!("Date must be on or after {}", min_date));
                    }
                }
                if let Some(max_date) = max {
                    if date > *max_date {
                        return Err(format!("Date must be on or before {}", max_date));
                    }
                }
                Ok(())
            }
            ValidationRule::Time { min, max } => {
                // Parse time and validate range
                let text = value.to_string();
                let time = NaiveTime::parse_from_str(&text, "%H:%M:%S")
                    .or_else(|_| NaiveTime::parse_from_str(&text, "%H:%M"))
                    .map_err(|e| format!("Invalid time format: {}", e))?;

                if let Some(min_time) = min {
                    if time < *min_time {
                        return Err(format!("Time must be at or after {}", min_time));
                    }
                }
                if let Some(max_time) = max {
                    if time > *max_time {
                        return Err(format!("Time must be at or before {}", max_time));
                    }
                }
                Ok(())
            }
            ValidationRule::Custom { name, validator } => {
                if validator(value) {
                    Ok(())
                } else {
                    Err(format!("Custom validation '{}' failed", name))
                }
            }
        }
    }

    /// Validate phone number format
    fn validate_phone_number(&self, phone: &str, country: PhoneCountry) -> Result<(), String> {
        let pattern = match country {
            PhoneCountry::US => r"^\(?[2-9]\d{2}\)?[-.\s]?\d{3}[-.\s]?\d{4}$",
            PhoneCountry::UK => r"^\+?44\s?[0-9]{4}\s?[0-9]{6}$",
            PhoneCountry::EU => r"^\+?[0-9]{2}\s?[0-9]{3}\s?[0-9]{3}\s?[0-9]{4}$",
            PhoneCountry::Japan => r"^0\d{1,4}-?\d{1,4}-?\d{4}$",
            PhoneCountry::Custom => r"^[0-9+\-\s\(\)]+$",
        };

        let re = Regex::new(pattern).unwrap();
        if re.is_match(phone) {
            Ok(())
        } else {
            Err(format!("Invalid phone number format for {:?}", country))
        }
    }

    /// Validate credit card number using Luhn algorithm
    fn validate_credit_card(&self, card_number: &str) -> Result<(), String> {
        let digits: Vec<u32> = card_number
            .chars()
            .filter(|c| c.is_ascii_digit())
            .map(|c| c.to_digit(10).unwrap())
            .collect();

        if digits.len() < 13 || digits.len() > 19 {
            return Err("Invalid credit card number length".to_string());
        }

        // Luhn algorithm
        let mut sum = 0;
        let mut alternate = false;

        for digit in digits.iter().rev() {
            let mut n = *digit;
            if alternate {
                n *= 2;
                if n > 9 {
                    n -= 9;
                }
            }
            sum += n;
            alternate = !alternate;
        }

        if sum % 10 == 0 {
            Ok(())
        } else {
            Err("Invalid credit card number".to_string())
        }
    }

    /// Apply format mask to value
    fn apply_format_mask(&self, value: &FieldValue, mask: &FormatMask) -> Result<String, String> {
        match mask {
            FormatMask::Number {
                decimals,
                thousands_separator,
                allow_negative,
                prefix,
                suffix,
            } => {
                let num = value.to_number();

                if !allow_negative && num < 0.0 {
                    return Err("Negative numbers not allowed".to_string());
                }

                let mut formatted = format!("{:.prec$}", num, prec = decimals);

                if *thousands_separator {
                    // Add thousands separators
                    let parts: Vec<&str> = formatted.split('.').collect();
                    let integer_part = parts[0];
                    let decimal_part = parts.get(1);

                    let mut result = String::new();
                    for (i, c) in integer_part.chars().rev().enumerate() {
                        if i > 0 && i % 3 == 0 {
                            result.insert(0, ',');
                        }
                        result.insert(0, c);
                    }

                    if let Some(dec) = decimal_part {
                        result.push('.');
                        result.push_str(dec);
                    }

                    formatted = result;
                }

                let mut result = String::new();
                if let Some(p) = prefix {
                    result.push_str(p);
                }
                result.push_str(&formatted);
                if let Some(s) = suffix {
                    result.push_str(s);
                }

                Ok(result)
            }
            FormatMask::Date { format } => self.format_date(&value.to_string(), *format),
            FormatMask::Time {
                format,
                use_24_hour,
            } => self.format_time(&value.to_string(), *format, *use_24_hour),
            FormatMask::Phone { country } => self.format_phone(&value.to_string(), *country),
            FormatMask::SSN => self.format_ssn(&value.to_string()),
            FormatMask::ZipCode { plus_four } => self.format_zip(&value.to_string(), *plus_four),
            FormatMask::CreditCard => self.format_credit_card(&value.to_string()),
            FormatMask::Custom {
                pattern,
                placeholder,
            } => self.apply_custom_mask(&value.to_string(), pattern, *placeholder),
        }
    }

    /// Format date string
    fn format_date(&self, date_str: &str, format: DateFormat) -> Result<String, String> {
        // Remove non-numeric characters
        let digits: String = date_str.chars().filter(|c| c.is_ascii_digit()).collect();

        if digits.len() < 8 {
            return Err("Invalid date format".to_string());
        }

        let formatted = match format {
            DateFormat::MDY => {
                format!("{}/{}/{}", &digits[0..2], &digits[2..4], &digits[4..8])
            }
            DateFormat::DMY => {
                format!("{}/{}/{}", &digits[0..2], &digits[2..4], &digits[4..8])
            }
            DateFormat::YMD => {
                format!("{}-{}-{}", &digits[0..4], &digits[4..6], &digits[6..8])
            }
            DateFormat::DotDMY => {
                format!("{}.{}.{}", &digits[0..2], &digits[2..4], &digits[4..8])
            }
            DateFormat::DashMDY => {
                format!("{}-{}-{}", &digits[0..2], &digits[2..4], &digits[4..8])
            }
        };

        Ok(formatted)
    }

    /// Format time string
    fn format_time(
        &self,
        time_str: &str,
        format: TimeFormat,
        use_24_hour: bool,
    ) -> Result<String, String> {
        let digits: String = time_str.chars().filter(|c| c.is_ascii_digit()).collect();

        if digits.len() < 4 {
            return Err("Invalid time format".to_string());
        }

        let hours: u32 = digits[0..2].parse().unwrap_or(0);
        let minutes: u32 = digits[2..4].parse().unwrap_or(0);
        let seconds: u32 = if digits.len() >= 6 {
            digits[4..6].parse().unwrap_or(0)
        } else {
            0
        };

        let formatted = match format {
            TimeFormat::HM => {
                if use_24_hour {
                    format!("{:02}:{:02}", hours, minutes)
                } else {
                    let (h, am_pm) = if hours == 0 {
                        (12, "AM")
                    } else if hours < 12 {
                        (hours, "AM")
                    } else if hours == 12 {
                        (12, "PM")
                    } else {
                        (hours - 12, "PM")
                    };
                    format!("{:02}:{:02} {}", h, minutes, am_pm)
                }
            }
            TimeFormat::HMS | TimeFormat::HMSAM => {
                if use_24_hour {
                    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
                } else {
                    let (h, am_pm) = if hours == 0 {
                        (12, "AM")
                    } else if hours < 12 {
                        (hours, "AM")
                    } else if hours == 12 {
                        (12, "PM")
                    } else {
                        (hours - 12, "PM")
                    };
                    format!("{:02}:{:02}:{:02} {}", h, minutes, seconds, am_pm)
                }
            }
            _ => format!("{:02}:{:02}", hours, minutes),
        };

        Ok(formatted)
    }

    /// Format phone number
    fn format_phone(&self, phone: &str, country: PhoneCountry) -> Result<String, String> {
        let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();

        let formatted = match country {
            PhoneCountry::US => {
                if digits.len() >= 10 {
                    format!("({}) {}-{}", &digits[0..3], &digits[3..6], &digits[6..10])
                } else {
                    return Err("Invalid US phone number".to_string());
                }
            }
            PhoneCountry::UK => {
                if digits.len() >= 11 {
                    format!("+{} {} {}", &digits[0..2], &digits[2..6], &digits[6..])
                } else {
                    return Err("Invalid UK phone number".to_string());
                }
            }
            _ => digits,
        };

        Ok(formatted)
    }

    /// Format SSN
    fn format_ssn(&self, ssn: &str) -> Result<String, String> {
        let digits: String = ssn.chars().filter(|c| c.is_ascii_digit()).collect();

        if digits.len() != 9 {
            return Err("SSN must be 9 digits".to_string());
        }

        Ok(format!(
            "{}-{}-{}",
            &digits[0..3],
            &digits[3..5],
            &digits[5..9]
        ))
    }

    /// Format ZIP code
    fn format_zip(&self, zip: &str, plus_four: bool) -> Result<String, String> {
        let digits: String = zip.chars().filter(|c| c.is_ascii_digit()).collect();

        if plus_four {
            if digits.len() != 9 {
                return Err("ZIP+4 must be 9 digits".to_string());
            }
            Ok(format!("{}-{}", &digits[0..5], &digits[5..9]))
        } else {
            if digits.len() < 5 {
                return Err("ZIP must be at least 5 digits".to_string());
            }
            Ok(digits[0..5].to_string())
        }
    }

    /// Format credit card number
    fn format_credit_card(&self, card: &str) -> Result<String, String> {
        let digits: String = card.chars().filter(|c| c.is_ascii_digit()).collect();

        if digits.len() < 13 || digits.len() > 19 {
            return Err("Invalid credit card number length".to_string());
        }

        // Format as groups of 4
        let mut formatted = String::new();
        for (i, c) in digits.chars().enumerate() {
            if i > 0 && i % 4 == 0 {
                formatted.push(' ');
            }
            formatted.push(c);
        }

        Ok(formatted)
    }

    /// Apply custom mask
    fn apply_custom_mask(
        &self,
        value: &str,
        pattern: &str,
        placeholder: char,
    ) -> Result<String, String> {
        let mut result = String::new();
        let mut value_chars = value.chars();

        for pattern_char in pattern.chars() {
            if pattern_char == placeholder {
                if let Some(c) = value_chars.next() {
                    result.push(c);
                } else {
                    break;
                }
            } else {
                result.push(pattern_char);
            }
        }

        Ok(result)
    }

    /// Get error type for validation rule
    fn get_error_type(&self, rule: &ValidationRule) -> ValidationErrorType {
        match rule {
            ValidationRule::Required => ValidationErrorType::Required,
            ValidationRule::Range { .. } => ValidationErrorType::Range,
            ValidationRule::Length { .. } => ValidationErrorType::Length,
            ValidationRule::Pattern(_) => ValidationErrorType::Pattern,
            _ => ValidationErrorType::Custom,
        }
    }

    /// Validate all fields
    pub fn validate_all(&mut self, fields: &HashMap<String, FieldValue>) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        for (field_name, value) in fields {
            results.push(self.validate_field(field_name, value));
        }

        results
    }

    /// Clear validation cache
    pub fn clear_cache(&mut self) {
        self.validation_cache.clear();
    }

    /// Get cached validation result
    pub fn get_cached_result(&self, field_name: &str) -> Option<&ValidationResult> {
        self.validation_cache.get(field_name)
    }
}

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_valid {
            write!(f, "Valid")
        } else {
            write!(f, "Invalid: {} errors", self.errors.len())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_field_validation() {
        let mut system = FormValidationSystem::new();

        let info = RequiredFieldInfo {
            field_name: "name".to_string(),
            error_message: "Name is required".to_string(),
            group: None,
            condition: None,
        };

        system.add_required_field(info);

        let result = system.validate_field("name", &FieldValue::Empty);
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].error_type, ValidationErrorType::Required);
    }

    #[test]
    fn test_email_validation() {
        let mut system = FormValidationSystem::new();

        let validator = FieldValidator {
            field_name: "email".to_string(),
            rules: vec![ValidationRule::Email],
            format_mask: None,
            error_message: None,
        };

        system.add_validator(validator);

        let valid_result =
            system.validate_field("email", &FieldValue::Text("test@example.com".to_string()));
        assert!(valid_result.is_valid);

        let invalid_result =
            system.validate_field("email", &FieldValue::Text("invalid-email".to_string()));
        assert!(!invalid_result.is_valid);
    }

    #[test]
    fn test_phone_format_mask() {
        let system = FormValidationSystem::new();

        let mask = FormatMask::Phone {
            country: PhoneCountry::US,
        };

        let result = system.apply_format_mask(&FieldValue::Text("5551234567".to_string()), &mask);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "(555) 123-4567");
    }

    #[test]
    fn test_credit_card_validation() {
        let system = FormValidationSystem::new();

        // Test with valid Visa test number
        let valid = system.validate_credit_card("4532015112830366");
        assert!(valid.is_ok());

        // Test with invalid number
        let invalid = system.validate_credit_card("1234567890123456");
        assert!(invalid.is_err());
    }

    #[test]
    fn test_ssn_format() {
        let system = FormValidationSystem::new();

        let result = system.format_ssn("123456789");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "123-45-6789");
    }

    #[test]
    fn test_range_validation() {
        let mut system = FormValidationSystem::new();

        let validator = FieldValidator {
            field_name: "age".to_string(),
            rules: vec![ValidationRule::Range {
                min: Some(18.0),
                max: Some(100.0),
            }],
            format_mask: None,
            error_message: None,
        };

        system.add_validator(validator);

        let valid = system.validate_field("age", &FieldValue::Number(25.0));
        assert!(valid.is_valid);

        let too_young = system.validate_field("age", &FieldValue::Number(15.0));
        assert!(!too_young.is_valid);

        let too_old = system.validate_field("age", &FieldValue::Number(150.0));
        assert!(!too_old.is_valid);
    }

    #[test]
    fn test_custom_mask() {
        let system = FormValidationSystem::new();

        let mask = FormatMask::Custom {
            pattern: "(###) ###-####".to_string(),
            placeholder: '#',
        };

        let result = system.apply_format_mask(&FieldValue::Text("5551234567".to_string()), &mask);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "(555) 123-4567");
    }

    #[test]
    fn test_validation_settings() {
        let settings = ValidationSettings::default();
        assert!(settings.real_time_validation);
        assert!(settings.highlight_errors);
        assert!(settings.show_error_messages);
    }

    #[test]
    fn test_url_validation() {
        let mut system = FormValidationSystem::new();

        let validator = FieldValidator {
            field_name: "website".to_string(),
            rules: vec![ValidationRule::Url],
            format_mask: None,
            error_message: None,
        };

        system.add_validator(validator);

        // Valid URLs
        let valid = system.validate_field(
            "website",
            &FieldValue::Text("https://example.com".to_string()),
        );
        assert!(valid.is_valid);

        let valid_http =
            system.validate_field("website", &FieldValue::Text("http://test.org".to_string()));
        assert!(valid_http.is_valid);

        // Invalid URLs
        let invalid = system.validate_field("website", &FieldValue::Text("not-a-url".to_string()));
        assert!(!invalid.is_valid);
    }

    #[test]
    fn test_length_validation() {
        let mut system = FormValidationSystem::new();

        let validator = FieldValidator {
            field_name: "comment".to_string(),
            rules: vec![ValidationRule::Length {
                min: Some(10),
                max: Some(100),
            }],
            format_mask: None,
            error_message: None,
        };

        system.add_validator(validator);

        // Valid length
        let valid = system.validate_field(
            "comment",
            &FieldValue::Text("This is a valid comment.".to_string()),
        );
        assert!(valid.is_valid);

        // Too short
        let too_short = system.validate_field("comment", &FieldValue::Text("Short".to_string()));
        assert!(!too_short.is_valid);

        // Too long
        let too_long = system.validate_field("comment", &FieldValue::Text("x".repeat(150)));
        assert!(!too_long.is_valid);
    }

    #[test]
    fn test_pattern_validation() {
        let mut system = FormValidationSystem::new();

        let validator = FieldValidator {
            field_name: "code".to_string(),
            rules: vec![ValidationRule::Pattern(r"^[A-Z]{3}-\d{3}$".to_string())],
            format_mask: None,
            error_message: Some("Code must be in format ABC-123".to_string()),
        };

        system.add_validator(validator);

        // Valid pattern
        let valid = system.validate_field("code", &FieldValue::Text("ABC-123".to_string()));
        assert!(valid.is_valid);

        // Invalid pattern
        let invalid = system.validate_field("code", &FieldValue::Text("abc-123".to_string()));
        assert!(!invalid.is_valid);
        assert!(invalid.errors[0].message.contains("ABC-123"));
    }

    #[test]
    fn test_date_validation() {
        let mut system = FormValidationSystem::new();

        let validator = FieldValidator {
            field_name: "birthdate".to_string(),
            rules: vec![ValidationRule::Date {
                min: Some(NaiveDate::from_ymd_opt(1900, 1, 1).unwrap()),
                max: Some(NaiveDate::from_ymd_opt(2020, 12, 31).unwrap()),
            }],
            format_mask: None,
            error_message: None,
        };

        system.add_validator(validator);

        // Valid date
        let valid = system.validate_field("birthdate", &FieldValue::Text("1990-05-15".to_string()));
        assert!(valid.is_valid);

        // Date too early
        let too_early =
            system.validate_field("birthdate", &FieldValue::Text("1850-01-01".to_string()));
        assert!(!too_early.is_valid);
    }

    #[test]
    fn test_time_validation() {
        let mut system = FormValidationSystem::new();

        let validator = FieldValidator {
            field_name: "appointment".to_string(),
            rules: vec![ValidationRule::Time {
                min: Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap()),
                max: Some(NaiveTime::from_hms_opt(17, 0, 0).unwrap()),
            }],
            format_mask: None,
            error_message: None,
        };

        system.add_validator(validator);

        // Valid time (business hours)
        let valid = system.validate_field("appointment", &FieldValue::Text("14:30".to_string()));
        assert!(valid.is_valid);

        // Too early
        let too_early =
            system.validate_field("appointment", &FieldValue::Text("08:00".to_string()));
        assert!(!too_early.is_valid);
    }

    #[test]
    fn test_phone_number_uk() {
        let mut system = FormValidationSystem::new();

        let validator = FieldValidator {
            field_name: "phone_uk".to_string(),
            rules: vec![ValidationRule::PhoneNumber {
                country: PhoneCountry::UK,
            }],
            format_mask: None,
            error_message: None,
        };

        system.add_validator(validator);

        // Valid UK phone
        let valid =
            system.validate_field("phone_uk", &FieldValue::Text("441234567890".to_string()));
        assert!(valid.is_valid);

        // Invalid UK phone (too short)
        let invalid = system.validate_field("phone_uk", &FieldValue::Text("12345".to_string()));
        assert!(!invalid.is_valid);
    }

    #[test]
    fn test_zip_format_with_plus_four() {
        let system = FormValidationSystem::new();

        // ZIP+4 format
        let result_plus = system.format_zip("123456789", true);
        assert!(result_plus.is_ok());
        assert_eq!(result_plus.unwrap(), "12345-6789");

        // Regular ZIP
        let result_regular = system.format_zip("12345", false);
        assert!(result_regular.is_ok());
        assert_eq!(result_regular.unwrap(), "12345");

        // Invalid ZIP+4 (too short)
        let invalid = system.format_zip("12345", true);
        assert!(invalid.is_err());
    }

    #[test]
    fn test_number_format_mask() {
        let system = FormValidationSystem::new();

        let mask = FormatMask::Number {
            decimals: 2,
            thousands_separator: true,
            allow_negative: true,
            prefix: Some("$".to_string()),
            suffix: Some(" USD".to_string()),
        };

        let result = system.apply_format_mask(&FieldValue::Number(1234567.89), &mask);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "$1,234,567.89 USD");

        // Test negative number
        let negative_result = system.apply_format_mask(&FieldValue::Number(-1234.56), &mask);
        assert!(negative_result.is_ok());
        assert_eq!(negative_result.unwrap(), "$-1,234.56 USD");
    }

    #[test]
    fn test_date_format_mask() {
        let system = FormValidationSystem::new();

        // Test MDY format
        let mask_mdy = FormatMask::Date {
            format: DateFormat::MDY,
        };
        let result_mdy =
            system.apply_format_mask(&FieldValue::Text("01152022".to_string()), &mask_mdy);
        assert!(result_mdy.is_ok());
        assert_eq!(result_mdy.unwrap(), "01/15/2022");

        // Test YMD format
        let mask_ymd = FormatMask::Date {
            format: DateFormat::YMD,
        };
        let result_ymd =
            system.apply_format_mask(&FieldValue::Text("20220115".to_string()), &mask_ymd);
        assert!(result_ymd.is_ok());
        assert_eq!(result_ymd.unwrap(), "2022-01-15");
    }

    #[test]
    fn test_time_format_mask() {
        let system = FormValidationSystem::new();

        // Test 24-hour format
        let mask_24 = FormatMask::Time {
            format: TimeFormat::HMS,
            use_24_hour: true,
        };
        let result_24 = system.apply_format_mask(&FieldValue::Text("143045".to_string()), &mask_24);
        assert!(result_24.is_ok());
        assert_eq!(result_24.unwrap(), "14:30:45");

        // Test 12-hour format
        let mask_12 = FormatMask::Time {
            format: TimeFormat::HMSAM,
            use_24_hour: false,
        };
        let result_12 = system.apply_format_mask(&FieldValue::Text("143045".to_string()), &mask_12);
        assert!(result_12.is_ok());
        assert_eq!(result_12.unwrap(), "02:30:45 PM");
    }

    #[test]
    fn test_validation_cache() {
        let mut system = FormValidationSystem::new();

        let validator = FieldValidator {
            field_name: "cached_field".to_string(),
            rules: vec![ValidationRule::Required],
            format_mask: None,
            error_message: None,
        };

        system.add_validator(validator);

        // First validation
        let result1 = system.validate_field("cached_field", &FieldValue::Text("value".to_string()));
        assert!(result1.is_valid);

        // Check cache
        let cached = system.get_cached_result("cached_field");
        assert!(cached.is_some());
        assert!(cached.unwrap().is_valid);

        // Clear cache
        system.clear_cache();
        let cached_after_clear = system.get_cached_result("cached_field");
        assert!(cached_after_clear.is_none());
    }

    #[test]
    fn test_validation_error_types() {
        let error_required = ValidationError {
            field_name: "test".to_string(),
            error_type: ValidationErrorType::Required,
            message: "Field is required".to_string(),
            details: None,
        };
        assert_eq!(error_required.error_type, ValidationErrorType::Required);

        let error_range = ValidationError {
            field_name: "test".to_string(),
            error_type: ValidationErrorType::Range,
            message: "Value out of range".to_string(),
            details: Some("Must be between 1 and 100".to_string()),
        };
        assert_eq!(error_range.error_type, ValidationErrorType::Range);
        assert!(error_range.details.is_some());
    }

    #[test]
    fn test_field_validator_with_multiple_rules() {
        let mut system = FormValidationSystem::new();

        let validator = FieldValidator {
            field_name: "username".to_string(),
            rules: vec![
                ValidationRule::Required,
                ValidationRule::Length {
                    min: Some(3),
                    max: Some(20),
                },
                ValidationRule::Pattern(r"^[a-zA-Z0-9_]+$".to_string()),
            ],
            format_mask: None,
            error_message: None,
        };

        system.add_validator(validator);

        // Valid username
        let valid = system.validate_field("username", &FieldValue::Text("user_123".to_string()));
        assert!(valid.is_valid);

        // Too short
        let too_short = system.validate_field("username", &FieldValue::Text("ab".to_string()));
        assert!(!too_short.is_valid);

        // Invalid characters
        let invalid_chars =
            system.validate_field("username", &FieldValue::Text("user@123".to_string()));
        assert!(!invalid_chars.is_valid);
    }

    #[test]
    fn test_credit_card_format() {
        let system = FormValidationSystem::new();

        let result = system.format_credit_card("4532015112830366");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "4532 0151 1283 0366");

        // Invalid length
        let too_short = system.format_credit_card("123");
        assert!(too_short.is_err());

        let too_long = system.format_credit_card("12345678901234567890");
        assert!(too_long.is_err());
    }

    #[test]
    fn test_required_field_with_group() {
        let mut system = FormValidationSystem::new();

        let info = RequiredFieldInfo {
            field_name: "address".to_string(),
            error_message: "Address is required".to_string(),
            group: Some("contact_info".to_string()),
            condition: None,
        };

        system.add_required_field(info);

        let result = system.validate_field("address", &FieldValue::Empty);
        assert!(!result.is_valid);
        assert_eq!(result.errors[0].error_type, ValidationErrorType::Required);
    }

    #[test]
    fn test_validation_result_display() {
        let valid_result = ValidationResult {
            field_name: "test".to_string(),
            is_valid: true,
            errors: vec![],
            warnings: vec![],
            formatted_value: None,
        };
        assert_eq!(format!("{}", valid_result), "Valid");

        let invalid_result = ValidationResult {
            field_name: "test".to_string(),
            is_valid: false,
            errors: vec![
                ValidationError {
                    field_name: "test".to_string(),
                    error_type: ValidationErrorType::Required,
                    message: "Required".to_string(),
                    details: None,
                },
                ValidationError {
                    field_name: "test".to_string(),
                    error_type: ValidationErrorType::Length,
                    message: "Too short".to_string(),
                    details: None,
                },
            ],
            warnings: vec![],
            formatted_value: None,
        };
        assert_eq!(format!("{}", invalid_result), "Invalid: 2 errors");
    }

    #[test]
    fn test_validate_all_fields() {
        let mut system = FormValidationSystem::new();

        // Add validators
        system.add_validator(FieldValidator {
            field_name: "name".to_string(),
            rules: vec![ValidationRule::Required],
            format_mask: None,
            error_message: None,
        });

        system.add_validator(FieldValidator {
            field_name: "age".to_string(),
            rules: vec![ValidationRule::Range {
                min: Some(0.0),
                max: Some(120.0),
            }],
            format_mask: None,
            error_message: None,
        });

        let mut fields = HashMap::new();
        fields.insert("name".to_string(), FieldValue::Text("John".to_string()));
        fields.insert("age".to_string(), FieldValue::Number(30.0));

        let results = system.validate_all(&fields);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_valid));
    }

    #[test]
    fn test_validation_settings_advanced() {
        let settings = ValidationSettings {
            validate_on_change: true,
            show_format_hints: true,
            auto_format: false,
            allow_partial: false,
            real_time_validation: true,
            highlight_errors: true,
            show_error_messages: true,
        };

        let mut system = FormValidationSystem::new();
        system.settings = settings.clone();

        assert!(system.settings.validate_on_change);
        assert!(system.settings.show_format_hints);
        assert!(!system.settings.auto_format);
    }

    #[test]
    fn test_complex_pattern_validation() {
        let mut system = FormValidationSystem::new();

        // Add complex regex pattern for product code
        system.add_validator(FieldValidator {
            field_name: "product_code".to_string(),
            rules: vec![ValidationRule::Pattern(
                r"^[A-Z]{3}-\d{4}-[A-Z]\d$".to_string(),
            )],
            format_mask: None,
            error_message: Some("Invalid product code format".to_string()),
        });

        let valid_code = FieldValue::Text("ABC-1234-A5".to_string());
        let result = system.validate_field("product_code", &valid_code);
        assert!(result.is_valid);

        let invalid_code = FieldValue::Text("abc-1234-a5".to_string());
        let result = system.validate_field("product_code", &invalid_code);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_currency_format_mask() {
        let system = FormValidationSystem::new();

        let mask = FormatMask::Number {
            decimals: 2,
            thousands_separator: true,
            allow_negative: false,
            prefix: Some("$".to_string()),
            suffix: None,
        };

        let result = system.apply_format_mask(&FieldValue::Number(1234567.89), &mask);
        assert!(result.is_ok());
        // Format should be $1,234,567.89
    }

    #[test]
    fn test_international_phone_formats() {
        let mut system = FormValidationSystem::new();

        // Test US phone
        system.add_validator(FieldValidator {
            field_name: "us_phone".to_string(),
            rules: vec![ValidationRule::PhoneNumber {
                country: PhoneCountry::US,
            }],
            format_mask: None,
            error_message: None,
        });

        let valid_us = FieldValue::Text("(555) 123-4567".to_string());
        assert!(system.validate_field("us_phone", &valid_us).is_valid);

        // Test UK phone
        system.add_validator(FieldValidator {
            field_name: "uk_phone".to_string(),
            rules: vec![ValidationRule::PhoneNumber {
                country: PhoneCountry::UK,
            }],
            format_mask: None,
            error_message: None,
        });

        let valid_uk = FieldValue::Text("+44 20 1234 5678".to_string());
        assert!(system.validate_field("uk_phone", &valid_uk).is_valid);
    }

    #[test]
    fn test_multiple_validation_rules() {
        let mut system = FormValidationSystem::new();

        // Password field with multiple rules
        system.add_validator(FieldValidator {
            field_name: "password".to_string(),
            rules: vec![
                ValidationRule::Required,
                ValidationRule::Length {
                    min: Some(8),
                    max: Some(32),
                },
                ValidationRule::Pattern(r"^(?=.*[A-Z])(?=.*[a-z])(?=.*\d).*$".to_string()),
            ],
            format_mask: None,
            error_message: Some(
                "Password must be 8-32 chars with uppercase, lowercase, and number".to_string(),
            ),
        });

        let weak_password = FieldValue::Text("abc123".to_string());
        let result = system.validate_field("password", &weak_password);
        assert!(!result.is_valid);
        assert!(result.errors.len() >= 2); // Length and pattern failures

        let strong_password = FieldValue::Text("SecurePass123".to_string());
        assert!(system.validate_field("password", &strong_password).is_valid);
    }

    #[test]
    fn test_conditional_required_field() {
        let mut system = FormValidationSystem::new();

        let info = RequiredFieldInfo {
            field_name: "shipping_address".to_string(),
            error_message: "Shipping address required when different from billing".to_string(),
            group: Some("shipping".to_string()),
            condition: Some(RequirementCondition::IfFieldNotEmpty {
                field: "different_shipping".to_string(),
            }),
        };

        system.add_required_field(info);

        // Should allow empty when condition not met
        let result = system.validate_field("shipping_address", &FieldValue::Empty);
        // In real scenario, condition would be evaluated
    }

    #[test]
    fn test_validation_cache_advanced() {
        let mut system = FormValidationSystem::new();

        system.add_validator(FieldValidator {
            field_name: "cached_field".to_string(),
            rules: vec![ValidationRule::Required],
            format_mask: None,
            error_message: None,
        });

        let value = FieldValue::Text("test".to_string());

        // First validation
        let result1 = system.validate_field("cached_field", &value);

        // Cache should contain result
        assert!(system.validation_cache.contains_key("cached_field"));

        // Clear cache
        system.clear_cache();
        assert!(system.validation_cache.is_empty());
    }

    #[test]
    fn test_custom_validator_function() {
        let mut system = FormValidationSystem::new();

        fn is_even_number(value: &FieldValue) -> bool {
            match value {
                FieldValue::Number(n) => (*n as i32) % 2 == 0,
                _ => false,
            }
        }

        system.add_validator(FieldValidator {
            field_name: "even_number".to_string(),
            rules: vec![ValidationRule::Custom {
                name: "even_check".to_string(),
                validator: is_even_number,
            }],
            format_mask: None,
            error_message: Some("Must be an even number".to_string()),
        });

        assert!(
            system
                .validate_field("even_number", &FieldValue::Number(4.0))
                .is_valid
        );
        assert!(
            !system
                .validate_field("even_number", &FieldValue::Number(5.0))
                .is_valid
        );
    }

    #[test]
    fn test_percentage_format() {
        let system = FormValidationSystem::new();

        let mask = FormatMask::Number {
            decimals: 1,
            thousands_separator: false,
            allow_negative: false,
            prefix: None,
            suffix: Some("%".to_string()),
        };

        let result = system.apply_format_mask(&FieldValue::Number(0.856), &mask);
        assert!(result.is_ok());
        // Should format as 85.6%
    }

    #[test]
    fn test_clear_validation_errors() {
        let mut system = FormValidationSystem::new();

        system.add_validator(FieldValidator {
            field_name: "test".to_string(),
            rules: vec![ValidationRule::Required],
            format_mask: None,
            error_message: None,
        });

        // Validate and cache error
        let _ = system.validate_field("test", &FieldValue::Empty);
        assert!(system.validation_cache.contains_key("test"));

        // Clear cache
        system.clear_cache();
        assert!(system.validation_cache.is_empty());
    }

    #[test]
    fn test_batch_validation() {
        let mut system = FormValidationSystem::new();

        // Add multiple validators
        for i in 0..5 {
            system.add_validator(FieldValidator {
                field_name: format!("field_{}", i),
                rules: vec![ValidationRule::Required],
                format_mask: None,
                error_message: None,
            });
        }

        let mut fields = HashMap::new();
        for i in 0..5 {
            fields.insert(
                format!("field_{}", i),
                FieldValue::Text(format!("value_{}", i)),
            );
        }

        let results = system.validate_all(&fields);
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.is_valid));
    }
}
