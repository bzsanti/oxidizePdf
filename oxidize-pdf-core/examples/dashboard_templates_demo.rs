//! Dashboard Templates Demo
//!
//! This example demonstrates the pre-built dashboard templates for common use cases.
//! Templates provide quick dashboard generation by simply providing data.

use oxidize_pdf::{
    dashboard::templates::{
        AnalyticsDashboardTemplate, ChartData, FinancialReportTemplate, KpiData, PieSegmentData,
        SalesDashboardTemplate, SeriesData, TemplateData,
    },
    dashboard::TrendDirection,
    graphics::Color,
    Document, Page, Result,
};

fn main() -> Result<()> {
    println!("📊 Generating Dashboard Templates Demo...");

    // Create document
    let mut document = Document::new();
    document.set_title("Dashboard Templates Demo");
    document.set_creator("oxidize-pdf Dashboard Framework");

    // 1. Sales Dashboard Template
    println!("   Creating Sales Dashboard...");
    let sales_dashboard = create_sales_dashboard()?;
    let mut page1 = Page::a4_landscape();
    sales_dashboard.render_to_page(&mut page1)?;
    document.add_page(page1);

    // 2. Financial Report Template
    println!("   Creating Financial Report...");
    let financial_dashboard = create_financial_report()?;
    let mut page2 = Page::a4_landscape();
    financial_dashboard.render_to_page(&mut page2)?;
    document.add_page(page2);

    // 3. Analytics Dashboard Template
    println!("   Creating Analytics Dashboard...");
    let analytics_dashboard = create_analytics_dashboard()?;
    let mut page3 = Page::a4_landscape();
    analytics_dashboard.render_to_page(&mut page3)?;
    document.add_page(page3);

    // Save document
    let output_path = "examples/results/dashboard_templates_demo.pdf";
    document.save(output_path)?;

    println!("✅ Dashboard templates demo completed!");
    println!("   📄 Saved to: {}", output_path);
    println!("   📊 Pages: 3 (Sales, Financial, Analytics)");

    Ok(())
}

/// Create a sales dashboard using the template
fn create_sales_dashboard() -> Result<oxidize_pdf::dashboard::Dashboard> {
    let data = TemplateData::new()
        // KPIs
        .add_kpi(KpiData {
            name: "Total Revenue".to_string(),
            value: "$2,543,890".to_string(),
            subtitle: Some("vs Q3 2024".to_string()),
            trend_value: Some(12.5),
            trend_direction: Some(TrendDirection::Up),
            color: Some(Color::hex("#28a745")),
            sparkline: Some(vec![2100000.0, 2200000.0, 2350000.0, 2400000.0, 2543890.0]),
        })
        .add_kpi(KpiData {
            name: "New Customers".to_string(),
            value: "1,247".to_string(),
            subtitle: Some("this quarter".to_string()),
            trend_value: Some(8.3),
            trend_direction: Some(TrendDirection::Up),
            color: Some(Color::hex("#007bff")),
            sparkline: Some(vec![980.0, 1050.0, 1120.0, 1200.0, 1247.0]),
        })
        .add_kpi(KpiData {
            name: "Conversion Rate".to_string(),
            value: "3.2%".to_string(),
            subtitle: Some("website visitors".to_string()),
            trend_value: Some(-0.1),
            trend_direction: Some(TrendDirection::Down),
            color: Some(Color::hex("#ffc107")),
            sparkline: None,
        })
        .add_kpi(KpiData {
            name: "Avg Order Value".to_string(),
            value: "$2,041".to_string(),
            subtitle: Some("per transaction".to_string()),
            trend_value: Some(15.7),
            trend_direction: Some(TrendDirection::Up),
            color: Some(Color::hex("#17a2b8")),
            sparkline: Some(vec![1850.0, 1900.0, 1980.0, 2020.0, 2041.0]),
        })
        // Monthly sales chart
        .with_chart(
            "monthly_sales",
            ChartData::Bar {
                labels: vec!["Oct".to_string(), "Nov".to_string(), "Dec".to_string()],
                values: vec![720000.0, 850000.0, 973890.0],
                colors: Some(vec![
                    Color::hex("#007bff"),
                    Color::hex("#28a745"),
                    Color::hex("#17a2b8"),
                ]),
            },
        )
        // Product breakdown
        .with_chart(
            "product_breakdown",
            ChartData::Pie {
                segments: vec![
                    PieSegmentData {
                        label: "Electronics".to_string(),
                        value: 1980000.0,
                        color: Color::hex("#007bff"),
                    },
                    PieSegmentData {
                        label: "Software".to_string(),
                        value: 1870000.0,
                        color: Color::hex("#28a745"),
                    },
                    PieSegmentData {
                        label: "Services".to_string(),
                        value: 693890.0,
                        color: Color::hex("#ffc107"),
                    },
                ],
            },
        )
        // Regional performance heatmap
        .with_chart(
            "regional_performance",
            ChartData::HeatMap {
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
            },
        );

    SalesDashboardTemplate::new()
        .title("Q4 2024 Sales Performance")
        .subtitle("Executive Summary & Key Metrics")
        .theme("corporate")
        .build(data)
}

