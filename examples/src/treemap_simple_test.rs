//! Simple TreeMap Test Example
//!
//! Tests the TreeMap component in isolation

use oxidize_pdf::{
    dashboard::{
        ComponentPosition, DashboardComponent, DashboardTheme, TreeMap, TreeMapNode, TreeMapOptions,
    },
    Document, Page, Result,
};

fn main() -> Result<()> {
    println!("🌳 Testing TreeMap component...");

    // Create sample hierarchical data - market share by category
    let data = vec![
        TreeMapNode {
            name: "Electronics".to_string(),
            value: 4500.0,
            color: None,
            children: vec![],
        },
        TreeMapNode {
            name: "Software".to_string(),
            value: 3200.0,
            color: None,
            children: vec![],
        },
        TreeMapNode {
            name: "Services".to_string(),
            value: 2800.0,
            color: None,
            children: vec![],
        },
        TreeMapNode {
            name: "Hardware".to_string(),
            value: 2100.0,
            color: None,
            children: vec![],
        },
        TreeMapNode {
            name: "Consulting".to_string(),
            value: 1500.0,
            color: None,
            children: vec![],
        },
        TreeMapNode {
            name: "Training".to_string(),
            value: 900.0,
            color: None,
            children: vec![],
        },
    ];

    // Create treemap with options
    let options = TreeMapOptions {
        title: Some("Revenue by Product Category ($1000s)".to_string()),
        show_labels: true,
        padding: 3.0,
    };

    let treemap = TreeMap::new(data).with_options(options);

    // Create PDF
    let mut document = Document::new();
    document.set_title("TreeMap Test");

    let mut page = Page::a4();

    // Render treemap
    let position = ComponentPosition {
        x: 50.0,
        y: 100.0,
        width: 500.0,
        height: 400.0,
    };

    let theme = DashboardTheme::default();

    match treemap.render(&mut page, position, &theme) {
        Ok(_) => println!("✅ TreeMap rendered successfully"),
        Err(e) => println!("❌ Error rendering TreeMap: {}", e),
    }

    document.add_page(page);

    let output_path = "examples/results/treemap_simple_test.pdf";
    document.save(output_path)?;

    println!("📄 Saved to: {}", output_path);

    Ok(())
}
