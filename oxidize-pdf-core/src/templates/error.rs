use std::fmt;

/// Template-specific errors
#[derive(Debug, Clone)]
pub enum TemplateError {
    /// Variable not found in context
    VariableNotFound(String),
    /// Invalid placeholder syntax
    InvalidPlaceholder(String),
    /// Circular reference in variables
    CircularReference(String),
    /// Invalid variable name
    InvalidVariableName(String),
    /// Template parsing error
    ParseError(String),
    /// Rendering error
    RenderError(String),
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VariableNotFound(var) => {
                write!(f, "Variable '{}' not found in template context", var)
            }
            Self::InvalidPlaceholder(placeholder) => {
                write!(f, "Invalid placeholder syntax: '{}'", placeholder)
            }
            Self::CircularReference(var) => {
                write!(f, "Circular reference detected for variable '{}'", var)
            }
            Self::InvalidVariableName(name) => {
                write!(f, "Invalid variable name: '{}' (must contain only alphanumeric characters, underscores, and dots)", name)
            }
            Self::ParseError(msg) => {
                write!(f, "Template parsing error: {}", msg)
            }
            Self::RenderError(msg) => {
                write!(f, "Template rendering error: {}", msg)
            }
        }
    }
}

impl std::error::Error for TemplateError {}

/// Result type for template operations
pub type TemplateResult<T> = std::result::Result<T, TemplateError>;

impl From<regex::Error> for TemplateError {
    fn from(err: regex::Error) -> Self {
        TemplateError::ParseError(format!("Regex error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TemplateError::VariableNotFound("name".to_string());
        assert_eq!(
            format!("{}", err),
            "Variable 'name' not found in template context"
        );

        let err = TemplateError::InvalidPlaceholder("{{invalid}}".to_string());
        assert!(format!("{}", err).contains("Invalid placeholder syntax"));
    }

    #[test]
    fn test_error_debug() {
        let err = TemplateError::CircularReference("var1".to_string());
        assert!(format!("{:?}", err).contains("CircularReference"));
    }
}
