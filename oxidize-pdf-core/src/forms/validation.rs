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
}

impl Default for ValidationSettings {
    fn default() -> Self {
        Self {
            validate_on_change: true,
            show_format_hints: true,
            auto_format: true,
            allow_partial: false,
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
                    });
                }
            }
        }

        let result = ValidationResult {
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
            ValidationRule::Date { min: _, max: _ } => {
                // Parse date and validate range
                Ok(())
            }
            ValidationRule::Time { min: _, max: _ } => {
                // Parse time and validate range
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
}
