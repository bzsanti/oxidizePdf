//! Tests for calibrated color spaces (ISO 32000-1 Section 8.6.5)

use oxidize_pdf::graphics::{
    CalGrayColorSpace, CalRgbColorSpace, CalibratedColor, GraphicsContext,
};
use oxidize_pdf::objects::Object;

#[test]
fn test_cal_gray_construction() {
    let cs = CalGrayColorSpace::new();

    // Test default values (D50 illuminant)
    assert_eq!(cs.white_point, [0.9505, 1.0000, 1.0890]);
    assert_eq!(cs.black_point, [0.0, 0.0, 0.0]);
    assert_eq!(cs.gamma, 1.0);
}

#[test]
fn test_cal_gray_builder() {
    let cs = CalGrayColorSpace::new()
        .with_white_point([0.95, 1.0, 1.09])
        .with_black_point([0.01, 0.01, 0.01])
        .with_gamma(2.2);

    assert_eq!(cs.white_point, [0.95, 1.0, 1.09]);
    assert_eq!(cs.black_point, [0.01, 0.01, 0.01]);
    assert_eq!(cs.gamma, 2.2);
}

#[test]
fn test_cal_gray_d65_illuminant() {
    let cs = CalGrayColorSpace::d65();
    assert_eq!(cs.white_point, [0.9504, 1.0000, 1.0888]);
}

#[test]
fn test_cal_gray_gamma_clamping() {
    let cs = CalGrayColorSpace::new().with_gamma(-1.0);
    assert_eq!(cs.gamma, 0.0); // Should clamp negative values to 0
}

#[test]
fn test_cal_gray_to_pdf_array() {
    let cs = CalGrayColorSpace::new()
        .with_gamma(2.2)
        .with_black_point([0.01, 0.01, 0.01]);

    let array = cs.to_pdf_array();
    assert_eq!(array.len(), 2);

    // First element should be the color space name
    if let Object::Name(name) = &array[0] {
        assert_eq!(name, "CalGray");
    } else {
        panic!("First element should be Name");
    }

    // Second element should be dictionary
    if let Object::Dictionary(dict) = &array[1] {
        assert!(dict.get("WhitePoint").is_some());
        assert!(dict.get("Gamma").is_some());
        assert!(dict.get("BlackPoint").is_some());
    } else {
        panic!("Second element should be Dictionary");
    }
}

#[test]
fn test_cal_gray_gamma_correction() {
    let cs = CalGrayColorSpace::new().with_gamma(2.2);

    let corrected = cs.apply_gamma(0.5);
    let expected = 0.5_f64.powf(2.2);

    assert!((corrected - expected).abs() < 1e-10);
}

#[test]
fn test_cal_rgb_construction() {
    let cs = CalRgbColorSpace::new();

    assert_eq!(cs.white_point, [0.9505, 1.0000, 1.0890]);
    assert_eq!(cs.black_point, [0.0, 0.0, 0.0]);
    assert_eq!(cs.gamma, [1.0, 1.0, 1.0]);

    // Identity matrix
    let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    assert_eq!(cs.matrix, identity);
}

#[test]
fn test_cal_rgb_srgb_preset() {
    let cs = CalRgbColorSpace::srgb();

    assert_eq!(cs.gamma, [2.2, 2.2, 2.2]);

    // Should not be identity matrix
    let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    assert_ne!(cs.matrix, identity);
}

#[test]
fn test_cal_rgb_adobe_rgb_preset() {
    let cs = CalRgbColorSpace::adobe_rgb();

    assert_eq!(cs.gamma, [2.2, 2.2, 2.2]);

    // Should have Adobe RGB matrix values
    assert!((cs.matrix[0] - 0.6097).abs() < 0.001);
    assert!((cs.matrix[4] - 0.6256).abs() < 0.001);
    assert!((cs.matrix[8] - 0.7441).abs() < 0.001);
}

