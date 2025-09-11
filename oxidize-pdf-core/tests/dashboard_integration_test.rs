#[cfg(test)]
mod dashboard_tests {
    use oxidize_pdf::dashboard::{DashboardBuilder, DashboardComponent, KpiCard, TrendDirection};
    use oxidize_pdf::{Document, Page};
    use std::fs;

    #[test]
    fn test_dashboard_renders_to_pdf() {
        // Create a dashboard with real data
        let dashboard = DashboardBuilder::new()
            .title("Test Dashboard")
            .subtitle("Integration Test")
            .add_kpi_row(vec![
                KpiCard::new("Revenue", "$100K")
                    .with_trend(5.2, TrendDirection::Up)
                    .with_subtitle("Monthly"),
                KpiCard::new("Users", "1,234")
                    .with_trend(2.1, TrendDirection::Up)
                    .with_sparkline(vec![100.0, 120.0, 115.0, 130.0, 125.0]),
            ])
            .build()
            .expect("Dashboard should build successfully");

        // Create document and render
        let mut document = Document::new();
        let mut page = Page::new(595.0, 842.0);

        // This should not panic or fail
        dashboard
            .render_to_page(&mut page)
            .expect("Dashboard should render without error");

        document.add_page(page);

        // Save to file
        let output_path = "examples/results/test_dashboard.pdf";
        std::fs::create_dir_all("examples/results").unwrap();
        document
            .save(output_path)
            .expect("Should save PDF successfully");

        // Verify file was created and has reasonable size
        let metadata = fs::metadata(output_path).expect("PDF file should exist");

        assert!(
            metadata.len() > 1000,
            "PDF file should be larger than 1KB, got {} bytes",
            metadata.len()
        );

        // Verify dashboard statistics are reasonable
        let stats = dashboard.get_stats();
        assert_eq!(stats.component_count, 2, "Should have 2 KPI components");
        assert!(
            stats.estimated_render_time_ms > 0,
            "Should have positive render time"
        );
        assert!(
            stats.memory_usage_mb > 0.0,
            "Should have positive memory usage"
        );
        assert!(stats.complexity_score <= 100, "Complexity should be 0-100");
    }

    #[test]
    fn test_kpi_card_with_all_features() {
        let kpi = KpiCard::new("Complete Test", "$999.99")
            .with_trend(12.5, TrendDirection::Up)
            .with_subtitle("All features test")
            .with_sparkline(vec![10.0, 15.0, 12.0, 18.0, 20.0, 22.0, 25.0]);

        // Test that component reports correct complexity
        let render_time = kpi.estimated_render_time_ms();
        assert!(render_time > 0, "Should have positive render time");

        let memory = kpi.estimated_memory_mb();
        assert!(memory > 0.0, "Should have positive memory usage");

        let complexity = kpi.complexity_score();
        assert!(complexity > 0, "Should have positive complexity");
    }

    #[test]
    fn test_dashboard_with_varied_data() {
        // Test with different trend directions and values
        let dashboard = DashboardBuilder::new()
            .title("Varied Data Test")
            .add_kpi_row(vec![
                KpiCard::new("Positive", "$1M").with_trend(50.0, TrendDirection::Up),
                KpiCard::new("Negative", "500").with_trend(10.5, TrendDirection::Down),
                KpiCard::new("Flat", "2.5%").with_trend(0.0, TrendDirection::Flat),
            ])
            .build()
            .expect("Should build dashboard with varied data");

        let stats = dashboard.get_stats();
        assert_eq!(stats.component_count, 3);

        // Render to verify no panics
        let mut document = Document::new();
        let mut page = Page::new(595.0, 842.0);

        dashboard
            .render_to_page(&mut page)
            .expect("Should render varied data dashboard");

        document.add_page(page);
        document
            .save("examples/results/varied_data_test.pdf")
            .expect("Should save varied data PDF");
    }

    #[test]
    fn test_empty_dashboard() {
        let dashboard = DashboardBuilder::new()
            .title("Empty Dashboard")
            .build()
            .expect("Should build empty dashboard");

        let stats = dashboard.get_stats();
        assert_eq!(stats.component_count, 0);

        // Should still render without error
        let mut document = Document::new();
        let mut page = Page::new(595.0, 842.0);

        dashboard
            .render_to_page(&mut page)
            .expect("Should render empty dashboard");

        document.add_page(page);
        document
            .save("examples/results/empty_dashboard_test.pdf")
            .expect("Should save empty dashboard PDF");
    }

    #[test]
    fn test_large_dashboard() {
        // Create dashboard with many components
        let mut kpi_cards = Vec::new();
        for i in 1..=12 {
            kpi_cards.push(
                KpiCard::new(format!("KPI {}", i), format!("${},000", i * 100))
                    .with_trend(
                        i as f64 * 2.5,
                        if i % 2 == 0 {
                            TrendDirection::Up
                        } else {
                            TrendDirection::Down
                        },
                    )
                    .with_sparkline(vec![
                        i as f64,
                        i as f64 + 5.0,
                        i as f64 + 2.0,
                        i as f64 + 8.0,
                        i as f64 + 10.0,
                    ]),
            );
        }

        let dashboard = DashboardBuilder::new()
            .title("Large Dashboard Test")
            .subtitle("12 KPI Cards")
            .add_kpi_row(kpi_cards)
            .build()
            .expect("Should build large dashboard");

        let stats = dashboard.get_stats();
        assert_eq!(stats.component_count, 12);
        assert!(
            stats.complexity_score > 20,
            "Large dashboard should have higher complexity"
        );

        // Should render without error
        let mut document = Document::new();
        let mut page = Page::new(595.0, 842.0);

        dashboard
            .render_to_page(&mut page)
            .expect("Should render large dashboard");

        document.add_page(page);
        document
            .save("examples/results/large_dashboard_test.pdf")
            .expect("Should save large dashboard PDF");
    }
}
