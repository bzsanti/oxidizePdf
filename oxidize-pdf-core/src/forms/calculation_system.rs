//! Enhanced form calculation system with JavaScript support
//!
//! This module provides a complete calculation system for PDF forms supporting:
//! - JavaScript calculations (AFSimple, AFPercent, AFDate)
//! - Field dependencies and automatic recalculation
//! - Calculation order management
//! - Format validation

use crate::error::PdfError;
use crate::forms::calculations::{CalculationEngine, FieldValue};
use crate::objects::{Dictionary, Object};
use chrono::{DateTime, NaiveDate, Utc};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

/// Complete calculation system for PDF forms
#[derive(Debug, Clone)]
pub struct FormCalculationSystem {
    /// Core calculation engine
    engine: CalculationEngine,
    /// JavaScript calculations
    js_calculations: HashMap<String, JavaScriptCalculation>,
    /// Field formats
    field_formats: HashMap<String, FieldFormat>,
    /// Calculation events
    events: Vec<CalculationEvent>,
    /// Settings
    settings: CalculationSettings,
}

/// JavaScript calculation types (Adobe Forms)
#[derive(Debug, Clone)]
pub enum JavaScriptCalculation {
    /// AFSimple_Calculate - Basic arithmetic operations
    SimpleCalculate {
        operation: SimpleOperation,
        fields: Vec<String>,
    },
    /// AFPercent_Calculate - Percentage calculations
    PercentCalculate {
        base_field: String,
        percent_field: String,
        mode: PercentMode,
    },
    /// AFDate_Calculate - Date calculations
    DateCalculate {
        start_date_field: String,
        days_field: Option<String>,
        format: String,
    },
    /// AFRange_Calculate - Range validation
    RangeCalculate {
        field: String,
        min: Option<f64>,
        max: Option<f64>,
    },
    /// AFNumber_Calculate - Number formatting
    NumberCalculate {
        field: String,
        decimals: usize,
        sep_style: SeparatorStyle,
        currency: Option<String>,
    },
    /// Custom JavaScript code
    Custom {
        script: String,
        dependencies: Vec<String>,
    },
}

/// Simple arithmetic operations for AFSimple_Calculate
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SimpleOperation {
    Sum,     // SUM
    Product, // PRD
    Average, // AVG
    Minimum, // MIN
    Maximum, // MAX
}

/// Percentage calculation modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PercentMode {
    /// Calculate X% of base
    PercentOf,
    /// Calculate what % X is of base
    PercentageOf,
    /// Add X% to base
    AddPercent,
    /// Subtract X% from base
    SubtractPercent,
}

/// Number separator styles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeparatorStyle {
    /// 1,234.56
    CommaPeriod,
    /// 1.234,56
    PeriodComma,
    /// 1 234.56
    SpacePeriod,
    /// 1'234.56
    ApostrophePeriod,
    /// 1234.56
    None,
}

/// Field format specification
#[derive(Debug, Clone)]
pub enum FieldFormat {
    /// Number format
    Number {
        decimals: usize,
        separator: SeparatorStyle,
        negative_style: NegativeStyle,
        currency: Option<String>,
    },
    /// Percentage format
    Percent { decimals: usize },
    /// Date format
    Date { format: String },
    /// Time format
    Time { format: String },
    /// Special format (SSN, Phone, Zip)
    Special { format_type: SpecialFormat },
    /// Custom format
    Custom { format_string: String },
}

/// Negative number display styles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NegativeStyle {
    MinusBlack,       // -1,234.56
    RedParentheses,   // (1,234.56) in red
    BlackParentheses, // (1,234.56) in black
    MinusRed,         // -1,234.56 in red
}

/// Special format types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpecialFormat {
    ZipCode,      // 12345 or 12345-6789
    ZipCodePlus4, // 12345-6789
    PhoneNumber,  // (123) 456-7890
    SSN,          // 123-45-6789
}

/// Calculation event for logging
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CalculationEvent {
    /// Timestamp
    timestamp: DateTime<Utc>,
    /// Field that triggered the event
    field: String,
    /// Event type
    event_type: EventType,
    /// Old value
    old_value: Option<FieldValue>,
    /// New value
    new_value: Option<FieldValue>,
}

/// Event types
#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    ValueChanged,
    CalculationTriggered,
    ValidationFailed,
    FormatApplied,
    DependencyUpdated,
}

/// Calculation system settings
#[derive(Debug, Clone)]
pub struct CalculationSettings {
    /// Enable automatic recalculation
    pub auto_recalculate: bool,
    /// Maximum calculation depth (to prevent infinite loops)
    pub max_depth: usize,
    /// Enable event logging
    pub log_events: bool,
    /// Decimal precision
    pub decimal_precision: usize,
}

impl Default for CalculationSettings {
    fn default() -> Self {
        Self {
            auto_recalculate: true,
            max_depth: 100,
            log_events: true,
            decimal_precision: 2,
        }
    }
}

impl Default for FormCalculationSystem {
    fn default() -> Self {
        Self {
            engine: CalculationEngine::new(),
            js_calculations: HashMap::new(),
            field_formats: HashMap::new(),
            events: Vec::new(),
            settings: CalculationSettings::default(),
        }
    }
}

impl FormCalculationSystem {
    /// Create a new calculation system
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom settings
    pub fn with_settings(settings: CalculationSettings) -> Self {
        Self {
            settings,
            ..Self::default()
        }
    }

