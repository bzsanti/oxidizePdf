use crate::text::extraction::TextFragment;

/// Trait for reading order strategies.
pub trait ReadingOrder {
    /// Sort fragments in reading order (in-place).
    fn order(&self, fragments: &mut [TextFragment]);
}

/// Simple reading order: top-to-bottom, left-to-right within lines.
///
/// Fragments within `line_threshold` Y-distance of each other are treated
/// as the same line and sorted left-to-right.
pub struct SimpleReadingOrder {
    pub line_threshold: f64,
}

impl SimpleReadingOrder {
    pub fn new(line_threshold: f64) -> Self {
        Self { line_threshold }
    }
}

impl Default for SimpleReadingOrder {
    fn default() -> Self {
        Self {
            line_threshold: 5.0,
        }
    }
}

impl ReadingOrder for SimpleReadingOrder {
    fn order(&self, fragments: &mut [TextFragment]) {
        if fragments.is_empty() {
            return;
        }

        // Pre-pass: assign each fragment to a line ID using greedy clustering.
        // Sort by Y descending first to process top-to-bottom.
        let mut indexed: Vec<(usize, f64, f64)> = fragments
            .iter()
            .enumerate()
            .map(|(i, f)| (i, f.y, f.x))
            .collect();
        indexed.sort_by(|a, b| b.1.total_cmp(&a.1));

        let threshold = self.line_threshold;
        let mut line_ids = vec![0u32; fragments.len()];
        let mut line_id = 0u32;
        let mut prev_y = indexed[0].1;

        for &(idx, y, _) in &indexed {
            // Compare against previous fragment's Y (chain-based grouping).
            // Since fragments are sorted by Y descending, consecutive fragments
            // within threshold form a line. This is transitive because line_id
            // is a discrete integer assigned once.
            if (prev_y - y).abs() > threshold {
                line_id += 1;
            }
            line_ids[idx] = line_id;
            prev_y = y;
        }

        // Sort by line_id ASC (top-to-bottom), then X ASC (left-to-right).
        // This is transitive because line_id is a discrete value.
        let mut order: Vec<usize> = (0..fragments.len()).collect();
        order.sort_by(|&a, &b| {
            let line_cmp = line_ids[a].cmp(&line_ids[b]);
            if line_cmp != std::cmp::Ordering::Equal {
                line_cmp
            } else {
                fragments[a].x.total_cmp(&fragments[b].x)
            }
        });

        let reordered: Vec<TextFragment> = order.iter().map(|&i| fragments[i].clone()).collect();
        fragments.clone_from_slice(&reordered);
    }
}

/// XY-Cut recursive reading order algorithm.
///
/// Splits the page recursively by finding the largest horizontal or vertical
/// whitespace gap. This correctly handles multi-column layouts by reading
/// each column top-to-bottom before moving to the next.
///
/// Reference: Ha, Haralick, Phillips (1992) — "Recursive X-Y Cut"
pub struct XYCutReadingOrder {
    /// Minimum gap size (in points) to trigger a split.
    pub min_gap: f64,
}

impl XYCutReadingOrder {
    pub fn new(min_gap: f64) -> Self {
        Self { min_gap }
    }
}

impl Default for XYCutReadingOrder {
    fn default() -> Self {
        Self { min_gap: 20.0 }
    }
}

impl ReadingOrder for XYCutReadingOrder {
    fn order(&self, fragments: &mut [TextFragment]) {
        if fragments.len() <= 1 {
            return;
        }

        let mut result = Vec::with_capacity(fragments.len());
        let indices: Vec<usize> = (0..fragments.len()).collect();
        self.xycut_recursive(fragments, &indices, &mut result);

        // Reorder fragments according to result
        let reordered: Vec<TextFragment> = result.iter().map(|&i| fragments[i].clone()).collect();
        fragments.clone_from_slice(&reordered);
    }
}

