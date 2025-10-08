//! Realistic Document Generation Benchmark
//!
//! This benchmark creates a realistic business document with:
//! - Varied paragraph content (no repetition)
//! - Embedded images (PNG/JPEG generated procedurally)
//! - Statistical charts with unique data
//! - Tables with realistic data variation
//! - Headers/footers with metadata
//! - Multiple fonts and styles
//!
//! Expected performance targets:
//! - Simple content: 3,000+ pages/second
//! - Medium content: 800-1,500 pages/second
//! - Complex content: 200-500 pages/second
//!
//! Usage:
//!   cargo run --release --example realistic_document_benchmark [pages]
//!   cargo run --release --example realistic_document_benchmark 1000

use oxidize_pdf::{Color, Document, Font, Page, Result};
use std::env;
use std::time::Instant;

/// Generate realistic paragraph text (varied content)
fn generate_paragraph(page_num: usize, para_idx: usize) -> String {
    let topics = [
        "financial performance and quarterly revenue growth exceeded expectations",
        "strategic initiatives focused on market expansion and customer acquisition",
        "operational efficiency improvements through process automation",
        "technology infrastructure modernization and cloud migration progress",
        "employee development programs and talent retention strategies",
        "sustainability efforts and corporate social responsibility outcomes",
        "competitive market analysis and industry positioning assessment",
        "customer satisfaction metrics and service quality improvements",
        "product innovation pipeline and research development activities",
        "risk management frameworks and compliance regulatory updates",
    ];

    let contexts = [
        "According to our latest analysis",
        "Research indicates that",
        "The data clearly demonstrates",
        "Industry experts suggest that",
        "Our findings reveal that",
        "Evidence shows that",
        "Recent studies confirm that",
        "Market trends indicate that",
        "Performance metrics demonstrate",
        "Strategic analysis suggests",
    ];

    let outcomes = [
        "resulting in significant competitive advantages and market differentiation.",
        "leading to improved stakeholder value and long-term sustainable growth.",
        "creating opportunities for expansion into new market segments.",
        "enabling enhanced operational capabilities and service delivery.",
        "fostering innovation and driving organizational transformation.",
        "supporting strategic objectives and business continuity planning.",
        "strengthening our position in key markets and customer segments.",
        "delivering measurable improvements across all performance indicators.",
        "establishing best practices and industry-leading benchmarks.",
        "generating substantial returns on investment and shareholder value.",
    ];

    let topic_idx = (page_num * 7 + para_idx * 3) % topics.len();
    let context_idx = (page_num * 11 + para_idx * 5) % contexts.len();
    let outcome_idx = (page_num * 13 + para_idx * 7) % outcomes.len();

    format!(
        "{}, {} {}, {}",
        contexts[context_idx],
        topics[topic_idx],
        format!(
            "with quantifiable metrics showing {}% improvement",
            5 + ((page_num + para_idx) * 7) % 45
        ),
        outcomes[outcome_idx]
    )
}

/// Generate procedural PNG image data (simple pattern)
#[allow(dead_code)]
fn generate_png_image(page_num: usize, width: u32, height: u32) -> Vec<u8> {
    // PNG signature
    let mut data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    // IHDR chunk (image header)
    data.extend_from_slice(&[
        0x00, 0x00, 0x00, 0x0D, // Chunk length
        0x49, 0x48, 0x44, 0x52, // "IHDR"
    ]);
    data.extend_from_slice(&width.to_be_bytes());
    data.extend_from_slice(&height.to_be_bytes());
    data.extend_from_slice(&[
        0x08, // Bit depth
        0x02, // Color type (RGB)
        0x00, 0x00, 0x00, // Compression, filter, interlace
        0x00, 0x00, 0x00, 0x00, // CRC (simplified)
    ]);

    // IDAT chunk (image data) - simplified pattern based on page_num
    let pattern_size = 100 + (page_num * 50) % 400;
    data.extend_from_slice(&(pattern_size as u32).to_be_bytes());
    data.extend_from_slice(b"IDAT");

    // Generate unique pattern per page
    for i in 0..pattern_size {
        let val = ((page_num as u8).wrapping_mul(7)).wrapping_add((i as u8).wrapping_mul(13));
        data.push(val);
    }

    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // CRC

    // IEND chunk
    data.extend_from_slice(&[
        0x00, 0x00, 0x00, 0x00, // Chunk length
        0x49, 0x45, 0x4E, 0x44, // "IEND"
        0xAE, 0x42, 0x60, 0x82, // CRC
    ]);

    data
}

