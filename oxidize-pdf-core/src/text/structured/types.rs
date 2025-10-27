//! Core data types for structured data extraction.

use serde::{Deserialize, Serialize};

/// Bounding box for spatial positioning.
///
/// Coordinates are in PDF user space units (typically 1/72 inch).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    /// X coordinate of bottom-left corner
    pub x: f64,
    /// Y coordinate of bottom-left corner
    pub y: f64,
    /// Width of the bounding box
    pub width: f64,
    /// Height of the bounding box
    pub height: f64,
}

impl BoundingBox {
    /// Creates a new bounding box.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns the right edge X coordinate.
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// Returns the top edge Y coordinate.
    pub fn top(&self) -> f64 {
        self.y + self.height
    }

    /// Checks if this bounding box contains a point.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.right() && y >= self.y && y <= self.top()
    }
}

/// A detected table structure.
///
/// Tables are detected by analyzing vertical and horizontal text alignment
/// using clustering algorithms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    /// Table rows
    pub rows: Vec<Row>,
    /// Column definitions
    pub columns: Vec<Column>,
    /// Spatial extent of the entire table
    pub bounding_box: BoundingBox,
    /// Detection confidence score (0.0 to 1.0)
    ///
    /// Higher scores indicate more regular alignment and structure.
    pub confidence: f64,
}

impl Table {
    /// Creates a new table.
    pub fn new(
        rows: Vec<Row>,
        columns: Vec<Column>,
        bounding_box: BoundingBox,
        confidence: f64,
    ) -> Self {
        Self {
            rows,
            columns,
            bounding_box,
            confidence,
        }
    }

    /// Returns the number of rows.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Returns the number of columns.
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// Gets a cell at the specified row and column index.
    pub fn get_cell(&self, row_idx: usize, col_idx: usize) -> Option<&Cell> {
        self.rows.get(row_idx)?.cells.get(col_idx)
    }
}

/// A single row in a table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    /// Cells in this row
    pub cells: Vec<Cell>,
    /// Y position of the row baseline
    pub y_position: f64,
    /// Height of the row
    pub height: f64,
}

impl Row {
    /// Creates a new row.
    pub fn new(cells: Vec<Cell>, y_position: f64, height: f64) -> Self {
        Self {
            cells,
            y_position,
            height,
        }
    }
}

/// A single cell in a table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    /// Text content of the cell
    pub text: String,
    /// Column index (0-based)
    pub column_index: usize,
    /// Spatial extent of the cell
    pub bounding_box: BoundingBox,
}

impl Cell {
    /// Creates a new empty cell.
    pub fn new(column_index: usize, bounding_box: BoundingBox) -> Self {
        Self {
            text: String::new(),
            column_index,
            bounding_box,
        }
    }

    /// Adds text to this cell.
    pub fn add_text(&mut self, text: &str) {
        if !self.text.is_empty() {
            self.text.push(' ');
        }
        self.text.push_str(text);
    }

    /// Checks if the cell is empty.
    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }
}

/// Column definition in a table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    /// X position of the column center
    pub x_position: f64,
    /// Width of the column
    pub width: f64,
    /// Detected text alignment
    pub alignment: Alignment,
}

impl Column {
    /// Creates a new column.
    pub fn new(x_position: f64, width: f64, alignment: Alignment) -> Self {
        Self {
            x_position,
            width,
            alignment,
        }
    }

    /// Returns the left edge of the column.
    pub fn left(&self) -> f64 {
        self.x_position - self.width / 2.0
    }

    /// Returns the right edge of the column.
    pub fn right(&self) -> f64 {
        self.x_position + self.width / 2.0
    }
}

/// Text alignment within a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alignment {
    /// Left-aligned text
    Left,
    /// Right-aligned text
    Right,
    /// Center-aligned text
    Center,
    /// Justified text
    Justified,
}

impl Default for Alignment {
    fn default() -> Self {
        Alignment::Left
    }
}

/// A detected key-value pair.
///
/// Key-value pairs are detected using multiple pattern matching strategies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValuePair {
    /// The key (label) text
    pub key: String,
    /// The value text
    pub value: String,
    /// Detection confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// The pattern used to detect this pair
    pub pattern: KeyValuePattern,
}

impl KeyValuePair {
    /// Creates a new key-value pair.
    pub fn new(key: String, value: String, confidence: f64, pattern: KeyValuePattern) -> Self {
        Self {
            key,
            value,
            confidence,
            pattern,
        }
    }
}

/// Pattern used to detect a key-value pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyValuePattern {
    /// Colon-separated format: "Label: Value"
    ColonSeparated,
    /// Spatially aligned format: "Label      Value"
    SpatialAlignment,
    /// Tab-separated format: "Label\tValue"
    Tabular,
}

/// Configuration for structured data detection.
#[derive(Debug, Clone)]
pub struct StructuredDataConfig {
    /// Minimum number of rows to consider something a table
    pub min_table_rows: usize,
    /// Minimum number of columns to consider something a table
    pub min_table_columns: usize,
    /// Tolerance for column alignment (in PDF units)
    ///
    /// Text fragments within this distance are considered aligned.
    pub column_alignment_tolerance: f64,
    /// Tolerance for row alignment (in PDF units)
    pub row_alignment_tolerance: f64,
    /// Enable table detection
    pub detect_tables: bool,
    /// Enable key-value pair detection
    pub detect_key_value: bool,
    /// Enable multi-column layout detection
    pub detect_multi_column: bool,
    /// Minimum horizontal gap to consider a column boundary (in PDF units)
    pub min_column_gap: f64,
}

