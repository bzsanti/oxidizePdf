//! Sales Dashboard Example
//!
//! This example demonstrates how to create a professional sales dashboard
//! with KPI cards, charts, and data tables using the oxidize-pdf dashboard framework.

use oxidize_pdf::{
    dashboard::{
        DashboardBuilder, KpiCard, KpiCardBuilder, TrendDirection, 
        HeatMap, ScatterPlot, PivotTable, DashboardTheme
    },
    charts::{BarChart, PieChart, LineChart},
    graphics::Color,
    Document, Page, Result,
};
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("🚀 Generating Sales Dashboard PDF...");
    
    // Create sample data for the dashboard
    let sales_data = create_sample_sales_data();
    
    // Build the dashboard
    let dashboard = DashboardBuilder::new()
        .title("Q4 2024 Sales Performance Dashboard")
        .subtitle("Executive Summary & Key Metrics")
        .author("Sales Analytics Team")
        .data_source("CRM Database")
        .data_source("Google Analytics")
        .tag("sales")
        .tag("q4")
        .tag("executive")
        .version("1.0.0")
        
        // Use corporate theme
        .theme_by_name("corporate")
        
        // Row 1: Key Performance Indicators (4 KPI cards, 3 columns each)
        .add_kpi_row(vec![
            KpiCardBuilder::new("Total Revenue", "$2,543,890")
                .trend(12.5, TrendDirection::Up)
                .subtitle("vs Q3 2024")
                .color(Color::hex("#28a745"))
                .sparkline(vec![2100000.0, 2200000.0, 2350000.0, 2400000.0, 2543890.0])
                .build(),
                
            KpiCardBuilder::new("New Customers", "1,247")
                .trend(8.3, TrendDirection::Up)
                .subtitle("this quarter")
                .color(Color::hex("#007bff"))
                .sparkline(vec![980.0, 1050.0, 1120.0, 1200.0, 1247.0])
                .build(),
                
            KpiCardBuilder::new("Conversion Rate", "3.2%")
                .trend(-0.1, TrendDirection::Down)
                .subtitle("website visitors")
                .color(Color::hex("#ffc107"))
                .build(),
                
            KpiCardBuilder::new("Avg Order Value", "$2,041")
                .trend(15.7, TrendDirection::Up)
                .subtitle("per transaction")
                .color(Color::hex("#17a2b8"))
                .sparkline(vec![1850.0, 1900.0, 1980.0, 2020.0, 2041.0])
                .build(),
        ])
        
        // Row 2: Charts (2 charts, 6 columns each)
        .start_row()
        .add_to_row(create_monthly_sales_chart())
        .add_to_row(create_product_breakdown_pie())
        .finish_row()
        
        // Row 3: Advanced Visualizations (2 components, 6 columns each)
        .start_row()
        .add_to_row(create_regional_heatmap())
        .add_to_row(create_customer_scatter_plot())
        .finish_row()
        
        // Row 4: Data Table (full width)
        .add_component(create_sales_pivot_table(sales_data))
        
        .build()?;
    
    // Generate PDF
    let mut document = Document::new();
    document.set_title("Sales Dashboard Q4 2024");
    document.set_creator("oxidize-pdf Dashboard Framework");
    document.set_subject("Sales Performance Analysis");
    
    let mut page = Page::a4_landscape(); // Use landscape for dashboard
    
    // Render dashboard to the page
    dashboard.render_to_page(&mut page)?;
    
    document.add_page(page);
    
    let output_path = "examples/results/dashboard_sales_report.pdf";
    document.save(output_path)?;
    
    // Show dashboard statistics
    let stats = dashboard.get_stats();
    println!("✅ Dashboard created successfully!");
    println!("   📊 Components: {}", stats.component_count);
    println!("   ⏱️  Est. render time: {}ms", stats.estimated_render_time_ms);
    println!("   💾 Est. memory usage: {:.1}MB", stats.memory_usage_mb);
    println!("   🎯 Complexity score: {}/100", stats.complexity_score);
    println!("   📄 Saved to: {}", output_path);
    
    Ok(())
}

