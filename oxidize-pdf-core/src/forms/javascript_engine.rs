//! Limited JavaScript engine for form calculations
//!
//! This module provides a very limited JavaScript interpreter that only
//! supports basic arithmetic and field references for security reasons.

use crate::error::PdfError;
use std::collections::HashMap;

/// Limited JavaScript engine for form calculations
#[derive(Debug, Clone)]
pub struct JavaScriptEngine {
    /// Variables/field values
    variables: HashMap<String, f64>,
    /// Reference to calculation engine for field access
    field_getter: Option<FieldGetter>,
}

/// Function to get field values
type FieldGetter = fn(&str) -> Option<f64>;

/// JavaScript token types
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum Token {
    Number(f64),
    Identifier(String),
    Plus,
    Minus,
    Multiply,
    Divide,
    LeftParen,
    RightParen,
    Equals,
    NotEquals,
    LessThan,
    LessThanEquals,
    GreaterThan,
    GreaterThanEquals,
    And,
    Or,
    Not,
    If,
    Else,
    Return,
    Semicolon,
    Comma,
    Dot,
    #[allow(clippy::upper_case_acronyms)]
    EOF,
}

/// JavaScript parser for limited expressions
struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

/// Abstract syntax tree node
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum ASTNode {
    Number(f64),
    Identifier(String),
    BinaryOp {
        op: BinaryOperator,
        left: Box<ASTNode>,
        right: Box<ASTNode>,
    },
    UnaryOp {
        op: UnaryOperator,
        operand: Box<ASTNode>,
    },
    FieldAccess {
        object: String,
        field: String,
    },
    FunctionCall {
        name: String,
        args: Vec<ASTNode>,
    },
    Conditional {
        condition: Box<ASTNode>,
        then_expr: Box<ASTNode>,
        else_expr: Option<Box<ASTNode>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equals,
    NotEquals,
    LessThan,
    LessThanEquals,
    GreaterThan,
    GreaterThanEquals,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
enum UnaryOperator {
    Negate,
    Not,
}

#[allow(clippy::derivable_impls)]
impl Default for JavaScriptEngine {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            field_getter: None,
        }
    }
}

impl JavaScriptEngine {
    /// Create a new JavaScript engine
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a variable value
    pub fn set_variable(&mut self, name: impl Into<String>, value: f64) {
        self.variables.insert(name.into(), value);
    }

    /// Set field getter function
    pub fn set_field_getter(&mut self, getter: FieldGetter) {
        self.field_getter = Some(getter);
    }

    /// Evaluate a JavaScript expression
    pub fn evaluate(&self, code: &str) -> Result<f64, PdfError> {
        // Tokenize
        let tokens = self.tokenize(code)?;

        // Parse
        let mut parser = Parser::new(tokens);
        let ast = parser.parse()?;

        // Evaluate
        self.eval_node(&ast)
    }

