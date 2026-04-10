/// Calculate dimensions that fit an image within a bounding box while preserving aspect ratio.
///
/// Given the original image dimensions in pixels and a maximum bounding box in points,
/// returns the final (width, height) in points that fits within the box without distortion.
///
/// # Arguments
///
/// * `img_width` - Original image width in pixels
/// * `img_height` - Original image height in pixels
/// * `max_width` - Maximum allowed width in points
/// * `max_height` - Maximum allowed height in points
///
/// # Returns
///
/// `(final_width, final_height)` in points, preserving the original aspect ratio.
///
/// # Example
///
/// ```rust
/// use oxidize_pdf::layout::fit_image_dimensions;
///
/// // 400×200 image into 200×300 box → width-constrained → 200×100
/// let (w, h) = fit_image_dimensions(400, 200, 200.0, 300.0);
/// assert!((w - 200.0).abs() < 0.001);
/// assert!((h - 100.0).abs() < 0.001);
/// ```
pub fn fit_image_dimensions(
    img_width: u32,
    img_height: u32,
    max_width: f64,
    max_height: f64,
) -> (f64, f64) {
    if img_width == 0 || img_height == 0 {
        return (0.0, 0.0);
    }

    let ratio = img_width as f64 / img_height as f64;

    // Try fitting by height constraint
    let w_from_h = max_height * ratio;
    if w_from_h <= max_width {
        (w_from_h, max_height)
    } else {
        // Fit by width constraint
        let h_from_w = max_width / ratio;
        (max_width, h_from_w)
    }
}

/// Calculate the x coordinate to center an image horizontally within the content area.
///
/// # Arguments
///
/// * `margin_left` - Left margin in points
/// * `content_width` - Available content width in points (page width - left margin - right margin)
/// * `image_width` - Width of the image to center in points
///
/// # Returns
///
/// The x coordinate where the image should be placed.
pub fn centered_image_x(margin_left: f64, content_width: f64, image_width: f64) -> f64 {
    margin_left + (content_width - image_width).max(0.0) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_dimensions() {
        assert_eq!(fit_image_dimensions(0, 100, 200.0, 200.0), (0.0, 0.0));
        assert_eq!(fit_image_dimensions(100, 0, 200.0, 200.0), (0.0, 0.0));
    }

    #[test]
    fn test_square_image_in_square_box() {
        let (w, h) = fit_image_dimensions(500, 500, 100.0, 100.0);
        assert!((w - 100.0).abs() < 0.001);
        assert!((h - 100.0).abs() < 0.001);
    }
}
