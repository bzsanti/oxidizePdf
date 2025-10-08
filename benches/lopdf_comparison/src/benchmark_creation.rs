//! PDF Creation Benchmark: oxidize-pdf vs lopdf
//!
//! Compares performance creating identical PDFs:
//! - Simple document (text only)
//! - Medium complexity (text + tables + images)
//! - High complexity (graphics, gradients, patterns)

use std::io::Write;
use std::time::{Duration, Instant};

const NUM_PAGES: usize = 1000;

#[derive(serde::Serialize)]
struct BenchmarkResult {
    library: String,
    test_name: String,
    num_pages: usize,
    duration_ms: u128,
    pages_per_second: f64,
    file_size_bytes: usize,
}

fn main() {
    println!("🔥 PDF Creation Benchmark: oxidize-pdf vs lopdf");
    println!("================================================\n");

    let mut results = Vec::new();

    // Test 1: Simple text document
    println!("📄 Test 1: Simple text document ({} pages)", NUM_PAGES);
    results.push(bench_oxidize_simple());
    results.push(bench_lopdf_simple());

    // Test 2: Medium complexity
    println!("\n📊 Test 2: Medium complexity ({} pages)", NUM_PAGES);
    results.push(bench_oxidize_medium());
    results.push(bench_lopdf_medium());

    // Test 3: High complexity
    println!("\n🎨 Test 3: High complexity ({} pages)", NUM_PAGES);
    results.push(bench_oxidize_high());
    results.push(bench_lopdf_high());

    // Save results
    save_results(&results);

    // Print summary
    print_summary(&results);
}

fn bench_oxidize_simple() -> BenchmarkResult {
    let start = Instant::now();

    let mut doc = oxidize_pdf::Document::new();

    for page_num in 0..NUM_PAGES {
        let mut page = oxidize_pdf::Page::new(595.0, 842.0); // A4

        page.add_text(
            &format!("Page {} - Simple text document", page_num + 1),
            50.0,
            800.0,
            oxidize_pdf::Font::Helvetica,
            12.0,
            oxidize_pdf::Color::black(),
        );

        // Add 5 paragraphs per page
        for i in 0..5 {
            let text = format!(
                "Paragraph {} on page {}. This is realistic content with variation.",
                i + 1,
                page_num + 1
            );
            page.add_text(
                &text,
                50.0,
                750.0 - (i as f32 * 30.0),
                oxidize_pdf::Font::Helvetica,
                10.0,
                oxidize_pdf::Color::black(),
            );
        }

        doc.add_page(page);
    }

    let pdf_bytes = doc.write().expect("Failed to write PDF");
    let duration = start.elapsed();

    let result = BenchmarkResult {
        library: "oxidize-pdf".to_string(),
        test_name: "simple".to_string(),
        num_pages: NUM_PAGES,
        duration_ms: duration.as_millis(),
        pages_per_second: NUM_PAGES as f64 / duration.as_secs_f64(),
        file_size_bytes: pdf_bytes.len(),
    };

    std::fs::write(
        "benches/lopdf_comparison/results/oxidize_simple.pdf",
        pdf_bytes,
    )
    .ok();

    println!(
        "  oxidize-pdf: {:.2} pages/sec | {} bytes | {:.2}ms",
        result.pages_per_second,
        result.file_size_bytes,
        result.duration_ms
    );

    result
}

