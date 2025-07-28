//! Page labels for custom page numbering according to ISO 32000-1 Section 12.4.2
//!
//! Page labels allow PDFs to display custom page numbers that differ from the
//! physical page order, such as roman numerals for preface pages.

mod page_label;
mod page_label_tree;

pub use page_label::{PageLabel, PageLabelRange, PageLabelStyle};
pub use page_label_tree::{PageLabelBuilder, PageLabelTree};