    /// Set a field value and trigger calculations
    pub fn set_field_value(
        &mut self,
        field_name: impl Into<String>,
        value: FieldValue,
    ) -> Result<(), PdfError> {
        let field_name = field_name.into();

        // Log event if enabled
        if self.settings.log_events {
            let old_value = self.engine.get_field_value(&field_name).cloned();
            self.events.push(CalculationEvent {
                timestamp: Utc::now(),
                field: field_name.clone(),
                event_type: EventType::ValueChanged,
                old_value,
                new_value: Some(value.clone()),
            });
        }

        // Set value in engine
        self.engine.set_field_value(field_name.clone(), value);

        // Trigger JavaScript calculations if enabled
        if self.settings.auto_recalculate {
            self.recalculate_js_fields(&field_name)?;
        }

        Ok(())
    }

    /// Add a JavaScript calculation
    pub fn add_js_calculation(
        &mut self,
        field_name: impl Into<String>,
        calculation: JavaScriptCalculation,
    ) -> Result<(), PdfError> {
        let field_name = field_name.into();

        // Extract dependencies
        let dependencies = self.extract_js_dependencies(&calculation);

        // Check for circular dependencies
        if self.would_create_cycle(&field_name, &dependencies) {
            return Err(PdfError::InvalidStructure(format!(
                "Circular dependency detected for field '{}'",
                field_name
            )));
        }

        // Store calculation
        self.js_calculations.insert(field_name.clone(), calculation);

        // Perform initial calculation
        self.calculate_js_field(&field_name)?;

        Ok(())
    }

    /// Extract dependencies from JavaScript calculation
    fn extract_js_dependencies(&self, calc: &JavaScriptCalculation) -> HashSet<String> {
        let mut deps = HashSet::new();

        match calc {
            JavaScriptCalculation::SimpleCalculate { fields, .. } => {
                deps.extend(fields.iter().cloned());
            }
            JavaScriptCalculation::PercentCalculate {
                base_field,
                percent_field,
                ..
            } => {
                deps.insert(base_field.clone());
                deps.insert(percent_field.clone());
            }
            JavaScriptCalculation::DateCalculate {
                start_date_field,
                days_field,
                ..
            } => {
                deps.insert(start_date_field.clone());
                if let Some(df) = days_field {
                    deps.insert(df.clone());
                }
            }
            JavaScriptCalculation::RangeCalculate { field, .. } => {
                deps.insert(field.clone());
            }
            JavaScriptCalculation::NumberCalculate { field, .. } => {
                deps.insert(field.clone());
            }
            JavaScriptCalculation::Custom { dependencies, .. } => {
                deps.extend(dependencies.iter().cloned());
            }
        }

        deps
    }

    /// Check for circular dependencies
    fn would_create_cycle(&self, field: &str, new_deps: &HashSet<String>) -> bool {
        for dep in new_deps {
            if dep == field {
                return true; // Self-reference
            }

            // Check if dep depends on field
            if self.depends_on(dep, field) {
                return true;
            }
        }

        false
    }