/// Create a financial report using the template
fn create_financial_report() -> Result<oxidize_pdf::dashboard::Dashboard> {
    let data = TemplateData::new()
        // Financial KPIs
        .add_kpi(KpiData {
            name: "Revenue".to_string(),
            value: "$5.2M".to_string(),
            subtitle: Some("YTD".to_string()),
            trend_value: Some(18.3),
            trend_direction: Some(TrendDirection::Up),
            color: Some(Color::hex("#28a745")),
            sparkline: None,
        })
        .add_kpi(KpiData {
            name: "Net Profit".to_string(),
            value: "$1.4M".to_string(),
            subtitle: Some("27% margin".to_string()),
            trend_value: Some(22.1),
            trend_direction: Some(TrendDirection::Up),
            color: Some(Color::hex("#007bff")),
            sparkline: None,
        })
        .add_kpi(KpiData {
            name: "Operating Costs".to_string(),
            value: "$3.1M".to_string(),
            subtitle: Some("60% of revenue".to_string()),
            trend_value: Some(5.2),
            trend_direction: Some(TrendDirection::Up),
            color: Some(Color::hex("#ffc107")),
            sparkline: None,
        })
        .add_kpi(KpiData {
            name: "EBITDA".to_string(),
            value: "$1.8M".to_string(),
            subtitle: Some("35% margin".to_string()),
            trend_value: Some(15.4),
            trend_direction: Some(TrendDirection::Up),
            color: Some(Color::hex("#17a2b8")),
            sparkline: None,
        })
        // Revenue trend
        .with_chart(
            "revenue_trend",
            ChartData::Line {
                series: vec![
                    SeriesData {
                        name: "Revenue".to_string(),
                        data: vec![
                            (0.0, 800000.0),
                            (1.0, 950000.0),
                            (2.0, 1100000.0),
                            (3.0, 1300000.0),
                            (4.0, 1450000.0),
                            (5.0, 1600000.0),
                        ],
                        color: Color::hex("#28a745"),
                    },
                    SeriesData {
                        name: "Profit".to_string(),
                        data: vec![
                            (0.0, 200000.0),
                            (1.0, 240000.0),
                            (2.0, 280000.0),
                            (3.0, 340000.0),
                            (4.0, 380000.0),
                            (5.0, 420000.0),
                        ],
                        color: Color::hex("#007bff"),
                    },
                ],
            },
        )
        // Expense breakdown
        .with_chart(
            "expense_breakdown",
            ChartData::Pie {
                segments: vec![
                    PieSegmentData {
                        label: "Salaries".to_string(),
                        value: 1500000.0,
                        color: Color::hex("#007bff"),
                    },
                    PieSegmentData {
                        label: "Operations".to_string(),
                        value: 900000.0,
                        color: Color::hex("#28a745"),
                    },
                    PieSegmentData {
                        label: "Marketing".to_string(),
                        value: 450000.0,
                        color: Color::hex("#ffc107"),
                    },
                    PieSegmentData {
                        label: "R&D".to_string(),
                        value: 250000.0,
                        color: Color::hex("#17a2b8"),
                    },
                ],
            },
        )
        // Cost structure by quarter
        .with_chart(
            "cost_structure",
            ChartData::Bar {
                labels: vec![
                    "Q1".to_string(),
                    "Q2".to_string(),
                    "Q3".to_string(),
                    "Q4".to_string(),
                ],
                values: vec![700000.0, 750000.0, 800000.0, 850000.0],
                colors: Some(vec![
                    Color::hex("#007bff"),
                    Color::hex("#28a745"),
                    Color::hex("#ffc107"),
                    Color::hex("#17a2b8"),
                ]),
            },
        );

    FinancialReportTemplate::new()
        .title("2024 Financial Report")
        .subtitle("Annual Performance Summary")
        .theme("corporate")
        .build(data)
}

