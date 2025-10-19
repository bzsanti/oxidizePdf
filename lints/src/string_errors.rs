//! Detects the use of String for errors instead of proper error types
//!
//! # Problem
//! Using `String` or `&str` for errors loses type information and makes
//! error handling and pattern matching difficult.
//!
//! # Solution
//! Use proper error types that implement `std::error::Error` and provide
//! structured error information.
//!
//! # Example
//! ```rust,ignore
//! // ❌ BAD
//! fn process() -> Result<Data, String> { }
//!
//! // ✅ GOOD
//! #[derive(Debug, thiserror::Error)]
//! enum ProcessingError {
//!     #[error("parsing failed: {0}")]
//!     ParseError(String),
//! }
//! fn process() -> Result<Data, ProcessingError> { }
//! ```

use clippy_utils::diagnostics::span_lint_and_help;
use if_chain::if_chain;
use rustc_hir::{self as hir, FnRetTy, ItemKind, QPath, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// **What it does:** Checks for functions that return `Result<T, String>` or
    /// `Result<T, &str>` instead of a proper error type.
    ///
    /// **Why is this bad?**
    /// - String errors don't provide structured information
    /// - Makes error handling and pattern matching difficult
    /// - No backtrace or source error information
    /// - Doesn't implement std::error::Error properly
    ///
    /// **Known problems:** May have false positives for intentionally simple errors.
    ///
    /// **Example:**
    /// ```rust,ignore
    /// // Bad
    /// fn parse_pdf() -> Result<Document, String> { }
    ///
    /// // Good
    /// fn parse_pdf() -> Result<Document, ParseError> { }
    /// ```
    pub STRING_ERRORS,
    Warn,
    "using String or &str for errors instead of proper error types"
}

declare_lint_pass!(StringErrors => [STRING_ERRORS]);

impl<'tcx> LateLintPass<'tcx> for StringErrors {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        // Check function signatures
        if let ItemKind::Fn { sig, .. } = &item.kind {
            let fn_name = cx.tcx.item_name(item.owner_id.to_def_id());
            check_fn_return_type(cx, fn_name.as_str(), &sig.decl.output, item.span);
        }
    }

    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::ImplItem<'tcx>) {
        // Check method signatures in impl blocks
        if let hir::ImplItemKind::Fn(sig, _) = &item.kind {
            check_fn_return_type(cx, item.ident.name.as_str(), &sig.decl.output, item.span);
        }
    }

    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::TraitItem<'tcx>) {
        // Check trait method signatures
        if let hir::TraitItemKind::Fn(sig, _) = &item.kind {
            check_fn_return_type(cx, item.ident.name.as_str(), &sig.decl.output, item.span);
        }
    }
}

fn check_fn_return_type(
    cx: &LateContext<'_>,
    fn_name: &str,
    ret_ty: &FnRetTy<'_>,
    span: rustc_span::Span,
) {
    if_chain! {
        if let FnRetTy::Return(ty) = ret_ty;
        if let TyKind::Path(QPath::Resolved(_, path)) = &ty.kind;

        // Check if this is a Result type
        if let Some(segment) = path.segments.last();
        if segment.ident.name.as_str() == "Result";

        // Check the error type (second generic argument)
        if let Some(args) = segment.args;
        if args.args.len() == 2;

        then {
            if let Some(error_ty) = args.args.get(1) {
                if let hir::GenericArg::Type(error_ty_hir) = error_ty {
                    let is_string_error = match &error_ty_hir.kind {
                        TyKind::Path(QPath::Resolved(_, error_path)) => {
                            let error_type_name = error_path.segments.last()
                                .map(|s| s.ident.name.as_str())
                                .unwrap_or("");

                            error_type_name == "String"
                        }
                        TyKind::Ref(_, inner_ty) => {
                            // Check for &str
                            matches!(inner_ty.ty.kind, TyKind::Path(QPath::Resolved(_, p))
                                if p.segments.last()
                                    .map(|s| s.ident.name.as_str())
                                    .unwrap_or("") == "str")
                        }
                        _ => false,
                    };

                    if is_string_error {
                        span_lint_and_help(
                            cx,
                            STRING_ERRORS,
                            span,
                            format!(
                                "function `{}` returns Result with String/&str error type",
                                fn_name
                            ),
                            None,
                            "consider using a proper error type that implements std::error::Error. \
                             Use thiserror or define a custom error enum with context information."
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
        assert_eq!(STRING_ERRORS.name, "string_errors");
    }
}
