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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_columns_error() {
        let err = TableError::NoColumns;
        assert_eq!(format!("{}", err), "Table must have at least one column");
    }

    #[test]
    fn test_column_mismatch_error() {
        let err = TableError::ColumnMismatch {
            row: 2,
            found: 3,
            expected: 5,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Row 2"));
        assert!(msg.contains("3 cells"));
        assert!(msg.contains("5 columns"));
    }

    #[test]
    fn test_header_out_of_bounds_error() {
        let err = TableError::HeaderOutOfBounds {
            level: 1,
            start: 3,
            span: 4,
            total: 5,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("level 1"));
        assert!(msg.contains("3 + 4 > 5"));
    }

    #[test]
    fn test_header_overlap_error() {
        let err = TableError::HeaderOverlap {
            level: 0,
            column: 2,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Overlapping"));
        assert!(msg.contains("level 0"));
        assert!(msg.contains("column 2"));
    }

    #[test]
    fn test_invalid_column_width_error() {
        let err = TableError::InvalidColumnWidth {
            column: 3,
            width: -10.5,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid column width"));
        assert!(msg.contains("-10.5"));
        assert!(msg.contains("column 3"));
        assert!(msg.contains("must be positive"));
    }

    #[test]
    fn test_invalid_column_width_zero() {
        let err = TableError::InvalidColumnWidth {
            column: 0,
            width: 0.0,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("0"));
    }

    #[test]
    fn test_invalid_row_height_error() {
        let err = TableError::InvalidRowHeight {
            row: 5,
            height: -2.0,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid row height"));
        assert!(msg.contains("-2"));
        assert!(msg.contains("row 5"));
        assert!(msg.contains("must be positive"));
    }

    #[test]
    fn test_invalid_cell_span_colspan() {
        let err = TableError::InvalidCellSpan {
            span_type: "colspan".to_string(),
            row: 1,
            col: 2,
            span: 0,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid colspan span"));
        assert!(msg.contains("row 1"));
        assert!(msg.contains("col 2"));
        assert!(msg.contains("must be at least 1"));
    }

    #[test]
    fn test_invalid_cell_span_rowspan() {
        let err = TableError::InvalidCellSpan {
            span_type: "rowspan".to_string(),
            row: 3,
            col: 4,
            span: 0,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid rowspan span"));
    }

    #[test]
    fn test_error_clone() {
        let err1 = TableError::NoColumns;
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_error_partial_eq() {
        let err1 = TableError::ColumnMismatch {
            row: 1,
            found: 2,
            expected: 3,
        };
        let err2 = TableError::ColumnMismatch {
            row: 1,
            found: 2,
            expected: 3,
        };
        let err3 = TableError::ColumnMismatch {
            row: 1,
            found: 2,
            expected: 4,
        };

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_error_debug() {
        let err = TableError::NoColumns;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NoColumns"));
    }

    #[test]
    fn test_all_variants_debug() {
        let variants: Vec<TableError> = vec![
            TableError::NoColumns,
            TableError::ColumnMismatch {
                row: 0,
                found: 1,
                expected: 2,
            },
            TableError::HeaderOutOfBounds {
                level: 0,
                start: 0,
                span: 1,
                total: 1,
            },
            TableError::HeaderOverlap {
                level: 0,
                column: 0,
            },
            TableError::InvalidColumnWidth {
                column: 0,
                width: -1.0,
            },
            TableError::InvalidRowHeight {
                row: 0,
                height: -1.0,
            },
            TableError::InvalidCellSpan {
                span_type: "test".to_string(),
                row: 0,
                col: 0,
                span: 0,
            },
        ];

        for err in variants {
            let debug_str = format!("{:?}", err);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TableError>();
    }

    #[test]
    fn test_error_is_std_error() {
        fn assert_error<T: std::error::Error>(_: &T) {}
        let err = TableError::NoColumns;
        assert_error(&err);
    }

    #[test]
    fn test_large_indices() {
        let err = TableError::ColumnMismatch {
            row: usize::MAX,
            found: usize::MAX - 1,
            expected: 100,
        };
        let msg = format!("{}", err);
        assert!(!msg.is_empty());
    }

    #[test]
    fn test_special_float_values() {
        // Test with infinity
        let err = TableError::InvalidColumnWidth {
            column: 0,
            width: f64::INFINITY,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("inf"));

        // Test with NaN
        let err = TableError::InvalidRowHeight {
            row: 0,
            height: f64::NAN,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("NaN") || msg.contains("nan"));
    }
}
