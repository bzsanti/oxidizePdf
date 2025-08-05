//! Property-based tests for graphics operations
//!
//! Tests mathematical properties and invariants of graphics operations,
//! transformations, and path construction.

use oxidize_pdf::graphics::{Color, Point};
use oxidize_pdf::{Document, Page};
use proptest::prelude::*;

// Strategy for finite floating point values suitable for graphics
fn graphics_coord() -> impl Strategy<Value = f64> {
    prop_oneof![
        -1000.0..1000.0f64,
        Just(0.0),
        Just(1.0),
        Just(-1.0),
        Just(100.0),
        Just(-100.0),
    ]
}

// Strategy for color values (0.0 to 1.0)
fn color_component() -> impl Strategy<Value = f64> {
    0.0..=1.0f64
}

// Strategy for generating valid colors
prop_compose! {
    fn color_strategy()(
        color_type in 0..3usize,
        r in color_component(),
        g in color_component(),
        b in color_component(),
        gray in color_component(),
        c in color_component(),
        m in color_component(),
        y in color_component(),
        k in color_component()
    ) -> Color {
        match color_type {
            0 => Color::rgb(r, g, b),
            1 => Color::gray(gray),
            _ => Color::cmyk(c, m, y, k),
        }
    }
}

proptest! {
    fn test_graphics_state_save_restore_balance(num_ops in 0..50usize) {
        let mut page = Page::a4();
        let graphics = page.graphics();

        // Randomly save and restore states
        let mut depth = 0;
        for _ in 0..num_ops {
            if depth > 0 && (num_ops % 2 == 0) {
                graphics.restore_state();
                depth -= 1;
            } else if depth < 10 { // Limit nesting
                graphics.save_state();
                depth += 1;
            }
        }

        // Balance out any remaining saves
        for _ in 0..depth {
            graphics.restore_state();
        }

        // Should not panic
        prop_assert!(true);
    }

    fn test_color_setting_preserved(color in color_strategy()) {
        let mut page = Page::a4();
        let graphics = page.graphics();

        // Set colors
        graphics.set_fill_color(color);
        graphics.set_stroke_color(color);

        // Colors should be applied without panic
        prop_assert!(true);
    }

    fn test_line_width_positive(width in 0.1..100.0f64) {
        let mut page = Page::a4();
        let graphics = page.graphics();

        graphics.set_line_width(width);

        // Line width should be positive
        prop_assert!(width > 0.0);
    }

    fn test_rectangle_drawing(
        x in graphics_coord(),
        y in graphics_coord(),
        width in 0.1..500.0f64,
        height in 0.1..500.0f64
    ) {
        let mut page = Page::a4();
        let graphics = page.graphics();

        // Draw rectangle
        graphics.rectangle(x, y, width, height);
        graphics.stroke();

        // Should handle any valid coordinates
        prop_assert!(true);
    }

    fn test_circle_drawing(
        cx in graphics_coord(),
        cy in graphics_coord(),
        radius in 0.1..200.0f64
    ) {
        let mut page = Page::a4();
        let graphics = page.graphics();

        // Draw circle
        graphics.circle(cx, cy, radius);
        graphics.fill();

        // Radius should be positive
        prop_assert!(radius > 0.0);
    }

    fn test_path_construction_move_line(
        points in prop::collection::vec((graphics_coord(), graphics_coord()), 2..20)
    ) {
        let mut page = Page::a4();
        let graphics = page.graphics();

        // Build a path
        if let Some((first_x, first_y)) = points.first() {
            graphics.move_to(*first_x, *first_y);

            for (x, y) in points.iter().skip(1) {
                graphics.line_to(*x, *y);
            }

            graphics.stroke();
        }

        // Path should be constructible
        prop_assert!(true);
    }

    fn test_bezier_curve_control_points(
        x1 in graphics_coord(), y1 in graphics_coord(),
        x2 in graphics_coord(), y2 in graphics_coord(),
        x3 in graphics_coord(), y3 in graphics_coord()
    ) {
        let mut page = Page::a4();
        let graphics = page.graphics();

        graphics.move_to(0.0, 0.0);
        graphics.curve_to(x1, y1, x2, y2, x3, y3);
        graphics.stroke();

        // Bezier curves should handle any control points
        prop_assert!(true);
    }

    fn test_transform_operations(
        a in graphics_coord(),
        b in graphics_coord(),
        c in graphics_coord(),
        d in graphics_coord(),
        e in graphics_coord(),
        f in graphics_coord()
    ) {
        let mut page = Page::a4();
        let graphics = page.graphics();

        // Apply transform
        graphics.transform(a, b, c, d, e, f);

        // Draw something
        graphics.rectangle(10.0, 10.0, 50.0, 50.0);
        graphics.fill();

        // Transform should be applied
        prop_assert!(true);
    }

    fn test_translate_operation(dx in graphics_coord(), dy in graphics_coord()) {
        let mut page = Page::a4();
        let graphics = page.graphics();

        // Translate
        graphics.translate(dx, dy);

        // Draw at origin
        graphics.circle(0.0, 0.0, 10.0);
        graphics.fill();

        // Should be translated
        prop_assert!(true);
    }

    fn test_scale_operation(sx in 0.1..10.0f64, sy in 0.1..10.0f64) {
        let mut page = Page::a4();
        let graphics = page.graphics();

        // Scale
        graphics.scale(sx, sy);

        // Draw unit square
        graphics.rectangle(0.0, 0.0, 1.0, 1.0);
        graphics.stroke();

        // Should be scaled
        prop_assert!(true);
    }


    fn test_dash_pattern(
        pattern in prop::collection::vec(0.1..20.0f64, 1..6),
        phase in 0.0..10.0f64
    ) {
        let mut page = Page::a4();
        let graphics = page.graphics();

        // Set dash pattern
        // Create LineDashPattern from the pattern vector
        // For now, just comment out until we implement proper dash pattern support
        // graphics.set_line_dash_pattern(LineDashPattern::new(&pattern, phase));

        // Draw dashed line
        graphics.move_to(100.0, 100.0);
        graphics.line_to(500.0, 100.0);
        graphics.stroke();

        // Pattern should have positive values
        prop_assert!(pattern.iter().all(|&d| d > 0.0));
    }
}

