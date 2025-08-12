//! Form field calculations support according to ISO 32000-1 Section 12.7.5.3
//!
//! This module provides calculation support for form fields including
//! basic arithmetic operations, field dependencies, and calculation order.

use crate::error::PdfError;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

/// Calculation engine for form fields
#[derive(Debug, Clone)]
pub struct CalculationEngine {
    /// Field values (field_name -> value)
    field_values: HashMap<String, FieldValue>,
    /// Calculations (field_name -> calculation)
    calculations: HashMap<String, Calculation>,
    /// Dependencies (field_name -> fields that depend on it)
    dependencies: HashMap<String, HashSet<String>>,
    /// Calculation order
    calculation_order: Vec<String>,
}

/// Value types for form fields
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// Numeric value
    Number(f64),
    /// String value
    Text(String),
    /// Boolean value (for checkboxes)
    Boolean(bool),
    /// Empty/null value
    Empty,
}

impl FieldValue {
    /// Convert to number, returns 0.0 for non-numeric values
    pub fn to_number(&self) -> f64 {
        match self {
            FieldValue::Number(n) => *n,
            FieldValue::Text(s) => s.parse::<f64>().unwrap_or(0.0),
            FieldValue::Boolean(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            FieldValue::Empty => 0.0,
        }
    }

    /// Convert to string
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        match self {
            FieldValue::Number(n) => {
                // Format number with appropriate decimal places
                if n.fract() == 0.0 {
                    format!("{:.0}", n)
                } else {
                    format!("{:.2}", n)
                }
            }
            FieldValue::Text(s) => s.clone(),
            FieldValue::Boolean(b) => b.to_string(),
            FieldValue::Empty => String::new(),
        }
    }
}

/// Calculation types
#[derive(Debug, Clone)]
pub enum Calculation {
    /// Simple arithmetic expression
    Arithmetic(ArithmeticExpression),
    /// Predefined function
    Function(CalculationFunction),
    /// Custom JavaScript (limited subset)
    JavaScript(String),
    /// Constant value
    Constant(FieldValue),
}

/// Arithmetic expression for calculations
#[derive(Debug, Clone)]
pub struct ArithmeticExpression {
    /// Expression tokens
    tokens: Vec<ExpressionToken>,
}

/// Expression tokens
#[derive(Debug, Clone)]
pub enum ExpressionToken {
    /// Field reference
    Field(String),
    /// Number literal
    Number(f64),
    /// Operator
    Operator(Operator),
    /// Left parenthesis
    LeftParen,
    /// Right parenthesis
    RightParen,
}

/// Arithmetic operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
}

impl Operator {
    /// Get operator precedence (higher = higher precedence)
    pub fn precedence(&self) -> i32 {
        match self {
            Operator::Power => 3,
            Operator::Multiply | Operator::Divide | Operator::Modulo => 2,
            Operator::Add | Operator::Subtract => 1,
        }
    }

    /// Apply operator to two values
    pub fn apply(&self, left: f64, right: f64) -> f64 {
        match self {
            Operator::Add => left + right,
            Operator::Subtract => left - right,
            Operator::Multiply => left * right,
            Operator::Divide => {
                if right != 0.0 {
                    left / right
                } else {
                    0.0 // Avoid division by zero
                }
            }
            Operator::Modulo => {
                if right != 0.0 {
                    left % right
                } else {
                    0.0
                }
            }
            Operator::Power => left.powf(right),
        }
    }
}

/// Predefined calculation functions
#[derive(Debug, Clone)]
pub enum CalculationFunction {
    /// Sum of specified fields
    Sum(Vec<String>),
    /// Average of specified fields
    Average(Vec<String>),
    /// Minimum value among fields
    Min(Vec<String>),
    /// Maximum value among fields
    Max(Vec<String>),
    /// Product of specified fields
    Product(Vec<String>),
    /// Count of non-empty fields
    Count(Vec<String>),
    /// If-then-else condition
    If {
        condition_field: String,
        true_value: Box<Calculation>,
        false_value: Box<Calculation>,
    },
}

