//! Structured data extraction from PDF text.
//!
//! This module provides advanced text analysis capabilities to automatically detect
//! and extract structured data patterns from PDF documents:
//!
//! - **Table Detection**: Automatically identifies tables by analyzing text alignment
//!   and spatial positioning. Uses clustering algorithms to detect rows and columns.
//! - **Key-Value Pairs**: Extracts form fields and labeled data using pattern matching
//!   for colon-separated, spatially-aligned, and tabular formats.
//! - **Multi-Column Layouts**: Detects column boundaries in multi-column text layouts
//!   like newspapers or academic papers.
//!
//! # Architecture
//!
//! ```text
//! structured/
//! ├── types.rs        - Core data types (Table, KeyValuePair, Config)
//! ├── detector.rs     - Main detection engine with builder pattern
//! ├── table.rs        - Table detection algorithm (clustering)
//! ├── keyvalue.rs     - Key-value pair detection (pattern matching)
//! └── layout.rs       - Multi-column layout detection
//! ```
//!
//! # Examples
//!
//! ```rust,no_run
//! use oxidize_pdf_core::text::structured::{StructuredDataDetector, StructuredDataConfig};
//! use oxidize_pdf_core::text::extraction::TextFragment;
//!
//! let config = StructuredDataConfig::default();
//! let detector = StructuredDataDetector::new(config);
//!
//! let fragments: Vec<TextFragment> = vec![]; // from PDF extraction
//! let result = detector.detect(&fragments)?;
//!
//! println!("Found {} tables", result.tables.len());
//! println!("Found {} key-value pairs", result.key_value_pairs.len());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod detector;
mod keyvalue;
mod layout;
mod table;
mod types;

pub use types::{
    Alignment, BoundingBox, Cell, Column, KeyValuePair, KeyValuePattern, Row, StructuredDataConfig,
    StructuredDataResult, Table,
};

pub use detector::StructuredDataDetector;