/// Create an analytics dashboard using the template
fn create_analytics_dashboard() -> Result<oxidize_pdf::dashboard::Dashboard> {
    let data = TemplateData::new()
        // Analytics KPIs
        .add_kpi(KpiData {
            name: "Active Users".to_string(),
            value: "12,547".to_string(),
            subtitle: Some("this month".to_string()),
            trend_value: Some(14.2),
            trend_direction: Some(TrendDirection::Up),
            color: None,
            sparkline: Some(vec![10200.0, 10800.0, 11400.0, 11900.0, 12547.0]),
        })
        .add_kpi(KpiData {
            name: "Engagement Rate".to_string(),
            value: "68.5%".to_string(),
            subtitle: Some("daily active".to_string()),
            trend_value: Some(3.2),
            trend_direction: Some(TrendDirection::Up),
            color: None,
            sparkline: Some(vec![64.0, 65.5, 66.8, 67.2, 68.5]),
        })
        .add_kpi(KpiData {
            name: "Avg Session".to_string(),
            value: "8m 42s".to_string(),
            subtitle: Some("per user".to_string()),
            trend_value: Some(12.8),
            trend_direction: Some(TrendDirection::Up),
            color: None,
            sparkline: Some(vec![7.2, 7.6, 8.0, 8.3, 8.7]),
        })
        .add_kpi(KpiData {
            name: "Retention".to_string(),
            value: "84.2%".to_string(),
            subtitle: Some("30-day".to_string()),
            trend_value: Some(2.1),
            trend_direction: Some(TrendDirection::Up),
            color: None,
            sparkline: Some(vec![81.0, 82.0, 83.0, 83.5, 84.2]),
        })
        // Trend analysis
        .with_chart(
            "trends",
            ChartData::Line {
                series: vec![
                    SeriesData {
                        name: "Users".to_string(),
                        data: vec![
                            (0.0, 10000.0),
                            (1.0, 10500.0),
                            (2.0, 11000.0),
                            (3.0, 11800.0),
                            (4.0, 12547.0),
                        ],
                        color: Color::hex("#007bff"),
                    },
                    SeriesData {
                        name: "Sessions".to_string(),
                        data: vec![
                            (0.0, 25000.0),
                            (1.0, 27000.0),
                            (2.0, 29500.0),
                            (3.0, 31200.0),
                            (4.0, 33850.0),
                        ],
                        color: Color::hex("#28a745"),
                    },
                ],
            },
        )
        // Feature usage comparison
        .with_chart(
            "comparison",
            ChartData::Bar {
                labels: vec![
                    "Dashboard".to_string(),
                    "Reports".to_string(),
                    "Analytics".to_string(),
                    "Settings".to_string(),
                ],
                values: vec![8547.0, 6234.0, 4891.0, 2145.0],
                colors: None,
            },
        )
        // User distribution by platform
        .with_chart(
            "distribution",
            ChartData::Pie {
                segments: vec![
                    PieSegmentData {
                        label: "Web".to_string(),
                        value: 6500.0,
                        color: Color::hex("#007bff"),
                    },
                    PieSegmentData {
                        label: "Mobile".to_string(),
                        value: 4200.0,
                        color: Color::hex("#28a745"),
                    },
                    PieSegmentData {
                        label: "Desktop App".to_string(),
                        value: 1847.0,
                        color: Color::hex("#ffc107"),
                    },
                ],
            },
        )
        // Activity heatmap
        .with_chart(
            "heatmap",
            ChartData::HeatMap {
                values: vec![
                    vec![120.0, 135.0, 128.0, 142.0, 155.0],
                    vec![98.0, 105.0, 112.0, 125.0, 138.0],
                    vec![145.0, 152.0, 148.0, 165.0, 178.0],
                    vec![88.0, 92.0, 95.0, 102.0, 110.0],
                ],
                row_labels: vec![
                    "Mon".to_string(),
                    "Tue".to_string(),
                    "Wed".to_string(),
                    "Thu".to_string(),
                ],
                column_labels: vec![
                    "Week 1".to_string(),
                    "Week 2".to_string(),
                    "Week 3".to_string(),
                    "Week 4".to_string(),
                    "Week 5".to_string(),
                ],
            },
        );

    AnalyticsDashboardTemplate::new()
        .title("User Analytics Dashboard")
        .subtitle("Monthly Performance Overview")
        .theme("colorful")
        .build(data)
}
