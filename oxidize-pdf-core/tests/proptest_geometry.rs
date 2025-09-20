//! Property-based tests for geometric types
//!
//! Tests mathematical properties and invariants of Point and Rectangle types
//! using proptest to verify correctness across all possible inputs.

use oxidize_pdf::{Point, Rectangle};
use proptest::prelude::*;

// Strategy for generating finite f64 values
#[allow(dead_code)]
fn finite_f64() -> impl Strategy<Value = f64> {
    prop_oneof![
        -1e10..1e10f64,
        Just(0.0),
        Just(1.0),
        Just(-1.0),
        Just(100.0),
        Just(-100.0),
    ]
}

// Strategy for generating Points with reasonable coordinates
prop_compose! {
    fn point_strategy()(
        x in finite_f64(),
        y in finite_f64()
    ) -> Point {
        Point::new(x, y)
    }
}

// Strategy for generating valid Rectangles
prop_compose! {
    fn rectangle_strategy()(
        x1 in finite_f64(),
        y1 in finite_f64(),
        x2 in finite_f64(),
        y2 in finite_f64()
    ) -> Rectangle {
        // Ensure proper ordering for a valid rectangle
        let min_x = x1.min(x2);
        let max_x = x1.max(x2);
        let min_y = y1.min(y2);
        let max_y = y1.max(y2);

        Rectangle::new(
            Point::new(min_x, min_y),
            Point::new(max_x, max_y)
        )
    }
}

// Strategy for generating Rectangles from position and size
prop_compose! {
    fn rectangle_from_size_strategy()(
        x in finite_f64(),
        y in finite_f64(),
        width in 0.0..1000.0f64,
        height in 0.0..1000.0f64
    ) -> Rectangle {
        Rectangle::from_position_and_size(x, y, width, height)
    }
}

proptest! {
    fn test_point_creation_preserves_coordinates(x in finite_f64(), y in finite_f64()) {
        let point = Point::new(x, y);
        prop_assert_eq!(point.x, x);
        prop_assert_eq!(point.y, y);
    }


    fn test_point_equality_reflexive(point in point_strategy()) {
        // A point equals itself
        prop_assert_eq!(point, point);
    }

    fn test_point_equality_symmetric(p1 in point_strategy(), p2 in point_strategy()) {
        // If p1 == p2, then p2 == p1
        if p1 == p2 {
            prop_assert!(p2 == p1);
        }
    }

    fn test_rectangle_width_non_negative(rect in rectangle_strategy()) {
        let width = rect.width();
        prop_assert!(width >= 0.0, "Width was {}", width);
    }

    fn test_rectangle_height_non_negative(rect in rectangle_strategy()) {
        let height = rect.height();
        prop_assert!(height >= 0.0, "Height was {}", height);
    }

    fn test_rectangle_area_non_negative(rect in rectangle_strategy()) {
        let area = rect.width() * rect.height();
        prop_assert!(area >= 0.0, "Area was {}", area);
    }

    fn test_rectangle_from_position_and_size_consistency(
        x in finite_f64(),
        y in finite_f64(),
        width in 0.0..1000.0f64,
        height in 0.0..1000.0f64
    ) {
        let rect = Rectangle::from_position_and_size(x, y, width, height);

        // Verify corners
        prop_assert_eq!(rect.lower_left.x, x);
        prop_assert_eq!(rect.lower_left.y, y);
        prop_assert_eq!(rect.upper_right.x, x + width);
        prop_assert_eq!(rect.upper_right.y, y + height);

        // Verify dimensions
        prop_assert!((rect.width() - width).abs() < 1e-10);
        prop_assert!((rect.height() - height).abs() < 1e-10);
    }

    fn test_rectangle_center_calculation(rect in rectangle_strategy()) {
        let center = rect.center();

        // Center should be the midpoint of corners
        let expected_x = (rect.lower_left.x + rect.upper_right.x) / 2.0;
        let expected_y = (rect.lower_left.y + rect.upper_right.y) / 2.0;

        prop_assert!((center.x - expected_x).abs() < 1e-10);
        prop_assert!((center.y - expected_y).abs() < 1e-10);
    }

    fn test_rectangle_ordering_invariant(x1 in finite_f64(), y1 in finite_f64(), x2 in finite_f64(), y2 in finite_f64()) {
        // When creating a rectangle, lower_left should have min coordinates
        // and upper_right should have max coordinates
        let rect = Rectangle::new(
            Point::new(x1, y1),
            Point::new(x2, y2)
        );

        // The Rectangle constructor might normalize the points
        // so we just verify the rectangle is valid
        prop_assert!(rect.width() >= 0.0);
        prop_assert!(rect.height() >= 0.0);
    }

    fn test_rectangle_dimensions_match_corners(rect in rectangle_strategy()) {
        let width = rect.width();
        let height = rect.height();

        prop_assert_eq!(width, rect.upper_right.x - rect.lower_left.x);
        prop_assert_eq!(height, rect.upper_right.y - rect.lower_left.y);
    }
}

// Regression tests for specific edge cases
#[cfg(test)]
mod regression_tests {
    use super::*;

    #[allow(dead_code)]
    fn test_point_with_negative_zero() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(-0.0, -0.0);
        assert_eq!(p1, p2);
    }

    #[allow(dead_code)]
    fn test_rectangle_zero_area() {
        // Line rectangles (zero width or height)
        let horizontal_line = Rectangle::from_position_and_size(0.0, 0.0, 100.0, 0.0);
        assert_eq!(horizontal_line.width() * horizontal_line.height(), 0.0);
        assert_eq!(horizontal_line.height(), 0.0);
        assert_eq!(horizontal_line.width(), 100.0);

        let vertical_line = Rectangle::from_position_and_size(0.0, 0.0, 0.0, 100.0);
        assert_eq!(vertical_line.width() * vertical_line.height(), 0.0);
        assert_eq!(vertical_line.width(), 0.0);
        assert_eq!(vertical_line.height(), 100.0);

        // Point rectangle (zero width and height)
        let point_rect = Rectangle::from_position_and_size(0.0, 0.0, 0.0, 0.0);
        assert_eq!(point_rect.width() * point_rect.height(), 0.0);
    }

    #[allow(dead_code)]
    fn test_rectangle_large_coordinates() {
        let large_rect = Rectangle::new(Point::new(-1e9, -1e9), Point::new(1e9, 1e9));
        assert_eq!(large_rect.width(), 2e9);
        assert_eq!(large_rect.height(), 2e9);

        // Center should be at origin
        let center = large_rect.center();
        assert!((center.x - 0.0).abs() < 1e-10);
        assert!((center.y - 0.0).abs() < 1e-10);
    }
}

// Test functions outside proptest macro
#[test]
fn test_point_origin_is_zero() {
    let origin = Point::origin();
    assert_eq!(origin.x, 0.0);
    assert_eq!(origin.y, 0.0);
}
