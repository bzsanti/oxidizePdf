//! Tests for DeviceN color space implementation (ISO 32000-1 §8.6.6.5)

use oxidize_pdf::graphics::{
    ColorantDefinition, ColorantType, DeviceNAlternateColorSpace, DeviceNAttributes,
    DeviceNColorSpace, LinearTransform, SampledFunction, TintTransformFunction,
};
use std::collections::HashMap;

#[test]
fn test_devicen_basic_creation() {
    let colorants = vec![
        "Cyan".to_string(),
        "Magenta".to_string(),
        "Yellow".to_string(),
        "Black".to_string(),
    ];

    let transform = TintTransformFunction::Linear(LinearTransform {
        matrix: vec![
            vec![1.0, 0.0, 0.0, 0.0], // Cyan
            vec![0.0, 1.0, 0.0, 0.0], // Magenta
            vec![0.0, 0.0, 1.0, 0.0], // Yellow
            vec![0.0, 0.0, 0.0, 1.0], // Black
        ],
        black_generation: None,
        undercolor_removal: None,
    });

    let devicen = DeviceNColorSpace::new(
        colorants.clone(),
        DeviceNAlternateColorSpace::DeviceCMYK,
        transform,
    );

    assert_eq!(devicen.colorant_count(), 4);
    assert_eq!(devicen.colorant_name(0), Some("Cyan"));
    assert_eq!(devicen.colorant_name(1), Some("Magenta"));
    assert_eq!(devicen.colorant_name(2), Some("Yellow"));
    assert_eq!(devicen.colorant_name(3), Some("Black"));
    assert!(devicen.has_process_colors());
    assert!(devicen.spot_color_names().is_empty());
}

#[test]
fn test_devicen_cmyk_plus_spots() {
    let spot_colors = vec!["PANTONE 185 C".to_string(), "PANTONE 286 C".to_string()];

    let devicen = DeviceNColorSpace::cmyk_plus_spots(spot_colors.clone());

    assert_eq!(devicen.colorant_count(), 6); // 4 CMYK + 2 spots
    assert!(devicen.has_process_colors());

    let spots = devicen.spot_color_names();
    assert_eq!(spots.len(), 2);
    assert!(spots.contains(&"PANTONE 185 C"));
    assert!(spots.contains(&"PANTONE 286 C"));

    // Test CMYK colorants are present
    assert!(devicen.colorant_name(0) == Some("Cyan"));
    assert!(devicen.colorant_name(1) == Some("Magenta"));
    assert!(devicen.colorant_name(2) == Some("Yellow"));
    assert!(devicen.colorant_name(3) == Some("Black"));
}

#[test]
fn test_devicen_linear_transform() {
    let colorants = vec!["Cyan".to_string(), "Magenta".to_string()];

    let transform = TintTransformFunction::Linear(LinearTransform {
        matrix: vec![
            vec![1.0, 0.0, 0.0], // Cyan -> RGB
            vec![0.0, 1.0, 0.0], // Magenta -> RGB
        ],
        black_generation: None,
        undercolor_removal: None,
    });

    let devicen =
        DeviceNColorSpace::new(colorants, DeviceNAlternateColorSpace::DeviceRGB, transform);

    // Test conversion
    let input = vec![0.5, 0.8]; // 50% cyan, 80% magenta
    let result = devicen.convert_to_alternate(&input).unwrap();

    assert_eq!(result.len(), 3); // RGB output
    assert!((result[0] - 0.5).abs() < 1e-10); // Red from cyan
    assert!((result[1] - 0.8).abs() < 1e-10); // Green from magenta
    assert!((result[2] - 0.0).abs() < 1e-10); // Blue zero
}

#[test]
fn test_devicen_transform_validation() {
    let colorants = vec!["Cyan".to_string(), "Magenta".to_string()];

    let transform = TintTransformFunction::Linear(LinearTransform {
        matrix: vec![
            vec![0.5], // Cyan contribution to gray
            vec![0.3], // Magenta contribution to gray
        ],
        black_generation: None,
        undercolor_removal: None,
    });

    let devicen =
        DeviceNColorSpace::new(colorants, DeviceNAlternateColorSpace::DeviceGray, transform);

    // Test wrong input size
    let wrong_input = vec![0.5]; // Only 1 value for 2 colorants
    let result = devicen.convert_to_alternate(&wrong_input);
    assert!(result.is_err());

    // Test correct input
    let correct_input = vec![0.5, 0.3];
    let result = devicen.convert_to_alternate(&correct_input);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1); // Grayscale output
}

