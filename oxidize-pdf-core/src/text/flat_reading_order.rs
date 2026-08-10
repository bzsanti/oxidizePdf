//! Flat-path reading order (issue #448).
//!
//! The default `.text` flat path emits fragments in content-stream order,
//! which transposes multi-column and out-of-order layouts. This module is the
//! isolated ordering primitive: a pure function from line-group boxes to a
//! permutation of their indices. It is deliberately decoupled from
//! `extraction.rs` — the wiring into the show-text arms is a separate, later
//! step (design doc §5.8) that rebases onto #456.
//!
//! Two properties set it apart from the pre-existing
//! [`crate::pipeline::reading_order::XYCutReadingOrder`], which serves the
//! partition/Element pipeline and is unsuitable here:
//!
//! - **Scale-relative significance** (§5.4). A gap counts as a column gutter or
//!   a section break only when it exceeds a multiple of the region's median
//!   glyph size, never an absolute point threshold. The absolute `min_gap` of
//!   the existing cut is the exact bug family that motivated #448.
//! - **Stream-order leaf** (§5.4). A region the cut cannot split comes back in
//!   input (content-stream) order, not re-sorted geometrically. The probe
//!   measured this as the larger share of the gain (stream leaf −17% vs a
//!   geometric leaf −13%), and it is what makes the identity permutation
//!   byte-preserving for the caller (§5.2).

// Staged ahead of its call site: the show-text wiring (§5.8) is a later step
// that rebases onto #456, so in a non-test build nothing references this module
// yet. Remove this attribute when the wiring lands and the module is live.
#![allow(dead_code)]

/// A box the flat-path orderer permutes: one per line group (§5.1).
///
/// Coordinates are page space with `/Rotate` already applied by the caller
/// (§5.3), so the orderer never looks at rotation itself. PDF convention: `y`
/// grows upward, `min_y` is the bottom edge and `max_y` the top edge.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OrderBox {
    /// Left edge.
    pub min_x: f64,
    /// Right edge.
    pub max_x: f64,
    /// Bottom edge (PDF space: smaller y is lower on the page).
    pub min_y: f64,
    /// Top edge.
    pub max_y: f64,
    /// Representative glyph size of the group; the unit for the gutter and the
    /// line-break scale in [`CutConfig`].
    pub font_size: f64,
}

/// Scale-relative cut thresholds (§5.4). A gap must exceed `k ×` the region's
/// median [`OrderBox::font_size`] to be taken as a significant cut. One
/// constant per axis, calibrated against the order gate — the calibration is
/// deferred until #456/#458 land, so these stay parameters, not constants.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CutConfig {
    /// Horizontal gutter significance, in multiples of the median glyph size.
    pub horizontal_k: f64,
    /// Vertical section-break significance, in multiples of the median glyph
    /// size.
    pub vertical_k: f64,
}

/// Order line-group boxes and return a permutation of `0..boxes.len()`.
///
/// The recursion cuts on whichever axis carries the widest scale-relative gap;
/// a region with no significant gap is a leaf and is returned in input order,
/// so `reading_order` is the identity permutation whenever the stream order is
/// already the reading order.
pub(crate) fn reading_order(boxes: &[OrderBox], cfg: &CutConfig) -> Vec<usize> {
    let mut order = Vec::with_capacity(boxes.len());
    let indices: Vec<usize> = (0..boxes.len()).collect();
    cut_recursive(boxes, &indices, cfg, &mut order);
    order
}

/// The scale unit for a region: the median glyph size of its boxes. Returns
/// `None` for an empty region or a non-positive median (degenerate input),
/// which forces the region to be treated as a leaf rather than dividing by a
/// meaningless scale (§5.5 guard 6).
fn median_font_size(boxes: &[OrderBox], indices: &[usize]) -> Option<f64> {
    if indices.is_empty() {
        return None;
    }
    let mut sizes: Vec<f64> = indices.iter().map(|&i| boxes[i].font_size).collect();
    sizes.sort_by(|a, b| a.total_cmp(b));
    let median = sizes[sizes.len() / 2];
    (median.is_finite() && median > 0.0).then_some(median)
}

