//! Simple HeatMap Test Example
//!
//! Tests the HeatMap component in isolation

use oxidize_pdf::{
    dashboard::{ColorScale, HeatMap, HeatMapData},
    graphics::Color,
    Document, Page, Result,
};

fn main() -> Result<()> {
    println!("🔥 Testing HeatMap component...");

    // Create sample heatmap data
    let heatmap_data = HeatMapData {
        values: vec![
            vec![100.0, 85.0, 92.0],
            vec![78.0, 95.0, 88.0],
            vec![91.0, 82.0, 97.0],
        ],
        row_labels: vec![
            "North America".to_string(),
            "Europe".to_string(),
            "Asia".to_string(),
        ],
        column_labels: vec!["Q2".to_string(), "Q3".to_string(), "Q4".to_string()],
    };

    // Create color scale from blue (low) to red (high)
    let color_scale = ColorScale {
        colors: vec![
            Color::hex("#0000ff"), // Blue for minimum
            Color::hex("#ffff00"), // Yellow for middle
            Color::hex("#ff0000"), // Red for maximum
        ],
        min_value: Some(75.0),
        max_value: Some(100.0),
    };

    let mut heatmap = HeatMap::new(heatmap_data);
    heatmap = heatmap.with_color_scale(color_scale);

    // Create a simple PDF page
    let mut document = Document::new();
    document.set_title("HeatMap Test");

    let mut page = Page::a4();

    // Manually render the heatmap at a specific position
    use oxidize_pdf::dashboard::{ComponentPosition, DashboardComponent, DashboardTheme};

    let position = ComponentPosition {
        x: 50.0,
        y: 100.0,
        width: 400.0,
        height: 300.0,
    };

    let theme = DashboardTheme::default();

    match heatmap.render(&mut page, position, &theme) {
        Ok(_) => println!("✅ HeatMap rendered successfully"),
        Err(e) => println!("❌ Error rendering HeatMap: {}", e),
    }

    document.add_page(page);

    let output_path = "examples/results/heatmap_simple_test.pdf";
    document.save(output_path)?;

    println!("📄 Saved to: {}", output_path);

    Ok(())
}
