//! Unit tests for charts functionality

use oxidize_pdf::charts::{
    BarChartBuilder, BarOrientation, ChartBuilder, ChartData, ChartType, DataSeries,
    LegendPosition, LineChartBuilder, PieChartBuilder, PieSegment,
};
use oxidize_pdf::graphics::Color;

#[test]
fn test_bar_chart_creation() {
    let chart = BarChartBuilder::new()
        .title("Test Bar Chart")
        .simple_data(vec![10.0, 20.0, 30.0, 40.0])
        .build();

    assert_eq!(chart.title, "Test Bar Chart");
    assert_eq!(chart.data.len(), 4);
    assert_eq!(chart.max_value(), 40.0);
    assert_eq!(chart.min_value(), 0.0); // Always includes 0 for bar charts
}

#[test]
fn test_bar_chart_with_labels() {
    let chart = BarChartBuilder::new()
        .labeled_data(vec![
            ("Q1", 100.0),
            ("Q2", 150.0),
            ("Q3", 125.0),
            ("Q4", 180.0),
        ])
        .build();

    assert_eq!(chart.data.len(), 4);
    assert_eq!(chart.data[0].label, "Q1");
    assert_eq!(chart.data[0].value, 100.0);
    assert_eq!(chart.data[3].label, "Q4");
    assert_eq!(chart.data[3].value, 180.0);
}

#[test]
fn test_bar_chart_orientations() {
    let vertical = BarChartBuilder::new()
        .orientation(BarOrientation::Vertical)
        .simple_data(vec![1.0, 2.0])
        .build();

    assert_eq!(vertical.orientation, BarOrientation::Vertical);

    let horizontal = BarChartBuilder::new()
        .orientation(BarOrientation::Horizontal)
        .simple_data(vec![1.0, 2.0])
        .build();

    assert_eq!(horizontal.orientation, BarOrientation::Horizontal);
}

#[test]
fn test_bar_chart_colors() {
    let custom_colors = vec![Color::red(), Color::green(), Color::blue()];
    let chart = BarChartBuilder::new()
        .colors(custom_colors.clone())
        .simple_data(vec![1.0, 2.0, 3.0])
        .build();

    assert_eq!(chart.colors, custom_colors);
    assert_eq!(chart.color_for_index(0), Color::red());
    assert_eq!(chart.color_for_index(1), Color::green());
    assert_eq!(chart.color_for_index(2), Color::blue());
    // Should cycle for indices beyond array length
    assert_eq!(chart.color_for_index(3), Color::red());
}

#[test]
fn test_bar_chart_styles() {
    let chart = BarChartBuilder::new()
        .financial_style()
        .simple_data(vec![100.0, -50.0, 75.0])
        .build();

    assert!(chart.show_grid);
    assert!(chart.bar_border_color.is_some());

    let minimal = BarChartBuilder::new()
        .minimal_style()
        .simple_data(vec![1.0, 2.0])
        .build();

    assert!(!minimal.show_grid);
    assert!(!minimal.show_values);
    assert!(minimal.bar_border_color.is_none());
}

#[test]
fn test_pie_chart_creation() {
    let chart = PieChartBuilder::new()
        .title("Test Pie Chart")
        .simple_data(vec![25.0, 35.0, 40.0])
        .build();

    assert_eq!(chart.title, "Test Pie Chart");
    assert_eq!(chart.segments.len(), 3);
    assert_eq!(chart.total_value(), 100.0);
}

#[test]
fn test_pie_segment_creation() {
    let segment = PieSegment::new("Test Segment", 50.0, Color::blue())
        .exploded(0.2)
        .show_percentage(false);

    assert_eq!(segment.label, "Test Segment");
    assert_eq!(segment.value, 50.0);
    assert_eq!(segment.color, Color::blue());
    assert!(segment.exploded);
    assert_eq!(segment.explosion_distance, 0.2);
    assert!(!segment.show_percentage);
}

#[test]
fn test_pie_chart_percentages() {
    let chart = PieChartBuilder::new()
        .labeled_data(vec![("A", 25.0), ("B", 25.0), ("C", 50.0)])
        .build();

    assert_eq!(chart.percentage_for_index(0), 25.0);
    assert_eq!(chart.percentage_for_index(1), 25.0);
    assert_eq!(chart.percentage_for_index(2), 50.0);
}

#[test]
fn test_pie_chart_angles() {
    let chart = PieChartBuilder::new().simple_data(vec![50.0, 50.0]).build();

    let angles = chart.cumulative_angles();
    assert_eq!(angles.len(), 2);

    // Each segment should be π radians (half circle)
    let segment_angle = chart.segments[0].angle_radians(chart.total_value());
    assert!((segment_angle - std::f64::consts::PI).abs() < 0.001);
}

#[test]
fn test_line_chart_creation() {
    let chart = LineChartBuilder::new()
        .title("Test Line Chart")
        .axis_labels("X Axis", "Y Axis")
        .add_simple_series("Series 1", vec![1.0, 2.0, 3.0, 4.0], Color::blue())
        .build();

    assert_eq!(chart.title, "Test Line Chart");
    assert_eq!(chart.x_axis_label, "X Axis");
    assert_eq!(chart.y_axis_label, "Y Axis");
    assert_eq!(chart.series.len(), 1);
    assert_eq!(chart.series[0].name, "Series 1");
}

