//! Custom lints for oxidize-pdf to enforce idiomatic Rust patterns
//!
//! This crate provides custom lints using Dylint to detect anti-patterns
//! specific to the oxidize-pdf project and enforce best practices.

#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

mod bool_option_pattern;
mod duration_primitives;
mod library_unwraps;
mod missing_context;
mod string_errors;

use dylint_linting::dylint_library;

dylint_library!();

#[doc(hidden)]
#[no_mangle]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register P0 lints (critical)
    lint_store.register_lints(&[
        bool_option_pattern::BOOL_OPTION_PATTERN,
        string_errors::STRING_ERRORS,
        missing_context::MISSING_ERROR_CONTEXT,
        library_unwraps::LIBRARY_UNWRAPS,
    ]);

    lint_store.register_late_pass(|_| Box::new(bool_option_pattern::BoolOptionPattern));
    lint_store.register_late_pass(|_| Box::new(string_errors::StringErrors));
    lint_store.register_late_pass(|_| Box::new(missing_context::MissingErrorContext));
    lint_store.register_late_pass(|_| Box::new(library_unwraps::LibraryUnwraps));

    // Register P1 lints (important) - will be added later
    lint_store.register_lints(&[duration_primitives::DURATION_PRIMITIVES]);
    lint_store.register_late_pass(|_| Box::new(duration_primitives::DurationPrimitives));
}
