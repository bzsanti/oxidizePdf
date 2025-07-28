//! Basic geometric types for PDF

/// A point in 2D space
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// X coordinate
    pub x: f64,
    /// Y coordinate  
    pub y: f64,
}

impl Point {
    /// Create a new point
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Origin point (0, 0)
    pub fn origin() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// A rectangle defined by two points
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rectangle {
    /// Lower-left corner
    pub lower_left: Point,
    /// Upper-right corner
    pub upper_right: Point,
}

impl Rectangle {
    /// Create a new rectangle from two points
    pub fn new(lower_left: Point, upper_right: Point) -> Self {
        Self {
            lower_left,
            upper_right,
        }
    }

    /// Create a rectangle from position and size
    pub fn from_position_and_size(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            lower_left: Point::new(x, y),
            upper_right: Point::new(x + width, y + height),
        }
    }

    /// Get the width
    pub fn width(&self) -> f64 {
        self.upper_right.x - self.lower_left.x
    }

    /// Get the height
    pub fn height(&self) -> f64 {
        self.upper_right.y - self.lower_left.y
    }

    /// Get the center point
    pub fn center(&self) -> Point {
        Point::new(
            (self.lower_left.x + self.upper_right.x) / 2.0,
            (self.lower_left.y + self.upper_right.y) / 2.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point() {
        let p = Point::new(10.0, 20.0);
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);

        let origin = Point::origin();
        assert_eq!(origin.x, 0.0);
        assert_eq!(origin.y, 0.0);
    }

    #[test]
    fn test_rectangle() {
        let rect = Rectangle::new(Point::new(10.0, 20.0), Point::new(110.0, 120.0));

        assert_eq!(rect.width(), 100.0);
        assert_eq!(rect.height(), 100.0);

        let center = rect.center();
        assert_eq!(center.x, 60.0);
        assert_eq!(center.y, 70.0);
    }

    #[test]
    fn test_rectangle_from_position_and_size() {
        let rect = Rectangle::from_position_and_size(10.0, 20.0, 50.0, 30.0);
        assert_eq!(rect.lower_left.x, 10.0);
        assert_eq!(rect.lower_left.y, 20.0);
        assert_eq!(rect.upper_right.x, 60.0);
        assert_eq!(rect.upper_right.y, 50.0);
    }

    #[test]
    fn test_point_edge_cases() {
        // Test with negative coordinates
        let p = Point::new(-10.0, -20.0);
        assert_eq!(p.x, -10.0);
        assert_eq!(p.y, -20.0);

        // Test with zero coordinates
        let p = Point::new(0.0, 0.0);
        assert_eq!(p.x, 0.0);
        assert_eq!(p.y, 0.0);

        // Test with very large coordinates
        let p = Point::new(f64::MAX, f64::MIN);
        assert_eq!(p.x, f64::MAX);
        assert_eq!(p.y, f64::MIN);

        // Test with infinity and NaN
        let p = Point::new(f64::INFINITY, f64::NEG_INFINITY);
        assert_eq!(p.x, f64::INFINITY);
        assert_eq!(p.y, f64::NEG_INFINITY);

        let p = Point::new(f64::NAN, 0.0);
        assert!(p.x.is_nan());
        assert_eq!(p.y, 0.0);
    }

    #[test]
    fn test_point_copy_clone_debug() {
        let p1 = Point::new(5.0, 10.0);
        let p2 = p1; // Copy
        let p3 = p1.clone(); // Clone

        assert_eq!(p1, p2);
        assert_eq!(p1, p3);
        assert_eq!(p2, p3);

        // Test debug formatting
        let debug_str = format!("{:?}", p1);
        assert!(debug_str.contains("Point"));
        assert!(debug_str.contains("5.0"));
        assert!(debug_str.contains("10.0"));
    }

    #[test]
    fn test_point_partial_eq() {
        let p1 = Point::new(1.0, 2.0);
        let p2 = Point::new(1.0, 2.0);
        let p3 = Point::new(1.0, 3.0);
        let p4 = Point::new(2.0, 2.0);

        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
        assert_ne!(p1, p4);
        assert_ne!(p3, p4);

        // Test with special values
        let p_inf = Point::new(f64::INFINITY, f64::INFINITY);
        let p_inf2 = Point::new(f64::INFINITY, f64::INFINITY);
        assert_eq!(p_inf, p_inf2);

        let p_nan = Point::new(f64::NAN, 0.0);
        let p_nan2 = Point::new(f64::NAN, 0.0);
        assert_ne!(p_nan, p_nan2); // NaN != NaN
    }

    #[test]
    fn test_rectangle_edge_cases() {
        // Test with negative dimensions
        let rect = Rectangle::from_position_and_size(-10.0, -20.0, 5.0, 10.0);
        assert_eq!(rect.width(), 5.0);
        assert_eq!(rect.height(), 10.0);
        assert_eq!(rect.lower_left.x, -10.0);
        assert_eq!(rect.lower_left.y, -20.0);
        assert_eq!(rect.upper_right.x, -5.0);
        assert_eq!(rect.upper_right.y, -10.0);

        // Test with zero dimensions
        let rect = Rectangle::from_position_and_size(0.0, 0.0, 0.0, 0.0);
        assert_eq!(rect.width(), 0.0);
        assert_eq!(rect.height(), 0.0);
        let center = rect.center();
        assert_eq!(center.x, 0.0);
        assert_eq!(center.y, 0.0);

        // Test with very large dimensions
        let rect = Rectangle::from_position_and_size(0.0, 0.0, f64::MAX / 2.0, f64::MAX / 2.0);
        assert_eq!(rect.width(), f64::MAX / 2.0);
        assert_eq!(rect.height(), f64::MAX / 2.0);
    }

    #[test]
    fn test_rectangle_negative_dimensions() {
        // Test inverted rectangle (lower_left actually higher than upper_right)
        let rect = Rectangle::new(Point::new(10.0, 20.0), Point::new(5.0, 15.0));
        assert_eq!(rect.width(), -5.0); // Negative width
        assert_eq!(rect.height(), -5.0); // Negative height

        let center = rect.center();
        assert_eq!(center.x, 7.5);
        assert_eq!(center.y, 17.5);
    }

    #[test]
    fn test_rectangle_center_precision() {
        // Test center calculation with odd dimensions
        let rect = Rectangle::from_position_and_size(1.0, 1.0, 3.0, 5.0);
        let center = rect.center();
        assert_eq!(center.x, 2.5); // (1 + 4) / 2
        assert_eq!(center.y, 3.5); // (1 + 6) / 2

        // Test with floating point precision issues
        let rect = Rectangle::from_position_and_size(0.1, 0.1, 0.2, 0.2);
        let center = rect.center();
        assert!((center.x - 0.2).abs() < f64::EPSILON);
        assert!((center.y - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rectangle_copy_clone_debug_partial_eq() {
        let rect1 = Rectangle::new(Point::new(0.0, 0.0), Point::new(10.0, 20.0));
        let rect2 = rect1; // Copy
        let rect3 = rect1.clone(); // Clone
        let rect4 = Rectangle::new(Point::new(0.0, 0.0), Point::new(10.0, 21.0));

        assert_eq!(rect1, rect2);
        assert_eq!(rect1, rect3);
        assert_ne!(rect1, rect4);

        // Test debug formatting
        let debug_str = format!("{:?}", rect1);
        assert!(debug_str.contains("Rectangle"));
        assert!(debug_str.contains("lower_left"));
        assert!(debug_str.contains("upper_right"));
    }

    #[test]
    fn test_rectangle_with_special_float_values() {
        // Test with infinity
        let rect = Rectangle::new(
            Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
            Point::new(f64::INFINITY, f64::INFINITY),
        );
        assert_eq!(rect.width(), f64::INFINITY);
        assert_eq!(rect.height(), f64::INFINITY);

        // Test with NaN
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(f64::NAN, 10.0));
        assert!(rect.width().is_nan());
        assert_eq!(rect.height(), 10.0);

        let center = rect.center();
        assert!(center.x.is_nan());
        assert_eq!(center.y, 5.0);
    }

    #[test]
    fn test_rectangle_extreme_coordinates() {
        // Test with maximum and minimum finite values
        let rect = Rectangle::new(
            Point::new(f64::MIN, f64::MIN),
            Point::new(f64::MAX, f64::MAX),
        );

        // Width and height should be positive infinity due to overflow
        assert!(rect.width().is_infinite() && rect.width() > 0.0);
        assert!(rect.height().is_infinite() && rect.height() > 0.0);
    }

    #[test]
    fn test_point_origin_function() {
        let origin1 = Point::origin();
        let origin2 = Point::new(0.0, 0.0);

        assert_eq!(origin1, origin2);
        assert_eq!(origin1.x, 0.0);
        assert_eq!(origin1.y, 0.0);

        // Test multiple calls return equal points
        let origin3 = Point::origin();
        assert_eq!(origin1, origin3);
    }

    #[test]
    fn test_rectangle_construction_methods() {
        // Test both construction methods produce equivalent results
        let rect1 = Rectangle::new(Point::new(5.0, 10.0), Point::new(15.0, 25.0));
        let rect2 = Rectangle::from_position_and_size(5.0, 10.0, 10.0, 15.0);

        assert_eq!(rect1.lower_left, rect2.lower_left);
        assert_eq!(rect1.upper_right, rect2.upper_right);
        assert_eq!(rect1.width(), rect2.width());
        assert_eq!(rect1.height(), rect2.height());
        assert_eq!(rect1.center(), rect2.center());
    }

    #[test]
    fn test_geometric_calculations() {
        let rect = Rectangle::from_position_and_size(2.0, 3.0, 8.0, 6.0);

        // Verify all calculations
        assert_eq!(rect.lower_left.x, 2.0);
        assert_eq!(rect.lower_left.y, 3.0);
        assert_eq!(rect.upper_right.x, 10.0);
        assert_eq!(rect.upper_right.y, 9.0);
        assert_eq!(rect.width(), 8.0);
        assert_eq!(rect.height(), 6.0);

        let center = rect.center();
        assert_eq!(center.x, 6.0); // (2 + 10) / 2
        assert_eq!(center.y, 6.0); // (3 + 9) / 2
    }

    #[test]
    fn test_floating_point_precision() {
        // Test with values that might have precision issues
        let p1 = Point::new(1.0 / 3.0, 2.0 / 3.0);
        let p2 = Point::new(4.0 / 3.0, 5.0 / 3.0);
        let rect = Rectangle::new(p1, p2);

        let width = rect.width();
        let height = rect.height();
        let center = rect.center();

        // These should be exactly 1.0 due to the arithmetic
        assert!((width - 1.0).abs() < f64::EPSILON);
        assert!((height - 1.0).abs() < f64::EPSILON);

        // Center should be at (5/6, 7/6)
        let expected_center_x = 5.0 / 6.0;
        let expected_center_y = 7.0 / 6.0;
        assert!((center.x - expected_center_x).abs() < f64::EPSILON);
        assert!((center.y - expected_center_y).abs() < f64::EPSILON);
    }

    #[test]
    fn test_point_with_mathematical_constants() {
        use std::f64::consts::*;
        let p = Point::new(PI, E);
        assert_eq!(p.x, PI);
        assert_eq!(p.y, E);

        let origin = Point::origin();
        assert_ne!(p, origin);
    }

    #[test]
    fn test_rectangle_area_and_perimeter_concepts() {
        // While not implemented, test the values needed for these calculations
        let rect = Rectangle::from_position_and_size(0.0, 0.0, 4.0, 3.0);

        let width = rect.width();
        let height = rect.height();

        // These values would be used for area and perimeter
        let calculated_area = width * height;
        let calculated_perimeter = 2.0 * (width + height);

        assert_eq!(calculated_area, 12.0);
        assert_eq!(calculated_perimeter, 14.0);
    }

    #[test]
    fn test_point_distance_concepts() {
        // Test values that would be used for distance calculations
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);

        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        let distance_squared = dx * dx + dy * dy;
        let distance = distance_squared.sqrt();

        assert_eq!(dx, 3.0);
        assert_eq!(dy, 4.0);
        assert_eq!(distance_squared, 25.0);
        assert_eq!(distance, 5.0);
    }

    #[test]
    fn test_rectangle_contains_concepts() {
        // Test values for point-in-rectangle calculations
        let rect = Rectangle::from_position_and_size(10.0, 20.0, 30.0, 40.0);
        let test_point = Point::new(25.0, 35.0);

        // Values for contains check
        let within_x = test_point.x >= rect.lower_left.x && test_point.x <= rect.upper_right.x;
        let within_y = test_point.y >= rect.lower_left.y && test_point.y <= rect.upper_right.y;
        let would_contain = within_x && within_y;

        assert!(within_x);
        assert!(within_y);
        assert!(would_contain);

        // Test point outside
        let outside_point = Point::new(5.0, 35.0);
        let outside_x =
            outside_point.x >= rect.lower_left.x && outside_point.x <= rect.upper_right.x;
        assert!(!outside_x);
    }

    #[test]
    fn test_rectangle_intersection_concepts() {
        let rect1 = Rectangle::from_position_and_size(0.0, 0.0, 10.0, 10.0);
        let rect2 = Rectangle::from_position_and_size(5.0, 5.0, 10.0, 10.0);

        // Calculate intersection bounds
        let left = rect1.lower_left.x.max(rect2.lower_left.x);
        let bottom = rect1.lower_left.y.max(rect2.lower_left.y);
        let right = rect1.upper_right.x.min(rect2.upper_right.x);
        let top = rect1.upper_right.y.min(rect2.upper_right.y);

        let intersects = left < right && bottom < top;

        assert_eq!(left, 5.0);
        assert_eq!(bottom, 5.0);
        assert_eq!(right, 10.0);
        assert_eq!(top, 10.0);
        assert!(intersects);

        // Non-intersecting rectangles
        let rect3 = Rectangle::from_position_and_size(20.0, 20.0, 5.0, 5.0);
        let left3 = rect1.lower_left.x.max(rect3.lower_left.x);
        let right3 = rect1.upper_right.x.min(rect3.upper_right.x);
        let no_intersection = left3 >= right3;
        assert!(no_intersection);
    }
}
