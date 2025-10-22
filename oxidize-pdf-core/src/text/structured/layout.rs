//! Multi-column layout detection using gap analysis.

use super::types::{BoundingBox, ColumnBoundary, ColumnSection, StructuredDataConfig};
use crate::text::extraction::TextFragment;

/// Detects multi-column layouts by identifying vertical gaps.
///
/// Algorithm:
/// 1. Analyze X positions to find significant vertical gaps
/// 2. Gaps wider than `min_column_gap` are considered column boundaries
/// 3. Assign text fragments to columns based on boundaries
/// 4. Return column sections in reading order
pub fn detect_column_layout(
    fragments: &[TextFragment],
    config: &StructuredDataConfig,
) -> Vec<ColumnSection> {
    if fragments.is_empty() {
        return vec![];
    }

    // Detect column boundaries
    let boundaries = detect_column_boundaries(fragments, config.min_column_gap);

    if boundaries.is_empty() {
        // Single column layout - return all text as one section
        return vec![create_single_column_section(fragments)];
    }

    // Assign fragments to columns
    assign_to_columns(fragments, &boundaries)
}

/// Detects vertical gaps that indicate column boundaries.
///
/// Algorithm:
/// 1. Sort all X positions
/// 2. Find gaps between consecutive positions
/// 3. Gaps wider than threshold are column boundaries
fn detect_column_boundaries(fragments: &[TextFragment], min_gap: f64) -> Vec<ColumnBoundary> {
    if fragments.is_empty() {
        return vec![];
    }

    // Collect all X ranges (start and end of each fragment)
    let mut x_ranges: Vec<(f64, f64)> = fragments.iter().map(|f| (f.x, f.x + f.width)).collect();

    x_ranges.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .expect("f64 coordinates extracted from PDF are never NaN")
    });

    let mut boundaries = Vec::new();

    // Find gaps between consecutive fragments
    for i in 0..x_ranges.len() - 1 {
        let current_end = x_ranges[i].1;
        let next_start = x_ranges[i + 1].0;
        let gap = next_start - current_end;

        if gap >= min_gap {
            // Found a column boundary
            let boundary_x = current_end + gap / 2.0; // Middle of the gap
            boundaries.push(ColumnBoundary::new(boundary_x, gap));
        }
    }

    boundaries
}

