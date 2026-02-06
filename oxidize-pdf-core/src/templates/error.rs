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

    #[test]
    fn test_variable_not_found_display() {
        let err = TemplateError::VariableNotFound("username".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("username"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_invalid_placeholder_display() {
        let err = TemplateError::InvalidPlaceholder("{{bad syntax".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid placeholder syntax"));
        assert!(msg.contains("{{bad syntax"));
    }

    #[test]
    fn test_circular_reference_display() {
        let err = TemplateError::CircularReference("var_a".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Circular reference"));
        assert!(msg.contains("var_a"));
    }

    #[test]
    fn test_invalid_variable_name_display() {
        let err = TemplateError::InvalidVariableName("123invalid".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid variable name"));
        assert!(msg.contains("123invalid"));
        assert!(msg.contains("alphanumeric"));
    }

    #[test]
    fn test_parse_error_display() {
        let err = TemplateError::ParseError("Unexpected token".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Template parsing error"));
        assert!(msg.contains("Unexpected token"));
    }

    #[test]
    fn test_render_error_display() {
        let err = TemplateError::RenderError("Failed to render".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Template rendering error"));
        assert!(msg.contains("Failed to render"));
    }

    #[test]
    fn test_error_clone() {
        let err1 = TemplateError::VariableNotFound("test".to_string());
        let err2 = err1.clone();
        assert_eq!(format!("{}", err1), format!("{}", err2));
    }

    #[test]
    fn test_from_regex_error() {
        // Create an invalid regex to get a regex::Error
        let regex_result = regex::Regex::new("[invalid(");
        if let Err(regex_err) = regex_result {
            let template_err: TemplateError = regex_err.into();
            let msg = format!("{}", template_err);
            assert!(msg.contains("Regex error"));
        }
    }

    #[test]
    fn test_template_result_ok() {
        fn returns_ok() -> TemplateResult<i32> {
            Ok(42)
        }
        assert_eq!(returns_ok().unwrap(), 42);
    }

    #[test]
    fn test_template_result_err() {
        fn returns_err() -> TemplateResult<i32> {
            Err(TemplateError::ParseError("test".to_string()))
        }
        assert!(returns_err().is_err());
    }

    #[test]
    fn test_error_is_std_error() {
        // Verify it implements std::error::Error
        fn assert_error<T: std::error::Error>(_: &T) {}
        let err = TemplateError::ParseError("test".to_string());
        assert_error(&err);
    }

    #[test]
    fn test_all_variants_debug() {
        // Test Debug impl for all variants
        let variants: Vec<TemplateError> = vec![
            TemplateError::VariableNotFound("v".to_string()),
            TemplateError::InvalidPlaceholder("p".to_string()),
            TemplateError::CircularReference("c".to_string()),
            TemplateError::InvalidVariableName("n".to_string()),
            TemplateError::ParseError("pe".to_string()),
            TemplateError::RenderError("re".to_string()),
        ];

        for err in variants {
            let debug_str = format!("{:?}", err);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_empty_string_errors() {
        // Test with empty strings
        let err = TemplateError::VariableNotFound(String::new());
        assert!(format!("{}", err).contains("''"));

        let err = TemplateError::ParseError(String::new());
        assert!(format!("{}", err).contains("Template parsing error:"));
    }

    #[test]
    fn test_unicode_in_errors() {
        let err = TemplateError::VariableNotFound("变量名".to_string());
        assert!(format!("{}", err).contains("变量名"));

        let err = TemplateError::RenderError("🎯 Error".to_string());
        assert!(format!("{}", err).contains("🎯"));
    }
}
