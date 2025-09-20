//! Extreme Complexity PDF Benchmark - Analytics Dashboard Simulation
//!
//! This benchmark creates an extremely complex analytics report with:
//! - Multiple charts and graphs per page (4-6)
//! - Dense data tables with 50+ rows
//! - KPI cards with metrics and indicators
//! - Simulated heat maps and complex visualizations
//! - Multi-column layouts
//! - Gradients, patterns, and advanced graphics
//! - Heavy use of colors, shapes, and text formatting
//!
//! Expected performance: 20-50 pages/second (extreme complexity)

use oxidize_pdf::{Color, Document, Font, Page, Result};
use std::env;
use std::time::Instant;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let page_count = if args.len() > 1 {
        args[1].parse().unwrap_or(25)
    } else {
        25
    };

    let start_time = Instant::now();
    let mut doc = Document::new();
    doc.set_title("Analytics Dashboard - Extreme Complexity Benchmark");
    doc.set_author("oxidize-pdf Performance Test");
    doc.set_subject("Business Intelligence Report");

    let metrics = ["Revenue", "Users", "Conversion", "Retention", "ARPU", "CAC"];
    let regions = [
        "North America",
        "Europe",
        "Asia-Pacific",
        "Latin America",
        "Middle East",
        "Africa",
    ];
    let _time_periods = ["Q1", "Q2", "Q3", "Q4", "YTD"];

    for page_num in 0..page_count {
        let mut page = Page::a4();

        // === DASHBOARD HEADER WITH GRADIENT SIMULATION ===
        // Background gradient simulation with multiple overlays
        for i in 0..8 {
            let gradient_alpha = 0.15 - (i as f64 * 0.015);
            page.graphics()
                .set_fill_color(Color::rgb(
                    0.1 * gradient_alpha,
                    0.2 * gradient_alpha,
                    0.4 * gradient_alpha,
                ))
                .rectangle(
                    0.0,
                    750.0 + (i as f64 * 5.0),
                    595.0,
                    50.0 - (i as f64 * 5.0),
                )
                .fill();
        }

        // Dashboard title with large typography
        page.text()
            .set_font(Font::HelveticaBold, 24.0)
            .set_fill_color(Color::white())
            .at(30.0, 780.0)
            .write("Executive Analytics Dashboard")?;

        page.text()
            .set_font(Font::Helvetica, 12.0)
            .set_fill_color(Color::rgb(0.9, 0.95, 1.0))
            .at(30.0, 755.0)
            .write(&format!(
                "Real-time Business Intelligence | Report #{} | Sept 2025",
                page_num + 1001
            ))?;

        // Status indicators (unique per page)
        let statuses = [
            "🟢 Systems Online",
            "🟡 Partial Data",
            "🔴 Alert",
            "🔵 Updating",
            "⚪ Maintenance",
            "🟠 Warning",
        ];
        for i in 0..4 {
            let status_idx = (page_num + i) % statuses.len();
            page.text()
                .set_font(Font::Helvetica, 8.0)
                .set_fill_color(Color::rgb(0.9, 0.9, 0.9))
                .at(350.0 + (i as f64 * 60.0), 770.0)
                .write(statuses[status_idx])?;
        }

        let mut y_pos = 720.0;

        // === KPI CARDS SECTION (6 cards per row) ===
        page.text()
            .set_font(Font::HelveticaBold, 14.0)
            .set_fill_color(Color::rgb(0.2, 0.2, 0.2))
            .at(30.0, y_pos)
            .write("Key Performance Indicators")?;

        y_pos -= 30.0;

        // Two rows of KPI cards (unique per page)
        for row in 0..2 {
            for col in 0..3 {
                let card_x = 30.0 + (col as f64 * 175.0);
                let card_y = y_pos - (row as f64 * 90.0);
                let metric_idx = (page_num * 2 + row * 3 + col) % metrics.len();
                let metric = metrics[metric_idx];

                // Card background with shadow effect
                for shadow_offset in 0..3 {
                    let shadow_color = Color::rgb(0.0, 0.0, 0.0);
                    page.graphics()
                        .set_fill_color(shadow_color)
                        .rectangle(
                            card_x + (shadow_offset as f64 * 2.0),
                            card_y - 70.0 - (shadow_offset as f64 * 2.0),
                            160.0,
                            65.0,
                        )
                        .fill();
                }

                // Card main background
                let card_color = match metric_idx % 3 {
                    0 => Color::rgb(0.95, 1.0, 0.95),
                    1 => Color::rgb(0.95, 0.95, 1.0),
                    _ => Color::rgb(1.0, 0.95, 0.95),
                };
                page.graphics()
                    .set_fill_color(card_color)
                    .set_stroke_color(Color::rgb(0.8, 0.8, 0.8))
                    .set_line_width(1.0)
                    .rectangle(card_x, card_y - 65.0, 160.0, 60.0)
                    .fill();

                // Metric title
                page.text()
                    .set_font(Font::HelveticaBold, 11.0)
                    .set_fill_color(Color::rgb(0.3, 0.3, 0.3))
                    .at(card_x + 10.0, card_y - 15.0)
                    .write(metric)?;

                // Large metric value (unique per page and position)
                let base_multiplier = (page_num * 7 + row * 11 + col * 13) as f64;
                let value = match metric {
                    "Revenue" => format!("${:.1}M", 8.5 + (base_multiplier * 0.1) % 15.0),
                    "Users" => format!("{:.0}K", 180.0 + (base_multiplier * 2.3) % 200.0),
                    "Conversion" => format!("{:.2}%", 2.15 + (base_multiplier * 0.05) % 3.5),
                    "Retention" => format!("{:.1}%", 75.0 + (base_multiplier * 0.3) % 20.0),
                    "ARPU" => format!("${:.0}", 120.0 + (base_multiplier * 0.8) % 80.0),
                    "CAC" => format!("${:.0}", 45.0 + (base_multiplier * 0.5) % 35.0),
                    "CLV" => format!("${:.0}", 850.0 + (base_multiplier * 1.2) % 300.0),
                    "Sessions" => format!("{:.0}K", 150.0 + (base_multiplier * 1.8) % 100.0),
                    _ => format!("{:.2}%", 15.0 + (base_multiplier * 0.2) % 25.0),
                };

                page.text()
                    .set_font(Font::HelveticaBold, 18.0)
                    .set_fill_color(Color::rgb(0.1, 0.1, 0.4))
                    .at(card_x + 10.0, card_y - 35.0)
                    .write(&value)?;

                // Trend indicator (unique per card)
                let trend_val = (page_num * 7 + metric_idx * 3 + row * 5 + col * 2) % 20;
                let trend = match trend_val % 4 {
                    0 => format!("↗ +{:.1}%", 5.0 + (trend_val as f64 * 0.4)),
                    1 => format!("↘ -{:.1}%", 1.0 + (trend_val as f64 * 0.2)),
                    2 => format!("→ +{:.1}%", 0.1 + (trend_val as f64 * 0.1)),
                    _ => format!("↗ +{:.1}%", 8.0 + (trend_val as f64 * 0.3)),
                };
                let trend_color = if trend.contains("↗") {
                    Color::rgb(0.0, 0.6, 0.0)
                } else if trend.contains("↘") {
                    Color::rgb(0.8, 0.2, 0.2)
                } else {
                    Color::rgb(0.6, 0.6, 0.6)
                };

                page.text()
                    .set_font(Font::Helvetica, 10.0)
                    .set_fill_color(trend_color)
                    .at(card_x + 10.0, card_y - 50.0)
                    .write(&trend)?;

                // Mini sparkline chart (unique per card)
                for spark_i in 0..12 {
                    let spark_x = card_x + 100.0 + (spark_i as f64 * 4.0);
                    let spark_height = 3.0
                        + ((spark_i * 7 + page_num * 11 + metric_idx * 5 + row * 3 + col * 2) % 12)
                            as f64;

                    let spark_color_shift = (page_num + metric_idx + col) as f64 * 0.1;
                    page.graphics()
                        .set_fill_color(Color::rgb(
                            0.3 + (spark_color_shift % 0.4),
                            0.5 + (spark_color_shift % 0.3),
                            0.7 - (spark_color_shift % 0.2),
                        ))
                        .rectangle(spark_x, card_y - 50.0, 3.0, spark_height)
                        .fill();
                }
            }
        }

        y_pos -= 200.0;

        // === MAIN CHARTS SECTION ===
        page.text()
            .set_font(Font::HelveticaBold, 14.0)
            .set_fill_color(Color::rgb(0.2, 0.2, 0.2))
            .at(30.0, y_pos)
            .write("Performance Analytics")?;

        y_pos -= 30.0;

        // LEFT CHART: Revenue Timeline
        let chart1_x = 30.0;
        let chart1_y = y_pos;
        let chart1_width = 250.0;
        let chart1_height = 120.0;

        // Chart background
        page.graphics()
            .set_fill_color(Color::rgb(0.98, 0.98, 0.98))
            .set_stroke_color(Color::rgb(0.8, 0.8, 0.8))
            .set_line_width(1.0)
            .rectangle(
                chart1_x,
                chart1_y - chart1_height,
                chart1_width,
                chart1_height,
            )
            .fill();

        page.text()
            .set_font(Font::HelveticaBold, 10.0)
            .at(chart1_x + 10.0, chart1_y - 15.0)
            .write("Revenue Trend (12M)")?;

        // Grid lines
        for i in 1..5 {
            let grid_y = chart1_y - 20.0 - (i as f64 * 20.0);
            page.graphics()
                .set_stroke_color(Color::rgb(0.9, 0.9, 0.9))
                .set_line_width(0.5)
                .move_to(chart1_x + 20.0, grid_y)
                .line_to(chart1_x + chart1_width - 20.0, grid_y)
                .stroke();
        }

        // Revenue line chart (unique data per page)
        let mut prev_x = chart1_x + 20.0;
        let base_offset = (page_num * 13) % 40;
        let mut prev_y = chart1_y - 40.0 - base_offset as f64;

        for month in 1..12 {
            let curr_x = chart1_x + 20.0 + (month as f64 * 18.0);
            let data_variance = (page_num * 7 + month * 11) % 25;
            let curr_y = chart1_y - 40.0 - (base_offset + data_variance) as f64;

            // Line segment with page-specific color
            let line_color_shift = page_num as f64 * 0.05;
            page.graphics()
                .set_stroke_color(Color::rgb(
                    0.1 + (line_color_shift % 0.3),
                    0.4,
                    0.8 - (line_color_shift % 0.2),
                ))
                .set_line_width(2.0)
                .move_to(prev_x, prev_y)
                .line_to(curr_x, curr_y)
                .stroke();

            // Data point
            page.graphics()
                .set_fill_color(Color::rgb(
                    0.1 + (line_color_shift % 0.3),
                    0.4,
                    0.8 - (line_color_shift % 0.2),
                ))
                .circle(curr_x, curr_y, 3.0)
                .fill();

            prev_x = curr_x;
            prev_y = curr_y;
        }

        // RIGHT CHART: Regional Performance Heat Map
        let chart2_x = 300.0;
        let chart2_y = y_pos;
        let chart2_width = 250.0;
        let chart2_height = 120.0;

        page.graphics()
            .set_fill_color(Color::rgb(0.98, 0.98, 0.98))
            .set_stroke_color(Color::rgb(0.8, 0.8, 0.8))
            .set_line_width(1.0)
            .rectangle(
                chart2_x,
                chart2_y - chart2_height,
                chart2_width,
                chart2_height,
            )
            .fill();

        page.text()
            .set_font(Font::HelveticaBold, 10.0)
            .at(chart2_x + 10.0, chart2_y - 15.0)
            .write("Regional Heat Map")?;

        // Heat map cells (6x4 grid) with unique data per page
        for row in 0..4 {
            for col in 0..6 {
                let cell_x = chart2_x + 20.0 + (col as f64 * 35.0);
                let cell_y = chart2_y - 35.0 - (row as f64 * 20.0);
                let intensity = ((row * 7 + col * 11 + page_num * 13) % 10) as f64 / 10.0;

                let heat_color = Color::rgb(
                    0.1 + intensity * 0.7,
                    0.9 - intensity * 0.5,
                    0.2 + intensity * 0.3,
                );

                page.graphics()
                    .set_fill_color(heat_color)
                    .rectangle(cell_x, cell_y, 30.0, 15.0)
                    .fill();

                // Value in cell with better contrast
                let text_color = if intensity > 0.5 {
                    Color::white()
                } else {
                    Color::rgb(0.1, 0.1, 0.1)
                };
                page.text()
                    .set_font(Font::Helvetica, 6.0)
                    .set_fill_color(text_color)
                    .at(cell_x + 8.0, cell_y + 8.0)
                    .write(&format!("{:.0}", intensity * 100.0))?;
            }
        }

        y_pos -= 140.0;

        // === DENSE DATA TABLE ===
        page.text()
            .set_font(Font::HelveticaBold, 12.0)
            .at(30.0, y_pos)
            .write("Detailed Performance Matrix")?;

        y_pos -= 25.0;

        // Table headers
        let headers = [
            "Region", "Metric", "Q1", "Q2", "Q3", "Q4", "YTD", "Goal", "Variance", "Rank",
        ];
        let col_widths = [60.0, 60.0, 40.0, 40.0, 40.0, 40.0, 50.0, 50.0, 50.0, 40.0];

        // Header background
        page.graphics()
            .set_fill_color(Color::rgb(0.2, 0.2, 0.4))
            .rectangle(30.0, y_pos - 15.0, 470.0, 15.0)
            .fill();

        let mut col_x = 30.0;
        for (header, width) in headers.iter().zip(col_widths.iter()) {
            page.text()
                .set_font(Font::HelveticaBold, 8.0)
                .set_fill_color(Color::white())
                .at(col_x + 2.0, y_pos - 8.0)
                .write(header)?;
            col_x += width;
        }

        y_pos -= 20.0;

        // 30 rows of dense data (unique per page)
        for row in 0..30 {
            let region_idx = (page_num * 3 + row * 7) % regions.len();
            let metric_idx = (page_num * 5 + row * 11) % metrics.len();
            let region = regions[region_idx];
            let metric = metrics[metric_idx];
            let row_y = y_pos - (row as f64 * 12.0);

            // Alternating row colors
            if row % 2 == 0 {
                page.graphics()
                    .set_fill_color(Color::rgb(0.97, 0.97, 0.97))
                    .rectangle(30.0, row_y - 10.0, 470.0, 12.0)
                    .fill();
            }

            // Row data
            let mut col_x = 30.0;

            // Region
            page.text()
                .set_font(Font::Helvetica, 7.0)
                .at(col_x + 2.0, row_y - 5.0)
                .write(region)?;
            col_x += col_widths[0];

            // Metric
            page.text()
                .set_font(Font::Helvetica, 7.0)
                .at(col_x + 2.0, row_y - 5.0)
                .write(metric)?;
            col_x += col_widths[1];

            // Quarterly data (unique per page and row)
            for quarter in 0..4 {
                let value = 75 + ((page_num * 13 + row * 7 + quarter * 19) % 50);
                page.text()
                    .set_font(Font::Courier, 7.0)
                    .set_fill_color(Color::rgb(0.2, 0.2, 0.2))
                    .at(col_x + 2.0, row_y - 5.0)
                    .write(&format!("{}", value))?;
                col_x += col_widths[2 + quarter];
            }

            // YTD (unique per page and row)
            let ytd = 280 + ((page_num * 11 + row * 23) % 120);
            page.text()
                .set_font(Font::CourierBold, 7.0)
                .set_fill_color(Color::rgb(0.1, 0.1, 0.3))
                .at(col_x + 2.0, row_y - 5.0)
                .write(&format!("{}", ytd))?;
            col_x += col_widths[6];

            // Goal (unique per page and row)
            let goal = 300 + ((page_num * 17 + row * 29) % 80);
            page.text()
                .set_font(Font::Courier, 7.0)
                .set_fill_color(Color::rgb(0.2, 0.2, 0.2))
                .at(col_x + 2.0, row_y - 5.0)
                .write(&format!("{}", goal))?;
            col_x += col_widths[7];

            // Variance
            let variance = ytd as i32 - goal as i32;
            let var_color = if variance > 0 {
                Color::rgb(0.0, 0.6, 0.0)
            } else {
                Color::rgb(0.8, 0.0, 0.0)
            };
            page.text()
                .set_font(Font::Courier, 7.0)
                .set_fill_color(var_color)
                .at(col_x + 2.0, row_y - 5.0)
                .write(&format!("{:+}", variance))?;
            col_x += col_widths[8];

            // Rank
            let rank = (row % 15) + 1;
            page.text()
                .set_font(Font::Helvetica, 7.0)
                .at(col_x + 2.0, row_y - 5.0)
                .write(&format!("#{}", rank))?;
        }

        y_pos -= 380.0;

        // === BOTTOM CHARTS ROW ===
        // PIE CHART
        let pie_center_x = 80.0;
        let pie_center_y = y_pos - 50.0;
        let pie_radius = 35.0;

        page.text()
            .set_font(Font::HelveticaBold, 9.0)
            .at(50.0, y_pos)
            .write("Market Share")?;

        // Pie slices (unique colors per page)
        let page_shift = page_num as f64 * 0.1;
        let pie_data = [
            (
                25.0 + (page_num as f64 % 10.0),
                Color::rgb(0.7 + (page_shift % 0.2), 0.2, 0.2),
            ),
            (
                30.0 + (page_num as f64 * 1.5) % 15.0,
                Color::rgb(0.2, 0.5 + (page_shift % 0.3), 0.8),
            ),
            (
                20.0 + (page_num as f64 * 0.8) % 12.0,
                Color::rgb(0.1, 0.7 + (page_shift % 0.2), 0.3),
            ),
            (
                25.0 - (page_num as f64 % 8.0),
                Color::rgb(0.8 + (page_shift % 0.1), 0.6 + (page_shift % 0.2), 0.1),
            ),
        ];
        let mut start_angle = 0.0;

        for (percentage, color) in pie_data.iter() {
            let end_angle = start_angle + (percentage / 100.0) * 360.0;

            // Simple pie slice approximation using triangles
            let segments = 8;
            for i in 0..segments {
                let angle1 = start_angle + (i as f64 * (end_angle - start_angle) / segments as f64);
                let angle2 =
                    start_angle + ((i + 1) as f64 * (end_angle - start_angle) / segments as f64);

                let x1 = pie_center_x + pie_radius * (angle1 * std::f64::consts::PI / 180.0).cos();
                let y1 = pie_center_y + pie_radius * (angle1 * std::f64::consts::PI / 180.0).sin();
                let x2 = pie_center_x + pie_radius * (angle2 * std::f64::consts::PI / 180.0).cos();
                let y2 = pie_center_y + pie_radius * (angle2 * std::f64::consts::PI / 180.0).sin();

                page.graphics()
                    .set_fill_color(*color)
                    .move_to(pie_center_x, pie_center_y)
                    .line_to(x1, y1)
                    .line_to(x2, y2)
                    .close_path()
                    .fill();
            }

            start_angle = end_angle;
        }

        // DONUT CHART
        let donut_center_x = 250.0;
        let donut_center_y = y_pos - 50.0;

        page.text()
            .set_font(Font::HelveticaBold, 9.0)
            .at(220.0, y_pos)
            .write("Completion Status")?;

        // Outer ring
        page.graphics()
            .set_fill_color(Color::rgb(0.0, 0.7, 0.0))
            .circle(donut_center_x, donut_center_y, 30.0)
            .fill();

        // Inner ring (white)
        page.graphics()
            .set_fill_color(Color::white())
            .circle(donut_center_x, donut_center_y, 15.0)
            .fill();

        let completion_percent = 65 + (page_num * 3) % 30;
        page.text()
            .set_font(Font::HelveticaBold, 12.0)
            .set_fill_color(Color::rgb(0.2, 0.2, 0.2))
            .at(donut_center_x - 12.0, donut_center_y - 3.0)
            .write(&format!("{}%", completion_percent))?;

        // BAR CHART
        page.text()
            .set_font(Font::HelveticaBold, 9.0)
            .at(350.0, y_pos)
            .write("Top Performers")?;

        for i in 0..6 {
            let bar_x = 350.0;
            let bar_y = y_pos - 20.0 - (i as f64 * 12.0);
            let bar_width = 30.0 + (i as f64 * 15.0);

            page.graphics()
                .set_fill_color(Color::rgb(
                    0.1 + (i as f64 * 0.1),
                    0.4,
                    0.8 - (i as f64 * 0.1),
                ))
                .rectangle(bar_x, bar_y, bar_width, 10.0)
                .fill();

            page.text()
                .set_font(Font::Helvetica, 7.0)
                .at(bar_x + bar_width + 5.0, bar_y + 6.0)
                .write(&format!("Item {}: {:.0}%", i + 1, 95.0 - (i as f64 * 8.0)))?;
        }

        // === FOOTER WITH BRANDING ===
        page.graphics()
            .set_fill_color(Color::rgb(0.1, 0.1, 0.2))
            .rectangle(0.0, 0.0, 595.0, 30.0)
            .fill();

        page.text()
            .set_font(Font::HelveticaBold, 10.0)
            .set_fill_color(Color::white())
            .at(30.0, 15.0)
            .write("Analytics Pro Dashboard | Confidential Business Intelligence")?;

        page.text()
            .set_font(Font::Helvetica, 8.0)
            .set_fill_color(Color::rgb(0.8, 0.8, 0.8))
            .at(450.0, 15.0)
            .write(&format!("Page {} of {}", page_num + 1, page_count))?;

        doc.add_page(page);
    }

    let generation_time = start_time.elapsed();

    // Separate write timing
    let write_start = Instant::now();
    doc.save("examples/results/extreme_complexity_benchmark.pdf")?;
    let write_time = write_start.elapsed();

    let total_time = start_time.elapsed();

    // Output for parsing by benchmark scripts
    println!("PAGES={}", page_count);
    println!("GENERATION_MS={}", generation_time.as_millis());
    println!("WRITE_MS={}", write_time.as_millis());
    println!("TOTAL_MS={}", total_time.as_millis());
    println!(
        "PAGES_PER_SEC={:.2}",
        page_count as f64 / total_time.as_secs_f64()
    );
    println!("COMPLEXITY=EXTREME");

    println!("\n📊 Extreme Complexity Benchmark Results:");
    println!("  📄 Pages: {}", page_count);
    println!("  ⚡ Generation: {}ms", generation_time.as_millis());
    println!("  💾 Write: {}ms", write_time.as_millis());
    println!("  🕐 Total: {}ms", total_time.as_millis());
    println!(
        "  📈 Performance: {:.1} pages/second",
        page_count as f64 / total_time.as_secs_f64()
    );
    println!("  📋 Content: Dense dashboards + multiple charts + complex tables + heat maps");

    Ok(())
}