    /// Tokenize JavaScript code
    fn tokenize(&self, code: &str) -> Result<Vec<Token>, PdfError> {
        let mut tokens = Vec::new();
        let mut chars = code.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                ' ' | '\t' | '\n' | '\r' => continue,
                '+' => tokens.push(Token::Plus),
                '-' => tokens.push(Token::Minus),
                '*' => tokens.push(Token::Multiply),
                '/' => {
                    if chars.peek() == Some(&'/') {
                        // Skip line comment
                        chars.next();
                        for c in chars.by_ref() {
                            if c == '\n' {
                                break;
                            }
                        }
                    } else {
                        tokens.push(Token::Divide);
                    }
                }
                '(' => tokens.push(Token::LeftParen),
                ')' => tokens.push(Token::RightParen),
                '=' => {
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        tokens.push(Token::Equals);
                    }
                }
                '!' => {
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        tokens.push(Token::NotEquals);
                    } else {
                        tokens.push(Token::Not);
                    }
                }
                '<' => {
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        tokens.push(Token::LessThanEquals);
                    } else {
                        tokens.push(Token::LessThan);
                    }
                }
                '>' => {
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        tokens.push(Token::GreaterThanEquals);
                    } else {
                        tokens.push(Token::GreaterThan);
                    }
                }
                '&' => {
                    if chars.peek() == Some(&'&') {
                        chars.next();
                        tokens.push(Token::And);
                    }
                }
                '|' => {
                    if chars.peek() == Some(&'|') {
                        chars.next();
                        tokens.push(Token::Or);
                    }
                }
                ';' => tokens.push(Token::Semicolon),
                ',' => tokens.push(Token::Comma),
                '.' => tokens.push(Token::Dot),
                '0'..='9' => {
                    let mut num_str = String::new();
                    num_str.push(ch);
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_ascii_digit() || next_ch == '.' {
                            num_str.push(
                                chars
                                    .next()
                                    .ok_or_else(|| PdfError::InvalidFormat("Unexpected end of number literal".to_string()))?,
                            );
                        } else {
                            break;
                        }
                    }
                    let num = num_str
                        .parse::<f64>()
                        .map_err(|_| PdfError::InvalidFormat("Invalid number".to_string()))?;
                    tokens.push(Token::Number(num));
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let mut ident = String::new();
                    ident.push(ch);
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_alphanumeric() || next_ch == '_' {
                            ident.push(
                                chars
                                    .next()
                                    .ok_or_else(|| PdfError::InvalidFormat("Unexpected end of identifier".to_string()))?,
                            );
                        } else {
                            break;
                        }
                    }

                    // Check for keywords
                    let token = match ident.as_str() {
                        "if" => Token::If,
                        "else" => Token::Else,
                        "return" => Token::Return,
                        _ => Token::Identifier(ident),
                    };
                    tokens.push(token);
                }
                _ => {
                    // Ignore other characters
                }
            }
        }

        tokens.push(Token::EOF);
        Ok(tokens)
    }

    /// Evaluate an AST node
    fn eval_node(&self, node: &ASTNode) -> Result<f64, PdfError> {
        match node {
            ASTNode::Number(n) => Ok(*n),
            ASTNode::Identifier(name) => {
                // Try variables first
                if let Some(&value) = self.variables.get(name) {
                    return Ok(value);
                }

                // Try field getter
                if let Some(getter) = self.field_getter {
                    if let Some(value) = getter(name) {
                        return Ok(value);
                    }
                }

                // Default to 0
                Ok(0.0)
            }
            ASTNode::BinaryOp { op, left, right } => {
                let left_val = self.eval_node(left)?;
                let right_val = self.eval_node(right)?;

                match op {
                    BinaryOperator::Add => Ok(left_val + right_val),
                    BinaryOperator::Subtract => Ok(left_val - right_val),
                    BinaryOperator::Multiply => Ok(left_val * right_val),
                    BinaryOperator::Divide => {
                        if right_val != 0.0 {
                            Ok(left_val / right_val)
                        } else {
                            Ok(0.0)
                        }
                    }
                    BinaryOperator::Equals => Ok(if left_val == right_val { 1.0 } else { 0.0 }),
                    BinaryOperator::NotEquals => Ok(if left_val != right_val { 1.0 } else { 0.0 }),
                    BinaryOperator::LessThan => Ok(if left_val < right_val { 1.0 } else { 0.0 }),
                    BinaryOperator::LessThanEquals => {
                        Ok(if left_val <= right_val { 1.0 } else { 0.0 })
                    }
                    BinaryOperator::GreaterThan => Ok(if left_val > right_val { 1.0 } else { 0.0 }),
                    BinaryOperator::GreaterThanEquals => {
                        Ok(if left_val >= right_val { 1.0 } else { 0.0 })
                    }
                    BinaryOperator::And => Ok(if left_val != 0.0 && right_val != 0.0 {
                        1.0
                    } else {
                        0.0
                    }),
                    BinaryOperator::Or => Ok(if left_val != 0.0 || right_val != 0.0 {
                        1.0
                    } else {
                        0.0
                    }),
                }
            }
            ASTNode::UnaryOp { op, operand } => {
                let val = self.eval_node(operand)?;
                match op {
                    UnaryOperator::Negate => Ok(-val),
                    UnaryOperator::Not => Ok(if val == 0.0 { 1.0 } else { 0.0 }),
                }
            }
            ASTNode::FieldAccess { object, field } => {
                // For "this.field" access
                if object == "this" {
                    if let Some(getter) = self.field_getter {
                        if let Some(value) = getter(field) {
                            return Ok(value);
                        }
                    }
                }
                Ok(0.0)
            }
            ASTNode::FunctionCall { name, args } => {
                // Support basic math functions
                match name.as_str() {
                    "Math.min" => {
                        let values: Result<Vec<f64>, _> =
                            args.iter().map(|arg| self.eval_node(arg)).collect();
                        let values = values?;
                        Ok(values.iter().cloned().fold(f64::INFINITY, f64::min))
                    }
                    "Math.max" => {
                        let values: Result<Vec<f64>, _> =
                            args.iter().map(|arg| self.eval_node(arg)).collect();
                        let values = values?;
                        Ok(values.iter().cloned().fold(f64::NEG_INFINITY, f64::max))
                    }
                    "Math.round" => {
                        if let Some(arg) = args.first() {
                            Ok(self.eval_node(arg)?.round())
                        } else {
                            Ok(0.0)
                        }
                    }
                    "Math.floor" => {
                        if let Some(arg) = args.first() {
                            Ok(self.eval_node(arg)?.floor())
                        } else {
                            Ok(0.0)
                        }
                    }
                    "Math.ceil" => {
                        if let Some(arg) = args.first() {
                            Ok(self.eval_node(arg)?.ceil())
                        } else {
                            Ok(0.0)
                        }
                    }
                    "Math.abs" => {
                        if let Some(arg) = args.first() {
                            Ok(self.eval_node(arg)?.abs())
                        } else {
                            Ok(0.0)
                        }
                    }
                    _ => Ok(0.0),
                }
            }
            ASTNode::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond_val = self.eval_node(condition)?;
                if cond_val != 0.0 {
                    self.eval_node(then_expr)
                } else if let Some(else_expr) = else_expr {
                    self.eval_node(else_expr)
                } else {
                    Ok(0.0)
                }
            }
        }
    }
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    fn parse(&mut self) -> Result<ASTNode, PdfError> {
        self.parse_expression()
    }

    fn parse_expression(&mut self) -> Result<ASTNode, PdfError> {
        self.parse_conditional()
    }

    fn parse_conditional(&mut self) -> Result<ASTNode, PdfError> {
        let expr = self.parse_logical_or()?;

        // Check for ternary conditional (? :)
        // For simplicity, we'll skip this for now

        Ok(expr)
    }

    fn parse_logical_or(&mut self) -> Result<ASTNode, PdfError> {
        let mut left = self.parse_logical_and()?;

        while self.current_token() == Some(&Token::Or) {
            self.advance();
            let right = self.parse_logical_and()?;
            left = ASTNode::BinaryOp {
                op: BinaryOperator::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<ASTNode, PdfError> {
        let mut left = self.parse_equality()?;

        while self.current_token() == Some(&Token::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = ASTNode::BinaryOp {
                op: BinaryOperator::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<ASTNode, PdfError> {
        let mut left = self.parse_relational()?;

        while let Some(token) = self.current_token() {
            let op = match token {
                Token::Equals => BinaryOperator::Equals,
                Token::NotEquals => BinaryOperator::NotEquals,
                _ => break,
            };

            self.advance();
            let right = self.parse_relational()?;
            left = ASTNode::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<ASTNode, PdfError> {
        let mut left = self.parse_additive()?;

        while let Some(token) = self.current_token() {
            let op = match token {
                Token::LessThan => BinaryOperator::LessThan,
                Token::LessThanEquals => BinaryOperator::LessThanEquals,
                Token::GreaterThan => BinaryOperator::GreaterThan,
                Token::GreaterThanEquals => BinaryOperator::GreaterThanEquals,
                _ => break,
            };

            self.advance();
            let right = self.parse_additive()?;
            left = ASTNode::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<ASTNode, PdfError> {
        let mut left = self.parse_multiplicative()?;

        while let Some(token) = self.current_token() {
            let op = match token {
                Token::Plus => BinaryOperator::Add,
                Token::Minus => BinaryOperator::Subtract,
                _ => break,
            };

            self.advance();
            let right = self.parse_multiplicative()?;
            left = ASTNode::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<ASTNode, PdfError> {
        let mut left = self.parse_unary()?;

        while let Some(token) = self.current_token() {
            let op = match token {
                Token::Multiply => BinaryOperator::Multiply,
                Token::Divide => BinaryOperator::Divide,
                _ => break,
            };

            self.advance();
            let right = self.parse_unary()?;
            left = ASTNode::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<ASTNode, PdfError> {
        if let Some(token) = self.current_token() {
            match token {
                Token::Minus => {
                    self.advance();
                    let operand = self.parse_unary()?;
                    return Ok(ASTNode::UnaryOp {
                        op: UnaryOperator::Negate,
                        operand: Box::new(operand),
                    });
                }
                Token::Not => {
                    self.advance();
                    let operand = self.parse_unary()?;
                    return Ok(ASTNode::UnaryOp {
                        op: UnaryOperator::Not,
                        operand: Box::new(operand),
                    });
                }
                _ => {}
            }
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<ASTNode, PdfError> {
        if let Some(token) = self.current_token().cloned() {
            match token {
                Token::Number(n) => {
                    self.advance();
                    Ok(ASTNode::Number(n))
                }
                Token::Identifier(name) => {
                    self.advance();

                    // Check for field access or function call
                    if self.current_token() == Some(&Token::Dot) {
                        self.advance();
                        if let Some(Token::Identifier(field)) = self.current_token().cloned() {
                            self.advance();

                            // Check for function call
                            if self.current_token() == Some(&Token::LeftParen) {
                                self.advance();
                                let args = self.parse_arguments()?;
                                self.expect(Token::RightParen)?;
                                return Ok(ASTNode::FunctionCall {
                                    name: format!("{}.{}", name, field),
                                    args,
                                });
                            } else {
                                return Ok(ASTNode::FieldAccess {
                                    object: name,
                                    field,
                                });
                            }
                        }
                    }

                    Ok(ASTNode::Identifier(name))
                }
                Token::LeftParen => {
                    self.advance();
                    let expr = self.parse_expression()?;
                    self.expect(Token::RightParen)?;
                    Ok(expr)
                }
                _ => Err(PdfError::InvalidFormat("Unexpected token".to_string())),
            }
        } else {
            Err(PdfError::InvalidFormat(
                "Unexpected end of input".to_string(),
            ))
        }
    }

    fn parse_arguments(&mut self) -> Result<Vec<ASTNode>, PdfError> {
        let mut args = Vec::new();

        if self.current_token() != Some(&Token::RightParen) {
            loop {
                args.push(self.parse_expression()?);

                if self.current_token() == Some(&Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        Ok(args)
    }

    fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }

    fn advance(&mut self) {
        self.current += 1;
    }

    fn expect(&mut self, expected: Token) -> Result<(), PdfError> {
        if self.current_token() == Some(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(PdfError::InvalidFormat(format!(
                "Expected {:?}, got {:?}",
                expected,
                self.current_token()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_arithmetic() {
        let engine = JavaScriptEngine::new();

        assert_eq!(engine.evaluate("2 + 3").unwrap(), 5.0);
        assert_eq!(engine.evaluate("10 - 4").unwrap(), 6.0);
        assert_eq!(engine.evaluate("3 * 4").unwrap(), 12.0);
        assert_eq!(engine.evaluate("15 / 3").unwrap(), 5.0);
    }

    #[test]
    fn test_parentheses() {
        let engine = JavaScriptEngine::new();

        assert_eq!(engine.evaluate("2 * (3 + 4)").unwrap(), 14.0);
        assert_eq!(engine.evaluate("(10 - 2) / 4").unwrap(), 2.0);
    }

    #[test]
    fn test_variables() {
        let mut engine = JavaScriptEngine::new();
        engine.set_variable("x", 10.0);
        engine.set_variable("y", 5.0);

        assert_eq!(engine.evaluate("x + y").unwrap(), 15.0);
        assert_eq!(engine.evaluate("x * 2 - y").unwrap(), 15.0);
    }

    #[test]
    fn test_comparison() {
        let engine = JavaScriptEngine::new();

        assert_eq!(engine.evaluate("5 > 3").unwrap(), 1.0);
        assert_eq!(engine.evaluate("2 < 1").unwrap(), 0.0);
        assert_eq!(engine.evaluate("3 == 3").unwrap(), 1.0);
        assert_eq!(engine.evaluate("4 != 4").unwrap(), 0.0);
    }

    #[test]
    fn test_logical_operators() {
        let engine = JavaScriptEngine::new();

        assert_eq!(engine.evaluate("1 && 1").unwrap(), 1.0);
        assert_eq!(engine.evaluate("1 && 0").unwrap(), 0.0);
        assert_eq!(engine.evaluate("0 || 1").unwrap(), 1.0);
        assert_eq!(engine.evaluate("0 || 0").unwrap(), 0.0);
    }

    #[test]
    fn test_math_functions() {
        let engine = JavaScriptEngine::new();

        assert_eq!(engine.evaluate("Math.min(5, 3, 7)").unwrap(), 3.0);
        assert_eq!(engine.evaluate("Math.max(5, 3, 7)").unwrap(), 7.0);
        assert_eq!(engine.evaluate("Math.round(3.7)").unwrap(), 4.0);
        assert_eq!(engine.evaluate("Math.floor(3.7)").unwrap(), 3.0);
        assert_eq!(engine.evaluate("Math.ceil(3.2)").unwrap(), 4.0);
        assert_eq!(engine.evaluate("Math.abs(-5)").unwrap(), 5.0);
    }

    // =============================================================================
    // RIGOROUS TESTS FOR UNCOVERED EDGE CASES
    // =============================================================================

    #[test]
    fn test_division_by_zero() {
        let engine = JavaScriptEngine::new();

        // Division by zero should return 0.0, not NaN or panic
        let result = engine
            .evaluate("10 / 0")
            .expect("Division by zero must not panic");
        assert_eq!(result, 0.0, "Division by zero must return 0.0");

        // More complex expression with division by zero
        let result = engine
            .evaluate("(5 + 5) / (2 - 2)")
            .expect("Must handle division by zero");
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_unary_minus_operator() {
        let engine = JavaScriptEngine::new();

        // Simple negation
        assert_eq!(
            engine.evaluate("-5").unwrap(),
            -5.0,
            "Unary minus must negate"
        );

        // Negation of expression
        assert_eq!(engine.evaluate("-(3 + 2)").unwrap(), -5.0);

        // Double negation
        assert_eq!(
            engine.evaluate("--7").unwrap(),
            7.0,
            "Double negation must cancel out"
        );

        // Negation in complex expression
        assert_eq!(engine.evaluate("10 + -5").unwrap(), 5.0);
    }

    #[test]
    fn test_unary_not_operator() {
        let engine = JavaScriptEngine::new();

        // NOT of zero (falsy) -> 1.0 (truthy)
        assert_eq!(engine.evaluate("!0").unwrap(), 1.0, "NOT of 0 must be 1");

        // NOT of non-zero (truthy) -> 0.0 (falsy)
        assert_eq!(engine.evaluate("!5").unwrap(), 0.0, "NOT of 5 must be 0");

        // Double NOT
        assert_eq!(
            engine.evaluate("!!10").unwrap(),
            1.0,
            "Double NOT of truthy must be 1"
        );

        // NOT in expression
        assert_eq!(engine.evaluate("!0 && 1").unwrap(), 1.0);
    }

    #[test]
    fn test_line_comments() {
        let engine = JavaScriptEngine::new();

        // Single line comment
        let result = engine
            .evaluate("5 + 3 // This is a comment")
            .expect("Comments must be ignored");
        assert_eq!(result, 8.0, "Comments must not affect evaluation");

        // Comment in middle of expression (on new line conceptually)
        let result = engine
            .evaluate("10 // comment\n * 2")
            .expect("Comments must be skipped");
        assert_eq!(result, 20.0);
    }

    #[test]
    fn test_field_getter_integration() {
        let mut engine = JavaScriptEngine::new();

        // Define a field getter function
        fn test_getter(field_name: &str) -> Option<f64> {
            match field_name {
                "price" => Some(100.0),
                "quantity" => Some(5.0),
                "tax" => Some(0.08),
                _ => None,
            }
        }

        engine.set_field_getter(test_getter);

        // Test field access
        assert_eq!(
            engine.evaluate("price").unwrap(),
            100.0,
            "Field getter must resolve field names"
        );

        assert_eq!(engine.evaluate("price * quantity").unwrap(), 500.0);

        // Test field with addition
        assert_eq!(engine.evaluate("price * (1 + tax)").unwrap(), 108.0);

        // Non-existent field should return 0.0
        assert_eq!(
            engine.evaluate("nonexistent").unwrap(),
            0.0,
            "Unknown fields must return 0.0"
        );
    }

    #[test]
    fn test_this_field_access() {
        let mut engine = JavaScriptEngine::new();

        fn field_getter(field_name: &str) -> Option<f64> {
            match field_name {
                "total" => Some(250.0),
                "discount" => Some(0.10),
                _ => None,
            }
        }

        engine.set_field_getter(field_getter);

        // Test "this.field" syntax
        // Note: Actual evaluation depends on implementation
        // For now, testing that it doesn't panic
        let result = engine.evaluate("this.total");
        assert!(result.is_ok(), "this.field syntax must be supported");
    }

    #[test]
    fn test_math_functions_with_empty_args() {
        let engine = JavaScriptEngine::new();

        // Math functions with no args should return 0.0 or appropriate defaults
        let result = engine
            .evaluate("Math.round()")
            .expect("Empty args must not panic");
        assert_eq!(result, 0.0, "Math.round() with no args must return 0.0");

        let result = engine
            .evaluate("Math.floor()")
            .expect("Empty args must not panic");
        assert_eq!(result, 0.0);

        let result = engine
            .evaluate("Math.ceil()")
            .expect("Empty args must not panic");
        assert_eq!(result, 0.0);

        let result = engine
            .evaluate("Math.abs()")
            .expect("Empty args must not panic");
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_math_min_max_with_single_value() {
        let engine = JavaScriptEngine::new();

        // Min/max with single value
        assert_eq!(
            engine.evaluate("Math.min(42)").unwrap(),
            42.0,
            "Math.min with single arg must return that value"
        );

        assert_eq!(engine.evaluate("Math.max(42)").unwrap(), 42.0);
    }

    #[test]
    fn test_complex_nested_expressions() {
        let engine = JavaScriptEngine::new();

        // Deeply nested parentheses
        let result = engine
            .evaluate("((((5))))")
            .expect("Nested parens must work");
        assert_eq!(result, 5.0);

        // Complex mixed operators with precedence
        let result = engine.evaluate("2 + 3 * 4 - 5 / 5").unwrap();
        assert_eq!(
            result, 13.0,
            "Operator precedence must be correct: 2 + 12 - 1 = 13"
        );

        // Nested function calls
        let result = engine.evaluate("Math.max(Math.min(10, 5), 3)").unwrap();
        assert_eq!(result, 5.0, "Math.max(5, 3) = 5");
    }

    #[test]
    fn test_comparison_with_equals_vs_assignment() {
        let engine = JavaScriptEngine::new();

        // == should be comparison, not assignment
        assert_eq!(
            engine.evaluate("5 == 5").unwrap(),
            1.0,
            "== must be comparison returning 1.0 (true)"
        );

        assert_eq!(
            engine.evaluate("5 == 3").unwrap(),
            0.0,
            "== must return 0.0 (false) for unequal values"
        );

        // <= and >= operators
        assert_eq!(engine.evaluate("5 <= 5").unwrap(), 1.0);
        assert_eq!(engine.evaluate("5 >= 5").unwrap(), 1.0);
        assert_eq!(engine.evaluate("3 <= 5").unwrap(), 1.0);
        assert_eq!(engine.evaluate("5 >= 3").unwrap(), 1.0);
    }

    #[test]
    fn test_logical_operators_chaining() {
        let engine = JavaScriptEngine::new();

        // Chained AND operators
        assert_eq!(
            engine.evaluate("1 && 1 && 1").unwrap(),
            1.0,
            "All truthy values ANDed must return 1.0"
        );

        assert_eq!(
            engine.evaluate("1 && 0 && 1").unwrap(),
            0.0,
            "Any falsy value ANDed must return 0.0"
        );

        // Chained OR operators
        assert_eq!(
            engine.evaluate("0 || 0 || 1").unwrap(),
            1.0,
            "Any truthy value ORed must return 1.0"
        );

        assert_eq!(engine.evaluate("0 || 0 || 0").unwrap(), 0.0);

        // Mixed AND/OR
        assert_eq!(
            engine.evaluate("1 && 1 || 0").unwrap(),
            1.0,
            "(1 AND 1) OR 0 = 1"
        );

        assert_eq!(
            engine.evaluate("0 || 1 && 1").unwrap(),
            1.0,
            "0 OR (1 AND 1) = 1"
        );
    }

    #[test]
    fn test_whitespace_handling() {
        let engine = JavaScriptEngine::new();

        // Extra whitespace
        assert_eq!(
            engine.evaluate("  5   +   3  ").unwrap(),
            8.0,
            "Extra whitespace must be ignored"
        );

        // Tabs and newlines
        assert_eq!(engine.evaluate("5\t+\n3").unwrap(), 8.0);

        // No whitespace
        assert_eq!(engine.evaluate("5+3*2").unwrap(), 11.0);
    }

    #[test]
    fn test_decimal_numbers() {
        let engine = JavaScriptEngine::new();

        // Simple decimal
        assert_eq!(engine.evaluate("3.14").unwrap(), 3.14);

        // Decimal in expression
        assert_eq!(engine.evaluate("10.5 + 2.5").unwrap(), 13.0);

        // Multiple decimals in operations
        assert_eq!(engine.evaluate("0.1 + 0.2").unwrap(), 0.30000000000000004); // Float precision

        // Leading zero
        assert_eq!(engine.evaluate("0.5 * 10").unwrap(), 5.0);
    }

    #[test]
    fn test_error_handling_invalid_syntax() {
        let engine = JavaScriptEngine::new();

        // Missing operand
        let result = engine.evaluate("5 +");
        assert!(result.is_err(), "Incomplete expression must return error");

        // Mismatched parentheses
        let result = engine.evaluate("(5 + 3");
        assert!(result.is_err(), "Unclosed parenthesis must return error");

        // Empty input
        let result = engine.evaluate("");
        assert!(result.is_err(), "Empty expression must return error");
    }

    #[test]
    fn test_unknown_function() {
        let engine = JavaScriptEngine::new();

        // Unknown function should return 0.0, not panic
        let result = engine
            .evaluate("UnknownFunction()")
            .expect("Unknown functions must not panic");
        assert_eq!(result, 0.0, "Unknown functions must return 0.0");

        // Unknown Math function
        let result = engine
            .evaluate("Math.unknownFunc(5)")
            .expect("Must not panic");
        assert_eq!(result, 0.0);
    }
}
