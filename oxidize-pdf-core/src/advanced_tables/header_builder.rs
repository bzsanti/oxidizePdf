//! Header builder for complex table headers with spanning support

use super::cell_style::CellStyle;

/// A header cell that can span multiple columns and rows
#[derive(Debug, Clone)]
pub struct HeaderCell {
    /// Header text
    pub text: String,
    /// Number of columns this header spans
    pub colspan: usize,
    /// Number of rows this header spans (for multi-level headers)
    pub rowspan: usize,
    /// Custom style for this header cell
    pub style: Option<CellStyle>,
    /// Column index where this header starts
    pub start_col: usize,
    /// Row level (0 = top level)
    pub row_level: usize,
}

impl HeaderCell {
    /// Create a simple header cell
    pub fn new<S: Into<String>>(text: S) -> Self {
        Self {
            text: text.into(),
            colspan: 1,
            rowspan: 1,
            style: None,
            start_col: 0,
            row_level: 0,
        }
    }

    /// Set column span
    pub fn colspan(mut self, span: usize) -> Self {
        self.colspan = span.max(1);
        self
    }

    /// Set row span
    pub fn rowspan(mut self, span: usize) -> Self {
        self.rowspan = span.max(1);
        self
    }

    /// Set custom style
    pub fn style(mut self, style: CellStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Set starting column
    pub fn start_col(mut self, col: usize) -> Self {
        self.start_col = col;
        self
    }

    /// Set row level
    pub fn row_level(mut self, level: usize) -> Self {
        self.row_level = level;
        self
    }
}

/// Builder for creating complex multi-level table headers
#[derive(Debug, Clone)]
pub struct HeaderBuilder {
    /// All header cells organized by levels
    pub levels: Vec<Vec<HeaderCell>>,
    /// Total number of columns in the table
    pub total_columns: usize,
    /// Default style for headers
    pub default_style: CellStyle,
}

impl HeaderBuilder {
    /// Create a new header builder
    pub fn new(total_columns: usize) -> Self {
        Self {
            levels: Vec::new(),
            total_columns,
            default_style: CellStyle::header(),
        }
    }

    /// Create a new header builder without specifying columns (for compatibility with tests)
    pub fn auto() -> Self {
        Self::new(0) // Will be calculated automatically
    }

    /// Add a header level with (text, colspan) pairs
    pub fn add_level(mut self, headers: Vec<(&str, usize)>) -> Self {
        let cells: Vec<HeaderCell> = headers
            .into_iter()
            .scan(0, |start_col, (text, colspan)| {
                let cell = HeaderCell::new(text)
                    .colspan(colspan)
                    .start_col(*start_col)
                    .row_level(self.levels.len());
                *start_col += colspan;
                Some(cell)
            })
            .collect();

        // Auto-calculate total columns if not set
        if self.total_columns == 0 {
            self.total_columns = cells.iter().map(|c| c.colspan).sum();
        }

        self.levels.push(cells);
        self
    }

    /// Set default header style
    pub fn default_style(mut self, style: CellStyle) -> Self {
        self.default_style = style;
        self
    }

    /// Add a simple single-level header row
    pub fn add_simple_row(mut self, headers: Vec<&str>) -> Self {
        let cells: Vec<HeaderCell> = headers
            .into_iter()
            .enumerate()
            .map(|(i, text)| {
                HeaderCell::new(text)
                    .start_col(i)
                    .row_level(self.levels.len())
            })
            .collect();

        self.levels.push(cells);
        self
    }

    /// Add a header row with custom cells
    pub fn add_custom_row(mut self, cells: Vec<HeaderCell>) -> Self {
        let level = self.levels.len();
        let updated_cells: Vec<HeaderCell> = cells
            .into_iter()
            .map(|mut cell| {
                cell.row_level = level;
                cell
            })
            .collect();

        self.levels.push(updated_cells);
        self
    }

    /// Add a grouped header (spans multiple columns) with sub-headers
    ///
    /// Example: "Sales Data" spanning 3 columns with sub-headers "Q1", "Q2", "Q3"
    pub fn add_group(mut self, group_header: &str, sub_headers: Vec<&str>) -> Self {
        let group_colspan = sub_headers.len();
        let start_col = self.calculate_next_start_col();

        // Add the group header at current level
        let group_level = self.levels.len();
        if self.levels.len() == group_level {
            self.levels.push(Vec::new());
        }

        let group_cell = HeaderCell::new(group_header)
            .colspan(group_colspan)
            .start_col(start_col)
            .row_level(group_level);

        self.levels[group_level].push(group_cell);

        // Add sub-headers at the next level
        let sub_level = group_level + 1;
        if self.levels.len() <= sub_level {
            self.levels.push(Vec::new());
        }

        for (i, sub_header) in sub_headers.into_iter().enumerate() {
            let sub_cell = HeaderCell::new(sub_header)
                .start_col(start_col + i)
                .row_level(sub_level);

            self.levels[sub_level].push(sub_cell);
        }

        self
    }

