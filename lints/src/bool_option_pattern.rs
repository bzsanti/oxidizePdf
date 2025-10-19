//! Detects the anti-pattern of using `bool` success field with `Option<Error>`
//!
//! # Problem
//! Using a boolean success field alongside an optional error field allows impossible states:
//! - `success: true` with `error: Some(...)`
//! - `success: false` with `error: None`
//!
//! # Solution
//! Use `Result<T, E>` which enforces mutual exclusivity of success and error states.
//!
//! # Example
//! ```rust,ignore
//! // ❌ BAD
//! struct ProcessingResult {
//!     success: bool,
//!     error: Option<String>,
//!     data: Option<Data>,
//! }
//!
//! // ✅ GOOD
//! type ProcessingResult = Result<Data, ProcessingError>;
//! ```

use clippy_utils::diagnostics::span_lint_and_help;
use if_chain::if_chain;
use rustc_hir::{self as hir, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// **What it does:** Checks for structs that have both a `bool` success field
    /// and an `Option<Error>` field, which allows impossible states.
    ///
    /// **Why is this bad?** This pattern allows invalid states like:
    /// - `success: true` with an error message
    /// - `success: false` without an error message
    ///
    /// Using `Result<T, E>` enforces mutual exclusivity and is the idiomatic Rust way.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// ```rust,ignore
    /// // Bad
    /// struct ProcessingResult {
    ///     success: bool,
    ///     error: Option<String>,
    /// }
    ///
    /// // Good
    /// type ProcessingResult = Result<ProcessingData, ProcessingError>;
    /// ```
    pub BOOL_OPTION_PATTERN,
    Warn,
    "using bool success field with Option error instead of Result<T, E>"
}

declare_lint_pass!(BoolOptionPattern => [BOOL_OPTION_PATTERN]);

impl<'tcx> LateLintPass<'tcx> for BoolOptionPattern {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        if_chain! {
            // Check if this is a struct definition
            if let ItemKind::Struct(_, _, variant_data) = &item.kind;

            then {
                // Get the fields
                let fields = variant_data.fields();

                let mut has_bool_success = false;
                let mut has_option_error = false;
                let mut success_field_span = None;
                let mut error_field_span = None;

                // Check each field
                for field in fields {
                    let field_name = field.ident.name.as_str();

                    // Use HIR type path instead of typeck_results
                    if let hir::TyKind::Path(ref qpath) = field.ty.kind {
                        // Check for bool field named "success"
                        if field_name == "success" {
                            if let hir::QPath::Resolved(_, path) = qpath {
                                if let Some(segment) = path.segments.last() {
                                    if segment.ident.name.as_str() == "bool" {
                                        has_bool_success = true;
                                        success_field_span = Some(field.span);
                                    }
                                }
                            }
                        }

                        // Check for Option<Error> or Option<String> field named "error"
                        if field_name.contains("error") {
                            if let hir::QPath::Resolved(_, path) = qpath {
                                if let Some(segment) = path.segments.last() {
                                    if segment.ident.name.as_str() == "Option" {
                                        has_option_error = true;
                                        error_field_span = Some(field.span);
                                    }
                                }
                            }
                        }
                    }
                }

                // If we found both patterns, emit the lint
                if has_bool_success && has_option_error {
                    let span = success_field_span.unwrap().to(error_field_span.unwrap());

                    // Get struct name from DefId
                    let struct_name = cx.tcx.item_name(item.owner_id.to_def_id());

                    span_lint_and_help(
                        cx,
                        BOOL_OPTION_PATTERN,
                        span,
                        format!(
                            "struct `{}` uses bool success field with Option error field",
                            struct_name
                        ),
                        None,
                        "consider using `Result<T, E>` instead, which enforces mutual exclusivity \
                         and is the idiomatic Rust way to represent success/failure"
                    );
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
        assert_eq!(BOOL_OPTION_PATTERN.name, "bool_option_pattern");
    }
}