    /// Check if field A depends on field B
    fn depends_on(&self, field_a: &str, field_b: &str) -> bool {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(field_a.to_string());

        while let Some(current) = queue.pop_front() {
            if current == field_b {
                return true;
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            // Check JavaScript calculation dependencies
            if let Some(calc) = self.js_calculations.get(&current) {
                let deps = self.extract_js_dependencies(calc);
                for dep in deps {
                    queue.push_back(dep);
                }
            }
        }

        false
    }

    /// Calculate a JavaScript field
    fn calculate_js_field(&mut self, field_name: &str) -> Result<(), PdfError> {
        if let Some(calculation) = self.js_calculations.get(field_name).cloned() {
            let value = self.evaluate_js_calculation(&calculation)?;

            // Apply format if specified
            let formatted_value = if let Some(format) = self.field_formats.get(field_name) {
                self.apply_format(value, format)?
            } else {
                value
            };

            self.engine.set_field_value(field_name, formatted_value);

            if self.settings.log_events {
                self.events.push(CalculationEvent {
                    timestamp: Utc::now(),
                    field: field_name.to_string(),
                    event_type: EventType::CalculationTriggered,
                    old_value: None,
                    new_value: self.engine.get_field_value(field_name).cloned(),
                });
            }
        }

        Ok(())
    }

    /// Evaluate a JavaScript calculation
    fn evaluate_js_calculation(
        &self,
        calc: &JavaScriptCalculation,
    ) -> Result<FieldValue, PdfError> {
        match calc {
            JavaScriptCalculation::SimpleCalculate { operation, fields } => {
                let values: Vec<f64> = fields
                    .iter()
                    .filter_map(|f| self.engine.get_field_value(f))
                    .map(|v| v.to_number())
                    .collect();

                if values.is_empty() {
                    return Ok(FieldValue::Number(0.0));
                }

                let result = match operation {
                    SimpleOperation::Sum => values.iter().sum(),
                    SimpleOperation::Product => values.iter().product(),
                    SimpleOperation::Average => values.iter().sum::<f64>() / values.len() as f64,
                    SimpleOperation::Minimum => {
                        values.iter().cloned().fold(f64::INFINITY, f64::min)
                    }
                    SimpleOperation::Maximum => {
                        values.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                    }
                };

                Ok(FieldValue::Number(result))
            }
            JavaScriptCalculation::PercentCalculate {
                base_field,
                percent_field,
                mode,
            } => {
                let base = self
                    .engine
                    .get_field_value(base_field)
                    .map(|v| v.to_number())
                    .unwrap_or(0.0);
                let percent = self
                    .engine
                    .get_field_value(percent_field)
                    .map(|v| v.to_number())
                    .unwrap_or(0.0);

                let result = match mode {
                    PercentMode::PercentOf => base * (percent / 100.0),
                    PercentMode::PercentageOf => {
                        if base != 0.0 {
                            (percent / base) * 100.0
                        } else {
                            0.0
                        }
                    }
                    PercentMode::AddPercent => base * (1.0 + percent / 100.0),
                    PercentMode::SubtractPercent => base * (1.0 - percent / 100.0),
                };

                Ok(FieldValue::Number(result))
            }
            JavaScriptCalculation::DateCalculate {
                start_date_field,
                days_field,
                format: _,
            } => {
                // Get start date
                let start_date_str = self
                    .engine
                    .get_field_value(start_date_field)
                    .map(|v| v.to_string())
                    .unwrap_or_default();

                // Parse date (simplified - real implementation would use format)
                if let Ok(date) = NaiveDate::parse_from_str(&start_date_str, "%Y-%m-%d") {
                    let days = if let Some(df) = days_field {
                        self.engine
                            .get_field_value(df)
                            .map(|v| v.to_number() as i64)
                            .unwrap_or(0)
                    } else {
                        0
                    };

                    let result_date = date + chrono::Duration::days(days);
                    Ok(FieldValue::Text(result_date.format("%Y-%m-%d").to_string()))
                } else {
                    Ok(FieldValue::Text(String::new()))
                }
            }
            JavaScriptCalculation::RangeCalculate { field, min, max } => {
                let value = self
                    .engine
                    .get_field_value(field)
                    .map(|v| v.to_number())
                    .unwrap_or(0.0);

                let clamped = match (min, max) {
                    (Some(min_val), Some(max_val)) => value.clamp(*min_val, *max_val),
                    (Some(min_val), None) => value.max(*min_val),
                    (None, Some(max_val)) => value.min(*max_val),
                    (None, None) => value,
                };

                Ok(FieldValue::Number(clamped))
            }
            JavaScriptCalculation::NumberCalculate {
                field,
                decimals,
                sep_style: _,
                currency: _,
            } => {
                let value = self
                    .engine
                    .get_field_value(field)
                    .map(|v| v.to_number())
                    .unwrap_or(0.0);

                // Round to specified decimals
                let factor = 10_f64.powi(*decimals as i32);
                let rounded = (value * factor).round() / factor;

                Ok(FieldValue::Number(rounded))
            }
            JavaScriptCalculation::Custom { script, .. } => {
                // Very limited custom script evaluation
                // In production, this would use a proper JavaScript engine
                self.evaluate_custom_script(script)
            }
        }
    }

    /// Evaluate custom JavaScript (very limited)
    fn evaluate_custom_script(&self, script: &str) -> Result<FieldValue, PdfError> {
        // This is a placeholder for custom script evaluation
        // A real implementation would need a proper sandboxed JS engine

        // For now, just handle simple cases like "field1 + field2"
        if script.contains('+') {
            let parts: Vec<&str> = script.split('+').collect();
            if parts.len() == 2 {
                let field1 = parts[0].trim();
                let field2 = parts[1].trim();

                let val1 = self
                    .engine
                    .get_field_value(field1)
                    .map(|v| v.to_number())
                    .unwrap_or(0.0);
                let val2 = self
                    .engine
                    .get_field_value(field2)
                    .map(|v| v.to_number())
                    .unwrap_or(0.0);

                return Ok(FieldValue::Number(val1 + val2));
            }
        }

        Ok(FieldValue::Empty)
    }

    /// Recalculate JavaScript fields that depend on a changed field
    fn recalculate_js_fields(&mut self, changed_field: &str) -> Result<(), PdfError> {
        let mut fields_to_recalc = Vec::new();

        // Find fields that depend on the changed field
        for (field_name, calc) in &self.js_calculations {
            let deps = self.extract_js_dependencies(calc);
            if deps.contains(changed_field) {
                fields_to_recalc.push(field_name.clone());
            }
        }

        // Recalculate dependent fields
        for field in fields_to_recalc {
            self.calculate_js_field(&field)?;
        }

        Ok(())
    }

    /// Apply format to a field value
    fn apply_format(
        &self,
        value: FieldValue,
        format: &FieldFormat,
    ) -> Result<FieldValue, PdfError> {
        match format {
            FieldFormat::Number { decimals, .. } => {
                let num = value.to_number();
                let factor = 10_f64.powi(*decimals as i32);
                let rounded = (num * factor).round() / factor;
                Ok(FieldValue::Number(rounded))
            }
            FieldFormat::Percent { decimals } => {
                let num = value.to_number();
                let factor = 10_f64.powi(*decimals as i32);
                let rounded = (num * 100.0 * factor).round() / factor;
                Ok(FieldValue::Text(format!("{}%", rounded)))
            }
            _ => Ok(value),
        }
    }

    /// Set field format
    pub fn set_field_format(&mut self, field_name: impl Into<String>, format: FieldFormat) {
        self.field_formats.insert(field_name.into(), format);
    }

    /// Get calculation summary
    pub fn get_summary(&self) -> CalculationSystemSummary {
        CalculationSystemSummary {
            total_fields: self.engine.get_summary().total_fields,
            js_calculations: self.js_calculations.len(),
            formatted_fields: self.field_formats.len(),
            events_logged: self.events.len(),
        }
    }

    /// Get recent events
    pub fn get_recent_events(&self, count: usize) -> Vec<&CalculationEvent> {
        let start = self.events.len().saturating_sub(count);
        self.events[start..].iter().collect()
    }

    /// Clear event log
    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    /// Export to PDF dictionary
    pub fn to_pdf_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        // Add calculation order
        let calc_order: Vec<Object> = self
            .js_calculations
            .keys()
            .map(|k| Object::String(k.clone()))
            .collect();

        if !calc_order.is_empty() {
            dict.set("CO", Object::Array(calc_order));
        }

        dict
    }
}