#[test]
fn test_devicen_sampled_function() {
    let colorants = vec!["Special1".to_string()];

    let sampled = SampledFunction {
        domain: vec![(0.0, 1.0)],
        range: vec![(0.0, 1.0), (0.0, 1.0), (0.0, 1.0)], // RGB
        size: vec![3],                                   // 3 samples
        samples: vec![
            255, 0, 0, // Red at 0.0
            0, 255, 0, // Green at 0.5
            0, 0, 255, // Blue at 1.0
        ],
        bits_per_sample: 8,
        order: 1,
    };

    let devicen = DeviceNColorSpace::new(
        colorants,
        DeviceNAlternateColorSpace::DeviceRGB,
        TintTransformFunction::Sampled(sampled),
    );

    // Test sampling
    let input = vec![0.0]; // First sample
    let result = devicen.convert_to_alternate(&input).unwrap();
    assert_eq!(result.len(), 3);
    assert!((result[0] - 1.0).abs() < 0.01); // Red
    assert!(result[1] < 0.01); // No green
    assert!(result[2] < 0.01); // No blue
}

#[test]
fn test_devicen_with_attributes() {
    let colorants = vec!["Cyan".to_string(), "PANTONE 185 C".to_string()];

    let mut colorant_defs = HashMap::new();
    colorant_defs.insert(
        "PANTONE 185 C".to_string(),
        ColorantDefinition::spot("PANTONE 185 C", [0.0, 1.0, 1.0, 0.0]), // Bright red
    );
    colorant_defs.insert(
        "Cyan".to_string(),
        ColorantDefinition::process([1.0, 0.0, 0.0, 0.0]),
    );

    let attributes = DeviceNAttributes {
        colorants: colorant_defs,
        process: Some("DeviceCMYK".to_string()),
        mix: None,
        dot_gain: HashMap::new(),
    };

    let transform = TintTransformFunction::Linear(LinearTransform {
        matrix: vec![
            vec![1.0, 0.0, 0.0, 0.0], // Cyan pass-through
            vec![0.0, 1.0, 1.0, 0.0], // Spot to MY
        ],
        black_generation: None,
        undercolor_removal: None,
    });

    let devicen =
        DeviceNColorSpace::new(colorants, DeviceNAlternateColorSpace::DeviceCMYK, transform)
            .with_attributes(attributes);

    assert!(devicen.attributes.is_some());
    let attrs = devicen.attributes.as_ref().unwrap();
    assert_eq!(attrs.process, Some("DeviceCMYK".to_string()));
    assert_eq!(attrs.colorants.len(), 2);
}

#[test]
fn test_devicen_colorant_definitions() {
    // Test process colorant
    let process_def = ColorantDefinition::process([0.8, 0.2, 0.0, 0.1]);
    assert_eq!(process_def.colorant_type, ColorantType::Process);
    assert_eq!(process_def.cmyk_equivalent, Some([0.8, 0.2, 0.0, 0.1]));
    assert!(process_def.rgb_approximation.is_some());

    // Test spot colorant
    let spot_def = ColorantDefinition::spot("Gold", [0.2, 0.3, 0.8, 0.1]);
    assert_eq!(spot_def.colorant_type, ColorantType::Spot);
    assert_eq!(spot_def.cmyk_equivalent, Some([0.2, 0.3, 0.8, 0.1]));

    // Test special effect colorant
    let special_def = ColorantDefinition::special_effect([0.9, 0.9, 0.8]);
    assert_eq!(special_def.colorant_type, ColorantType::Special);
    assert_eq!(special_def.rgb_approximation, Some([0.9, 0.9, 0.8]));
    assert_eq!(special_def.density, Some(0.5));
}

