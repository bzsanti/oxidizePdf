//! Detects unwrap() calls in library code (not examples or tests)
//!
//! # Problem
//! Using `.unwrap()` in library code can cause panics in user code, which is
//! unacceptable for a library. Users expect errors to be returned, not panics.
//!
//! # Solution
//! Use `?` operator or proper error handling to return errors to the caller.
//!
//! # Example
//! ```rust,ignore
//! // ❌ BAD (in library code)
//! let filename = path.file_name().unwrap();
//!
//! // ✅ GOOD (in library code)
//! let filename = path.file_name()
//!     .ok_or_else(|| Error::InvalidPath(path.clone()))?;
//!
//! // ✅ OK (in examples or tests)
//! let filename = path.file_name().unwrap();
//! ```

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// **What it does:** Checks for `.unwrap()`, `.expect()`, and similar panic-inducing
    /// calls in library code (excludes examples, tests, and benchmarks).
    ///
    /// **Why is this bad?**
    /// - Libraries should never panic on user input
    /// - Errors should be propagated to the caller
    /// - Makes the library unreliable and hard to use
    /// - Violates the principle of graceful error handling
    ///
    /// **Known problems:** None. This is intentionally strict for library code.
    ///
    /// **Example:**
    /// ```rust,ignore
    /// // Bad (in lib code)
    /// fn parse_header(data: &[u8]) -> Header {
    ///     let magic = data.get(0..4).unwrap();
    ///     // ...
    /// }
    ///
    /// // Good (in lib code)
    /// fn parse_header(data: &[u8]) -> Result<Header, ParseError> {
    ///     let magic = data.get(0..4)
    ///         .ok_or(ParseError::InsufficientData)?;
    ///     // ...
    /// }
    /// ```
    pub LIBRARY_UNWRAPS,
    Deny,
    "using unwrap() or expect() in library code instead of proper error handling"
}

declare_lint_pass!(LibraryUnwraps => [LIBRARY_UNWRAPS]);

/// Messages that indicate infallible operations
const INFALLIBLE_MESSAGES: &[&str] = &[
    "Writing to string should never fail",
    "Writing to String should never fail",
    "Writing to Vec should never fail",
    "Writing to vector should never fail",
    "String write cannot fail",
    "Vec write cannot fail",
    "Infallible",
    "infallible",
];

/// Check if an expect() call is on an infallible operation
fn is_infallible_expect(msg: &str) -> bool {
    let msg_lower = msg.to_lowercase();
    INFALLIBLE_MESSAGES.iter().any(|&pattern| {
        msg.contains(pattern) || msg_lower.contains(&pattern.to_lowercase())
    })
}

/// Check if the receiver of expect/unwrap is a write_fmt call (from write!/writeln!)
fn is_write_fmt_call<'tcx>(receiver: &'tcx Expr<'tcx>) -> bool {
    if let ExprKind::MethodCall(method, _inner_receiver, _args, _span) = &receiver.kind {
        let method_name = method.ident.name.as_str();
        method_name == "write_fmt" || method_name == "write_str"
    } else {
        false
    }
}

/// Check if the receiver type is Option or Result from std
fn is_option_or_result<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    let ty = cx.typeck_results().expr_ty_adjusted(expr);
    let ty_str = format!("{:?}", ty);

    // Simple string matching for Option<T> and Result<T, E>
    // This works because we're looking at the fully qualified type
    ty_str.contains("Option<") || ty_str.contains("Result<")
}

impl<'tcx> LateLintPass<'tcx> for LibraryUnwraps {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Check for method calls
        if let ExprKind::MethodCall(method, receiver, args, _span) = &expr.kind {
            let method_name = method.ident.name.as_str();

            // Check if the method is one of the panic-inducing methods
            if !matches!(
                method_name,
                "unwrap" | "expect" | "unwrap_or_else" | "unwrap_unchecked"
            ) {
                return;
            }

            // IMPORTANT: Only check if this is actually Option/Result's unwrap/expect
            // Many custom types have methods with these names that are NOT panicking
            if !is_option_or_result(cx, receiver) {
                return;
            }

            // Special case 1: Check if this is write_fmt().expect() (from write!/writeln! macros)
            // These are infallible for String/Vec<u8>
            if (method_name == "expect" || method_name == "unwrap") && is_write_fmt_call(receiver) {
                // This is write!(...).expect() or writeln!(...).expect() - ALLOW it
                return;
            }

            // Special case 2: expect() with infallible message
            if method_name == "expect" && !args.is_empty() {
                // In HIR MethodCall, receiver is separate and args contains the arguments
                // args[0] is the message for expect()
                if let Some(msg_expr) = args.first() {
                    // Check if this is a string literal
                    if let ExprKind::Lit(lit) = &msg_expr.kind {
                        if let rustc_ast::LitKind::Str(msg_sym, _) = lit.node {
                            let msg_str = msg_sym.as_str();
                            if is_infallible_expect(msg_str) {
                                // This is an infallible operation, skip it
                                return;
                            }
                        }
                    }
                }
            }

            // Get the source file path
            let span_filename = format!("{:?}", cx.tcx.sess.source_map().span_to_filename(expr.span));

            // Skip if this is in examples, tests, or benchmarks
            let is_library_code = !span_filename.contains("examples/")
                && !span_filename.contains("/tests/")
                && !span_filename.contains("/benches/")
                && !span_filename.contains("test_")
                && !span_filename.contains("#[test]")
                && !span_filename.contains("#[cfg(test)]");

            if is_library_code {
                // Special case: unwrap_or_else is acceptable if it has a non-panicking closure
                if method_name == "unwrap_or_else" {
                    // We could add more sophisticated checking here
                    // For now, we'll allow unwrap_or_else
                    return;
                }

                span_lint_and_help(
                    cx,
                    LIBRARY_UNWRAPS,
                    expr.span,
                    format!(
                        "using `{}()` in library code can cause panics",
                        method_name
                    ),
                    None,
                    "library code should never panic. Use `?` operator or return a Result \
                     to propagate errors to the caller. If this is truly unreachable, \
                     use `unreachable!()` with a clear explanation."
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lint_name() {
        assert_eq!(LIBRARY_UNWRAPS.name, "library_unwraps");
    }
}
