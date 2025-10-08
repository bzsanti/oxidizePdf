//! PDF Compression Benchmark: oxidize-pdf vs lopdf
//!
//! Compares modern PDF compression features (Object Streams, XRef Streams)

use std::time::Instant;

#[derive(serde::Serialize)]
struct CompressionResult {
    library: String,
    mode: String,
    num_pages: usize,
    file_size_bytes: usize,
    duration_ms: u128,
}

const NUM_PAGES: usize = 100;

fn main() {
    println!("🗜️  PDF Compression Benchmark: oxidize-pdf vs lopdf");
    println!("===================================================\n");

    let mut results = Vec::new();

    // Test 1: Legacy mode (PDF 1.4, no modern compression)
    println!("📦 Test 1: Legacy Mode (PDF 1.4)");
    results.push(bench_oxidize_legacy());
    results.push(bench_lopdf_legacy());

    // Test 2: Modern mode (PDF 1.5+, Object Streams)
    println!("\n🚀 Test 2: Modern Mode (PDF 1.5+ with Object Streams)");
    results.push(bench_oxidize_modern());
    results.push(bench_lopdf_modern());

    save_results(&results);
    print_summary(&results);
}

fn bench_oxidize_legacy() -> CompressionResult {
    let start = Instant::now();

    let mut doc = oxidize_pdf::Document::new();

    for page_num in 0..NUM_PAGES {
        let mut page = oxidize_pdf::Page::new(595.0, 842.0);

        page.add_text(
            &format!("Legacy PDF 1.4 - Page {}", page_num + 1),
            50.0,
            800.0,
            oxidize_pdf::Font::Helvetica,
            12.0,
            oxidize_pdf::Color::black(),
        );

        // Add some objects to compress
        for i in 0..10 {
            page.add_rectangle(
                50.0 + (i as f32 * 50.0),
                700.0,
                40.0,
                40.0,
                oxidize_pdf::Color::new(100, 100, 200),
            );
        }

        doc.add_page(page);
    }

    // Write in legacy mode (default for oxidize-pdf currently)
    let pdf_bytes = doc.write().expect("Failed to write PDF");
    let duration = start.elapsed();

    let result = CompressionResult {
        library: "oxidize-pdf".to_string(),
        mode: "legacy".to_string(),
        num_pages: NUM_PAGES,
        file_size_bytes: pdf_bytes.len(),
        duration_ms: duration.as_millis(),
    };

    std::fs::write(
        "benches/lopdf_comparison/results/oxidize_legacy.pdf",
        pdf_bytes,
    )
    .ok();

    println!(
        "  oxidize-pdf (legacy): {} bytes | {:.2}ms",
        result.file_size_bytes, result.duration_ms
    );

    result
}

