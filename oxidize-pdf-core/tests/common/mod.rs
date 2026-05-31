//! Shared test helpers. Each integration test that needs them declares
//! `#[path = "common/mod.rs"] mod common;` at file top.
//!
//! Because each `tests/*.rs` file becomes its own crate, items in this
//! module appear unused from the perspective of crates that don't import
//! them. Suppress `dead_code` at the module level — this is the
//! conventional pattern for `tests/common/` shared helpers in Rust.
#![allow(dead_code)]

pub mod pdf_assembler;
pub mod synthetic_pdf;

/// Count the number of /Type /Page entries in PDF bytes (excluding /Type /Pages).
pub fn count_pages(bytes: &[u8]) -> usize {
    let content = String::from_utf8_lossy(bytes);
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            (trimmed.contains("/Type /Page") || trimmed.contains("/Type/Page"))
                && !trimmed.contains("/Pages")
        })
        .count()
}