#[allow(clippy::derivable_impls)]
impl Default for CalculationEngine {
    fn default() -> Self {
        Self {
            field_values: HashMap::new(),
            calculations: HashMap::new(),
            dependencies: HashMap::new(),
            calculation_order: Vec::new(),
        }
    }
}

impl CalculationEngine {
    /// Create a new calculation engine
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a field value
    pub fn set_field_value(&mut self, field_name: impl Into<String>, value: FieldValue) {
        let field_name = field_name.into();
        self.field_values.insert(field_name.clone(), value);

        // Trigger recalculation of dependent fields
        self.recalculate_dependents(&field_name);
    }

    /// Get a field value
    pub fn get_field_value(&self, field_name: &str) -> Option<&FieldValue> {
        self.field_values.get(field_name)
    }

    /// Add a calculation for a field
    pub fn add_calculation(
        &mut self,
        field_name: impl Into<String>,
        calculation: Calculation,
    ) -> Result<(), PdfError> {
        let field_name = field_name.into();

        // Extract dependencies from calculation
        let deps = self.extract_dependencies(&calculation);

        // Check for circular dependencies
        if self.would_create_cycle(&field_name, &deps) {
            return Err(PdfError::InvalidStructure(format!(
                "Circular dependency detected for field '{}'",
                field_name
            )));
        }

        // Update dependencies map
        for dep in &deps {
            self.dependencies
                .entry(dep.clone())
                .or_default()
                .insert(field_name.clone());
        }

        // Store calculation
        self.calculations.insert(field_name.clone(), calculation);

        // Update calculation order
        self.update_calculation_order()?;

        // Perform initial calculation
        self.calculate_field(&field_name)?;

        Ok(())
    }

    /// Extract field dependencies from a calculation
    #[allow(clippy::only_used_in_recursion)]
    fn extract_dependencies(&self, calculation: &Calculation) -> HashSet<String> {
        let mut deps = HashSet::new();

        match calculation {
            Calculation::Arithmetic(expr) => {
                for token in &expr.tokens {
                    if let ExpressionToken::Field(field_name) = token {
                        deps.insert(field_name.clone());
                    }
                }
            }
            Calculation::Function(func) => match func {
                CalculationFunction::Sum(fields)
                | CalculationFunction::Average(fields)
                | CalculationFunction::Min(fields)
                | CalculationFunction::Max(fields)
                | CalculationFunction::Product(fields)
                | CalculationFunction::Count(fields) => {
                    deps.extend(fields.iter().cloned());
                }
                CalculationFunction::If {
                    condition_field,
                    true_value,
                    false_value,
                } => {
                    deps.insert(condition_field.clone());
                    deps.extend(self.extract_dependencies(true_value));
                    deps.extend(self.extract_dependencies(false_value));
                }
            },
            Calculation::JavaScript(_) => {
                // Would need to parse JavaScript to extract dependencies
                // For now, we don't support this
            }
            Calculation::Constant(_) => {
                // No dependencies
            }
        }

        deps
    }

    /// Check if adding a dependency would create a cycle
    fn would_create_cycle(&self, field: &str, new_deps: &HashSet<String>) -> bool {
        for dep in new_deps {
            if dep == field {
                return true; // Self-reference
            }

            // Check if dep depends on field (directly or indirectly)
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

            // Get dependencies of current field
            if let Some(calc) = self.calculations.get(&current) {
                let deps = self.extract_dependencies(calc);
                for dep in deps {
                    queue.push_back(dep);
                }
            }
        }

        false
    }

    /// Update calculation order using topological sort
    fn update_calculation_order(&mut self) -> Result<(), PdfError> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        for field in self.calculations.keys() {
            if !visited.contains(field) {
                self.topological_sort(field, &mut visited, &mut visiting, &mut order)?;
            }
        }