#[test]
fn test_devicen_alternate_color_spaces() {
    let colorants = vec!["Test".to_string()];

    // Test RGB alternate
    let rgb_devicen = DeviceNColorSpace::new(
        colorants.clone(),
        DeviceNAlternateColorSpace::DeviceRGB,
        TintTransformFunction::Linear(LinearTransform {
            matrix: vec![vec![0.5, 0.3, 0.8]],
            black_generation: None,
            undercolor_removal: None,
        }),
    );

    let result = rgb_devicen.convert_to_alternate(&[1.0]).unwrap();
    assert_eq!(result.len(), 3); // RGB

    // Test CMYK alternate
    let cmyk_devicen = DeviceNColorSpace::new(
        colorants.clone(),
        DeviceNAlternateColorSpace::DeviceCMYK,
        TintTransformFunction::Linear(LinearTransform {
            matrix: vec![vec![0.2, 0.4, 0.6, 0.8]],
            black_generation: None,
            undercolor_removal: None,
        }),
    );

    let result = cmyk_devicen.convert_to_alternate(&[1.0]).unwrap();
    assert_eq!(result.len(), 4); // CMYK

    // Test Gray alternate
    let gray_devicen = DeviceNColorSpace::new(
        colorants,
        DeviceNAlternateColorSpace::DeviceGray,
        TintTransformFunction::Linear(LinearTransform {
            matrix: vec![vec![0.7]],
            black_generation: None,
            undercolor_removal: None,
        }),
    );

    let result = gray_devicen.convert_to_alternate(&[1.0]).unwrap();
    assert_eq!(result.len(), 1); // Gray
    assert!((result[0] - 0.7).abs() < 1e-10);
}

#[test]
fn test_devicen_clamping() {
    let colorants = vec!["Test".to_string()];

    let transform = TintTransformFunction::Linear(LinearTransform {
        matrix: vec![vec![2.0, -0.5, 1.5]], // Values that exceed [0,1]
        black_generation: None,
        undercolor_removal: None,
    });

    let devicen =
        DeviceNColorSpace::new(colorants, DeviceNAlternateColorSpace::DeviceRGB, transform);

    let result = devicen.convert_to_alternate(&[1.0]).unwrap();

    // Values should be clamped to [0, 1]
    assert!(result[0] >= 0.0 && result[0] <= 1.0);
    assert!(result[1] >= 0.0 && result[1] <= 1.0);
    assert!(result[2] >= 0.0 && result[2] <= 1.0);

    assert_eq!(result[0], 1.0); // 2.0 clamped to 1.0
    assert_eq!(result[1], 0.0); // -0.5 clamped to 0.0
    assert_eq!(result[2], 1.0); // 1.5 clamped to 1.0
}

#[test]
fn test_devicen_pdf_object_creation() {
    let colorants = vec!["Cyan".to_string(), "PANTONE 185 C".to_string()];

    let transform = TintTransformFunction::Linear(LinearTransform {
        matrix: vec![vec![1.0, 0.0, 0.0, 0.0], vec![0.0, 1.0, 1.0, 0.0]],
        black_generation: None,
        undercolor_removal: None,
    });

    let devicen =
        DeviceNColorSpace::new(colorants, DeviceNAlternateColorSpace::DeviceCMYK, transform);

    let pdf_obj = devicen.to_pdf_object();

    // Should be an array representing the DeviceN color space
    match pdf_obj {
        oxidize_pdf::objects::Object::Array(arr) => {
            assert!(arr.len() >= 4); // [DeviceN, names, alternate, function, ...]

            // First element should be /DeviceN
            if let Some(oxidize_pdf::objects::Object::Name(name)) = arr.get(0) {
                assert_eq!(name, "DeviceN");
            } else {
                panic!("First element should be /DeviceN name");
            }

            // Second element should be names array
            if let Some(oxidize_pdf::objects::Object::Array(names)) = arr.get(1) {
                assert_eq!(names.len(), 2);
            } else {
                panic!("Second element should be names array");
            }
        }
        _ => panic!("DeviceN PDF object should be an array"),
    }
}