/// Recursively cut `indices` and append the resulting reading order to `order`.
fn cut_recursive(boxes: &[OrderBox], indices: &[usize], cfg: &CutConfig, order: &mut Vec<usize>) {
    if indices.len() <= 1 {
        order.extend_from_slice(indices);
        return;
    }
    let Some(scale) = median_font_size(boxes, indices) else {
        emit_leaf(indices, order);
        return;
    };

    // A column gutter (gap along X) is judged against `horizontal_k`; a section
    // break (gap along Y) against `vertical_k`. Take whichever significant cut
    // carries the wider scale-relative gap (§5.4, "hueco más ancho").
    let column = find_column_gap(boxes, indices).filter(|c| c.ratio(scale) >= cfg.horizontal_k);
    let section = find_section_gap(boxes, indices).filter(|c| c.ratio(scale) >= cfg.vertical_k);

    let choose_column = match (&column, &section) {
        (Some(c), Some(s)) => c.ratio(scale) >= s.ratio(scale),
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => {
            emit_leaf(indices, order);
            return;
        }
    };

    if choose_column {
        let (left, right) = partition_x(boxes, indices, column.unwrap().split);
        // Left column before right column.
        cut_recursive(boxes, &left, cfg, order);
        cut_recursive(boxes, &right, cfg, order);
    } else {
        let (top, bottom) = partition_y(boxes, indices, section.unwrap().split);
        // Higher-on-the-page section before lower.
        cut_recursive(boxes, &top, cfg, order);
        cut_recursive(boxes, &bottom, cfg, order);
    }
}

/// A leaf region: no significant cut, so preserve content-stream order (§5.4).
/// The original indices ascending are the stream order.
fn emit_leaf(indices: &[usize], order: &mut Vec<usize>) {
    let mut leaf = indices.to_vec();
    leaf.sort_unstable();
    order.extend(leaf);
}

/// A candidate cut: the absolute gap width and the coordinate to split at.
struct Gap {
    width: f64,
    split: f64,
}

impl Gap {
    fn ratio(&self, scale: f64) -> f64 {
        self.width / scale
    }
}

/// Largest whitespace gap along X (a column gutter). Sweeps left to right
/// tracking the running maximum right edge, so a short box nested inside a tall
/// column does not open a spurious gap.
fn find_column_gap(boxes: &[OrderBox], indices: &[usize]) -> Option<Gap> {
    let mut edges: Vec<(f64, f64)> = indices
        .iter()
        .map(|&i| (boxes[i].min_x, boxes[i].max_x))
        .collect();
    edges.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut max_gap = 0.0f64;
    let mut split = 0.0f64;
    let mut max_right = edges[0].1;
    for window in edges.windows(2) {
        let next_left = window[1].0;
        let gap = next_left - max_right;
        if gap > max_gap {
            max_gap = gap;
            split = max_right + gap / 2.0;
        }
        max_right = max_right.max(window[1].1);
    }
    (max_gap > 0.0).then_some(Gap {
        width: max_gap,
        split,
    })
}

/// Largest whitespace gap along Y (a section break). PDF space is y-up, so the
/// sweep runs top (large y) to bottom, tracking the running minimum bottom edge.
fn find_section_gap(boxes: &[OrderBox], indices: &[usize]) -> Option<Gap> {
    let mut edges: Vec<(f64, f64)> = indices
        .iter()
        .map(|&i| (boxes[i].min_y, boxes[i].max_y))
        .collect();
    // Sort by top edge descending (topmost first).
    edges.sort_by(|a, b| b.1.total_cmp(&a.1));

    let mut max_gap = 0.0f64;
    let mut split = 0.0f64;
    let mut min_bottom = edges[0].0;
    for window in edges.windows(2) {
        let next_top = window[1].1;
        let gap = min_bottom - next_top;
        if gap > max_gap {
            max_gap = gap;
            split = next_top + gap / 2.0;
        }
        min_bottom = min_bottom.min(window[1].0);
    }
    (max_gap > 0.0).then_some(Gap {
        width: max_gap,
        split,
    })
}

/// Split a region into (left, right) by box-center X against `split`. Both
/// partitions preserve the input order of `indices`.
fn partition_x(boxes: &[OrderBox], indices: &[usize], split: f64) -> (Vec<usize>, Vec<usize>) {
    indices
        .iter()
        .partition(|&&i| (boxes[i].min_x + boxes[i].max_x) / 2.0 < split)
}

/// Split a region into (top, bottom) by box-center Y against `split`. Top is the
/// higher-on-the-page half (larger y). Both partitions preserve input order.
fn partition_y(boxes: &[OrderBox], indices: &[usize], split: f64) -> (Vec<usize>, Vec<usize>) {
    indices
        .iter()
        .partition(|&&i| (boxes[i].min_y + boxes[i].max_y) / 2.0 >= split)
}