        self.calculation_order = order;
        Ok(())
    }

    /// Topological sort helper
    fn topological_sort(
        &self,
        field: &str,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) -> Result<(), PdfError> {
        if visiting.contains(field) {
            return Err(PdfError::InvalidStructure(
                "Circular dependency detected".to_string(),
            ));
        }

        if visited.contains(field) {
            return Ok(());
        }

        visiting.insert(field.to_string());

        // Visit dependencies first
        if let Some(calc) = self.calculations.get(field) {
            let deps = self.extract_dependencies(calc);
            for dep in deps {
                if self.calculations.contains_key(&dep) {
                    self.topological_sort(&dep, visited, visiting, order)?;
                }
            }
        }

        visiting.remove(field);
        visited.insert(field.to_string());
        order.push(field.to_string());

        Ok(())
    }

    /// Recalculate dependent fields
    fn recalculate_dependents(&mut self, changed_field: &str) {
        // First ensure calculation order is up to date
        let _ = self.update_calculation_order();

        // Find all fields that depend on the changed field
        let mut fields_to_recalc = HashSet::new();
        if let Some(dependents) = self.dependencies.get(changed_field) {
            fields_to_recalc.extend(dependents.clone());
        }

        // Clone calculation order to avoid borrow issues
        let calc_order = self.calculation_order.clone();

        // Recalculate in dependency order
        for field in calc_order {
            if fields_to_recalc.contains(&field) {
                let _ = self.calculate_field(&field);
                // Also recalculate fields that depend on this field
                if let Some(deps) = self.dependencies.get(&field).cloned() {
                    fields_to_recalc.extend(deps);
                }
            }
        }
    }

    /// Calculate a single field
    pub fn calculate_field(&mut self, field_name: &str) -> Result<(), PdfError> {
        if let Some(calculation) = self.calculations.get(field_name).cloned() {
            let value = self.evaluate_calculation(&calculation)?;
            self.field_values.insert(field_name.to_string(), value);
        }
        Ok(())
    }

    /// Evaluate a calculation
    fn evaluate_calculation(&self, calculation: &Calculation) -> Result<FieldValue, PdfError> {
        match calculation {
            Calculation::Arithmetic(expr) => {
                let result = self.evaluate_expression(expr)?;
                Ok(FieldValue::Number(result))
            }
            Calculation::Function(func) => self.evaluate_function(func),
            Calculation::JavaScript(code) => {
                // Limited JavaScript evaluation
                self.evaluate_javascript(code)
            }
            Calculation::Constant(value) => Ok(value.clone()),
        }
    }

    /// Evaluate an arithmetic expression
    fn evaluate_expression(&self, expr: &ArithmeticExpression) -> Result<f64, PdfError> {
        // Convert infix to postfix (Shunting Yard algorithm)
        let postfix = self.infix_to_postfix(&expr.tokens)?;

        // Evaluate postfix expression
        let mut stack = Vec::new();

        for token in postfix {
            match token {
                ExpressionToken::Number(n) => stack.push(n),
                ExpressionToken::Field(field_name) => {
                    let value = self
                        .field_values
                        .get(&field_name)
                        .map(|v| v.to_number())
                        .unwrap_or(0.0);
                    stack.push(value);
                }
                ExpressionToken::Operator(op) => {
                    if stack.len() < 2 {
                        return Err(PdfError::InvalidStructure("Invalid expression".to_string()));
                    }
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    stack.push(op.apply(left, right));
                }
                _ => {}
            }
        }

        stack
            .pop()
            .ok_or_else(|| PdfError::InvalidStructure("Invalid expression".to_string()))
    }

    /// Convert infix expression to postfix
    fn infix_to_postfix(
        &self,
        tokens: &[ExpressionToken],
    ) -> Result<Vec<ExpressionToken>, PdfError> {
        let mut output = Vec::new();
        let mut operators = Vec::new();

        for token in tokens {
            match token {
                ExpressionToken::Number(_) | ExpressionToken::Field(_) => {
                    output.push(token.clone());
                }
                ExpressionToken::Operator(op) => {
                    while let Some(ExpressionToken::Operator(top_op)) = operators.last() {
                        if top_op.precedence() >= op.precedence() {
                            output.push(operators.pop().unwrap());
                        } else {
                            break;
                        }
                    }
                    operators.push(token.clone());
                }
                ExpressionToken::LeftParen => {
                    operators.push(token.clone());
                }
                ExpressionToken::RightParen => {
                    while let Some(op) = operators.pop() {
                        if matches!(op, ExpressionToken::LeftParen) {
                            break;
                        }
                        output.push(op);
                    }
                }
            }
        }

        while let Some(op) = operators.pop() {
            output.push(op);
        }

        Ok(output)
    }

    /// Evaluate a calculation function
    fn evaluate_function(&self, func: &CalculationFunction) -> Result<FieldValue, PdfError> {
        match func {
            CalculationFunction::Sum(fields) => {
                let sum = fields
                    .iter()
                    .filter_map(|f| self.field_values.get(f))
                    .map(|v| v.to_number())
                    .sum();
                Ok(FieldValue::Number(sum))
            }
            CalculationFunction::Average(fields) => {
                let values: Vec<f64> = fields
                    .iter()
                    .filter_map(|f| self.field_values.get(f))
                    .map(|v| v.to_number())
                    .collect();

                if values.is_empty() {
                    Ok(FieldValue::Number(0.0))
                } else {
                    let avg = values.iter().sum::<f64>() / values.len() as f64;
                    Ok(FieldValue::Number(avg))
                }
            }
            CalculationFunction::Min(fields) => {
                let min = fields
                    .iter()
                    .filter_map(|f| self.field_values.get(f))
                    .map(|v| v.to_number())
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(0.0);
                Ok(FieldValue::Number(min))
            }
            CalculationFunction::Max(fields) => {
                let max = fields
                    .iter()
                    .filter_map(|f| self.field_values.get(f))
                    .map(|v| v.to_number())
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(0.0);
                Ok(FieldValue::Number(max))
            }
            CalculationFunction::Product(fields) => {
                let product = fields
                    .iter()
                    .filter_map(|f| self.field_values.get(f))
                    .map(|v| v.to_number())
                    .product();
                Ok(FieldValue::Number(product))
            }
            CalculationFunction::Count(fields) => {
                let count = fields
                    .iter()
                    .filter_map(|f| self.field_values.get(f))
                    .filter(|v| !matches!(v, FieldValue::Empty))
                    .count() as f64;
                Ok(FieldValue::Number(count))
            }
            CalculationFunction::If {
                condition_field,
                true_value,
                false_value,
            } => {
                let condition = self
                    .field_values
                    .get(condition_field)
                    .map(|v| match v {
                        FieldValue::Boolean(b) => *b,
                        FieldValue::Number(n) => *n != 0.0,
                        FieldValue::Text(s) => !s.is_empty(),
                        FieldValue::Empty => false,
                    })
                    .unwrap_or(false);

                if condition {
                    self.evaluate_calculation(true_value)
                } else {
                    self.evaluate_calculation(false_value)
                }
            }
        }
    }

    /// Evaluate limited JavaScript code
    fn evaluate_javascript(&self, _code: &str) -> Result<FieldValue, PdfError> {
        // Very basic JavaScript evaluation
        // Only supports simple arithmetic and field references

        // For now, just return empty
        // A real implementation would need a proper JavaScript parser
        Ok(FieldValue::Empty)
    }

    /// Recalculate all fields in dependency order
    pub fn recalculate_all(&mut self) -> Result<(), PdfError> {
        for field in self.calculation_order.clone() {
            self.calculate_field(&field)?;
        }
        Ok(())
    }

    /// Get calculation summary
    pub fn get_summary(&self) -> CalculationSummary {
        CalculationSummary {
            total_fields: self.field_values.len(),
            calculated_fields: self.calculations.len(),
            dependencies: self.dependencies.len(),
            calculation_order: self.calculation_order.clone(),
        }
    }
}

