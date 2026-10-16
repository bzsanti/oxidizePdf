//! Simple PivotTable Test Example
//!
//! Demonstrates propagation of the currently unsupported PivotTable error.
//! Running this example returns an error and does not save a PDF.

use oxidize_pdf::{
    dashboard::{ComponentPosition, DashboardComponent, DashboardTheme, PivotConfig, PivotTable},
    Document, Page, Result,
};
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("📊 Testing PivotTable component...");

    // Create sample data
    let data = vec![
        create_record("Electronics", "North America", "Q4", "1250000"),
        create_record("Software", "North America", "Q4", "890000"),
        create_record("Electronics", "Europe", "Q4", "750000"),
        create_record("Software", "Europe", "Q4", "420000"),
        create_record("Electronics", "Asia", "Q4", "980000"),
        create_record("Software", "Asia", "Q4", "560000"),
    ];

    // Create pivot table
    let config = PivotConfig {
        title: Some("Q4 Sales by Region and Category".to_string()),
        row_groups: vec!["Region".to_string()],
        column_groups: vec!["Category".to_string()],
        value_columns: vec!["Amount".to_string()],
        show_totals: true,
        show_subtotals: false,
        ..Default::default()
    };

    let pivot = PivotTable::new(data).with_config(config);

    // Create PDF
    let mut document = Document::new();
    document.set_title("PivotTable Test");

    let mut page = Page::a4();

    // Render pivot table
    let position = ComponentPosition {
        x: 50.0,
        y: 100.0,
        width: 500.0,
        height: 200.0,
    };

    let theme = DashboardTheme::default();

    // Propagate the unsupported operation instead of saving a misleading report.
    pivot.render(&mut page, position, &theme)?;

    document.add_page(page);

    let output_path = "examples/results/pivot_table_simple_test.pdf";
    document.save(output_path)?;

    println!("📄 Saved to: {}", output_path);

    Ok(())
}

fn create_record(
    category: &str,
    region: &str,
    quarter: &str,
    amount: &str,
) -> HashMap<String, String> {
    let mut record = HashMap::new();
    record.insert("Category".to_string(), category.to_string());
    record.insert("Region".to_string(), region.to_string());
    record.insert("Quarter".to_string(), quarter.to_string());
    record.insert("Amount".to_string(), amount.to_string());
    record
}
