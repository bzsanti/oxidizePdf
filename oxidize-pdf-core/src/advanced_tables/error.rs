//! Error types for advanced table building and validation

use thiserror::Error;

/// Errors that can occur during table building and validation
#[derive(Debug, Error, Clone, PartialEq)]
pub enum TableError {
    /// Table must have at least one column
    #[error("Table must have at least one column")]
    NoColumns,

    /// Row has incorrect number of cells
    #[error("Row {row} has {found} cells but expected {expected} columns")]
    ColumnMismatch {
        /// Row index (0-based)
        row: usize,
        /// Number of cells found in the row
        found: usize,
        /// Number of columns expected
        expected: usize,
    },

    /// Header cell extends beyond table width
    #[error(
        "Header cell at level {level} extends beyond table width ({start} + {span} > {total})"
    )]
    HeaderOutOfBounds {
        /// Header level (0-based)
        level: usize,
        /// Starting column index
        start: usize,
        /// Column span
        span: usize,
        /// Total table columns
        total: usize,
    },

    /// Header cells overlap
    #[error("Overlapping header cells at level {level} column {column}")]
    HeaderOverlap {
        /// Header level where overlap occurs
        level: usize,
        /// Column where overlap is detected
        column: usize,
    },

    /// Invalid column width
    #[error("Invalid column width {width} at column {column}: must be positive")]
    InvalidColumnWidth {
        /// Column index
        column: usize,
        /// Invalid width value
        width: f64,
    },

    /// Invalid row height
    #[error("Invalid row height {height} at row {row}: must be positive")]
    InvalidRowHeight {
        /// Row index
        row: usize,
        /// Invalid height value
        height: f64,
    },

    /// Cell span is invalid
    #[error("Invalid {span_type} span {span} at row {row} col {col}: must be at least 1")]
    InvalidCellSpan {
        /// Type of span (colspan/rowspan)
        span_type: String,
        /// Row index
        row: usize,
        /// Column index
        col: usize,
        /// Invalid span value
        span: usize,
    },
}
