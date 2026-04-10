use oxidize_pdf::layout::{centered_image_x, fit_image_dimensions};

#[test]
fn test_fit_width_constrained() {
    // 400×200 image (2:1 ratio) into 200×300 box → width constrains
    let (w, h) = fit_image_dimensions(400, 200, 200.0, 300.0);
    assert!(
        (w - 200.0).abs() < 0.001,
        "width should be 200.0, got {}",
        w
    );
    assert!(
        (h - 100.0).abs() < 0.001,
        "height should be 100.0, got {}",
        h
    );
}

#[test]
fn test_fit_height_constrained() {
    // 200×400 image (1:2 ratio) into 300×150 box → height constrains
    let (w, h) = fit_image_dimensions(200, 400, 300.0, 150.0);
    assert!((w - 75.0).abs() < 0.001, "width should be 75.0, got {}", w);
    assert!(
        (h - 150.0).abs() < 0.001,
        "height should be 150.0, got {}",
        h
    );
}

#[test]
fn test_fit_exact() {
    // 100×100 image into 100×100 box → fits exactly
    let (w, h) = fit_image_dimensions(100, 100, 100.0, 100.0);
    assert!(
        (w - 100.0).abs() < 0.001,
        "width should be 100.0, got {}",
        w
    );
    assert!(
        (h - 100.0).abs() < 0.001,
        "height should be 100.0, got {}",
        h
    );
}

#[test]
fn test_centered_x() {
    // page_width=595, margins 50 each → content_width=495
    // image_width=200 → x = 50 + (495 - 200) / 2 = 50 + 147.5 = 197.5
    let x = centered_image_x(50.0, 495.0, 200.0);
    assert!(
        (x - 197.5).abs() < 0.001,
        "centered x should be 197.5, got {}",
        x
    );
}