/// Assigns text fragments to columns based on boundaries.
fn assign_to_columns(
    fragments: &[TextFragment],
    boundaries: &[ColumnBoundary],
) -> Vec<ColumnSection> {
    // Number of columns = number of boundaries + 1
    let num_columns = boundaries.len() + 1;

    // Initialize column sections
    let mut columns: Vec<Vec<&TextFragment>> = vec![vec![]; num_columns];

    // Assign each fragment to a column
    for fragment in fragments {
        let column_idx = find_column_index(fragment.x, boundaries);
        columns[column_idx].push(fragment);
    }

    // Convert to ColumnSection structs
    columns
        .into_iter()
        .enumerate()
        .filter(|(_, frags)| !frags.is_empty())
        .map(|(idx, frags)| {
            // Sort fragments in reading order (top to bottom, left to right)
            let mut sorted = frags.clone();
            sorted.sort_by(|a, b| {
                b.y.partial_cmp(&a.y)
                    .expect("f64 coordinates extracted from PDF are never NaN")
                    .then_with(|| {
                        a.x.partial_cmp(&b.x)
                            .expect("f64 coordinates extracted from PDF are never NaN")
                    })
            });

            // Concatenate text
            let text = sorted
                .iter()
                .map(|f| f.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            // Calculate bounding box
            let bbox = calculate_column_bbox(&sorted);

            ColumnSection::new(idx, text, bbox)
        })
        .collect()
}

/// Finds the column index for a given X position.
fn find_column_index(x: f64, boundaries: &[ColumnBoundary]) -> usize {
    for (idx, boundary) in boundaries.iter().enumerate() {
        if x < boundary.x_position {
            return idx;
        }
    }
    boundaries.len() // Last column
}

/// Creates a single column section from all fragments.
fn create_single_column_section(fragments: &[TextFragment]) -> ColumnSection {
    let mut sorted = fragments.to_vec();
    sorted.sort_by(|a, b| {
        b.y.partial_cmp(&a.y)
            .expect("f64 coordinates extracted from PDF are never NaN")
            .then_with(|| {
                a.x.partial_cmp(&b.x)
                    .expect("f64 coordinates extracted from PDF are never NaN")
            })
    });

    let text = sorted
        .iter()
        .map(|f| f.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    let bbox = calculate_column_bbox(&sorted.iter().collect::<Vec<_>>());

    ColumnSection::new(0, text, bbox)
}

/// Calculates the bounding box for a set of fragments.
fn calculate_column_bbox(fragments: &[&TextFragment]) -> BoundingBox {
    if fragments.is_empty() {
        return BoundingBox::new(0.0, 0.0, 0.0, 0.0);
    }

    let min_x = fragments.iter().map(|f| f.x).fold(f64::INFINITY, f64::min);

    let max_x = fragments
        .iter()
        .map(|f| f.x + f.width)
        .fold(f64::NEG_INFINITY, f64::max);

    let min_y = fragments.iter().map(|f| f.y).fold(f64::INFINITY, f64::min);

    let max_y = fragments
        .iter()
        .map(|f| f.y + f.height)
        .fold(f64::NEG_INFINITY, f64::max);

    BoundingBox::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_fragment(text: &str, x: f64, y: f64, width: f64) -> TextFragment {
        TextFragment {
            text: text.to_string(),
            x,
            y,
            width,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
        }
    }

    #[test]
    fn test_detect_no_boundaries() {
        let fragments = vec![
            create_fragment("Line 1", 100.0, 700.0, 50.0),
            create_fragment("Line 2", 100.0, 680.0, 50.0),
        ];

        let boundaries = detect_column_boundaries(&fragments, 20.0);

        assert_eq!(boundaries.len(), 0); // Single column
    }

    #[test]
    fn test_detect_two_columns() {
        let fragments = vec![
            // Left column
            create_fragment("Left 1", 100.0, 700.0, 50.0),
            create_fragment("Left 2", 100.0, 680.0, 50.0),
            // Right column (gap of 100 units)
            create_fragment("Right 1", 250.0, 700.0, 50.0),
            create_fragment("Right 2", 250.0, 680.0, 50.0),
        ];

        let boundaries = detect_column_boundaries(&fragments, 20.0);

        assert_eq!(boundaries.len(), 1); // One boundary = two columns
        assert!((boundaries[0].x_position - 200.0).abs() < 5.0); // Middle of gap
        assert!((boundaries[0].gap_width - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_detect_three_columns() {
        let fragments = vec![
            create_fragment("Col1", 100.0, 700.0, 40.0),
            create_fragment("Col2", 200.0, 700.0, 40.0),
            create_fragment("Col3", 300.0, 700.0, 40.0),
        ];

        let boundaries = detect_column_boundaries(&fragments, 20.0);

        assert_eq!(boundaries.len(), 2); // Two boundaries = three columns
    }

    #[test]
    fn test_find_column_index() {
        let boundaries = vec![
            ColumnBoundary::new(200.0, 50.0),
            ColumnBoundary::new(400.0, 50.0),
        ];

        assert_eq!(find_column_index(150.0, &boundaries), 0); // Before first boundary
        assert_eq!(find_column_index(250.0, &boundaries), 1); // Between boundaries
        assert_eq!(find_column_index(450.0, &boundaries), 2); // After last boundary
    }

    #[test]
    fn test_assign_to_columns_two_columns() {
        let fragments = vec![
            create_fragment("Left 1", 100.0, 700.0, 50.0),
            create_fragment("Left 2", 100.0, 680.0, 50.0),
            create_fragment("Right 1", 250.0, 700.0, 50.0),
            create_fragment("Right 2", 250.0, 680.0, 50.0),
        ];

        let boundaries = vec![ColumnBoundary::new(200.0, 100.0)];
        let sections = assign_to_columns(&fragments, &boundaries);

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].column_index, 0);
        assert_eq!(sections[1].column_index, 1);

        // Check text content (sorted top to bottom)
        assert!(sections[0].text.contains("Left 1"));
        assert!(sections[0].text.contains("Left 2"));
        assert!(sections[1].text.contains("Right 1"));
        assert!(sections[1].text.contains("Right 2"));
    }

    #[test]
    fn test_column_layout_single_column() {
        let config = StructuredDataConfig::default();
        let fragments = vec![
            create_fragment("Line 1", 100.0, 700.0, 50.0),
            create_fragment("Line 2", 100.0, 680.0, 50.0),
        ];

        let sections = detect_column_layout(&fragments, &config);

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].column_index, 0);
        assert!(sections[0].text.contains("Line 1"));
        assert!(sections[0].text.contains("Line 2"));
    }

    #[test]
    fn test_column_layout_two_columns() {
        let mut config = StructuredDataConfig::default();
        config.min_column_gap = 50.0;

        let fragments = vec![
            create_fragment("Left 1", 100.0, 700.0, 40.0),
            create_fragment("Left 2", 100.0, 680.0, 40.0),
            create_fragment("Right 1", 250.0, 700.0, 40.0),
            create_fragment("Right 2", 250.0, 680.0, 40.0),
        ];

        let sections = detect_column_layout(&fragments, &config);

        assert_eq!(sections.len(), 2);
        assert!(sections[0].text.contains("Left"));
        assert!(sections[1].text.contains("Right"));
    }

    #[test]
    fn test_column_layout_empty() {
        let config = StructuredDataConfig::default();
        let sections = detect_column_layout(&[], &config);

        assert_eq!(sections.len(), 0);
    }

    #[test]
    fn test_calculate_column_bbox() {
        let fragments = vec![
            create_fragment("A", 100.0, 700.0, 50.0),
            create_fragment("B", 110.0, 680.0, 60.0),
        ];

        let refs: Vec<&TextFragment> = fragments.iter().collect();
        let bbox = calculate_column_bbox(&refs);

        assert_eq!(bbox.x, 100.0); // Min X
        assert_eq!(bbox.width, 70.0); // 110 + 60 - 100
        assert_eq!(bbox.y, 680.0); // Min Y
        assert_eq!(bbox.height, 32.0); // 700 + 12 - 680
    }

    #[test]
    fn test_column_sorting_reading_order() {
        let config = StructuredDataConfig::default();
        let fragments = vec![
            create_fragment("Bottom", 100.0, 650.0, 50.0),
            create_fragment("Top", 100.0, 700.0, 50.0),
            create_fragment("Middle", 100.0, 675.0, 50.0),
        ];

        let sections = detect_column_layout(&fragments, &config);

        assert_eq!(sections.len(), 1);
        // Should be sorted top to bottom
        let words: Vec<&str> = sections[0].text.split_whitespace().collect();
        assert_eq!(words[0], "Top");
        assert_eq!(words[1], "Middle");
        assert_eq!(words[2], "Bottom");
    }
}