fn bench_lopdf_simple() -> BenchmarkResult {
    let start = Instant::now();

    let mut doc = lopdf::Document::with_version("1.5");

    // lopdf requires manual page setup
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let mut page_ids = Vec::new();

    for page_num in 0..NUM_PAGES {
        // Create content stream
        let content = format!(
            "BT\n/F1 12 Tf\n50 800 Td\n(Page {} - Simple text document) Tj\n",
            page_num + 1
        );

        let mut content_full = content.clone();
        for i in 0..5 {
            let para = format!(
                "/F1 10 Tf\n50 {} Td\n(Paragraph {} on page {}. This is realistic content with variation.) Tj\n",
                750 - i * 30,
                i + 1,
                page_num + 1
            );
            content_full.push_str(&para);
        }
        content_full.push_str("ET");

        let content_id = doc.add_object(lopdf::Stream::new(
            lopdf::dictionary! {},
            content_full.into_bytes(),
        ));

        let page_id = doc.add_object(lopdf::dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => lopdf::dictionary! {
                "Font" => lopdf::dictionary! {
                    "F1" => font_id,
                },
            },
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });

        page_ids.push(page_id);
    }

    let pages_dict = lopdf::dictionary! {
        "Type" => "Pages",
        "Count" => NUM_PAGES as i64,
        "Kids" => page_ids.into_iter().map(lopdf::Object::Reference).collect::<Vec<_>>(),
    };
    doc.objects.insert(pages_id, lopdf::Object::Dictionary(pages_dict));

    let catalog_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });

    doc.trailer.set("Root", catalog_id);

    let mut pdf_bytes = Vec::new();
    doc.save_to(&mut pdf_bytes)
        .expect("Failed to save lopdf document");

    let duration = start.elapsed();

    let result = BenchmarkResult {
        library: "lopdf".to_string(),
        test_name: "simple".to_string(),
        num_pages: NUM_PAGES,
        duration_ms: duration.as_millis(),
        pages_per_second: NUM_PAGES as f64 / duration.as_secs_f64(),
        file_size_bytes: pdf_bytes.len(),
    };

    std::fs::write("benches/lopdf_comparison/results/lopdf_simple.pdf", pdf_bytes).ok();

    println!(
        "  lopdf:       {:.2} pages/sec | {} bytes | {:.2}ms",
        result.pages_per_second,
        result.file_size_bytes,
        result.duration_ms
    );

    result
}

fn bench_oxidize_medium() -> BenchmarkResult {
    let start = Instant::now();

    let mut doc = oxidize_pdf::Document::new();

    for page_num in 0..NUM_PAGES {
        let mut page = oxidize_pdf::Page::new(595.0, 842.0);

        // Title
        page.add_text(
            &format!("Business Report - Page {}", page_num + 1),
            50.0,
            800.0,
            oxidize_pdf::Font::HelveticaBold,
            14.0,
            oxidize_pdf::Color::new(0, 51, 102),
        );

        // 2 paragraphs
        for i in 0..2 {
            let text = format!(
                "Section {} analysis: Key metrics show {}% improvement over baseline.",
                i + 1,
                ((page_num + i) * 7) % 30 + 10
            );
            page.add_text(
                &text,
                50.0,
                750.0 - (i as f32 * 40.0),
                oxidize_pdf::Font::Helvetica,
                10.0,
                oxidize_pdf::Color::black(),
            );
        }

        // Simple chart (rectangles)
        let chart_y = 600.0;
        for i in 0..5 {
            let height = ((page_num + i) * 13) % 100 + 20;
            page.add_rectangle(
                100.0 + (i as f32 * 80.0),
                chart_y,
                60.0,
                height as f32,
                oxidize_pdf::Color::new(70, 130, 180),
            );
        }

        doc.add_page(page);
    }

    let pdf_bytes = doc.write().expect("Failed to write PDF");
    let duration = start.elapsed();

    let result = BenchmarkResult {
        library: "oxidize-pdf".to_string(),
        test_name: "medium".to_string(),
        num_pages: NUM_PAGES,
        duration_ms: duration.as_millis(),
        pages_per_second: NUM_PAGES as f64 / duration.as_secs_f64(),
        file_size_bytes: pdf_bytes.len(),
    };

    std::fs::write(
        "benches/lopdf_comparison/results/oxidize_medium.pdf",
        pdf_bytes,
    )
    .ok();

    println!(
        "  oxidize-pdf: {:.2} pages/sec | {} bytes | {:.2}ms",
        result.pages_per_second,
        result.file_size_bytes,
        result.duration_ms
    );

    result
}

