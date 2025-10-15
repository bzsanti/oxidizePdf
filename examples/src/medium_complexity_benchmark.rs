//! Medium Complexity PDF Benchmark - Sales Report Simulation
//!
//! This benchmark creates a realistic business report with:
//! - Corporate header with simulated logo
//! - Sales data tables (20 rows per page)
//! - Simple bar charts every 5 pages
//! - Footer with page number and date
//! - Multiple fonts and graphics elements
//!
//! Expected performance: 500-1,000 pages/second (moderate complexity)

use oxidize_pdf::{Color, Document, Font, Page, Result};
use std::env;
use std::time::Instant;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let page_count = if args.len() > 1 {
        args[1].parse().unwrap_or(50)
    } else {
        50
    };

    let start_time = Instant::now();
    let mut doc = Document::new();
    doc.set_title("Sales Report - Medium Complexity Benchmark");
    doc.set_author("oxidize-pdf Performance Test");

    // Sample sales data for realistic content (will be varied per page)
    let regions = [
        "North America",
        "Europe",
        "Asia-Pacific",
        "Latin America",
        "Middle East",
        "Africa",
        "Oceania",
    ];
    let products = [
        "Product Alpha",
        "Product Beta",
        "Product Gamma",
        "Product Delta",
        "Product Epsilon",
        "Product Zeta",
        "Product Omega",
    ];
    let sales_reps = [
        "John Smith",
        "Maria Garcia",
        "Chen Wei",
        "Ahmed Hassan",
        "Anna Mueller",
        "Sarah Johnson",
        "David Brown",
        "Lisa Wang",
        "Carlos Rodriguez",
        "Emma Thompson",
    ];

    for page_num in 0..page_count {
        let mut page = Page::a4();
        let mut y_pos = 790.0;

        // === CORPORATE HEADER ===
        // Simulated logo (rectangle + text)
        page.graphics()
            .set_fill_color(Color::rgb(0.0, 0.4, 0.8))
            .rectangle(50.0, y_pos - 30.0, 100.0, 25.0)
            .fill();

        page.text()
            .set_font(Font::HelveticaBold, 12.0)
            .set_fill_color(Color::white())
            .at(60.0, y_pos - 20.0)
            .write("ACME CORP")?;

        // Title
        page.text()
            .set_font(Font::HelveticaBold, 18.0)
            .set_fill_color(Color::rgb(0.0, 0.4, 0.8))
            .at(200.0, y_pos - 15.0)
            .write("Q3 2025 Sales Report")?;

        // Report date (unique per page)
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .set_fill_color(Color::rgb(0.3, 0.3, 0.3))
            .at(450.0, y_pos - 15.0)
            .write(&format!("Report #{} - Sept 2025", page_num + 1))?;

        y_pos -= 50.0;

        // Separator line
        page.graphics()
            .set_stroke_color(Color::rgb(0.0, 0.4, 0.8))
            .set_line_width(1.0)
            .move_to(50.0, y_pos)
            .line_to(545.0, y_pos)
            .stroke();

        y_pos -= 20.0;

        // === SALES DATA TABLE ===
        page.text()
            .set_font(Font::HelveticaBold, 14.0)
            .at(50.0, y_pos)
            .write(&format!("Page {} - Regional Sales Data", page_num + 1))?;

        y_pos -= 25.0;

        // Table header background
        page.graphics()
            .set_fill_color(Color::rgb(0.9, 0.9, 0.9))
            .rectangle(50.0, y_pos - 15.0, 495.0, 20.0)
            .fill();

        // Table headers
        page.text()
            .set_font(Font::HelveticaBold, 10.0)
            .at(55.0, y_pos - 5.0)
            .write("Region")?;

        page.text()
            .set_font(Font::HelveticaBold, 10.0)
            .at(150.0, y_pos - 5.0)
            .write("Product")?;

        page.text()
            .set_font(Font::HelveticaBold, 10.0)
            .at(250.0, y_pos - 5.0)
            .write("Sales Rep")?;

        page.text()
            .set_font(Font::HelveticaBold, 10.0)
            .at(350.0, y_pos - 5.0)
            .write("Q3 Sales")?;

        page.text()
            .set_font(Font::HelveticaBold, 10.0)
            .at(450.0, y_pos - 5.0)
            .write("Growth %")?;

        page.text()
            .set_font(Font::HelveticaBold, 10.0)
            .at(510.0, y_pos - 5.0)
            .write("Status")?;

        y_pos -= 25.0;

        // Table data (variable rows per page to avoid caching)
        let row_count = 18 + (page_num % 7); // 18-24 rows per page
        for row in 0..row_count {
            // Generate unique data per page and row (no repetition)
            let region_idx = (page_num * 7 + row * 3) % regions.len();
            let product_idx = (page_num * 11 + row * 5) % products.len();
            let rep_idx = (page_num * 13 + row * 7) % sales_reps.len();
            let region = regions[region_idx];
            let product = products[product_idx];
            let sales_rep = sales_reps[rep_idx];

            // Unique sales values with realistic variation
            let base_sales = 45000 + (page_num * 5000);
            let sales = base_sales + (row * 3456) + ((page_num * row * 123) % 150000);
            let growth = -20.0 + ((page_num as f32 * 3.7 + row as f32 * 1.9) % 45.0);
            let status = if growth > 10.0 {
                "🟢 Good"
            } else if growth > 0.0 {
                "🟡 Fair"
            } else {
                "🔴 Low"
            };

            // Alternating row colors
            if row % 2 == 0 {
                page.graphics()
                    .set_fill_color(Color::rgb(0.98, 0.98, 0.98))
                    .rectangle(50.0, y_pos - 12.0, 495.0, 15.0)
                    .fill();
            }

            page.text()
                .set_font(Font::Helvetica, 9.0)
                .at(55.0, y_pos - 5.0)
                .write(region)?;

            page.text()
                .set_font(Font::Helvetica, 9.0)
                .at(150.0, y_pos - 5.0)
                .write(product)?;

            page.text()
                .set_font(Font::Helvetica, 9.0)
                .at(250.0, y_pos - 5.0)
                .write(sales_rep)?;

            page.text()
                .set_font(Font::CourierBold, 9.0)
                .at(350.0, y_pos - 5.0)
                .write(&format!("${}", sales))?;

            page.text()
                .set_font(Font::Courier, 9.0)
                .set_fill_color(if growth > 0.0 {
                    Color::rgb(0.0, 0.6, 0.0)
                } else {
                    Color::rgb(0.8, 0.0, 0.0)
                })
                .at(430.0, y_pos - 5.0)
                .write(&format!("{:+.1}%", growth))?;

            page.text()
                .set_font(Font::Helvetica, 8.0)
                .at(480.0, y_pos - 5.0)
                .write(status)?;

            y_pos -= 15.0;
        }

        // === ENHANCED CHART EVERY 3 PAGES (with unique data and mini sparklines) ===
        if (page_num + 1) % 3 == 0 {
            y_pos -= 20.0;

            let chart_type = (page_num / 3) % 3;
            let chart_title = match chart_type {
                0 => "Regional Performance Comparison",
                1 => "Product Sales Distribution",
                _ => "Quarterly Growth Trends",
            };

            page.text()
                .set_font(Font::HelveticaBold, 12.0)
                .set_fill_color(Color::rgb(0.1, 0.1, 0.1))
                .at(50.0, y_pos)
                .write(&format!("{} - Page {}", chart_title, page_num + 1))?;

            y_pos -= 30.0;

            // Chart background
            page.graphics()
                .set_fill_color(Color::rgb(0.97, 0.97, 0.98))
                .rectangle(50.0, y_pos - 100.0, 495.0, 95.0)
                .fill();

            // Chart grid lines
            for i in 0..5 {
                let grid_y = y_pos - (i as f64 * 20.0);
                page.graphics()
                    .set_stroke_color(Color::rgb(0.85, 0.85, 0.85))
                    .set_line_width(0.5)
                    .move_to(70.0, grid_y)
                    .line_to(530.0, grid_y)
                    .stroke();
            }

            // Chart with unique data per page
            let chart_width = 400.0;
            let num_bars = 6;
            let bar_width = chart_width / num_bars as f64;

            for i in 0..num_bars {
                // Generate unique bar heights based on page
                let base_performance = 25.0 + (page_num as f64 * 3.0);
                let bar_height =
                    base_performance + (i as f64 * 7.0) + ((page_num * (i + 1) * 17) as f64 % 30.0);
                let x = 70.0 + (i as f64 * bar_width);

                // Gradient effect with multiple rectangles
                for j in 0..5 {
                    let segment_height = bar_height / 5.0;
                    let segment_y = y_pos - bar_height + (j as f64 * segment_height);
                    let alpha = 0.4 + ((5 - j) as f64 * 0.12);

                    let color_shift = (page_num as f64 * 0.07 + i as f64 * 0.11) % 1.0;
                    page.graphics()
                        .set_fill_color(Color::rgb(
                            (0.2 + color_shift * 0.3) * alpha,
                            (0.5 + color_shift * 0.2) * alpha,
                            (0.9 - color_shift * 0.3) * alpha,
                        ))
                        .rectangle(x + 10.0, segment_y, bar_width - 25.0, segment_height)
                        .fill();
                }

                // Bar border
                page.graphics()
                    .set_stroke_color(Color::rgb(0.3, 0.3, 0.5))
                    .set_line_width(1.0)
                    .rectangle(x + 10.0, y_pos - bar_height, bar_width - 25.0, bar_height)
                    .stroke();

                // Bar value (unique per page)
                let value = (bar_height * 2.5 + page_num as f64 * 75.0 + i as f64 * 120.0) as u32;
                page.text()
                    .set_font(Font::HelveticaBold, 8.0)
                    .set_fill_color(Color::rgb(0.1, 0.1, 0.3))
                    .at(x + bar_width / 2.0 - 12.0, y_pos - bar_height - 12.0)
                    .write(&format!("${}", value))?;

                // Mini sparkline trend below bar (unique per page)
                let sparkline_y = y_pos + 5.0;
                let sparkline_width = bar_width - 30.0;
                let num_points = 6;
                let mut last_x = x + 12.0;
                let mut last_y =
                    sparkline_y - ((page_num * (i + 1) * 7) % 15) as f64 - ((i * 2) as f64);

                for p in 1..num_points {
                    let px = x + 12.0 + (p as f64 * (sparkline_width / num_points as f64));
                    let trend_val = ((page_num * (p + 1) * (i + 1) * 13) % 15) as f64;
                    let py = sparkline_y - trend_val - ((i * 2) as f64);

                    page.graphics()
                        .set_stroke_color(Color::rgb(0.4, 0.6, 0.8))
                        .set_line_width(1.5)
                        .move_to(last_x, last_y)
                        .line_to(px, py)
                        .stroke();

                    last_x = px;
                    last_y = py;
                }

                // Label
                let label_text = match chart_type {
                    0 => {
                        let region_idx = (page_num * 3 + i) % regions.len();
                        regions[region_idx]
                    }
                    1 => {
                        let product_idx = (page_num * 5 + i) % products.len();
                        products[product_idx]
                    }
                    _ => &format!("Q{}", i + 1),
                };

                page.text()
                    .set_font(Font::Helvetica, 7.0)
                    .set_fill_color(Color::rgb(0.2, 0.2, 0.2))
                    .at(x + 5.0, y_pos + 20.0)
                    .write(label_text)?;
            }
        }

        // === FOOTER (with unique identifiers) ===
        let report_id = format!("RPT-{:04}-{:03}", 2025, page_num + 1);
        page.text()
            .set_font(Font::Helvetica, 8.0)
            .set_fill_color(Color::rgb(0.4, 0.4, 0.4))
            .at(50.0, 30.0)
            .write(&format!("CONFIDENTIAL - {} - ACME Corp", report_id))?;

        page.text()
            .set_font(Font::Helvetica, 8.0)
            .set_fill_color(Color::rgb(0.4, 0.4, 0.4))
            .at(450.0, 30.0)
            .write(&format!("Page {} of {}", page_num + 1, page_count))?;

        // Page border
        page.graphics()
            .set_stroke_color(Color::rgb(0.8, 0.8, 0.8))
            .set_line_width(0.5)
            .rectangle(40.0, 40.0, 515.0, 760.0)
            .stroke();

        doc.add_page(page);
    }

    let generation_time = start_time.elapsed();

    // Separate write timing
    let write_start = Instant::now();
    doc.save("examples/results/medium_complexity_benchmark.pdf")?;
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
    println!("COMPLEXITY=MEDIUM");

    println!("\n📊 Medium Complexity Benchmark Results:");
    println!("  📄 Pages: {}", page_count);
    println!("  ⚡ Generation: {}ms", generation_time.as_millis());
    println!("  💾 Write: {}ms", write_time.as_millis());
    println!("  🕐 Total: {}ms", total_time.as_millis());
    println!(
        "  📈 Performance: {:.1} pages/second",
        page_count as f64 / total_time.as_secs_f64()
    );
    println!("  📋 Content: Sales tables + charts + graphics per page");

    Ok(())
}