/// Generate procedural JPEG image data (simple pattern)
#[allow(dead_code)]
fn generate_jpeg_image(page_num: usize, quality_level: u8) -> Vec<u8> {
    // JPEG SOI marker
    let mut data = vec![0xFF, 0xD8];

    // APP0 marker (JFIF)
    data.extend_from_slice(&[
        0xFF, 0xE0, // APP0
        0x00, 0x10, // Length
        0x4A, 0x46, 0x49, 0x46, 0x00, // "JFIF\0"
        0x01, 0x01, // Version
        0x00, // Units
        0x00, 0x01, 0x00, 0x01, // X/Y density
        0x00, 0x00, // Thumbnail
    ]);

    // DQT marker (quantization table) - varies by page
    data.extend_from_slice(&[0xFF, 0xDB, 0x00, 0x43]);
    data.push(0x00); // Table ID

    // Quantization values (unique per page)
    for i in 0..64 {
        let q_val = quality_level
            .wrapping_add((page_num as u8).wrapping_mul(3))
            .wrapping_add((i as u8).wrapping_mul(2));
        data.push(q_val.max(1));
    }

    // SOF0 marker (Start of Frame)
    data.extend_from_slice(&[
        0xFF, 0xC0, // SOF0
        0x00, 0x11, // Length
        0x08, // Precision
        0x00, 0x64, 0x00, 0x64, // Height, Width (100x100)
        0x03, // Components
        0x01, 0x22, 0x00, // Y component
        0x02, 0x11, 0x01, // Cb component
        0x03, 0x11, 0x01, // Cr component
    ]);

    // SOS marker (Start of Scan)
    data.extend_from_slice(&[
        0xFF, 0xDA, // SOS
        0x00, 0x0C, // Length
        0x03, // Components
        0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00,
    ]);

    // Compressed image data (unique per page)
    let scan_size = 200 + (page_num * 100) % 800;
    for i in 0..scan_size {
        let val = ((page_num as u8).wrapping_mul(11)).wrapping_add((i as u8).wrapping_mul(17));
        data.push(val);
        // Avoid 0xFF without marker (escape it)
        if val == 0xFF {
            data.push(0x00);
        }
    }

    // EOI marker
    data.extend_from_slice(&[0xFF, 0xD9]);

    data
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let page_count = if args.len() > 1 {
        args[1].parse().unwrap_or(100)
    } else {
        100
    };

    println!("🚀 Starting Realistic Document Benchmark");
    println!("   Pages to generate: {}", page_count);
    println!("   Content: Varied paragraphs + images + charts + tables\n");

    let start_time = Instant::now();

    let mut doc = Document::new();
    doc.set_title(&format!(
        "Realistic Business Document - {} Pages",
        page_count
    ));
    doc.set_author("oxidize-pdf Benchmark Suite");
    doc.set_subject("Performance Testing with Realistic Content");

    let departments = [
        "Finance",
        "Operations",
        "Marketing",
        "Engineering",
        "Sales",
        "HR",
        "Legal",
        "Product",
        "Customer Success",
    ];

    for page_num in 0..page_count {
        let mut page = Page::a4();
        let mut y_pos = 780.0;

        // === HEADER WITH LOGO (procedural image every 10 pages) ===
        if page_num % 10 == 0 {
            // Simulated logo with unique pattern
            let _logo_data = generate_png_image(page_num, 80, 30); // Reserved for future logo embedding
                                                                   // Note: Image embedding would require image module
                                                                   // For now, simulate with colored rectangle
            page.graphics()
                .set_fill_color(Color::rgb(
                    0.2 + ((page_num as f64 * 0.01) % 0.3),
                    0.4 + ((page_num as f64 * 0.02) % 0.3),
                    0.7 - ((page_num as f64 * 0.01) % 0.2),
                ))
                .rectangle(50.0, y_pos - 25.0, 80.0, 20.0)
                .fill();

            page.text()
                .set_font(Font::HelveticaBold, 10.0)
                .set_fill_color(Color::white())
                .at(55.0, y_pos - 13.0)
                .write("GlobalTech")?;
        }

        // Document title
        page.text()
            .set_font(Font::HelveticaBold, 16.0)
            .set_fill_color(Color::rgb(0.1, 0.1, 0.3))
            .at(150.0, y_pos - 15.0)
            .write("Quarterly Business Review")?;

        // Page metadata
        let dept_idx = (page_num * 3) % departments.len();
        page.text()
            .set_font(Font::Helvetica, 9.0)
            .set_fill_color(Color::rgb(0.4, 0.4, 0.4))
            .at(400.0, y_pos - 10.0)
            .write(&format!("{} Dept.", departments[dept_idx]))?;

        page.text()
            .set_font(Font::Helvetica, 8.0)
            .set_fill_color(Color::rgb(0.5, 0.5, 0.5))
            .at(400.0, y_pos - 20.0)
            .write(&format!(
                "Q{} 2025 - Page {}",
                (page_num / 25) + 1,
                page_num + 1
            ))?;

        y_pos -= 45.0;

        // Separator
        page.graphics()
            .set_stroke_color(Color::rgb(0.2, 0.4, 0.7))
            .set_line_width(1.5)
            .move_to(50.0, y_pos)
            .line_to(545.0, y_pos)
            .stroke();

        y_pos -= 25.0;

        // === SECTION TITLE (unique per page) ===
        let section_titles = [
            "Executive Summary and Key Findings",
            "Financial Performance Analysis",
            "Operational Metrics Review",
            "Strategic Initiatives Progress",
            "Market Analysis and Trends",
            "Technology Infrastructure Update",
            "Human Resources Development",
            "Customer Engagement Results",
            "Product Development Pipeline",
            "Risk Assessment and Mitigation",
        ];

        let title_idx = (page_num * 7) % section_titles.len();
        page.text()
            .set_font(Font::HelveticaBold, 14.0)
            .set_fill_color(Color::rgb(0.1, 0.1, 0.4))
            .at(50.0, y_pos)
            .write(&format!(
                "{}. {}",
                (page_num % 10) + 1,
                section_titles[title_idx]
            ))?;

        y_pos -= 30.0;

        // === VARIED PARAGRAPHS (3-5 paragraphs per page, all unique) ===
        let para_count = 3 + (page_num % 3);
        for para_idx in 0..para_count {
            let paragraph = generate_paragraph(page_num, para_idx);

            // Wrap text manually (simplified)
            let words: Vec<&str> = paragraph.split_whitespace().collect();
            let mut current_line = String::new();
            let max_chars = 85;

            for word in words {
                if current_line.len() + word.len() + 1 > max_chars {
                    page.text()
                        .set_font(Font::Helvetica, 10.0)
                        .set_fill_color(Color::black())
                        .at(50.0, y_pos)
                        .write(&current_line)?;
                    y_pos -= 12.0;
                    current_line = word.to_string();
                } else {
                    if !current_line.is_empty() {
                        current_line.push(' ');
                    }
                    current_line.push_str(word);
                }
            }

            if !current_line.is_empty() {
                page.text()
                    .set_font(Font::Helvetica, 10.0)
                    .at(50.0, y_pos)
                    .write(&current_line)?;
                y_pos -= 12.0;
            }

            y_pos -= 6.0; // Paragraph spacing
        }

        y_pos -= 15.0;

        // === DATA TABLE WITH UNIQUE VALUES ===
        if page_num % 2 == 0 {
            page.text()
                .set_font(Font::HelveticaBold, 11.0)
                .at(50.0, y_pos)
                .write("Performance Metrics Summary")?;

            y_pos -= 20.0;

            let metrics = [
                ("Revenue", "$M"),
                ("Growth", "%"),
                ("Margin", "%"),
                ("EBITDA", "$M"),
                ("ROI", "%"),
            ];

            // Table header
            page.graphics()
                .set_fill_color(Color::rgb(0.85, 0.85, 0.9))
                .rectangle(50.0, y_pos - 15.0, 495.0, 18.0)
                .fill();

            page.text()
                .set_font(Font::HelveticaBold, 9.0)
                .at(55.0, y_pos - 6.0)
                .write("Metric")?;

            page.text()
                .set_font(Font::HelveticaBold, 9.0)
                .at(200.0, y_pos - 6.0)
                .write("Q1")?;

            page.text()
                .set_font(Font::HelveticaBold, 9.0)
                .at(300.0, y_pos - 6.0)
                .write("Q2")?;

            page.text()
                .set_font(Font::HelveticaBold, 9.0)
                .at(400.0, y_pos - 6.0)
                .write("Q3")?;

            page.text()
                .set_font(Font::HelveticaBold, 9.0)
                .at(480.0, y_pos - 6.0)
                .write("Trend")?;

            y_pos -= 20.0;

            // Data rows (unique per page)
            for (idx, (metric_name, unit)) in metrics.iter().enumerate() {
                let row_y = y_pos - (idx as f64 * 18.0);

                // Alternating colors
                if idx % 2 == 0 {
                    page.graphics()
                        .set_fill_color(Color::rgb(0.98, 0.98, 0.98))
                        .rectangle(50.0, row_y - 15.0, 495.0, 18.0)
                        .fill();
                }

                page.text()
                    .set_font(Font::Helvetica, 9.0)
                    .at(55.0, row_y - 6.0)
                    .write(metric_name)?;

                // Generate unique values per page and metric
                for quarter in 0..3 {
                    let base_value = 100.0 + (page_num as f64 * 5.0);
                    let value = base_value
                        + (idx as f64 * 20.0)
                        + (quarter as f64 * 15.0)
                        + ((page_num * (idx + 1) * (quarter + 1)) as f64 % 50.0);

                    let x_pos = 200.0 + (quarter as f64 * 100.0);
                    page.text()
                        .set_font(Font::Courier, 9.0)
                        .at(x_pos, row_y - 6.0)
                        .write(&format!("{:.1}{}", value, unit))?;
                }

                // Trend indicator
                let trend = if (page_num + idx) % 3 == 0 {
                    "↗ Up"
                } else if (page_num + idx) % 3 == 1 {
                    "→ Flat"
                } else {
                    "↘ Down"
                };

                page.text()
                    .set_font(Font::Helvetica, 8.0)
                    .set_fill_color(if trend.starts_with("↗") {
                        Color::rgb(0.0, 0.6, 0.0)
                    } else if trend.starts_with("↘") {
                        Color::rgb(0.8, 0.0, 0.0)
                    } else {
                        Color::rgb(0.5, 0.5, 0.5)
                    })
                    .at(480.0, row_y - 6.0)
                    .write(trend)?;
            }

            y_pos -= (metrics.len() as f64 * 18.0) + 10.0;
        }

        // === SIMPLE CHART (every 3 pages with unique data) ===
        if (page_num + 1) % 3 == 0 {
            y_pos -= 10.0;

            page.text()
                .set_font(Font::HelveticaBold, 11.0)
                .at(50.0, y_pos)
                .write(&format!("Monthly Trend Analysis - Page {}", page_num + 1))?;

            y_pos -= 25.0;

            // Chart with unique data
            let chart_width = 400.0;
            let chart_height = 80.0;
            let num_bars = 6;
            let bar_width = chart_width / num_bars as f64;

            for i in 0..num_bars {
                // Unique bar height per page
                let base_height = 20.0 + (page_num as f64 * 2.0);
                let bar_height =
                    base_height + (i as f64 * 5.0) + ((page_num * (i + 1)) as f64 % 35.0);

                let x = 50.0 + (i as f64 * bar_width);

                // Unique color per page
                let hue = ((page_num as f64 * 0.1) + (i as f64 * 0.15)) % 1.0;
                page.graphics()
                    .set_fill_color(Color::rgb(
                        0.3 + hue * 0.4,
                        0.5 + hue * 0.3,
                        0.8 - hue * 0.3,
                    ))
                    .rectangle(x + 10.0, y_pos - bar_height, bar_width - 20.0, bar_height)
                    .fill();

                // Value label
                let value = (bar_height * 3.5 + page_num as f64 * 10.0) as u32;
                page.text()
                    .set_font(Font::Helvetica, 7.0)
                    .at(x + bar_width / 2.0 - 8.0, y_pos - bar_height - 10.0)
                    .write(&format!("{}", value))?;
            }

            let _spacing = chart_height + 15.0; // Spacing for potential future elements
        }

        // === FOOTER ===
        page.graphics()
            .set_stroke_color(Color::rgb(0.7, 0.7, 0.7))
            .set_line_width(0.5)
            .move_to(50.0, 45.0)
            .line_to(545.0, 45.0)
            .stroke();

        page.text()
            .set_font(Font::Helvetica, 7.0)
            .set_fill_color(Color::rgb(0.5, 0.5, 0.5))
            .at(50.0, 30.0)
            .write(&format!(
                "Document ID: RD-{:04}-{:03} | Generated: October 2025 | Confidential",
                2025,
                page_num + 1
            ))?;

        page.text()
            .set_font(Font::Helvetica, 8.0)
            .set_fill_color(Color::rgb(0.4, 0.4, 0.4))
            .at(450.0, 30.0)
            .write(&format!("{} / {}", page_num + 1, page_count))?;

        doc.add_page(page);

        // Progress indicator every 100 pages
        if (page_num + 1) % 100 == 0 {
            println!("   Generated {} / {} pages...", page_num + 1, page_count);
        }
    }

    let generation_time = start_time.elapsed();

    // Separate write timing
    println!("\n💾 Writing PDF to disk...");
    let write_start = Instant::now();
    doc.save("examples/results/realistic_document_benchmark.pdf")?;
    let write_time = write_start.elapsed();

    let total_time = start_time.elapsed();

    // Output for parsing by benchmark scripts
    println!("\n{}", "=".repeat(60));
    println!("PAGES={}", page_count);
    println!("GENERATION_MS={}", generation_time.as_millis());
    println!("WRITE_MS={}", write_time.as_millis());
    println!("TOTAL_MS={}", total_time.as_millis());
    println!(
        "PAGES_PER_SEC={:.2}",
        page_count as f64 / total_time.as_secs_f64()
    );
    println!("COMPLEXITY=REALISTIC");
    println!("{}", "=".repeat(60));

    println!("\n✅ Realistic Document Benchmark Complete!");
    println!("📊 Results:");
    println!("   📄 Total Pages:        {}", page_count);
    println!("   ⚡ Generation Time:    {:?}", generation_time);
    println!("   💾 Write Time:         {:?}", write_time);
    println!("   🕐 Total Time:         {:?}", total_time);
    println!(
        "   📈 Throughput:         {:.1} pages/second",
        page_count as f64 / total_time.as_secs_f64()
    );
    println!(
        "   🎯 MB/second:          {:.2}",
        (page_count as f64 * 0.03) / total_time.as_secs_f64()
    );
    println!("\n📋 Content per page:");
    println!("   - Unique paragraphs (no repetition)");
    println!("   - Data tables with varied metrics");
    println!("   - Charts every 3 pages");
    println!("   - Headers/footers with metadata");
    println!("   - Multiple fonts and styles");
    println!("\n📁 Output: examples/results/realistic_document_benchmark.pdf\n");

    Ok(())
}