/// Summary of calculations
#[derive(Debug, Clone)]
pub struct CalculationSummary {
    /// Total number of fields
    pub total_fields: usize,
    /// Number of calculated fields
    pub calculated_fields: usize,
    /// Number of dependency relationships
    pub dependencies: usize,
    /// Calculation order
    pub calculation_order: Vec<String>,
}

impl fmt::Display for CalculationSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Calculation Summary:\n\
             - Total fields: {}\n\
             - Calculated fields: {}\n\
             - Dependencies: {}\n\
             - Calculation order: {}",
            self.total_fields,
            self.calculated_fields,
            self.dependencies,
            self.calculation_order.join(" -> ")
        )
    }
}

impl ArithmeticExpression {
    /// Create expression from string
    pub fn from_string(expr: &str) -> Result<Self, PdfError> {
        let tokens = Self::tokenize(expr)?;
        Ok(Self { tokens })
    }

    /// Tokenize expression string
    fn tokenize(expr: &str) -> Result<Vec<ExpressionToken>, PdfError> {
        let mut tokens = Vec::new();
        let mut chars = expr.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                ' ' | '\t' | '\n' => continue,
                '+' => tokens.push(ExpressionToken::Operator(Operator::Add)),
                '-' => tokens.push(ExpressionToken::Operator(Operator::Subtract)),
                '*' => tokens.push(ExpressionToken::Operator(Operator::Multiply)),
                '/' => tokens.push(ExpressionToken::Operator(Operator::Divide)),
                '%' => tokens.push(ExpressionToken::Operator(Operator::Modulo)),
                '^' => tokens.push(ExpressionToken::Operator(Operator::Power)),
                '(' => tokens.push(ExpressionToken::LeftParen),
                ')' => tokens.push(ExpressionToken::RightParen),
                '0'..='9' | '.' => {
                    let mut num_str = String::new();
                    num_str.push(ch);
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_ascii_digit() || next_ch == '.' {
                            num_str.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    let num = num_str
                        .parse::<f64>()
                        .map_err(|_| PdfError::InvalidFormat("Invalid number".to_string()))?;
                    tokens.push(ExpressionToken::Number(num));
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let mut field_name = String::new();
                    field_name.push(ch);
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_alphanumeric() || next_ch == '_' {
                            field_name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    tokens.push(ExpressionToken::Field(field_name));
                }
                _ => {
                    return Err(PdfError::InvalidFormat(format!(
                        "Invalid character in expression: '{}'",
                        ch
                    )));
                }
            }
        }

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_value_conversion() {
        assert_eq!(FieldValue::Number(42.5).to_number(), 42.5);
        assert_eq!(FieldValue::Text("123".to_string()).to_number(), 123.0);
        assert_eq!(FieldValue::Boolean(true).to_number(), 1.0);
        assert_eq!(FieldValue::Empty.to_number(), 0.0);
    }

    #[test]
    fn test_arithmetic_expression() {
        let expr = ArithmeticExpression::from_string("2 + 3 * 4").unwrap();
        assert_eq!(expr.tokens.len(), 5);
    }

    #[test]
    fn test_calculation_engine() {
        let mut engine = CalculationEngine::new();

        // Set field values
        engine.set_field_value("quantity", FieldValue::Number(5.0));
        engine.set_field_value("price", FieldValue::Number(10.0));

        // Add calculation for total
        let expr = ArithmeticExpression::from_string("quantity * price").unwrap();
        engine
            .add_calculation("total", Calculation::Arithmetic(expr))
            .unwrap();

        // Check calculated value
        let total = engine.get_field_value("total").unwrap();
        assert_eq!(total.to_number(), 50.0);
    }

    #[test]
    fn test_sum_function() {
        let mut engine = CalculationEngine::new();

        engine.set_field_value("field1", FieldValue::Number(10.0));
        engine.set_field_value("field2", FieldValue::Number(20.0));
        engine.set_field_value("field3", FieldValue::Number(30.0));

        let calc = Calculation::Function(CalculationFunction::Sum(vec![
            "field1".to_string(),
            "field2".to_string(),
            "field3".to_string(),
        ]));

        engine.add_calculation("total", calc).unwrap();

        let total = engine.get_field_value("total").unwrap();
        assert_eq!(total.to_number(), 60.0);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut engine = CalculationEngine::new();

        // A depends on B
        let expr1 = ArithmeticExpression::from_string("fieldB + 1").unwrap();
        engine
            .add_calculation("fieldA", Calculation::Arithmetic(expr1))
            .unwrap();

        // Try to make B depend on A (should fail)
        let expr2 = ArithmeticExpression::from_string("fieldA + 1").unwrap();
        let result = engine.add_calculation("fieldB", Calculation::Arithmetic(expr2));

        assert!(result.is_err());
    }
}
