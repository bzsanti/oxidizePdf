//! Table detection using spatial clustering algorithms.

use super::types::{Alignment, BoundingBox, Cell, Column, Row, StructuredDataConfig, Table};
use crate::text::extraction::TextFragment;

/// Detects tables in text fragments using spatial clustering.
///
/// Algorithm:
/// 1. Cluster X positions to detect columns
/// 2. Cluster Y positions to detect rows
/// 3. Filter by minimum row/column thresholds
/// 4. Assign text fragments to cells
/// 5. Calculate confidence score
pub fn detect_tables(fragments: &[TextFragment], config: &StructuredDataConfig) -> Vec<Table> {
    if fragments.is_empty() {
        return vec![];
    }

    // Extract and cluster X positions (columns)
    let x_positions: Vec<f64> = fragments.iter().map(|f| f.x).collect();
    let columns = cluster_columns(&x_positions, config.column_alignment_tolerance);

    // Extract and cluster Y positions (rows)
    let y_positions: Vec<f64> = fragments.iter().map(|f| f.y).collect();
    let row_positions = cluster_rows(&y_positions, config.row_alignment_tolerance);

    // Filter by minimum thresholds
    if row_positions.len() < config.min_table_rows || columns.len() < config.min_table_columns {
        return vec![];
    }

    // Create table structure
    let mut rows = Vec::new();
    for (row_idx, &row_y) in row_positions.iter().enumerate() {
        let mut cells = Vec::new();

        for (col_idx, column) in columns.iter().enumerate() {
            let bbox = BoundingBox::new(
                column.left(),
                row_y,
                column.width,
                estimate_row_height(&row_positions, row_idx),
            );
            cells.push(Cell::new(col_idx, bbox));
        }

        rows.push(Row::new(
            cells,
            row_y,
            estimate_row_height(&row_positions, row_idx),
        ));
    }

    // Assign fragments to cells
    for fragment in fragments {
        if let Some((row_idx, col_idx)) =
            find_cell_for_fragment(fragment, &row_positions, &columns, config)
        {
            if let Some(row) = rows.get_mut(row_idx) {
                if let Some(cell) = row.cells.get_mut(col_idx) {
                    cell.add_text(&fragment.text);
                }
            }
        }
    }

    // Calculate table bounding box
    let bbox = calculate_table_bbox(&row_positions, &columns);

    // Calculate confidence score
    let confidence = calculate_table_confidence(&rows, &columns);

    vec![Table::new(rows, columns, bbox, confidence)]
}

