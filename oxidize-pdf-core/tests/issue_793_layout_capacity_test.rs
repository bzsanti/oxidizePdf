use oxidize_pdf::{
    coordinate_system::TransformMatrix, page::LayoutManager, CoordinateSystem, Page,
};

fn page() -> Page {
    let mut page = Page::new(200.0, 200.0);
    page.set_margins(10.0, 10.0, 10.0, 10.0);
    page
}

#[test]
fn identity_exhausts_resets_and_refuses_without_mutation() {
    for system in [
        CoordinateSystem::PdfStandard,
        CoordinateSystem::Custom(TransformMatrix::IDENTITY),
    ] {
        let mut layout = LayoutManager::new(&page(), system);
        assert_eq!(layout.remaining_space(), 180.0);
        assert_eq!(layout.add_element(80.0), Some(190.0));
        assert_eq!(layout.remaining_space(), 90.0);
        assert!(layout.will_fit(80.0));
        assert_eq!(layout.add_element(80.0), Some(100.0));
        assert_eq!(layout.current_y, 10.0);
        assert_eq!(layout.remaining_space(), 0.0);
        for _ in 0..4 {
            assert!(!layout.will_fit(80.0));
            assert_eq!(layout.add_element(80.0), None);
            assert_eq!(layout.current_y, 10.0);
        }
        layout.new_page();
        assert_eq!(layout.add_element(170.0), Some(190.0));
        assert_eq!(layout.current_y, 10.0);
    }
}

#[test]
fn screen_and_custom_flip_have_the_same_top_anchor() {
    for system in [
        CoordinateSystem::ScreenSpace,
        CoordinateSystem::Custom(TransformMatrix::flip_y(200.0)),
    ] {
        let mut layout = LayoutManager::new(&page(), system);
        assert_eq!(layout.add_element(40.0), Some(10.0));
        assert_eq!(layout.current_y, 60.0);
        assert_eq!(layout.remaining_space(), 130.0);
        assert_eq!(layout.add_element(120.0), Some(60.0));
        assert_eq!(layout.add_element(1.0), None);
        layout.new_page();
        assert_eq!(layout.current_y, 10.0);
    }
}

#[test]
fn scaled_translated_custom_coordinates_use_local_units() {
    // PDF bounds x=[10,190], y=[10,190] map to local x=[0,90], y=[-10,80].
    let matrix = TransformMatrix::new(2.0, 0.0, 0.0, 2.0, 10.0, 30.0);
    let mut layout = LayoutManager::new(&page(), CoordinateSystem::Custom(matrix));
    assert_eq!(layout.left_x(), 0.0);
    assert_eq!(layout.center_x(20.0), 35.0);
    assert_eq!(layout.right_x(20.0), 70.0);
    assert_eq!(layout.remaining_space(), 90.0);
    assert_eq!(layout.add_element(40.0), Some(80.0));
    assert_eq!(layout.current_y, 30.0);
    assert_eq!(layout.remaining_space(), 40.0);
    assert_eq!(layout.add_element(30.0), Some(30.0));
    assert_eq!(layout.add_element(1.0), None);
    layout.new_page();
    assert_eq!(layout.current_y, 80.0);
}

#[test]
fn margins_spacing_boundary_and_overflow_are_consistent() {
    let mut p = page();
    p.set_margins(20.0, 30.0, 40.0, 50.0);
    for system in [
        CoordinateSystem::PdfStandard,
        CoordinateSystem::ScreenSpace,
        CoordinateSystem::Custom(TransformMatrix::IDENTITY),
    ] {
        let mut layout = LayoutManager::new(&p, system).with_element_spacing(5.0);
        let before = layout.current_y;
        assert_eq!(layout.remaining_space(), 110.0);
        assert!(!layout.will_fit(105.001));
        assert_eq!(layout.add_element(105.001), None);
        assert_eq!(layout.current_y, before);
        assert!(layout.will_fit(105.0));
        assert_eq!(layout.add_element(105.0), Some(before));
        assert_eq!(layout.remaining_space(), 0.0);
    }
}

#[test]
fn real_table_consumer_uses_top_not_bottom_or_spacing_offset() {
    use oxidize_pdf::advanced_tables::{AdvancedTableBuilder, AdvancedTableExt, TableRenderer};
    let mut p = page();
    let table = AdvancedTableBuilder::new()
        .add_column("Header", 80.0)
        .show_header(false)
        .table_border(false)
        .add_row_with_min_height(vec!["Cell"], 40.0)
        .build()
        .unwrap();
    let height = TableRenderer::new().calculate_table_height(&table);
    assert_eq!(height, 40.0);
    let mut layout = LayoutManager::new(&p, CoordinateSystem::PdfStandard);
    let y = layout.add_element(height).unwrap();
    assert_eq!(y, 190.0);
    let bottom = p.add_advanced_table(&table, layout.left_x(), y).unwrap();
    assert_eq!(bottom, 190.0 - height);
    assert_eq!(layout.current_y, bottom - 10.0);
}