#[test]
fn test_data_series_creation() {
    let series = DataSeries::new("Test Series", Color::red())
        .y_data(vec![10.0, 20.0, 30.0])
        .line_style(3.0)
        .markers(true, 5.0);

    assert_eq!(series.name, "Test Series");
    assert_eq!(series.color, Color::red());
    assert_eq!(series.data.len(), 3);
    assert_eq!(series.data[0], (0.0, 10.0));
    assert_eq!(series.data[2], (2.0, 30.0));
    assert_eq!(series.line_width, 3.0);
    assert!(series.show_markers);
    assert_eq!(series.marker_size, 5.0);
}

#[test]
fn test_data_series_xy_data() {
    let series = DataSeries::new("XY Series", Color::green()).xy_data(vec![
        (1.0, 10.0),
        (2.5, 20.0),
        (4.0, 15.0),
    ]);

    assert_eq!(series.data.len(), 3);
    assert_eq!(series.data[0], (1.0, 10.0));
    assert_eq!(series.data[1], (2.5, 20.0));
    assert_eq!(series.data[2], (4.0, 15.0));
}

#[test]
fn test_data_series_ranges() {
    let series = DataSeries::new("Range Test", Color::blue()).xy_data(vec![
        (0.0, 10.0),
        (5.0, 20.0),
        (10.0, 5.0),
    ]);

    let (min_x, max_x) = series.x_range();
    let (min_y, max_y) = series.y_range();

    assert_eq!(min_x, 0.0);
    assert_eq!(max_x, 10.0);
    assert_eq!(min_y, 5.0);
    assert_eq!(max_y, 20.0);
}

#[test]
fn test_line_chart_multiple_series() {
    let chart = LineChartBuilder::new()
        .add_simple_series("Series A", vec![1.0, 2.0, 3.0], Color::red())
        .add_simple_series("Series B", vec![3.0, 2.0, 1.0], Color::blue())
        .add_simple_series("Series C", vec![2.0, 2.0, 2.0], Color::green())
        .build();

    assert_eq!(chart.series.len(), 3);

    let (min_y, max_y) = chart.combined_y_range();
    assert!(min_y <= 1.0);
    assert!(max_y >= 3.0);
}

#[test]
fn test_generic_chart_builder() {
    let chart = ChartBuilder::new(ChartType::VerticalBar)
        .title("Generic Chart")
        .simple_data(vec![10.0, 20.0, 30.0])
        .legend_position(LegendPosition::Bottom)
        .show_values(true)
        .show_grid(true)
        .build();

    assert_eq!(chart.title, "Generic Chart");
    assert_eq!(chart.chart_type, ChartType::VerticalBar);
    assert_eq!(chart.data.len(), 3);
    assert_eq!(chart.legend_position, LegendPosition::Bottom);
    assert!(chart.show_values);
    assert!(chart.show_grid);
}

#[test]
fn test_chart_data_with_custom_color() {
    let data = ChartData::new("Custom", 42.0)
        .color(Color::rgb(0.5, 0.5, 0.5))
        .highlighted();

    assert_eq!(data.label, "Custom");
    assert_eq!(data.value, 42.0);
    assert_eq!(data.color, Some(Color::rgb(0.5, 0.5, 0.5)));
    assert!(data.highlighted);
}

#[test]
fn test_legend_positions() {
    assert_eq!(LegendPosition::None as i32, 0);
    assert_eq!(LegendPosition::Right as i32, 1);
    assert_eq!(LegendPosition::Bottom as i32, 2);
    assert_eq!(LegendPosition::Top as i32, 3);
    assert_eq!(LegendPosition::Left as i32, 4);
}

#[test]
fn test_bar_chart_width_calculation() {
    let chart = BarChartBuilder::new()
        .simple_data(vec![1.0, 2.0, 3.0])
        .bar_spacing(0.2)
        .bar_width_range(20.0, Some(100.0))
        .build();

    let width = chart.calculate_bar_width(400.0);
    assert!(width >= 20.0); // Min width
    assert!(width <= 100.0); // Max width
}

#[test]
fn test_pie_chart_with_exploded_segment() {
    let chart = PieChartBuilder::new()
        .add_segment(PieSegment::new("Normal", 50.0, Color::blue()))
        .add_segment(PieSegment::new("Exploded", 50.0, Color::red()).exploded(0.15))
        .build();

    assert!(!chart.segments[0].exploded);
    assert!(chart.segments[1].exploded);
    assert_eq!(chart.segments[1].explosion_distance, 0.15);
}

#[test]
fn test_line_chart_with_filled_area() {
    let series = DataSeries::new("Filled", Color::blue())
        .y_data(vec![1.0, 2.0, 3.0])
        .fill_area(Some(Color::rgb(0.8, 0.8, 1.0))); // Light blue instead of transparent

    assert!(series.fill_area);
    assert!(series.fill_color.is_some());
}
