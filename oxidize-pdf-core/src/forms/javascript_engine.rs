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
                            num_str.push(chars.next().unwrap());
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
                            ident.push(chars.next().unwrap());
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
}