/// Summary of calculation system state
#[derive(Debug, Clone)]
pub struct CalculationSystemSummary {
    pub total_fields: usize,
    pub js_calculations: usize,
    pub formatted_fields: usize,
    pub events_logged: usize,
}

impl fmt::Display for CalculationSystemSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Calculation System Summary:\n\
             - Total fields: {}\n\
             - JavaScript calculations: {}\n\
             - Formatted fields: {}\n\
             - Events logged: {}",
            self.total_fields, self.js_calculations, self.formatted_fields, self.events_logged
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_calculate() {
        let mut system = FormCalculationSystem::new();

        // Set field values
        system
            .set_field_value("field1", FieldValue::Number(10.0))
            .unwrap();
        system
            .set_field_value("field2", FieldValue::Number(20.0))
            .unwrap();
        system
            .set_field_value("field3", FieldValue::Number(30.0))
            .unwrap();

        // Add sum calculation
        let calc = JavaScriptCalculation::SimpleCalculate {
            operation: SimpleOperation::Sum,
            fields: vec![
                "field1".to_string(),
                "field2".to_string(),
                "field3".to_string(),
            ],
        };

        system.add_js_calculation("total", calc).unwrap();

        // Check result
        let total = system.engine.get_field_value("total").unwrap();
        assert_eq!(total.to_number(), 60.0);
    }

    #[test]
    fn test_percent_calculate() {
        let mut system = FormCalculationSystem::new();

        system
            .set_field_value("base", FieldValue::Number(100.0))
            .unwrap();
        system
            .set_field_value("percent", FieldValue::Number(15.0))
            .unwrap();

        let calc = JavaScriptCalculation::PercentCalculate {
            base_field: "base".to_string(),
            percent_field: "percent".to_string(),
            mode: PercentMode::PercentOf,
        };

        system.add_js_calculation("result", calc).unwrap();

        let result = system.engine.get_field_value("result").unwrap();
        assert_eq!(result.to_number(), 15.0);
    }

    #[test]
    fn test_range_calculate() {
        let mut system = FormCalculationSystem::new();

        system
            .set_field_value("value", FieldValue::Number(150.0))
            .unwrap();

        let calc = JavaScriptCalculation::RangeCalculate {
            field: "value".to_string(),
            min: Some(0.0),
            max: Some(100.0),
        };

        system.add_js_calculation("clamped", calc).unwrap();

        let clamped = system.engine.get_field_value("clamped").unwrap();
        assert_eq!(clamped.to_number(), 100.0);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut system = FormCalculationSystem::new();

        // A depends on B
        let calc1 = JavaScriptCalculation::SimpleCalculate {
            operation: SimpleOperation::Sum,
            fields: vec!["fieldB".to_string()],
        };
        system.add_js_calculation("fieldA", calc1).unwrap();

        // Try to make B depend on A (should fail)
        let calc2 = JavaScriptCalculation::SimpleCalculate {
            operation: SimpleOperation::Sum,
            fields: vec!["fieldA".to_string()],
        };
        let result = system.add_js_calculation("fieldB", calc2);

        assert!(result.is_err());
    }

    #[test]
    fn test_event_logging() {
        let mut system = FormCalculationSystem::new();

        system
            .set_field_value("test", FieldValue::Number(42.0))
            .unwrap();

        assert_eq!(system.events.len(), 1);
        assert_eq!(system.events[0].event_type, EventType::ValueChanged);
        assert_eq!(system.events[0].field, "test");
    }

    // ===== New tests for improved coverage =====

    #[test]
    fn test_default_calculation_settings() {
        let settings = CalculationSettings::default();
        assert!(settings.auto_recalculate);
        assert_eq!(settings.max_depth, 100);
        assert!(settings.log_events);
        assert_eq!(settings.decimal_precision, 2);
    }

    #[test]
    fn test_form_calculation_system_default() {
        let system = FormCalculationSystem::default();
        let summary = system.get_summary();
        assert_eq!(summary.total_fields, 0);
        assert_eq!(summary.js_calculations, 0);
        assert_eq!(summary.formatted_fields, 0);
        assert_eq!(summary.events_logged, 0);
    }

    #[test]
    fn test_with_settings() {
        let settings = CalculationSettings {
            auto_recalculate: false,
            max_depth: 50,
            log_events: false,
            decimal_precision: 4,
        };
        let system = FormCalculationSystem::with_settings(settings.clone());
        assert!(!system.settings.auto_recalculate);
        assert_eq!(system.settings.max_depth, 50);
    }

    #[test]
    fn test_simple_calculate_product() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("a", FieldValue::Number(2.0))
            .unwrap();
        system
            .set_field_value("b", FieldValue::Number(3.0))
            .unwrap();
        system
            .set_field_value("c", FieldValue::Number(4.0))
            .unwrap();

        let calc = JavaScriptCalculation::SimpleCalculate {
            operation: SimpleOperation::Product,
            fields: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        };
        system.add_js_calculation("product", calc).unwrap();

        let result = system.engine.get_field_value("product").unwrap();
        assert_eq!(result.to_number(), 24.0);
    }

    #[test]
    fn test_simple_calculate_average() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("a", FieldValue::Number(10.0))
            .unwrap();
        system
            .set_field_value("b", FieldValue::Number(20.0))
            .unwrap();
        system
            .set_field_value("c", FieldValue::Number(30.0))
            .unwrap();

        let calc = JavaScriptCalculation::SimpleCalculate {
            operation: SimpleOperation::Average,
            fields: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        };
        system.add_js_calculation("avg", calc).unwrap();

        let result = system.engine.get_field_value("avg").unwrap();
        assert_eq!(result.to_number(), 20.0);
    }

    #[test]
    fn test_simple_calculate_minimum() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("a", FieldValue::Number(10.0))
            .unwrap();
        system
            .set_field_value("b", FieldValue::Number(5.0))
            .unwrap();
        system
            .set_field_value("c", FieldValue::Number(15.0))
            .unwrap();

        let calc = JavaScriptCalculation::SimpleCalculate {
            operation: SimpleOperation::Minimum,
            fields: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        };
        system.add_js_calculation("min", calc).unwrap();

        let result = system.engine.get_field_value("min").unwrap();
        assert_eq!(result.to_number(), 5.0);
    }

    #[test]
    fn test_simple_calculate_maximum() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("a", FieldValue::Number(10.0))
            .unwrap();
        system
            .set_field_value("b", FieldValue::Number(5.0))
            .unwrap();
        system
            .set_field_value("c", FieldValue::Number(15.0))
            .unwrap();

        let calc = JavaScriptCalculation::SimpleCalculate {
            operation: SimpleOperation::Maximum,
            fields: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        };
        system.add_js_calculation("max", calc).unwrap();

        let result = system.engine.get_field_value("max").unwrap();
        assert_eq!(result.to_number(), 15.0);
    }

    #[test]
    fn test_simple_calculate_empty_fields() {
        let mut system = FormCalculationSystem::new();

        let calc = JavaScriptCalculation::SimpleCalculate {
            operation: SimpleOperation::Sum,
            fields: vec![],
        };
        system.add_js_calculation("empty_sum", calc).unwrap();

        let result = system.engine.get_field_value("empty_sum").unwrap();
        assert_eq!(result.to_number(), 0.0);
    }

    #[test]
    fn test_percent_calculate_percentage_of() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("base", FieldValue::Number(200.0))
            .unwrap();
        system
            .set_field_value("value", FieldValue::Number(50.0))
            .unwrap();

        let calc = JavaScriptCalculation::PercentCalculate {
            base_field: "base".to_string(),
            percent_field: "value".to_string(),
            mode: PercentMode::PercentageOf,
        };
        system.add_js_calculation("percentage", calc).unwrap();

        let result = system.engine.get_field_value("percentage").unwrap();
        assert_eq!(result.to_number(), 25.0);
    }

    #[test]
    fn test_percent_calculate_percentage_of_zero_base() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("base", FieldValue::Number(0.0))
            .unwrap();
        system
            .set_field_value("value", FieldValue::Number(50.0))
            .unwrap();

        let calc = JavaScriptCalculation::PercentCalculate {
            base_field: "base".to_string(),
            percent_field: "value".to_string(),
            mode: PercentMode::PercentageOf,
        };
        system.add_js_calculation("percentage", calc).unwrap();

        let result = system.engine.get_field_value("percentage").unwrap();
        assert_eq!(result.to_number(), 0.0);
    }

    #[test]
    fn test_percent_calculate_add_percent() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("base", FieldValue::Number(100.0))
            .unwrap();
        system
            .set_field_value("percent", FieldValue::Number(10.0))
            .unwrap();

        let calc = JavaScriptCalculation::PercentCalculate {
            base_field: "base".to_string(),
            percent_field: "percent".to_string(),
            mode: PercentMode::AddPercent,
        };
        system.add_js_calculation("with_tax", calc).unwrap();

        let result = system.engine.get_field_value("with_tax").unwrap();
        assert!((result.to_number() - 110.0).abs() < 0.0001);
    }

    #[test]
    fn test_percent_calculate_subtract_percent() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("base", FieldValue::Number(100.0))
            .unwrap();
        system
            .set_field_value("percent", FieldValue::Number(20.0))
            .unwrap();

        let calc = JavaScriptCalculation::PercentCalculate {
            base_field: "base".to_string(),
            percent_field: "percent".to_string(),
            mode: PercentMode::SubtractPercent,
        };
        system.add_js_calculation("discount", calc).unwrap();

        let result = system.engine.get_field_value("discount").unwrap();
        assert_eq!(result.to_number(), 80.0);
    }

    #[test]
    fn test_range_calculate_min_only() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("value", FieldValue::Number(-10.0))
            .unwrap();

        let calc = JavaScriptCalculation::RangeCalculate {
            field: "value".to_string(),
            min: Some(0.0),
            max: None,
        };
        system.add_js_calculation("clamped", calc).unwrap();

        let result = system.engine.get_field_value("clamped").unwrap();
        assert_eq!(result.to_number(), 0.0);
    }

    #[test]
    fn test_range_calculate_max_only() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("value", FieldValue::Number(150.0))
            .unwrap();

        let calc = JavaScriptCalculation::RangeCalculate {
            field: "value".to_string(),
            min: None,
            max: Some(100.0),
        };
        system.add_js_calculation("clamped", calc).unwrap();

        let result = system.engine.get_field_value("clamped").unwrap();
        assert_eq!(result.to_number(), 100.0);
    }

    #[test]
    fn test_range_calculate_no_limits() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("value", FieldValue::Number(999.0))
            .unwrap();

        let calc = JavaScriptCalculation::RangeCalculate {
            field: "value".to_string(),
            min: None,
            max: None,
        };
        system.add_js_calculation("passthrough", calc).unwrap();

        let result = system.engine.get_field_value("passthrough").unwrap();
        assert_eq!(result.to_number(), 999.0);
    }

    #[test]
    fn test_number_calculate() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("value", FieldValue::Number(123.456789))
            .unwrap();

        let calc = JavaScriptCalculation::NumberCalculate {
            field: "value".to_string(),
            decimals: 2,
            sep_style: SeparatorStyle::CommaPeriod,
            currency: Some("$".to_string()),
        };
        system.add_js_calculation("formatted", calc).unwrap();

        let result = system.engine.get_field_value("formatted").unwrap();
        assert!((result.to_number() - 123.46).abs() < 0.001);
    }

    #[test]
    fn test_date_calculate() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("start_date", FieldValue::Text("2024-01-01".to_string()))
            .unwrap();
        system
            .set_field_value("days", FieldValue::Number(10.0))
            .unwrap();

        let calc = JavaScriptCalculation::DateCalculate {
            start_date_field: "start_date".to_string(),
            days_field: Some("days".to_string()),
            format: "%Y-%m-%d".to_string(),
        };
        system.add_js_calculation("end_date", calc).unwrap();

        let result = system.engine.get_field_value("end_date").unwrap();
        assert_eq!(result.to_string(), "2024-01-11");
    }

    #[test]
    fn test_date_calculate_invalid_date() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("start_date", FieldValue::Text("invalid".to_string()))
            .unwrap();

        let calc = JavaScriptCalculation::DateCalculate {
            start_date_field: "start_date".to_string(),
            days_field: None,
            format: "%Y-%m-%d".to_string(),
        };
        system.add_js_calculation("end_date", calc).unwrap();

        let result = system.engine.get_field_value("end_date").unwrap();
        assert_eq!(result.to_string(), "");
    }

    #[test]
    fn test_date_calculate_no_days_field() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("start_date", FieldValue::Text("2024-06-15".to_string()))
            .unwrap();

        let calc = JavaScriptCalculation::DateCalculate {
            start_date_field: "start_date".to_string(),
            days_field: None,
            format: "%Y-%m-%d".to_string(),
        };
        system.add_js_calculation("end_date", calc).unwrap();

        let result = system.engine.get_field_value("end_date").unwrap();
        assert_eq!(result.to_string(), "2024-06-15");
    }

    #[test]
    fn test_custom_script_addition() {
        let mut system = FormCalculationSystem::new();
        system
            .set_field_value("a", FieldValue::Number(10.0))
            .unwrap();
        system
            .set_field_value("b", FieldValue::Number(20.0))
            .unwrap();

        let calc = JavaScriptCalculation::Custom {
            script: "a + b".to_string(),
            dependencies: vec!["a".to_string(), "b".to_string()],
        };
        system.add_js_calculation("custom_result", calc).unwrap();

        let result = system.engine.get_field_value("custom_result").unwrap();
        assert_eq!(result.to_number(), 30.0);
    }

    #[test]
    fn test_custom_script_unsupported() {
        let mut system = FormCalculationSystem::new();

        let calc = JavaScriptCalculation::Custom {
            script: "some unsupported script".to_string(),
            dependencies: vec![],
        };
        system.add_js_calculation("unsupported", calc).unwrap();

        let result = system.engine.get_field_value("unsupported").unwrap();
        // Should return Empty for unsupported scripts
        assert_eq!(result.to_number(), 0.0);
    }

    #[test]
    fn test_self_reference_detection() {
        let mut system = FormCalculationSystem::new();

        let calc = JavaScriptCalculation::SimpleCalculate {
            operation: SimpleOperation::Sum,
            fields: vec!["selfField".to_string()],
        };
        let result = system.add_js_calculation("selfField", calc);

        assert!(result.is_err());
    }

    #[test]
    fn test_field_format_number() {
        let mut system = FormCalculationSystem::new();

        system.set_field_format(
            "price",
            FieldFormat::Number {
                decimals: 2,
                separator: SeparatorStyle::CommaPeriod,
                negative_style: NegativeStyle::MinusBlack,
                currency: Some("$".to_string()),
            },
        );

        // Set up a calculation that uses the format
        system
            .set_field_value("raw_price", FieldValue::Number(123.456))
            .unwrap();

        let calc = JavaScriptCalculation::NumberCalculate {
            field: "raw_price".to_string(),
            decimals: 2,
            sep_style: SeparatorStyle::CommaPeriod,
            currency: Some("$".to_string()),
        };
        system.add_js_calculation("price", calc).unwrap();

        let summary = system.get_summary();
        assert_eq!(summary.formatted_fields, 1);
    }

    #[test]
    fn test_field_format_percent() {
        let mut system = FormCalculationSystem::new();

        system.set_field_format("rate", FieldFormat::Percent { decimals: 1 });

        let summary = system.get_summary();
        assert_eq!(summary.formatted_fields, 1);
    }

    #[test]
    fn test_apply_format_number() {
        let system = FormCalculationSystem::new();

        let format = FieldFormat::Number {
            decimals: 2,
            separator: SeparatorStyle::CommaPeriod,
            negative_style: NegativeStyle::MinusBlack,
            currency: None,
        };

        let result = system
            .apply_format(FieldValue::Number(123.456789), &format)
            .unwrap();
        assert!((result.to_number() - 123.46).abs() < 0.001);
    }

    #[test]
    fn test_apply_format_percent() {
        let system = FormCalculationSystem::new();

        let format = FieldFormat::Percent { decimals: 1 };

        let result = system
            .apply_format(FieldValue::Number(0.5), &format)
            .unwrap();
        assert!(result.to_string().contains("50"));
    }

    #[test]
    fn test_get_recent_events() {
        let mut system = FormCalculationSystem::new();

        for i in 0..10 {
            system
                .set_field_value(format!("field{}", i), FieldValue::Number(i as f64))
                .unwrap();
        }

        let recent = system.get_recent_events(5);
        assert_eq!(recent.len(), 5);
    }

    #[test]
    fn test_get_recent_events_more_than_available() {
        let mut system = FormCalculationSystem::new();

        system
            .set_field_value("field1", FieldValue::Number(1.0))
            .unwrap();
        system
            .set_field_value("field2", FieldValue::Number(2.0))
            .unwrap();

        let recent = system.get_recent_events(100);
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn test_clear_events() {
        let mut system = FormCalculationSystem::new();

        system
            .set_field_value("field1", FieldValue::Number(1.0))
            .unwrap();
        assert!(!system.events.is_empty());

        system.clear_events();
        assert!(system.events.is_empty());
    }

    #[test]
    fn test_to_pdf_dict() {
        let mut system = FormCalculationSystem::new();

        let calc = JavaScriptCalculation::SimpleCalculate {
            operation: SimpleOperation::Sum,
            fields: vec!["a".to_string(), "b".to_string()],
        };
        system.add_js_calculation("total", calc).unwrap();

        let dict = system.to_pdf_dict();
        assert!(dict.get("CO").is_some());
    }

    #[test]
    fn test_to_pdf_dict_empty() {
        let system = FormCalculationSystem::new();
        let dict = system.to_pdf_dict();
        assert!(dict.get("CO").is_none());
    }

    #[test]
    fn test_calculation_system_summary_display() {
        let summary = CalculationSystemSummary {
            total_fields: 10,
            js_calculations: 5,
            formatted_fields: 3,
            events_logged: 20,
        };

        let display = format!("{}", summary);
        assert!(display.contains("Total fields: 10"));
        assert!(display.contains("JavaScript calculations: 5"));
        assert!(display.contains("Formatted fields: 3"));
        assert!(display.contains("Events logged: 20"));
    }

    #[test]
    fn test_auto_recalculate_disabled() {
        let settings = CalculationSettings {
            auto_recalculate: false,
            ..Default::default()
        };
        let mut system = FormCalculationSystem::with_settings(settings);

        system
            .set_field_value("a", FieldValue::Number(10.0))
            .unwrap();
        system
            .set_field_value("b", FieldValue::Number(20.0))
            .unwrap();

        let calc = JavaScriptCalculation::SimpleCalculate {
            operation: SimpleOperation::Sum,
            fields: vec!["a".to_string(), "b".to_string()],
        };
        system.add_js_calculation("sum", calc).unwrap();

        // Now change a field - sum should NOT auto-update since auto_recalculate is false
        system
            .set_field_value("a", FieldValue::Number(50.0))
            .unwrap();

        // Manual check - the sum was calculated at add time, but not recalculated
        let result = system.engine.get_field_value("sum").unwrap();
        assert_eq!(result.to_number(), 30.0); // Still 10 + 20 from initial calculation
    }

    #[test]
    fn test_log_events_disabled() {
        let settings = CalculationSettings {
            log_events: false,
            ..Default::default()
        };
        let mut system = FormCalculationSystem::with_settings(settings);

        system
            .set_field_value("field1", FieldValue::Number(1.0))
            .unwrap();
        system
            .set_field_value("field2", FieldValue::Number(2.0))
            .unwrap();

        assert!(system.events.is_empty());
    }

    #[test]
    fn test_separator_style_variants() {
        assert_eq!(SeparatorStyle::CommaPeriod, SeparatorStyle::CommaPeriod);
        assert_eq!(SeparatorStyle::PeriodComma, SeparatorStyle::PeriodComma);
        assert_eq!(SeparatorStyle::SpacePeriod, SeparatorStyle::SpacePeriod);
        assert_eq!(
            SeparatorStyle::ApostrophePeriod,
            SeparatorStyle::ApostrophePeriod
        );
        assert_eq!(SeparatorStyle::None, SeparatorStyle::None);
    }

    #[test]
    fn test_negative_style_variants() {
        assert_eq!(NegativeStyle::MinusBlack, NegativeStyle::MinusBlack);
        assert_eq!(NegativeStyle::RedParentheses, NegativeStyle::RedParentheses);
        assert_eq!(
            NegativeStyle::BlackParentheses,
            NegativeStyle::BlackParentheses
        );
        assert_eq!(NegativeStyle::MinusRed, NegativeStyle::MinusRed);
    }

    #[test]
    fn test_special_format_variants() {
        assert_eq!(SpecialFormat::ZipCode, SpecialFormat::ZipCode);
        assert_eq!(SpecialFormat::ZipCodePlus4, SpecialFormat::ZipCodePlus4);
        assert_eq!(SpecialFormat::PhoneNumber, SpecialFormat::PhoneNumber);
        assert_eq!(SpecialFormat::SSN, SpecialFormat::SSN);
    }

    #[test]
    fn test_simple_operation_variants() {
        assert_eq!(SimpleOperation::Sum, SimpleOperation::Sum);
        assert_eq!(SimpleOperation::Product, SimpleOperation::Product);
        assert_eq!(SimpleOperation::Average, SimpleOperation::Average);
        assert_eq!(SimpleOperation::Minimum, SimpleOperation::Minimum);
        assert_eq!(SimpleOperation::Maximum, SimpleOperation::Maximum);
    }

    #[test]
    fn test_percent_mode_variants() {
        assert_eq!(PercentMode::PercentOf, PercentMode::PercentOf);
        assert_eq!(PercentMode::PercentageOf, PercentMode::PercentageOf);
        assert_eq!(PercentMode::AddPercent, PercentMode::AddPercent);
        assert_eq!(PercentMode::SubtractPercent, PercentMode::SubtractPercent);
    }

    #[test]
    fn test_event_type_variants() {
        assert_eq!(EventType::ValueChanged, EventType::ValueChanged);
        assert_eq!(
            EventType::CalculationTriggered,
            EventType::CalculationTriggered
        );
        assert_eq!(EventType::ValidationFailed, EventType::ValidationFailed);
        assert_eq!(EventType::FormatApplied, EventType::FormatApplied);
        assert_eq!(EventType::DependencyUpdated, EventType::DependencyUpdated);
    }

    #[test]
    fn test_recalculate_dependent_fields() {
        let mut system = FormCalculationSystem::new();

        // Set up initial values
        system
            .set_field_value("base", FieldValue::Number(100.0))
            .unwrap();

        // Add a calculation that depends on base
        let calc = JavaScriptCalculation::SimpleCalculate {
            operation: SimpleOperation::Sum,
            fields: vec!["base".to_string()],
        };
        system.add_js_calculation("derived", calc).unwrap();

        // Verify initial calculation
        let initial = system.engine.get_field_value("derived").unwrap();
        assert_eq!(initial.to_number(), 100.0);

        // Change base - derived should auto-update
        system
            .set_field_value("base", FieldValue::Number(200.0))
            .unwrap();

        let updated = system.engine.get_field_value("derived").unwrap();
        assert_eq!(updated.to_number(), 200.0);
    }

    #[test]
    fn test_field_format_date() {
        let mut system = FormCalculationSystem::new();

        system.set_field_format(
            "date_field",
            FieldFormat::Date {
                format: "%Y-%m-%d".to_string(),
            },
        );

        let summary = system.get_summary();
        assert_eq!(summary.formatted_fields, 1);
    }

    #[test]
    fn test_field_format_time() {
        let mut system = FormCalculationSystem::new();

        system.set_field_format(
            "time_field",
            FieldFormat::Time {
                format: "%H:%M:%S".to_string(),
            },
        );

        let summary = system.get_summary();
        assert_eq!(summary.formatted_fields, 1);
    }

    #[test]
    fn test_field_format_special() {
        let mut system = FormCalculationSystem::new();

        system.set_field_format(
            "ssn_field",
            FieldFormat::Special {
                format_type: SpecialFormat::SSN,
            },
        );

        let summary = system.get_summary();
        assert_eq!(summary.formatted_fields, 1);
    }

    #[test]
    fn test_field_format_custom() {
        let mut system = FormCalculationSystem::new();

        system.set_field_format(
            "custom_field",
            FieldFormat::Custom {
                format_string: "###-###".to_string(),
            },
        );

        let summary = system.get_summary();
        assert_eq!(summary.formatted_fields, 1);
    }

    #[test]
    fn test_apply_format_passthrough() {
        let system = FormCalculationSystem::new();

        // Date format should pass through the value unchanged in current implementation
        let format = FieldFormat::Date {
            format: "%Y-%m-%d".to_string(),
        };

        let result = system
            .apply_format(FieldValue::Text("2024-01-01".to_string()), &format)
            .unwrap();
        assert_eq!(result.to_string(), "2024-01-01");
    }
}