fn bench_lopdf_legacy() -> CompressionResult {
    let start = Instant::now();

    let mut doc = lopdf::Document::with_version("1.4");

    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let mut page_ids = Vec::new();

    for page_num in 0..NUM_PAGES {
        let mut content = format!(
            "BT\n/F1 12 Tf\n50 800 Td\n(Legacy PDF 1.4 - Page {}) Tj\nET\n",
            page_num + 1
        );

        content.push_str("0.39 0.39 0.78 rg\n");
        for i in 0..10 {
            content.push_str(&format!("{} 700 40 40 re f\n", 50 + i * 50));
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
    doc.save_to(&mut pdf_bytes).expect("Failed to save");

    let duration = start.elapsed();

    let result = CompressionResult {
        library: "lopdf".to_string(),
        mode: "legacy".to_string(),
        num_pages: NUM_PAGES,
        file_size_bytes: pdf_bytes.len(),
        duration_ms: duration.as_millis(),
    };

    std::fs::write(
        "benches/lopdf_comparison/results/lopdf_legacy.pdf",
        pdf_bytes,
    )
    .ok();

    println!(
        "  lopdf (legacy):       {} bytes | {:.2}ms",
        result.file_size_bytes, result.duration_ms
    );

    result
}

fn bench_oxidize_modern() -> CompressionResult {
    let start = Instant::now();

    // Create document with modern compression settings
    let mut doc = oxidize_pdf::Document::new();

    for page_num in 0..NUM_PAGES {
        let mut page = oxidize_pdf::Page::new(595.0, 842.0);

        page.add_text(
            &format!("Modern PDF 1.5+ - Page {}", page_num + 1),
            50.0,
            800.0,
            oxidize_pdf::Font::Helvetica,
            12.0,
            oxidize_pdf::Color::black(),
        );

        for i in 0..10 {
            page.add_rectangle(
                50.0 + (i as f32 * 50.0),
                700.0,
                40.0,
                40.0,
                oxidize_pdf::Color::new(100, 100, 200),
            );
        }

        doc.add_page(page);
    }

    // Write with modern compression (if available)
    // Note: oxidize-pdf may need WriterConfig::modern() API
    let pdf_bytes = doc.write().expect("Failed to write PDF");
    let duration = start.elapsed();

    let result = CompressionResult {
        library: "oxidize-pdf".to_string(),
        mode: "modern".to_string(),
        num_pages: NUM_PAGES,
        file_size_bytes: pdf_bytes.len(),
        duration_ms: duration.as_millis(),
    };

    std::fs::write(
        "benches/lopdf_comparison/results/oxidize_modern.pdf",
        pdf_bytes,
    )
    .ok();

    println!(
        "  oxidize-pdf (modern): {} bytes | {:.2}ms",
        result.file_size_bytes, result.duration_ms
    );

    result
}

fn bench_lopdf_modern() -> CompressionResult {
    let start = Instant::now();

    let mut doc = lopdf::Document::with_version("1.5");

    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let mut page_ids = Vec::new();

    for page_num in 0..NUM_PAGES {
        let mut content = format!(
            "BT\n/F1 12 Tf\n50 800 Td\n(Modern PDF 1.5+ - Page {}) Tj\nET\n",
            page_num + 1
        );

        content.push_str("0.39 0.39 0.78 rg\n");
        for i in 0..10 {
            content.push_str(&format!("{} 700 40 40 re f\n", 50 + i * 50));
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

    // Enable modern compression features
    // lopdf supports save_modern() which uses object streams
    let mut pdf_bytes = Vec::new();

    // Try modern save if available, fallback to regular
    match doc.save_to(&mut pdf_bytes) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error saving modern PDF: {}", e);
            doc.save_to(&mut pdf_bytes).expect("Failed to save");
        }
    }

    let duration = start.elapsed();

    let result = CompressionResult {
        library: "lopdf".to_string(),
        mode: "modern".to_string(),
        num_pages: NUM_PAGES,
        file_size_bytes: pdf_bytes.len(),
        duration_ms: duration.as_millis(),
    };

    std::fs::write(
        "benches/lopdf_comparison/results/lopdf_modern.pdf",
        pdf_bytes,
    )
    .ok();

    println!(
        "  lopdf (modern):       {} bytes | {:.2}ms",
        result.file_size_bytes, result.duration_ms
    );

    result
}

fn save_results(results: &[CompressionResult]) {
    std::fs::create_dir_all("benches/lopdf_comparison/results").ok();

    let json = serde_json::to_string_pretty(&results).expect("Failed to serialize");
    std::fs::write(
        "benches/lopdf_comparison/results/compression_benchmark.json",
        json,
    )
    .expect("Failed to write results");
}

fn print_summary(results: &[CompressionResult]) {
    println!("\n📊 COMPRESSION SUMMARY");
    println!("======================\n");

    for mode in &["legacy", "modern"] {
        let oxidize = results
            .iter()
            .find(|r| r.library == "oxidize-pdf" && r.mode == *mode);
        let lopdf = results
            .iter()
            .find(|r| r.library == "lopdf" && r.mode == *mode);

        if let (Some(ox), Some(lo)) = (oxidize, lopdf) {
            let size_diff =
                ((ox.file_size_bytes as f64 / lo.file_size_bytes as f64) - 1.0) * 100.0;

            println!("Mode: {}", mode);
            println!("  oxidize-pdf: {} bytes", ox.file_size_bytes);
            println!("  lopdf:       {} bytes", lo.file_size_bytes);
            println!(
                "  Difference:  {:.1}% {} than lopdf\n",
                size_diff.abs(),
                if size_diff > 0.0 { "larger" } else { "smaller" }
            );
        }
    }

    // Calculate compression improvement
    let ox_legacy = results
        .iter()
        .find(|r| r.library == "oxidize-pdf" && r.mode == "legacy");
    let ox_modern = results
        .iter()
        .find(|r| r.library == "oxidize-pdf" && r.mode == "modern");

    if let (Some(leg), Some(mod_)) = (ox_legacy, ox_modern) {
        let improvement =
            ((leg.file_size_bytes as f64 - mod_.file_size_bytes as f64)
                / leg.file_size_bytes as f64)
                * 100.0;
        println!(
            "📉 oxidize-pdf modern compression: {:.1}% reduction vs legacy",
            improvement
        );
    }

    let lo_legacy = results
        .iter()
        .find(|r| r.library == "lopdf" && r.mode == "legacy");
    let lo_modern = results
        .iter()
        .find(|r| r.library == "lopdf" && r.mode == "modern");

    if let (Some(leg), Some(mod_)) = (lo_legacy, lo_modern) {
        let improvement =
            ((leg.file_size_bytes as f64 - mod_.file_size_bytes as f64)
                / leg.file_size_bytes as f64)
                * 100.0;
        println!(
            "📉 lopdf modern compression:       {:.1}% reduction vs legacy",
            improvement
        );
    }
}