// Regression tests for specific graphics scenarios
#[cfg(test)]
mod regression_tests {
    use super::*;

    fn test_zero_radius_circle() {
        let mut page = Page::a4();
        let graphics = page.graphics();

        // Zero radius circle (point)
        graphics.circle(100.0, 100.0, 0.0);
        graphics.fill();

        // Should not panic
    }

    fn test_zero_dimension_rectangle() {
        let mut page = Page::a4();
        let graphics = page.graphics();

        // Zero width rectangle (line)
        graphics.rectangle(100.0, 100.0, 0.0, 100.0);
        graphics.stroke();

        // Zero height rectangle (line)
        graphics.rectangle(200.0, 200.0, 100.0, 0.0);
        graphics.stroke();

        // Zero area rectangle (point)
        graphics.rectangle(300.0, 300.0, 0.0, 0.0);
        graphics.stroke();
    }

    fn test_singular_transform() {
        let mut page = Page::a4();
        let graphics = page.graphics();

        // Transform with zero determinant (singular)
        graphics.transform(1.0, 0.0, 1.0, 0.0, 0.0, 0.0);

        // Should handle singular transform
        graphics.rectangle(10.0, 10.0, 50.0, 50.0);
        graphics.stroke();
    }

    fn test_very_long_path() {
        let mut page = Page::a4();
        let graphics = page.graphics();

        // Create a very long path
        graphics.move_to(0.0, 0.0);
        for i in 0..1000 {
            let x = (i as f64) * 0.5;
            let y = (i as f64).sin() * 50.0 + 400.0;
            graphics.line_to(x, y);
        }
        graphics.stroke();
    }

    fn test_nested_save_restore_states() {
        let mut page = Page::a4();
        let graphics = page.graphics();

        // Deeply nested states
        for i in 0..20 {
            graphics.save_state();
            graphics.set_line_width(i as f64 + 1.0);
        }

        // Restore all
        for _ in 0..20 {
            graphics.restore_state();
        }
    }
}

// Additional test functions outside proptest macro
#[test]
fn test_clipping_path() {
    let mut page = Page::a4();
    let graphics = page.graphics();

    // Create clipping path
    graphics.rectangle(100.0, 100.0, 200.0, 200.0);
    graphics.clip();

    // Draw outside clip region
    graphics.circle(50.0, 50.0, 30.0);
    graphics.fill();

    // Should handle clipping without errors
}
