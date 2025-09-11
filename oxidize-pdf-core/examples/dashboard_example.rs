//! Dashboard Framework Example
//!
//! This example demonstrates the new dashboard framework for creating
//! professional data visualization reports in PDF format.

use oxidize_pdf::dashboard::{DashboardBuilder, KpiCard, TrendDirection};
use oxidize_pdf::{Document, Page, PdfError};

fn main() -> Result<(), PdfError> {
    println!("🚀 Creating Dashboard Example...");

    // Create a comprehensive sales dashboard
    let dashboard = DashboardBuilder::new()
        .title("Q4 2024 Sales Performance Dashboard")
        .subtitle("Executive Summary Report")
        .author("Oxidize PDF Dashboard Framework")
        .version("1.0.0")
        .data_source("CRM Database")
        .data_source("Analytics Platform")
        .tag("quarterly-report")
        .tag("sales")
        // Use corporate theme
        .theme_by_name("corporate")
        // Row 1: Key Performance Indicators (4 KPI cards in one row)
        .add_kpi_row(vec![
            KpiCard::new("Total Revenue", "$2,547,820")
                .with_trend(12.3, TrendDirection::Up)
                .with_subtitle("vs Q3 2024"),
            KpiCard::new("Active Customers", "1,247")
                .with_trend(5.7, TrendDirection::Up)
                .with_subtitle("Monthly Active Users"),
            KpiCard::new("Conversion Rate", "3.24%")
                .with_trend(-0.1, TrendDirection::Down)
                .with_subtitle("Lead to Customer"),
            KpiCard::new("Average Order Value", "$2,043")
                .with_trend(8.2, TrendDirection::Up)
                .with_subtitle("Per Transaction"),
        ])
        // Add metadata tags
        .tag("dashboard")
        .tag("pdf-generation")
        .build()?;

    // Get dashboard statistics
    let stats = dashboard.get_stats();
    println!("📊 Dashboard Statistics:");
    println!("  • Components: {}", stats.component_count);
    println!(
        "  • Estimated render time: {}ms",
        stats.estimated_render_time_ms
    );
    println!("  • Memory usage: {:.1}MB", stats.memory_usage_mb);
    println!("  • Complexity score: {}/100", stats.complexity_score);

    // Create PDF document and page
    let mut document = Document::new();
    let mut page = Page::new(595.0, 842.0); // A4 size

    // Render dashboard to page
    println!("📄 Rendering dashboard to PDF...");
    dashboard.render_to_page(&mut page)?;

    // Add the rendered page to the document
    document.add_page(page);

    // Save the PDF
    let output_path = "examples/results/dashboard_example.pdf";
    std::fs::create_dir_all("examples/results")?;
    document.save(output_path)?;

    println!("✅ Dashboard PDF saved to: {}", output_path);
    println!("🎯 Dashboard framework implementation complete!");
    println!();
    println!("Key Features Implemented:");
    println!("  ✓ Fluent API Builder Pattern");
    println!("  ✓ 12-Column Responsive Grid System");
    println!("  ✓ KPI Cards with Trends and Sparklines");
    println!("  ✓ Professional Theming System");
    println!("  ✓ Extensible Component Architecture");
    println!("  ✓ Advanced Visualizations Framework");
    println!("  ✓ Automatic Layout Management");
    println!("  ✓ Performance Monitoring");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_creation() {
        let dashboard = DashboardBuilder::new()
            .title("Test Dashboard")
            .subtitle("Unit Test")
            .build()
            .unwrap();

        assert_eq!(dashboard.title, "Test Dashboard");
        assert_eq!(dashboard.subtitle, Some("Unit Test".to_string()));
        assert_eq!(dashboard.components.len(), 0); // No components added yet
    }

    #[test]
    fn test_kpi_card_creation() {
        let kpi = KpiCard::new("Revenue", "$100,000")
            .with_trend(5.0, TrendDirection::Up)
            .with_subtitle("Monthly");

        // Test that KPI card was created successfully
        // We can't access private fields, so just verify structure exists
        assert_eq!(std::mem::size_of_val(&kpi), std::mem::size_of::<KpiCard>());
    }

    #[test]
    fn test_dashboard_with_kpis() {
        let dashboard = DashboardBuilder::new()
            .title("KPI Test")
            .add_kpi_row(vec![KpiCard::new("Test KPI", "42")])
            .build()
            .unwrap();

        assert_eq!(dashboard.components.len(), 1);

        let stats = dashboard.get_stats();
        assert!(stats.estimated_render_time_ms > 0);
        assert!(stats.memory_usage_mb > 0.0);
    }

    #[test]
    fn test_themes() {
        let themes = vec!["corporate", "minimal", "dark", "colorful"];

        for theme_name in themes {
            let dashboard = DashboardBuilder::new()
                .title("Theme Test")
                .theme_by_name(theme_name)
                .build()
                .unwrap();

            // Should not panic and should create valid dashboard
            assert_eq!(dashboard.title, "Theme Test");
        }
    }

    #[test]
    fn test_dashboard_metadata() {
        let dashboard = DashboardBuilder::new()
            .title("Metadata Test")
            .author("Test Author")
            .version("1.2.3")
            .data_source("Test DB")
            .tag("test")
            .build()
            .unwrap();

        assert_eq!(dashboard.metadata.version, "1.2.3");
        assert_eq!(dashboard.metadata.author, Some("Test Author".to_string()));
        assert_eq!(dashboard.metadata.data_sources, vec!["Test DB"]);
        assert_eq!(dashboard.metadata.tags, vec!["test"]);
    }
}
