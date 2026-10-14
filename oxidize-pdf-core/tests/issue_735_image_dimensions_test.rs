//! Dimension contracts for checked raw image constructors (#735).
//! Overflow probes have empty/tiny buffers. The pre-fix release probes below
//! wrap both pixel count and allocation capacities to tiny values, avoiding
//! multi-gigabyte allocations in the defective implementation.
use oxidize_pdf::graphics::Image;
use oxidize_pdf::PdfError;

fn assert_invalid(result: Result<Image, PdfError>) {
    assert!(
        matches!(result, Err(PdfError::InvalidImage(_))),
        "invalid dimensions or buffer must return InvalidImage, got {result:?}"
    );
}

#[test]
fn rgba_rejects_pixel_count_wraparound() {
    for (width, height, data) in [
        (65_536, 65_536, vec![]),
        (1_073_741_825, 4, vec![0; 16]),
        (u32::MAX, u32::MAX, vec![0; 4]),
    ] {
        assert_invalid(Image::from_rgba_data(data, width, height));
    }
}

// Exact reported dimensions: reject before reserving RGB/alpha buffers.
#[test]
fn rgba_rejects_component_count_overflow() {
    assert_invalid(Image::from_rgba_data(vec![], 65_536, 16_384));
}

#[test]
fn grayscale_rejects_pixel_count_wraparound() {
    for (width, height, data) in [
        (65_536, 65_536, vec![]),
        (1_073_741_825, 4, vec![0; 4]),
        (u32::MAX, u32::MAX, vec![0]),
    ] {
        assert_invalid(Image::from_gray_data(data, width, height));
    }
}

#[test]
fn zero_dimensions_are_invalid_even_with_an_empty_buffer() {
    for (width, height) in [(0, 0), (0, 1), (1, 0), (0, u32::MAX), (u32::MAX, 0)] {
        assert_invalid(Image::from_rgba_data(vec![], width, height));
        assert_invalid(Image::from_gray_data(vec![], width, height));
    }
}

#[test]
fn mismatched_buffers_are_invalid() {
    for data in [vec![], vec![1, 2, 3], vec![1, 2, 3, 4, 5]] {
        assert_invalid(Image::from_rgba_data(data, 1, 1));
    }
    for data in [vec![], vec![1], vec![1, 2, 3]] {
        assert_invalid(Image::from_gray_data(data, 2, 1));
    }
}

#[test]
fn valid_rgba_preserves_rgb_alpha_and_soft_mask_samples() {
    let image = Image::from_rgba_data(
        vec![
            255, 0, 7, 0, 8, 255, 9, 64, 10, 11, 255, 128, 12, 13, 14, 255,
        ],
        2,
        2,
    )
    .unwrap();
    assert_eq!(
        (image.width(), image.height(), image.bits_per_component()),
        (2, 2, 8)
    );
    assert_eq!(
        image.data(),
        &[255, 0, 7, 8, 255, 9, 10, 11, 255, 12, 13, 14]
    );
    assert_eq!(image.alpha_data(), Some([0, 64, 128, 255].as_slice()));
    let mask = image.soft_mask().expect("RGBA must retain its soft mask");
    assert_eq!(
        (mask.width(), mask.height(), mask.bits_per_component()),
        (2, 2, 8)
    );
    assert_eq!(mask.data(), &[0, 64, 128, 255]);
    assert!(!mask.has_transparency());
}

#[test]
fn valid_grayscale_preserves_dimensions_and_samples() {
    let image = Image::from_gray_data(vec![0, 1, 63, 127, 128, 255], 3, 2).unwrap();
    assert_eq!(
        (image.width(), image.height(), image.bits_per_component()),
        (3, 2, 8)
    );
    assert_eq!(image.data(), &[0, 1, 63, 127, 128, 255]);
    assert_eq!(image.alpha_data(), None);
    assert!(image.soft_mask().is_none());
    assert!(!image.has_transparency());
}