fn bench_lopdf_medium() -> BenchmarkResult {
    let start = Instant::now();

    let mut doc = lopdf::Document::with_version("1.5");

    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let font_bold_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica-Bold",
    });

    let mut page_ids = Vec::new();

    for page_num in 0..NUM_PAGES {
        let mut content = format!(
            "BT\n/F2 14 Tf\n0 0.2 0.4 rg\n50 800 Td\n(Business Report - Page {}) Tj\nET\n",
            page_num + 1
        );

        // Paragraphs
        content.push_str("BT\n/F1 10 Tf\n0 0 0 rg\n");
        for i in 0..2 {
            let improvement = ((page_num + i) * 7) % 30 + 10;
            content.push_str(&format!(
                "50 {} Td\n(Section {} analysis: Key metrics show {}% improvement over baseline.) Tj\n",
                750 - i * 40,
                i + 1,
                improvement
            ));
        }
        content.push_str("ET\n");

        // Chart (rectangles)
        content.push_str("0.27 0.51 0.71 rg\n");
        for i in 0..5 {
            let height = ((page_num + i) * 13) % 100 + 20;
            content.push_str(&format!(
                "{} 600 60 {} re f\n",
                100 + i * 80,
                height
            ));
        }

        let content_id = doc.add_object(lopdf::Stream::new(
            lopdf::dictionary! {},
            content.into_bytes(),
        ));

        let page_id = doc.add_object(lopdf::dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => lopdf::dictionary! {
                "Font" => lopdf::dictionary! {
                    "F1" => font_id,
                    "F2" => font_bold_id,
                },
            },
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });

        page_ids.push(page_id);
    }

    let pages_dict = lopdf::dictionary! {
        "Type" => "Pages",
        "Count" => NUM_PAGES as i64,
        "Kids" => page_ids.into_iter().map(lopdf::Object::Reference).collect::<Vec<_>>(),
    };
    doc.objects.insert(pages_id, lopdf::Object::Dictionary(pages_dict));

    let catalog_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });

    doc.trailer.set("Root", catalog_id);

    let mut pdf_bytes = Vec::new();
    doc.save_to(&mut pdf_bytes)
        .expect("Failed to save lopdf document");

    let duration = start.elapsed();

    let result = BenchmarkResult {
        library: "lopdf".to_string(),
        test_name: "medium".to_string(),
        num_pages: NUM_PAGES,
        duration_ms: duration.as_millis(),
        pages_per_second: NUM_PAGES as f64 / duration.as_secs_f64(),
        file_size_bytes: pdf_bytes.len(),
    };

    std::fs::write(
        "benches/lopdf_comparison/results/lopdf_medium.pdf",
        pdf_bytes,
    )
    .ok();

    println!(
        "  lopdf:       {:.2} pages/sec | {} bytes | {:.2}ms",
        result.pages_per_second,
        result.file_size_bytes,
        result.duration_ms
    );

    result
}

fn bench_oxidize_high() -> BenchmarkResult {
    let start = Instant::now();

    let mut doc = oxidize_pdf::Document::new();

    for page_num in 0..NUM_PAGES {
        let mut page = oxidize_pdf::Page::new(595.0, 842.0);

        // Complex graphics with gradients (simulated with multiple overlapping rects)
        for y in 0..10 {
            for x in 0..10 {
                let shade = ((page_num + x + y) * 17) % 200 + 55;
                page.add_rectangle(
                    50.0 + (x as f32 * 50.0),
                    700.0 - (y as f32 * 50.0),
                    50.0,
                    50.0,
                    oxidize_pdf::Color::new(shade as u8, (shade / 2) as u8, 100),
                );
            }
        }

        // Text overlay
        page.add_text(
            &format!("Complex Graphics - Page {}", page_num + 1),
            50.0,
            800.0,
            oxidize_pdf::Font::HelveticaBold,
            12.0,
            oxidize_pdf::Color::white(),
        );

        doc.add_page(page);
    }

    let pdf_bytes = doc.write().expect("Failed to write PDF");
    let duration = start.elapsed();

    let result = BenchmarkResult {
        library: "oxidize-pdf".to_string(),
        test_name: "high_complexity".to_string(),
        num_pages: NUM_PAGES,
        duration_ms: duration.as_millis(),
        pages_per_second: NUM_PAGES as f64 / duration.as_secs_f64(),
        file_size_bytes: pdf_bytes.len(),
    };

    std::fs::write(
        "benches/lopdf_comparison/results/oxidize_high.pdf",
        pdf_bytes,
    )
    .ok();

    println!(
        "  oxidize-pdf: {:.2} pages/sec | {} bytes | {:.2}ms",
        result.pages_per_second,
        result.file_size_bytes,
        result.duration_ms
    );

    result
}

