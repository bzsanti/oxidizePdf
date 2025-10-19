//! Detects errors being created without proper context
//!
//! # Problem
//! Creating errors with just a string message loses important context like:
//! - File path being processed
//! - Operation being performed
//! - Timestamp
//! - Original source error
//!
//! # Solution
//! Wrap errors with context using proper error types that capture relevant information.
//!
//! # Example
//! ```rust,ignore
//! // ❌ BAD
//! return Err("parsing failed".to_string());
//!
//! // ✅ GOOD
//! return Err(ParseError {
//!     context: ErrorContext {
//!         file_path: Some(path.clone()),
//!         operation: "parse_pdf".to_string(),
//!     },
//!     source: e,
//! });
//! ```

use clippy_utils::diagnostics::span_lint_and_help;
use if_chain::if_chain;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// **What it does:** Checks for error creation using just string literals or
    /// simple to_string() conversions without proper context.
    ///
    /// **Why is this bad?**
    /// - Loses important debugging information (file path, operation, etc.)
    /// - Makes error tracking and logging difficult
    /// - No structured data for error analysis
    /// - Harder to add error context later
    ///
    /// **Known problems:** May have false positives for simple utility functions.
    ///
    /// **Example:**
    /// ```rust,ignore
    /// // Bad
    /// return Err("parsing failed".to_string());
    /// return Err(format!("invalid value: {}", x));
    ///
    /// // Good
    /// return Err(ParseError::InvalidValue {
    ///     value: x,
    ///     context: ErrorContext::new(path, "parse_value"),
    /// });
    /// ```
    pub MISSING_ERROR_CONTEXT,
    Warn,
    "creating errors without proper context information"
}

declare_lint_pass!(MissingErrorContext => [MISSING_ERROR_CONTEXT]);

impl<'tcx> LateLintPass<'tcx> for MissingErrorContext {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if_chain! {
            // Look for Err(...) calls
            if let ExprKind::Call(func, args) = &expr.kind;
            if let ExprKind::Path(QPath::Resolved(_, path)) = &func.kind;
            if let Some(segment) = path.segments.last();
            if segment.ident.name.as_str() == "Err";
            if !args.is_empty();

            then {
                let error_arg = &args[0];

                // Check if the error is a simple string literal
                let is_simple_string = match &error_arg.kind {
                    // "string literal"
                    ExprKind::Lit(lit) => {
                        matches!(lit.node, rustc_ast::LitKind::Str(..))
                    }

                    // "string".to_string()
                    ExprKind::MethodCall(method, receiver, _, _) => {
                        let method_name = method.ident.name.as_str();
                        if method_name == "to_string" || method_name == "into" {
                            // Check if receiver is a string literal
                            matches!(receiver.kind, ExprKind::Lit(_))
                        } else {
                            false
                        }
                    }

                    // format!("...")
                    ExprKind::Call(inner_func, _) => {
                        if let ExprKind::Path(QPath::Resolved(_, inner_path)) = &inner_func.kind {
                            inner_path.segments.last()
                                .map(|s| s.ident.name.as_str() == "format")
                                .unwrap_or(false)
                        } else {
                            false
                        }
                    }

                    _ => false,
                };

                if is_simple_string {
                    // Check if we're in a library crate (not examples or tests)
                    let span_filename = format!("{:?}", cx.tcx.sess.source_map()
                        .span_to_filename(expr.span));
                    let is_library_code = !span_filename.contains("examples/");

                    if is_library_code {
                        span_lint_and_help(
                            cx,
                            MISSING_ERROR_CONTEXT,
                            expr.span,
                            "error created with simple string without context",
                            None,
                            "consider using a proper error type with context fields like file_path, \
                             operation, and timestamp. Use thiserror to define structured errors."
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lint_name() {
        assert_eq!(MISSING_ERROR_CONTEXT.name, "missing_error_context");
    }
}