impl XYCutReadingOrder {
    fn xycut_recursive(
        &self,
        fragments: &[TextFragment],
        indices: &[usize],
        result: &mut Vec<usize>,
    ) {
        if indices.is_empty() {
            return;
        }
        if indices.len() == 1 {
            result.push(indices[0]);
            return;
        }

        // Try vertical split (left/right columns) first — splits along X axis
        if let Some((left, right)) = self.find_vertical_split(fragments, indices) {
            self.xycut_recursive(fragments, &left, result);
            self.xycut_recursive(fragments, &right, result);
            return;
        }

        // Try horizontal split (top/bottom sections) — splits along Y axis
        if let Some((top, bottom)) = self.find_horizontal_split(fragments, indices) {
            self.xycut_recursive(fragments, &top, result);
            self.xycut_recursive(fragments, &bottom, result);
            return;
        }

        // Leaf: sort by Y desc, X asc (simple reading order)
        let mut leaf = indices.to_vec();
        leaf.sort_by(|&a, &b| {
            let y_cmp = fragments[b].y.total_cmp(&fragments[a].y);
            if y_cmp == std::cmp::Ordering::Equal {
                fragments[a].x.total_cmp(&fragments[b].x)
            } else {
                y_cmp
            }
        });
        result.extend(leaf);
    }

    /// Find the largest vertical gap (along X-axis) to split into left/right groups.
    fn find_vertical_split(
        &self,
        fragments: &[TextFragment],
        indices: &[usize],
    ) -> Option<(Vec<usize>, Vec<usize>)> {
        // Collect right edges sorted by X
        let mut edges: Vec<(f64, f64, usize)> = indices
            .iter()
            .map(|&i| (fragments[i].x, fragments[i].x + fragments[i].width, i))
            .collect();
        edges.sort_by(|a, b| a.0.total_cmp(&b.0));

        // Find largest gap between right-edge of one fragment and left-edge of next
        let mut max_gap = 0.0f64;
        let mut split_x = 0.0f64;

        // Use a sweep: track max right edge so far
        let mut max_right = edges[0].1;
        for window in edges.windows(2) {
            let current_right = max_right;
            let next_left = window[1].0;
            let gap = next_left - current_right;
            if gap > max_gap {
                max_gap = gap;
                split_x = current_right + gap / 2.0;
            }
            max_right = max_right.max(window[1].1);
        }

        if max_gap < self.min_gap {
            return None;
        }

        let left: Vec<usize> = indices
            .iter()
            .filter(|&&i| fragments[i].x + fragments[i].width / 2.0 < split_x)
            .copied()
            .collect();
        let right: Vec<usize> = indices
            .iter()
            .filter(|&&i| fragments[i].x + fragments[i].width / 2.0 >= split_x)
            .copied()
            .collect();

        if left.is_empty() || right.is_empty() {
            return None;
        }

        Some((left, right))
    }

    /// Find the largest horizontal gap (along Y-axis) to split into top/bottom groups.
    fn find_horizontal_split(
        &self,
        fragments: &[TextFragment],
        indices: &[usize],
    ) -> Option<(Vec<usize>, Vec<usize>)> {
        // Sort by Y descending (top of page first)
        let mut by_y: Vec<(f64, f64, usize)> = indices
            .iter()
            .map(|&i| (fragments[i].y, fragments[i].y + fragments[i].height, i))
            .collect();
        by_y.sort_by(|a, b| b.0.total_cmp(&a.0));

        let mut max_gap = 0.0f64;
        let mut split_y = 0.0f64;

        // Sweep from top to bottom: find gap between bottom of one fragment and top of next
        let mut min_bottom = by_y[0].0; // y (bottom edge, since y is bottom in PDF)
        for window in by_y.windows(2) {
            let current_bottom = min_bottom;
            let next_top = window[1].1; // y + height = top edge
            let gap = current_bottom - next_top;
            if gap > max_gap {
                max_gap = gap;
                split_y = next_top + gap / 2.0;
            }
            min_bottom = min_bottom.min(window[1].0);
        }

        if max_gap < self.min_gap {
            return None;
        }

        let top: Vec<usize> = indices
            .iter()
            .filter(|&&i| fragments[i].y >= split_y)
            .copied()
            .collect();
        let bottom: Vec<usize> = indices
            .iter()
            .filter(|&&i| fragments[i].y < split_y)
            .copied()
            .collect();

        if top.is_empty() || bottom.is_empty() {
            return None;
        }

        Some((top, bottom))
    }
}
