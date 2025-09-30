//! Simple ScatterPlot Test Example
//!
//! Tests the ScatterPlot component in isolation

use oxidize_pdf::{
    dashboard::{
        DashboardComponent, ComponentPosition, DashboardTheme,
        ScatterPlot, ScatterPoint, ScatterPlotOptions
    },
    graphics::Color,
    Document, Page, Result,
};

fn main() -> Result<()> {
    println!("📈 Testing ScatterPlot component...");

    // Create sample scatter plot data - correlation between advertising spend and sales
    let mut data = vec![];

    // Generate correlated data points with some noise
    for i in 0..30 {
        let x = 10.0 + i as f64 * 3.0; // Advertising spend (thousands)
        let y = 50.0 + x * 2.5 + (i % 7) as f64 * 15.0 - 25.0; // Sales (thousands) with noise

        let point = ScatterPoint {
            x,
            y,
            size: Some(4.0),
            color: Some(Color::hex("#007bff")),
            label: None,
        };

        data.push(point);
    }

    // Add some outliers
    data.push(ScatterPoint {
        x: 50.0,
        y: 180.0,
        size: Some(5.0),
        color: Some(Color::hex("#ff0000")),
        label: Some("Outlier".to_string()),
    });

    data.push(ScatterPoint {
        x: 80.0,
        y: 250.0,
        size: Some(5.0),
        color: Some(Color::hex("#ff0000")),
        label: Some("Outlier".to_string()),
    });

    // Create scatter plot with options
    let options = ScatterPlotOptions {
        title: Some("Advertising Spend vs Sales".to_string()),
        x_label: Some("Ad Spend ($1000s)".to_string()),
        y_label: Some("Sales ($1000s)".to_string()),
        show_trend_line: false,
    };

    let scatter = ScatterPlot::new(data).with_options(options);

    // Create PDF
    let mut document = Document::new();
    document.set_title("ScatterPlot Test");

    let mut page = Page::a4();

    // Render scatter plot
    let position = ComponentPosition {
        x: 50.0,
        y: 100.0,
        width: 500.0,
        height: 400.0,
    };

    let theme = DashboardTheme::default();

    match scatter.render(&mut page, position, &theme) {
        Ok(_) => println!("✅ ScatterPlot rendered successfully"),
        Err(e) => println!("❌ Error rendering ScatterPlot: {}", e),
    }

    document.add_page(page);

    let output_path = "examples/results/scatter_plot_simple_test.pdf";
    document.save(output_path)?;

    println!("📄 Saved to: {}", output_path);

    Ok(())
}