#[test]
fn test_devicen_function_fallbacks() {
    let colorants = vec!["Special".to_string()];

    // Test PostScript function fallback
    let ps_devicen = DeviceNColorSpace::new(
        colorants.clone(),
        DeviceNAlternateColorSpace::DeviceRGB,
        TintTransformFunction::Function(vec![0x7B, 0x30, 0x7D]), // Simple PS: {0}
    );

    let result = ps_devicen.convert_to_alternate(&[0.5]).unwrap();
    assert_eq!(result.len(), 3); // Should fall back to linear approximation

    // Test with grayscale alternate
    let gray_devicen = DeviceNColorSpace::new(
        colorants,
        DeviceNAlternateColorSpace::DeviceGray,
        TintTransformFunction::Function(vec![0x7B, 0x30, 0x7D]),
    );

    let result = gray_devicen.convert_to_alternate(&[0.8]).unwrap();
    assert_eq!(result.len(), 1);
    assert!((result[0] - 0.8).abs() < 1e-10); // Should use input value
}

#[test]
fn test_devicen_multi_ink_printing() {
    // Simulate a 6-color printing system (CMYK + Orange + Green)
    let colorants = vec![
        "Cyan".to_string(),
        "Magenta".to_string(),
        "Yellow".to_string(),
        "Black".to_string(),
        "PANTONE Orange 021 C".to_string(),
        "PANTONE Green C".to_string(),
    ];

    // Create transform matrix for 6->4 conversion (DeviceN to CMYK)
    let matrix = vec![
        vec![1.0, 0.0, 0.0, 0.0], // Cyan pass-through
        vec![0.0, 1.0, 0.0, 0.0], // Magenta pass-through
        vec![0.0, 0.0, 1.0, 0.0], // Yellow pass-through
        vec![0.0, 0.0, 0.0, 1.0], // Black pass-through
        vec![0.0, 0.5, 1.0, 0.0], // Orange -> MY
        vec![0.8, 0.0, 1.0, 0.0], // Green -> CY
    ];

    let devicen = DeviceNColorSpace::new(
        colorants,
        DeviceNAlternateColorSpace::DeviceCMYK,
        TintTransformFunction::Linear(LinearTransform {
            matrix,
            black_generation: None,
            undercolor_removal: None,
        }),
    );

    // Test pure orange ink
    let orange_input = vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let result = devicen.convert_to_alternate(&orange_input).unwrap();

    assert_eq!(result.len(), 4);
    assert_eq!(result[0], 0.0); // No cyan from orange
    assert_eq!(result[1], 0.5); // Some magenta from orange
    assert_eq!(result[2], 1.0); // Full yellow from orange
    assert_eq!(result[3], 0.0); // No black from orange

    // Test mixed colors
    let mixed_input = vec![0.2, 0.3, 0.4, 0.1, 0.5, 0.2]; // All colors
    let result = devicen.convert_to_alternate(&mixed_input).unwrap();

    // Should be sum of contributions, clamped to [0,1]
    let expected_c = 0.2 + 0.8 * 0.2; // Cyan + Green contribution
    let expected_m = 0.3 + 0.5 * 0.5; // Magenta + Orange contribution
    let _expected_y = 0.4 + 1.0 * 0.5 + 1.0 * 0.2; // Yellow + Orange + Green (clamped)
    let expected_k = 0.1; // Black only

    assert!((result[0] - expected_c).abs() < 1e-10);
    assert!((result[1] - expected_m).abs() < 1e-10);
    assert_eq!(result[2], 1.0); // Should be clamped to 1.0
    assert!((result[3] - expected_k).abs() < 1e-10);
}

#[test]
fn test_devicen_edge_cases() {
    // Test empty colorants (should still work)
    let empty_devicen = DeviceNColorSpace::new(
        vec![],
        DeviceNAlternateColorSpace::DeviceGray,
        TintTransformFunction::Linear(LinearTransform {
            matrix: vec![],
            black_generation: None,
            undercolor_removal: None,
        }),
    );

    assert_eq!(empty_devicen.colorant_count(), 0);
    assert!(!empty_devicen.has_process_colors());
    assert!(empty_devicen.spot_color_names().is_empty());

    // Test single colorant
    let single_devicen = DeviceNColorSpace::new(
        vec!["OnlyColor".to_string()],
        DeviceNAlternateColorSpace::DeviceGray,
        TintTransformFunction::Linear(LinearTransform {
            matrix: vec![vec![1.0]],
            black_generation: None,
            undercolor_removal: None,
        }),
    );

    assert_eq!(single_devicen.colorant_count(), 1);
    assert_eq!(single_devicen.colorant_name(0), Some("OnlyColor"));
    assert_eq!(single_devicen.colorant_name(1), None); // Out of bounds
}