    /// Add a complex header structure with manual positioning
    pub fn add_complex_header(
        mut self,
        text: &str,
        start_col: usize,
        colspan: usize,
        rowspan: usize,
    ) -> Self {
        let level = self.levels.len();
        if self.levels.is_empty() {
            self.levels.push(Vec::new());
        }

        let cell = HeaderCell::new(text)
            .start_col(start_col)
            .colspan(colspan)
            .rowspan(rowspan)
            .row_level(level);

        self.levels.last_mut().unwrap().push(cell);
        self
    }

    /// Calculate the next available starting column
    fn calculate_next_start_col(&self) -> usize {
        if let Some(last_level) = self.levels.last() {
            last_level
                .iter()
                .map(|cell| cell.start_col + cell.colspan)
                .max()
                .unwrap_or(0)
        } else {
            0
        }
    }

    /// Get the total number of header rows
    pub fn row_count(&self) -> usize {
        self.levels.len()
    }

    /// Get the height needed for headers (in points, assuming default font size)
    pub fn calculate_height(&self) -> f64 {
        // Assume each header row needs 20 points by default
        let base_height = 20.0;
        let row_count = self.row_count() as f64;

        // Add some padding between levels
        let padding = if row_count > 1.0 {
            (row_count - 1.0) * 5.0
        } else {
            0.0
        };

        row_count * base_height + padding
    }

    /// Validate the header structure
    pub fn validate(&self) -> Result<(), String> {
        for (level_idx, level) in self.levels.iter().enumerate() {
            let mut column_coverage = vec![false; self.total_columns];

            for cell in level {
                // Check if cell extends beyond table width
                if cell.start_col + cell.colspan > self.total_columns {
                    return Err(format!(
                        "Header cell at level {} extends beyond table width ({} + {} > {})",
                        level_idx, cell.start_col, cell.colspan, self.total_columns
                    ));
                }

                // Check for overlapping cells
                for (col, coverage) in column_coverage
                    .iter_mut()
                    .enumerate()
                    .skip(cell.start_col)
                    .take(cell.colspan)
                {
                    if *coverage {
                        return Err(format!(
                            "Overlapping header cells at level {} column {}",
                            level_idx, col
                        ));
                    }
                    *coverage = true;
                }
            }
        }

        Ok(())
    }

    /// Get all cells that should be rendered at a specific position
    pub fn get_cells_at_position(&self, level: usize, col: usize) -> Vec<&HeaderCell> {
        if level >= self.levels.len() {
            return Vec::new();
        }

        self.levels[level]
            .iter()
            .filter(|cell| col >= cell.start_col && col < (cell.start_col + cell.colspan))
            .collect()
    }

    /// Create a financial report header
    pub fn financial_report() -> Self {
        Self::new(6)
            .default_style(
                CellStyle::header().background_color(crate::graphics::Color::rgb(0.2, 0.4, 0.8)),
            )
            .add_group("Q1 2024", vec!["Revenue", "Expenses"])
            .add_group("Q2 2024", vec!["Revenue", "Expenses"])
            .add_group("Total", vec!["Revenue", "Expenses"])
    }

    /// Create a product comparison header
    pub fn product_comparison(products: Vec<&str>) -> Self {
        let total_cols = 1 + products.len(); // Feature column + product columns
        let mut builder = Self::new(total_cols).default_style(CellStyle::header());

        // Add "Features" as first column
        builder = builder.add_complex_header("Features", 0, 1, 2);

        // Add product group header
        builder = builder.add_complex_header("Products", 1, products.len(), 1);

        // Add individual product headers
        for (i, product) in products.into_iter().enumerate() {
            builder = builder.add_complex_header(product, i + 1, 1, 1);
        }

        builder
    }
}

impl Default for HeaderBuilder {
    fn default() -> Self {
        Self::new(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_header() {
        let header = HeaderBuilder::new(3).add_simple_row(vec!["Name", "Age", "Department"]);

        assert_eq!(header.row_count(), 1);
        assert_eq!(header.levels[0].len(), 3);
        assert_eq!(header.levels[0][0].text, "Name");
        assert_eq!(header.levels[0][1].text, "Age");
        assert_eq!(header.levels[0][2].text, "Department");
    }

    #[test]
    fn test_grouped_header() {
        let header = HeaderBuilder::new(4)
            .add_group("Personal Info", vec!["Name", "Age"])
            .add_group("Work Info", vec!["Department", "Salary"]);

        assert_eq!(header.row_count(), 4); // Two groups, each creates group + sub levels
        assert!(header.validate().is_ok());
    }

    #[test]
    fn test_header_validation() {
        let header = HeaderBuilder::new(2).add_complex_header("Too Wide", 0, 3, 1); // Spans 3 columns but table only has 2

        assert!(header.validate().is_err());
    }

    #[test]
    fn test_financial_header() {
        let header = HeaderBuilder::financial_report();
        assert!(header.validate().is_ok());
        assert_eq!(header.total_columns, 6);
    }
}
