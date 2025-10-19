//! Detects the use of primitive types for durations instead of std::time::Duration
//!
//! # Problem
//! Using `u64`, `f64`, or `i64` for durations loses type safety and intent:
//! - Is it milliseconds, seconds, or microseconds?
//! - No type checking prevents mixing different units
//! - Loses Duration's rich API for conversions and arithmetic
//!
//! # Solution
//! Use `std::time::Duration` which is explicit, type-safe, and has a rich API.
//!
//! # Example
//! ```rust,ignore
//! // ❌ BAD
//! struct Metrics {
//!     duration_ms: u64,
//!     elapsed_seconds: f64,
//! }
//!
//! // ✅ GOOD
//! use std::time::Duration;
//! struct Metrics {
//!     duration: Duration,
//!     elapsed: Duration,
//! }
//! ```

use clippy_utils::diagnostics::span_lint_and_help;
use if_chain::if_chain;
use rustc_hir::{self as hir, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// **What it does:** Checks for struct fields with names suggesting time duration
    /// but using primitive types (u64, f64, i64) instead of `std::time::Duration`.
    ///
    /// **Why is this bad?**
    /// - Ambiguous units (milliseconds? seconds? microseconds?)
    /// - No type safety prevents mixing different time units
    /// - Loses Duration's rich API (as_secs, as_millis, etc.)
    /// - Makes code less self-documenting
    ///
    /// **Known problems:** May have false positives for non-duration fields with
    /// similar names.
    ///
    /// **Example:**
    /// ```rust,ignore
    /// // Bad
    /// struct Metrics {
    ///     processing_time_ms: u64,
    ///     elapsed_seconds: f64,
    /// }
    ///
    /// // Good
    /// struct Metrics {
    ///     processing_time: Duration,
    ///     elapsed: Duration,
    /// }
    /// ```
    pub DURATION_PRIMITIVES,
    Warn,
    "using primitive types for durations instead of std::time::Duration"
}

declare_lint_pass!(DurationPrimitives => [DURATION_PRIMITIVES]);

impl<'tcx> LateLintPass<'tcx> for DurationPrimitives {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        if_chain! {
            // Check if this is a struct definition
            if let ItemKind::Struct(_, _, variant_data) = &item.kind;

            then {
                // Get struct name from DefId
                let struct_name = cx.tcx.item_name(item.owner_id.to_def_id());

                // Get the fields
                let fields = variant_data.fields();

                // Check each field
                for field in fields {
                    let field_name = field.ident.name.as_str();

                    // Check if field name suggests a duration
                    let suggests_duration = field_name.contains("duration")
                        || field_name.contains("elapsed")
                        || field_name.contains("timeout")
                        || field_name.ends_with("_ms")
                        || field_name.ends_with("_seconds")
                        || field_name.ends_with("_micros")
                        || field_name.ends_with("_nanos")
                        || (field_name.contains("time") && !field_name.contains("timestamp"));

                    if suggests_duration {
                        // Use HIR type path to check for primitive types
                        if let hir::TyKind::Path(ref qpath) = field.ty.kind {
                            if let hir::QPath::Resolved(_, path) = qpath {
                                if let Some(segment) = path.segments.last() {
                                    let ty_name = segment.ident.name.as_str();
                                    let is_primitive = matches!(
                                        ty_name,
                                        "u8" | "u16" | "u32" | "u64" | "u128" | "usize"
                                        | "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
                                        | "f32" | "f64"
                                    );

                                    if is_primitive {
                                        span_lint_and_help(
                                            cx,
                                            DURATION_PRIMITIVES,
                                            field.span,
                                            format!(
                                                "field `{}` in struct `{}` suggests a duration but uses primitive type `{}`",
                                                field_name, struct_name, ty_name
                                            ),
                                            None,
                                            "consider using `std::time::Duration` instead, which provides \
                                             type-safe duration handling with clear units and a rich API. \
                                             Use `.as_millis()`, `.as_secs()`, etc. when you need primitive values."
                                        );
                                    }
                                }
                            }
                        }
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
        assert_eq!(DURATION_PRIMITIVES.name, "duration_primitives");
    }
}