/// Clusters X positions to detect column boundaries.
///
/// Uses a simple clustering algorithm: positions within `tolerance` of each other
/// are grouped into the same cluster.
fn cluster_columns(x_positions: &[f64], tolerance: f64) -> Vec<Column> {
    if x_positions.is_empty() {
        return vec![];
    }

    let mut sorted = x_positions.to_vec();
    sorted.sort_by(|a, b| {
        // Use unwrap_or for f64 comparison (NaN sorts as Equal)
        a.partial_cmp(b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut clusters: Vec<Vec<f64>> = vec![vec![sorted[0]]];

    for &x in &sorted[1..] {
        // Safe: clusters guaranteed non-empty (initialized with first element above)
        if let Some(last_cluster) = clusters.last_mut() {
            // Safe: cluster guaranteed non-empty (never push empty clusters)
            if let Some(&last_value) = last_cluster.last() {
                if (x - last_value).abs() <= tolerance {
                    last_cluster.push(x);
                } else {
                    clusters.push(vec![x]);
                }
            }
        }
    }

    // Convert clusters to Column objects
    clusters
        .into_iter()
        .map(|cluster| {
            let mean_x = cluster.iter().sum::<f64>() / cluster.len() as f64;
            let width = estimate_column_width(&cluster);
            Column::new(mean_x, width, Alignment::Left)
        })
        .collect()
}

/// Clusters Y positions to detect row boundaries.
fn cluster_rows(y_positions: &[f64], tolerance: f64) -> Vec<f64> {
    if y_positions.is_empty() {
        return vec![];
    }

    let mut sorted = y_positions.to_vec();
    sorted.sort_by(|a, b| {
        // Use unwrap_or for f64 comparison (NaN sorts as Equal)
        b.partial_cmp(a)
            .unwrap_or(std::cmp::Ordering::Equal)
    }); // Sort descending (top to bottom)

    let mut clusters: Vec<Vec<f64>> = vec![vec![sorted[0]]];

    for &y in &sorted[1..] {
        // Safe: clusters guaranteed non-empty (initialized with first element above)
        if let Some(last_cluster) = clusters.last_mut() {
            // Safe: cluster guaranteed non-empty (never push empty clusters)
            if let Some(&last_value) = last_cluster.last() {
                if (y - last_value).abs() <= tolerance {
                    last_cluster.push(y);
                } else {
                    clusters.push(vec![y]);
                }
            }
        }
    }

    // Return mean Y position for each cluster
    clusters
        .into_iter()
        .map(|cluster| cluster.iter().sum::<f64>() / cluster.len() as f64)
        .collect()
}

/// Estimates the width of a column based on the spread of X positions.
fn estimate_column_width(x_values: &[f64]) -> f64 {
    if x_values.len() == 1 {
        return 50.0; // Default width for single-point columns
    }

    let min = x_values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = x_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    (max - min).max(50.0) // Minimum 50 units wide
}

/// Estimates the height of a row.
fn estimate_row_height(row_positions: &[f64], row_idx: usize) -> f64 {
    if row_idx + 1 < row_positions.len() {
        (row_positions[row_idx] - row_positions[row_idx + 1]).abs()
    } else {
        20.0 // Default height for last row
    }
}

/// Finds the cell (row, column) that contains a text fragment.
fn find_cell_for_fragment(
    fragment: &TextFragment,
    row_positions: &[f64],
    columns: &[Column],
    config: &StructuredDataConfig,
) -> Option<(usize, usize)> {
    // Find nearest row
    let row_idx = row_positions
        .iter()
        .enumerate()
        .min_by(|(_, &y1), (_, &y2)| {
            let dist1 = (fragment.y - y1).abs();
            let dist2 = (fragment.y - y2).abs();
            // Use unwrap_or for f64 comparison (NaN sorts as Equal)
            dist1
                .partial_cmp(&dist2)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(idx, &y)| {
            if (fragment.y - y).abs() <= config.row_alignment_tolerance * 2.0 {
                Some(idx)
            } else {
                None
            }
        })
        .flatten()?;

    // Find nearest column
    let col_idx = columns
        .iter()
        .enumerate()
        .min_by(|(_, col1), (_, col2)| {
            let dist1 = (fragment.x - col1.x_position).abs();
            let dist2 = (fragment.x - col2.x_position).abs();
            // Use unwrap_or for f64 comparison (NaN sorts as Equal)
            dist1
                .partial_cmp(&dist2)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(idx, col)| {
            if (fragment.x - col.x_position).abs() <= config.column_alignment_tolerance * 2.0 {
                Some(idx)
            } else {
                None
            }
        })
        .flatten()?;

    Some((row_idx, col_idx))
}

/// Calculates the bounding box of the entire table.
fn calculate_table_bbox(row_positions: &[f64], columns: &[Column]) -> BoundingBox {
    if row_positions.is_empty() || columns.is_empty() {
        return BoundingBox::new(0.0, 0.0, 0.0, 0.0);
    }

    let min_x = columns
        .iter()
        .map(|c| c.left())
        .fold(f64::INFINITY, f64::min);
    let max_x = columns
        .iter()
        .map(|c| c.right())
        .fold(f64::NEG_INFINITY, f64::max);

    // Safe: row_positions guaranteed non-empty by check above
    let max_y = row_positions.first().copied().unwrap_or(0.0);
    let min_y = row_positions.last().copied().unwrap_or(0.0);

    BoundingBox::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

/// Calculates a confidence score for table detection.
///
/// Confidence is based on:
/// - Regularity of cell population (fewer empty cells = higher confidence)
/// - Alignment consistency
/// - Number of rows and columns
fn calculate_table_confidence(rows: &[Row], columns: &[Column]) -> f64 {
    if rows.is_empty() || columns.is_empty() {
        return 0.0;
    }

    let total_cells = rows.len() * columns.len();
    let populated_cells = rows
        .iter()
        .flat_map(|row| &row.cells)
        .filter(|cell| !cell.is_empty())
        .count();

    // Base confidence on population ratio
    let population_ratio = populated_cells as f64 / total_cells as f64;

    // Bonus for larger tables (more likely to be intentional)
    let size_bonus = ((rows.len() + columns.len()) as f64 / 10.0).min(0.2);

    (population_ratio + size_bonus).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_fragment(text: &str, x: f64, y: f64) -> TextFragment {
        TextFragment {
            text: text.to_string(),
            x,
            y,
            width: 50.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
        }
    }

    #[test]
    fn test_cluster_columns_simple() {
        let x_positions = vec![100.0, 102.0, 200.0, 198.0];
        let columns = cluster_columns(&x_positions, 5.0);

        assert_eq!(columns.len(), 2);
        assert!((columns[0].x_position - 101.0).abs() < 1.0); // Mean of 100, 102
        assert!((columns[1].x_position - 199.0).abs() < 1.0); // Mean of 198, 200
    }

    #[test]
    fn test_cluster_columns_single() {
        let x_positions = vec![100.0];
        let columns = cluster_columns(&x_positions, 5.0);

        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].x_position, 100.0);
    }

    #[test]
    fn test_cluster_rows_descending() {
        let y_positions = vec![700.0, 698.0, 650.0, 652.0];
        let rows = cluster_rows(&y_positions, 5.0);

        assert_eq!(rows.len(), 2);
        assert!((rows[0] - 699.0).abs() < 1.0); // Mean of 700, 698
        assert!((rows[1] - 651.0).abs() < 1.0); // Mean of 650, 652
    }

    #[test]
    fn test_detect_simple_table() {
        let fragments = vec![
            create_fragment("A1", 100.0, 700.0),
            create_fragment("A2", 200.0, 700.0),
            create_fragment("B1", 100.0, 680.0),
            create_fragment("B2", 200.0, 680.0),
        ];

        let config = StructuredDataConfig::default();
        let tables = detect_tables(&fragments, &config);

        assert_eq!(tables.len(), 1);
        let table = &tables[0];
        assert_eq!(table.row_count(), 2);
        assert_eq!(table.column_count(), 2);
    }

    #[test]
    fn test_detect_table_with_threshold() {
        let fragments = vec![
            create_fragment("A1", 100.0, 700.0),
            create_fragment("A2", 200.0, 700.0),
        ];

        let mut config = StructuredDataConfig::default();
        config.min_table_rows = 3; // Require at least 3 rows

        let tables = detect_tables(&fragments, &config);

        assert_eq!(tables.len(), 0); // Only 1 row, below threshold
    }

    #[test]
    fn test_table_cell_assignment() {
        let fragments = vec![
            create_fragment("Header1", 100.0, 700.0),
            create_fragment("Header2", 200.0, 700.0),
            create_fragment("Data1", 100.0, 680.0),
            create_fragment("Data2", 200.0, 680.0),
        ];

        let config = StructuredDataConfig::default();
        let tables = detect_tables(&fragments, &config);

        assert_eq!(tables.len(), 1);
        let table = &tables[0];

        // Check cell contents
        assert_eq!(
            table
                .get_cell(0, 0)
                .expect("cell (0,0) should exist in 2x2 table")
                .text,
            "Header1"
        );
        assert_eq!(
            table
                .get_cell(0, 1)
                .expect("cell (0,1) should exist in 2x2 table")
                .text,
            "Header2"
        );
        assert_eq!(
            table
                .get_cell(1, 0)
                .expect("cell (1,0) should exist in 2x2 table")
                .text,
            "Data1"
        );
        assert_eq!(
            table
                .get_cell(1, 1)
                .expect("cell (1,1) should exist in 2x2 table")
                .text,
            "Data2"
        );
    }

    #[test]
    fn test_table_confidence_full() {
        let fragments = vec![
            create_fragment("A", 100.0, 700.0),
            create_fragment("B", 200.0, 700.0),
            create_fragment("C", 100.0, 680.0),
            create_fragment("D", 200.0, 680.0),
        ];

        let config = StructuredDataConfig::default();
        let tables = detect_tables(&fragments, &config);

        assert_eq!(tables.len(), 1);
        assert!(tables[0].confidence > 0.9); // All cells populated
    }

    #[test]
    fn test_table_confidence_sparse() {
        let fragments = vec![
            create_fragment("A", 100.0, 700.0),
            // Missing cell at (200, 700)
            // Missing cell at (100, 680)
            create_fragment("D", 200.0, 680.0),
        ];

        let config = StructuredDataConfig::default();
        let tables = detect_tables(&fragments, &config);

        assert_eq!(tables.len(), 1);
        assert!(tables[0].confidence < 0.8); // Only 50% populated
    }

    #[test]
    fn test_detect_empty_input() {
        let fragments = vec![];
        let config = StructuredDataConfig::default();
        let tables = detect_tables(&fragments, &config);

        assert_eq!(tables.len(), 0);
    }

    #[test]
    fn test_detect_irregular_table() {
        // 3x2 table with one empty cell
        let fragments = vec![
            create_fragment("A1", 100.0, 700.0),
            create_fragment("A2", 200.0, 700.0),
            create_fragment("A3", 300.0, 700.0),
            create_fragment("B1", 100.0, 680.0),
            // B2 missing
            create_fragment("B3", 300.0, 680.0),
        ];

        let config = StructuredDataConfig::default();
        let tables = detect_tables(&fragments, &config);

        assert_eq!(tables.len(), 1);
        let table = &tables[0];
        assert_eq!(table.row_count(), 2);
        assert_eq!(table.column_count(), 3);

        // Check that B2 cell exists but is empty
        assert!(table
            .get_cell(1, 1)
            .expect("cell (1,1) should exist in irregular table")
            .is_empty());
    }
}