/// Create sample sales data for the dashboard
fn create_sample_sales_data() -> Vec<HashMap<String, String>> {
    vec![
        create_sales_record("North America", "Electronics", "Q4", "1250000"),
        create_sales_record("North America", "Software", "Q4", "890000"),
        create_sales_record("Europe", "Electronics", "Q4", "750000"),
        create_sales_record("Europe", "Software", "Q4", "420000"),
        create_sales_record("Asia", "Electronics", "Q4", "980000"),
        create_sales_record("Asia", "Software", "Q4", "560000"),
    ]
}

fn create_sales_record(region: &str, category: &str, quarter: &str, amount: &str) -> HashMap<String, String> {
    let mut record = HashMap::new();
    record.insert("Region".to_string(), region.to_string());
    record.insert("Category".to_string(), category.to_string());
    record.insert("Quarter".to_string(), quarter.to_string());
    record.insert("Amount".to_string(), amount.to_string());
    record
}

/// Create monthly sales bar chart
fn create_monthly_sales_chart() -> Box<dyn oxidize_pdf::dashboard::DashboardComponent> {
    // This is a placeholder - in the full implementation, this would create
    // a proper bar chart component that implements DashboardComponent
    Box::new(KpiCard::new("Monthly Sales Chart", "Chart Placeholder"))
}

/// Create product breakdown pie chart  
fn create_product_breakdown_pie() -> Box<dyn oxidize_pdf::dashboard::DashboardComponent> {
    Box::new(KpiCard::new("Product Breakdown", "Pie Chart Placeholder"))
}

/// Create regional sales heatmap
fn create_regional_heatmap() -> Box<dyn oxidize_pdf::dashboard::DashboardComponent> {
    let heatmap_data = oxidize_pdf::dashboard::HeatMapData {
        values: vec![
            vec![100.0, 85.0, 92.0],
            vec![78.0, 95.0, 88.0],
            vec![91.0, 82.0, 97.0],
        ],
        row_labels: vec!["North America".to_string(), "Europe".to_string(), "Asia".to_string()],
        column_labels: vec!["Q2".to_string(), "Q3".to_string(), "Q4".to_string()],
    };
    
    Box::new(HeatMap::new(heatmap_data))
}

/// Create customer value scatter plot
fn create_customer_scatter_plot() -> Box<dyn oxidize_pdf::dashboard::DashboardComponent> {
    let scatter_data = vec![
        oxidize_pdf::dashboard::ScatterPoint {
            x: 25.0,
            y: 1200.0,
            size: Some(5.0),
            color: Some(Color::blue()),
            label: Some("Enterprise".to_string()),
        },
        oxidize_pdf::dashboard::ScatterPoint {
            x: 45.0,
            y: 800.0,
            size: Some(3.0),
            color: Some(Color::green()),
            label: Some("SMB".to_string()),
        },
    ];
    
    Box::new(ScatterPlot::new(scatter_data))
}

/// Create sales pivot table
fn create_sales_pivot_table(data: Vec<HashMap<String, String>>) -> Box<dyn oxidize_pdf::dashboard::DashboardComponent> {
    let pivot = PivotTable::new(data)
        .aggregate_by(&["sum", "count"]);
        
    Box::new(pivot)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dashboard_creation() {
        // Test that dashboard creation doesn't panic
        let result = std::panic::catch_unwind(|| {
            let _dashboard = DashboardBuilder::new()
                .title("Test Dashboard")
                .build();
        });
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_sample_data_creation() {
        let data = create_sample_sales_data();
        assert_eq!(data.len(), 6);
        assert!(data[0].contains_key("Region"));
        assert!(data[0].contains_key("Amount"));
    }
    
    #[test]
    fn test_kpi_cards() {
        let card = KpiCardBuilder::new("Test KPI", "100")
            .trend(5.0, TrendDirection::Up)
            .build();
            
        assert_eq!(card.component_type(), "KpiCard");
        assert!(card.complexity_score() > 0);
    }
}