impl Default for StructuredDataConfig {
    fn default() -> Self {
        Self {
            min_table_rows: 2,
            min_table_columns: 2,
            column_alignment_tolerance: 5.0,
            row_alignment_tolerance: 3.0,
            detect_tables: true,
            detect_key_value: true,
            detect_multi_column: true,
            min_column_gap: 20.0,
        }
    }
}

impl StructuredDataConfig {
    /// Creates a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the minimum number of rows for table detection.
    pub fn with_min_table_rows(mut self, rows: usize) -> Self {
        self.min_table_rows = rows;
        self
    }

    /// Sets the minimum number of columns for table detection.
    pub fn with_min_table_columns(mut self, columns: usize) -> Self {
        self.min_table_columns = columns;
        self
    }

    /// Sets the column alignment tolerance.
    pub fn with_column_tolerance(mut self, tolerance: f64) -> Self {
        self.column_alignment_tolerance = tolerance;
        self
    }

    /// Sets the row alignment tolerance.
    pub fn with_row_tolerance(mut self, tolerance: f64) -> Self {
        self.row_alignment_tolerance = tolerance;
        self
    }

    /// Enables or disables table detection.
    pub fn with_table_detection(mut self, enabled: bool) -> Self {
        self.detect_tables = enabled;
        self
    }

    /// Enables or disables key-value pair detection.
    pub fn with_key_value_detection(mut self, enabled: bool) -> Self {
        self.detect_key_value = enabled;
        self
    }

    /// Enables or disables multi-column layout detection.
    pub fn with_multi_column_detection(mut self, enabled: bool) -> Self {
        self.detect_multi_column = enabled;
        self
    }
}

/// Multi-column layout boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnBoundary {
    /// X position of the boundary
    pub x_position: f64,
    /// Width of the gap at this boundary
    pub gap_width: f64,
}

impl ColumnBoundary {
    /// Creates a new column boundary.
    pub fn new(x_position: f64, gap_width: f64) -> Self {
        Self {
            x_position,
            gap_width,
        }
    }
}

/// A section of text in a multi-column layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSection {
    /// Column index (0-based, left to right)
    pub column_index: usize,
    /// Text content in reading order
    pub text: String,
    /// Spatial extent of this column section
    pub bounding_box: BoundingBox,
}

impl ColumnSection {
    /// Creates a new column section.
    pub fn new(column_index: usize, text: String, bounding_box: BoundingBox) -> Self {
        Self {
            column_index,
            text,
            bounding_box,
        }
    }
}

/// Result of structured data detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredDataResult {
    /// Detected tables
    pub tables: Vec<Table>,
    /// Detected key-value pairs
    pub key_value_pairs: Vec<KeyValuePair>,
    /// Multi-column layout sections
    pub column_sections: Vec<ColumnSection>,
}

impl StructuredDataResult {
    /// Creates a new empty result.
    pub fn new() -> Self {
        Self {
            tables: Vec::new(),
            key_value_pairs: Vec::new(),
            column_sections: Vec::new(),
        }
    }
}

impl Default for StructuredDataResult {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_box_basic() {
        let bbox = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(bbox.x, 10.0);
        assert_eq!(bbox.y, 20.0);
        assert_eq!(bbox.width, 100.0);
        assert_eq!(bbox.height, 50.0);
        assert_eq!(bbox.right(), 110.0);
        assert_eq!(bbox.top(), 70.0);
    }

    #[test]
    fn test_bounding_box_contains() {
        let bbox = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
        assert!(bbox.contains(50.0, 40.0)); // inside
        assert!(bbox.contains(10.0, 20.0)); // bottom-left corner
        assert!(bbox.contains(110.0, 70.0)); // top-right corner
        assert!(!bbox.contains(5.0, 40.0)); // outside left
        assert!(!bbox.contains(120.0, 40.0)); // outside right
    }

    #[test]
    fn test_cell_operations() {
        let bbox = BoundingBox::new(0.0, 0.0, 50.0, 20.0);
        let mut cell = Cell::new(0, bbox);

        assert!(cell.is_empty());

        cell.add_text("Hello");
        assert_eq!(cell.text, "Hello");
        assert!(!cell.is_empty());

        cell.add_text("World");
        assert_eq!(cell.text, "Hello World");
    }

    #[test]
    fn test_column_edges() {
        let column = Column::new(100.0, 50.0, Alignment::Left);
        assert_eq!(column.left(), 75.0);
        assert_eq!(column.right(), 125.0);
    }

    #[test]
    fn test_table_accessors() {
        let bbox = BoundingBox::new(0.0, 0.0, 200.0, 100.0);
        let cell = Cell::new(0, BoundingBox::new(0.0, 0.0, 50.0, 25.0));
        let row = Row::new(vec![cell], 0.0, 25.0);
        let column = Column::new(25.0, 50.0, Alignment::Left);

        let table = Table::new(vec![row], vec![column], bbox, 0.95);

        assert_eq!(table.row_count(), 1);
        assert_eq!(table.column_count(), 1);
        assert!(table.get_cell(0, 0).is_some());
        assert!(table.get_cell(1, 0).is_none());
    }

    #[test]
    fn test_config_builder() {
        let config = StructuredDataConfig::new()
            .with_min_table_rows(3)
            .with_min_table_columns(4)
            .with_column_tolerance(10.0)
            .with_table_detection(false);

        assert_eq!(config.min_table_rows, 3);
        assert_eq!(config.min_table_columns, 4);
        assert_eq!(config.column_alignment_tolerance, 10.0);
        assert!(!config.detect_tables);
    }

    #[test]
    fn test_alignment_default() {
        assert_eq!(Alignment::default(), Alignment::Left);
    }
}