#[test]
fn test_cal_rgb_gamma_clamping() {
    let cs = CalRgbColorSpace::new().with_gamma([-1.0, 2.2, -0.5]);

    assert_eq!(cs.gamma[0], 0.0); // Clamped to 0
    assert_eq!(cs.gamma[1], 2.2); // Unchanged
    assert_eq!(cs.gamma[2], 0.0); // Clamped to 0
}

#[test]
fn test_cal_rgb_gamma_correction() {
    let cs = CalRgbColorSpace::new().with_gamma([2.0, 2.2, 1.8]);

    let corrected = cs.apply_gamma([0.5, 0.6, 0.7]);
    let expected = [0.5_f64.powf(2.0), 0.6_f64.powf(2.2), 0.7_f64.powf(1.8)];

    for i in 0..3 {
        assert!((corrected[i] - expected[i]).abs() < 1e-10);
    }
}

#[test]
fn test_cal_rgb_xyz_transform() {
    let cs = CalRgbColorSpace::new();
    let xyz = cs.to_xyz([1.0, 0.0, 0.0]); // Pure red

    // Should match first column of matrix
    assert_eq!(xyz[0], cs.matrix[0]);
    assert_eq!(xyz[1], cs.matrix[3]);
    assert_eq!(xyz[2], cs.matrix[6]);
}

#[test]
fn test_cal_rgb_to_pdf_array() {
    let cs = CalRgbColorSpace::srgb();
    let array = cs.to_pdf_array();

    assert_eq!(array.len(), 2);

    if let Object::Name(name) = &array[0] {
        assert_eq!(name, "CalRGB");
    } else {
        panic!("First element should be Name");
    }

    if let Object::Dictionary(dict) = &array[1] {
        assert!(dict.get("WhitePoint").is_some());
        assert!(dict.get("Gamma").is_some());
        assert!(dict.get("Matrix").is_some());
    } else {
        panic!("Second element should be Dictionary");
    }
}

#[test]
fn test_calibrated_color_creation() {
    let gray_cs = CalGrayColorSpace::new().with_gamma(2.2);
    let gray_color = CalibratedColor::cal_gray(0.5, gray_cs.clone());

    assert_eq!(gray_color.values(), vec![0.5]);

    let rgb_cs = CalRgbColorSpace::srgb();
    let rgb_color = CalibratedColor::cal_rgb([0.8, 0.4, 0.2], rgb_cs.clone());

    assert_eq!(rgb_color.values(), vec![0.8, 0.4, 0.2]);
}

#[test]
fn test_calibrated_color_clamping() {
    let gray_cs = CalGrayColorSpace::new();
    let gray_color = CalibratedColor::cal_gray(1.5, gray_cs); // Over 1.0

    assert_eq!(gray_color.values(), vec![1.0]); // Should be clamped

    let rgb_cs = CalRgbColorSpace::new();
    let rgb_color = CalibratedColor::cal_rgb([-0.1, 0.5, 1.2], rgb_cs);

    assert_eq!(rgb_color.values(), vec![0.0, 0.5, 1.0]); // Clamped to [0,1]
}

#[test]
fn test_graphics_context_calibrated_colors() {
    let mut ctx = GraphicsContext::new();

    let gray_cs = CalGrayColorSpace::new().with_gamma(2.2);
    let gray_color = CalibratedColor::cal_gray(0.7, gray_cs);

    ctx.set_fill_color_calibrated(gray_color);

    let ops = ctx.operations();
    assert!(ops.contains("/CalGray1 cs"));
    assert!(ops.contains("0.7000"));
    assert!(ops.contains("sc"));
}

#[test]
fn test_graphics_context_calibrated_stroke() {
    let mut ctx = GraphicsContext::new();

    let rgb_cs = CalRgbColorSpace::srgb();
    let rgb_color = CalibratedColor::cal_rgb([0.8, 0.4, 0.2], rgb_cs);

    ctx.set_stroke_color_calibrated(rgb_color);

    let ops = ctx.operations();
    assert!(ops.contains("/CalRGB1 CS"));
    assert!(ops.contains("0.8000"));
    assert!(ops.contains("0.4000"));
    assert!(ops.contains("0.2000"));
    assert!(ops.contains("SC"));
}