fn bench_lopdf_high() -> BenchmarkResult {
    let start = Instant::now();

    let mut doc = lopdf::Document::with_version("1.5");

    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica-Bold",
    });

    let mut page_ids = Vec::new();

    for page_num in 0..NUM_PAGES {
        let mut content = String::new();

        // Complex graphics grid
        for y in 0..10 {
            for x in 0..10 {
                let shade = ((page_num + x + y) * 17) % 200 + 55;
                let r = shade as f64 / 255.0;
                let g = (shade / 2) as f64 / 255.0;
                let b = 100.0 / 255.0;
                content.push_str(&format!(
                    "{} {} {} rg\n{} {} 50 50 re f\n",
                    r,
                    g,
                    b,
                    50 + x * 50,
                    700 - y * 50
                ));
            }
        }

        // Text overlay
        content.push_str(&format!(
            "BT\n1 1 1 rg\n/F1 12 Tf\n50 800 Td\n(Complex Graphics - Page {}) Tj\nET\n",
            page_num + 1
        ));

        let content_id = doc.add_object(lopdf::Stream::new(
            lopdf::dictionary! {},
            content.into_bytes(),
        ));

        let page_id = doc.add_object(lopdf::dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => lopdf::dictionary! {
                "Font" => lopdf::dictionary! {
                    "F1" => font_id,
                },
            },
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });

        page_ids.push(page_id);
    }

    let pages_dict = lopdf::dictionary! {
        "Type" => "Pages",
        "Count" => NUM_PAGES as i64,
        "Kids" => page_ids.into_iter().map(lopdf::Object::Reference).collect::<Vec<_>>(),
    };
    doc.objects.insert(pages_id, lopdf::Object::Dictionary(pages_dict));

    let catalog_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });

    doc.trailer.set("Root", catalog_id);

    let mut pdf_bytes = Vec::new();
    doc.save_to(&mut pdf_bytes)
        .expect("Failed to save lopdf document");

    let duration = start.elapsed();

    let result = BenchmarkResult {
        library: "lopdf".to_string(),
        test_name: "high_complexity".to_string(),
        num_pages: NUM_PAGES,
        duration_ms: duration.as_millis(),
        pages_per_second: NUM_PAGES as f64 / duration.as_secs_f64(),
        file_size_bytes: pdf_bytes.len(),
    };

    std::fs::write(
        "benches/lopdf_comparison/results/lopdf_high.pdf",
        pdf_bytes,
    )
    .ok();

    println!(
        "  lopdf:       {:.2} pages/sec | {} bytes | {:.2}ms",
        result.pages_per_second,
        result.file_size_bytes,
        result.duration_ms
    );

    result
}

fn save_results(results: &[BenchmarkResult]) {
    std::fs::create_dir_all("benches/lopdf_comparison/results").ok();

    let json = serde_json::to_string_pretty(&results).expect("Failed to serialize results");
    std::fs::write(
        "benches/lopdf_comparison/results/creation_benchmark.json",
        json,
    )
    .expect("Failed to write results");
}

fn print_summary(results: &[BenchmarkResult]) {
    println!("\n📊 SUMMARY");
    println!("==========\n");

    for test in &["simple", "medium", "high_complexity"] {
        let oxidize = results.iter().find(|r| {
            r.library == "oxidize-pdf" && r.test_name == *test
        });
        let lopdf_result = results.iter().find(|r| {
            r.library == "lopdf" && r.test_name == *test
        });

        if let (Some(ox), Some(lo)) = (oxidize, lopdf_result) {
            let speedup = ox.pages_per_second / lo.pages_per_second;
            let size_diff =
                ((ox.file_size_bytes as f64 / lo.file_size_bytes as f64) - 1.0) * 100.0;

            println!("Test: {}", test);
            println!(
                "  Speed: oxidize-pdf is {:.2}x {} than lopdf",
                speedup.abs(),
                if speedup > 1.0 { "faster" } else { "slower" }
            );
            println!(
                "  Size:  oxidize-pdf is {:.1}% {} than lopdf\n",
                size_diff.abs(),
                if size_diff > 0.0 { "larger" } else { "smaller" }
            );
        }
    }
}
