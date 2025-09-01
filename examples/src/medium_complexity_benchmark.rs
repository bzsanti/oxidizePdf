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

        // === CHART EVERY 4 PAGES (with unique data) ===
        if (page_num + 1) % 4 == 0 {
            y_pos -= 20.0;

            page.text()
                .set_font(Font::HelveticaBold, 12.0)
                .set_fill_color(Color::rgb(0.1, 0.1, 0.1))
                .at(50.0, y_pos)
                .write(&format!("Q{} Regional Performance", (page_num / 4) + 1))?;

            y_pos -= 30.0;

            // Chart with unique data per page
            let chart_width = 400.0;
            let _chart_height = 80.0;
            let bar_width = chart_width / 5.0; // Use first 5 regions

            for i in 0..5 {
                // Generate unique bar heights based on page
                let base_performance = 30.0 + (page_num as f64 * 5.0);
                let bar_height =
                    base_performance + (i as f64 * 8.0) + ((page_num * (i + 1)) as f64 % 25.0);
                let x = 50.0 + (i as f64 * bar_width);

                // Bar with different colors per page
                let color_shift = (page_num as f64 * 0.1) % 1.0;
                page.graphics()
                    .set_fill_color(Color::rgb(
                        0.2 + color_shift * 0.3,
                        0.4 + color_shift * 0.2,
                        0.8 - color_shift * 0.3,
                    ))
                    .rectangle(x + 10.0, y_pos - bar_height, bar_width - 20.0, bar_height)
                    .fill();

                // Bar value (unique per page)
                let value = (bar_height * 2.1 + page_num as f64 * 50.0) as u32;
                page.text()
                    .set_font(Font::Helvetica, 8.0)
                    .set_fill_color(Color::rgb(0.1, 0.1, 0.1))
                    .at(x + bar_width / 2.0 - 15.0, y_pos - bar_height - 12.0)
                    .write(&format!("{}K", value))?;

                // Region label
                page.text()
                    .set_font(Font::Helvetica, 8.0)
                    .set_fill_color(Color::rgb(0.1, 0.1, 0.1))
                    .at(x + 5.0, y_pos + 10.0)
                    .write(regions[i])?;
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