#[test]
fn test_calibrated_color_space_arrays() {
    let gray_cs = CalGrayColorSpace::new().with_gamma(1.8);
    let gray_color = CalibratedColor::cal_gray(0.5, gray_cs);

    let cs_array = gray_color.color_space_array();
    assert_eq!(cs_array.len(), 2);

    let rgb_cs = CalRgbColorSpace::new();
    let rgb_color = CalibratedColor::cal_rgb([0.5, 0.5, 0.5], rgb_cs);

    let cs_array = rgb_color.color_space_array();
    assert_eq!(cs_array.len(), 2);
}

#[test]
fn test_d50_vs_d65_illuminants() {
    let d50 = CalGrayColorSpace::d50();
    let d65 = CalGrayColorSpace::d65();

    assert_ne!(d50.white_point, d65.white_point);

    // D50 values
    assert_eq!(d50.white_point, [0.9505, 1.0000, 1.0890]);
    // D65 values
    assert_eq!(d65.white_point, [0.9504, 1.0000, 1.0888]);
}

#[test]
fn test_cal_rgb_d65() {
    let cs = CalRgbColorSpace::d65();
    assert_eq!(cs.white_point, [0.9504, 1.0000, 1.0888]);
}

#[test]
fn test_minimal_pdf_array_generation() {
    // CalGray with only gamma different from default
    let cs = CalGrayColorSpace::new().with_gamma(1.8);
    let array = cs.to_pdf_array();

    assert_eq!(array.len(), 2);

    if let Object::Dictionary(dict) = &array[1] {
        assert!(dict.get("WhitePoint").is_some());
        assert!(dict.get("Gamma").is_some());
        assert!(dict.get("BlackPoint").is_none()); // Default, shouldn't be included
    }
}

#[test]
fn test_identity_matrix_not_included() {
    // CalRGB with identity matrix shouldn't include Matrix entry
    let cs = CalRgbColorSpace::new(); // Has identity matrix by default
    let array = cs.to_pdf_array();

    if let Object::Dictionary(dict) = &array[1] {
        assert!(dict.get("WhitePoint").is_some());
        assert!(dict.get("Gamma").is_none()); // All 1.0, shouldn't be included
        assert!(dict.get("Matrix").is_none()); // Identity, shouldn't be included
        assert!(dict.get("BlackPoint").is_none()); // Default, shouldn't be included
    }
}

#[test]
fn test_floating_point_precision() {
    let cs = CalGrayColorSpace::new().with_gamma(2.199999);
    let corrected = cs.apply_gamma(0.5);

    // Should handle floating point precision appropriately
    let expected = 0.5_f64.powf(2.199999);
    assert!((corrected - expected).abs() < 1e-10);
}

#[test]
fn test_edge_case_gamma_values() {
    // Test very small gamma
    let cs = CalGrayColorSpace::new().with_gamma(0.001);
    let corrected = cs.apply_gamma(0.5);
    assert!(corrected > 0.0 && corrected <= 1.0);

    // Test large gamma
    let cs = CalGrayColorSpace::new().with_gamma(10.0);
    let corrected = cs.apply_gamma(0.9);
    assert!(corrected >= 0.0 && corrected <= 1.0);
}

#[test]
fn test_extreme_color_values() {
    let cs = CalGrayColorSpace::new();

    // Test boundary values
    assert_eq!(cs.apply_gamma(0.0), 0.0);
    assert_eq!(cs.apply_gamma(1.0), 1.0);

    // Test clamping behavior
    let cs_rgb = CalRgbColorSpace::new();
    let corrected = cs_rgb.apply_gamma([0.0, 0.5, 1.0]);
    assert_eq!(corrected[0], 0.0);
    assert_eq!(corrected[1], 0.5);
    assert_eq!(corrected[2], 1.0);
}