#[test]
fn reflections_preserve_physical_usable_bounds() {
    let matrix = TransformMatrix::new(-2.0, 0.0, 0.0, -2.0, 200.0, 200.0);
    let mut layout = LayoutManager::try_new(&page(), CoordinateSystem::Custom(matrix)).unwrap();
    // Physical [10,190] on each axis is local [5,95]. Physical top is local y=5.
    assert_eq!(layout.left_x(), 5.0);
    assert_eq!(layout.right_x(20.0), 75.0);
    assert_eq!(layout.center_x(20.0), 40.0);
    assert_eq!(layout.add_element(40.0), Some(5.0));
    assert_eq!(layout.current_y, 55.0);
    assert_eq!(layout.add_element(30.0), Some(55.0));
    assert_eq!(layout.current_y, 95.0);
    assert_eq!(layout.add_element(1.0), None);
    layout.new_page();
    assert_eq!(layout.current_y, 5.0);
}

#[test]
fn unsupported_transforms_are_explicit_and_have_no_capacity() {
    for m in [
        TransformMatrix::rotate(std::f64::consts::FRAC_PI_2),
        TransformMatrix::new(1.0, 0.1, 0.0, 1.0, 0.0, 0.0),
        TransformMatrix::new(1.0, 0.0, 0.1, 1.0, 0.0, 0.0),
        TransformMatrix::scale(0.0, 1.0),
        TransformMatrix::scale(1.0, 0.0),
        TransformMatrix::translate(f64::NAN, 0.0),
        TransformMatrix::scale(1.0, f64::INFINITY),
        TransformMatrix::scale(1.0, f64::MIN_POSITIVE),
    ] {
        let system = CoordinateSystem::Custom(m);
        assert!(LayoutManager::try_new(&page(), system).is_err(), "{m:?}");
        let mut layout = LayoutManager::new(&page(), system);
        assert_eq!(layout.remaining_space(), 0.0);
        assert_eq!(layout.add_element(1.0), None);
        assert_eq!(layout.current_y, 0.0);
        assert!(layout.left_x().is_nan());
    }
}

#[test]
fn invalid_reservations_and_public_state_do_not_advance() {
    let mut layout = LayoutManager::new(&page(), CoordinateSystem::PdfStandard);
    for height in [-1.0, 0.0, f64::NAN, f64::INFINITY, f64::MAX] {
        assert!(!layout.will_fit(height));
        assert_eq!(layout.add_element(height), None);
        assert_eq!(layout.current_y, 190.0);
    }
    for spacing in [-1.0, f64::NAN, f64::INFINITY] {
        layout.element_spacing = spacing;
        assert_eq!(layout.remaining_space(), 0.0);
        assert_eq!(layout.add_element(40.0), None);
        assert_eq!(layout.current_y, 190.0);
    }
    layout.element_spacing = 0.0;
    assert_eq!(
        layout.add_element(f64::MIN_POSITIVE),
        None,
        "rounded-away reservations must not pretend to advance"
    );
    assert_eq!(layout.add_element(180.0), Some(190.0));
    layout.current_y = 201.0;
    assert_eq!(layout.remaining_space(), 0.0);
    assert_eq!(layout.add_element(40.0), None);
    assert_eq!(layout.current_y, 201.0);
    layout.margins.top = 210.0;
    layout.new_page();
    assert_eq!(layout.add_element(40.0), None);
    layout.margins.top = 20.0;
    layout.coordinate_system = CoordinateSystem::ScreenSpace;
    layout.new_page();
    assert_eq!(layout.add_element(40.0), Some(20.0));
    assert_eq!(layout.current_y, 60.0);
}

#[test]
fn converted_local_anchors_place_real_tables_at_the_same_physical_top() {
    use oxidize_pdf::advanced_tables::{AdvancedTableBuilder, AdvancedTableExt, TableRenderer};
    let table = AdvancedTableBuilder::new()
        .add_column("Header", 80.0)
        .show_header(false)
        .table_border(false)
        .add_row_with_min_height(vec!["Cell"], 40.0)
        .build()
        .unwrap();
    let pdf_height = TableRenderer::new().calculate_table_height(&table);
    assert_eq!(pdf_height, 40.0);
    for (system, d, f, expected_y) in [
        (CoordinateSystem::ScreenSpace, -1.0, 200.0, 10.0),
        (
            CoordinateSystem::Custom(TransformMatrix::new(1.0, 0.0, 0.0, 2.0, 0.0, 30.0)),
            2.0,
            30.0,
            80.0,
        ),
        (
            CoordinateSystem::Custom(TransformMatrix::new(1.0, 0.0, 0.0, -2.0, 0.0, 200.0)),
            -2.0,
            200.0,
            5.0,
        ),
    ] {
        let mut p = page();
        let mut layout = LayoutManager::new(&p, system);
        let local_height = pdf_height / f64::abs(d);
        let y = layout.add_element(local_height).unwrap();
        assert_eq!(y, expected_y);
        // Explicit affine geometry, not LayoutManager's private bounds calculation.
        let bottom = p.add_advanced_table(&table, 10.0, d * y + f).unwrap();
        assert_eq!(bottom, 190.0 - pdf_height);
        assert_eq!(d * layout.current_y + f, bottom - 10.0 * f64::abs(d));
    }
}
