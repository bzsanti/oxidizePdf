//! PDF Parsing Benchmark: oxidize-pdf vs lopdf
//!
//! Compares parsing performance on real-world PDFs

use std::path::PathBuf;
use std::time::Instant;

#[derive(serde::Serialize)]
struct ParsingResult {
    library: String,
    pdf_count: usize,
    successful: usize,
    failed: usize,
    total_duration_ms: u128,
    pdfs_per_second: f64,
}

fn main() {
    println!("🔍 PDF Parsing Benchmark: oxidize-pdf vs lopdf");
    println!("===============================================\n");

    // Find test PDFs
    let test_pdfs = find_test_pdfs();
    println!("Found {} test PDFs\n", test_pdfs.len());

    if test_pdfs.is_empty() {
        println!("⚠️  No test PDFs found. Skipping parsing benchmark.");
        println!("   Place PDFs in: benches/lopdf_comparison/test_pdfs/");
        return;
    }

    // Benchmark oxidize-pdf
    println!("📦 Testing oxidize-pdf...");
    let oxidize_result = bench_oxidize_parsing(&test_pdfs);

    // Benchmark lopdf
    println!("\n📦 Testing lopdf...");
    let lopdf_result = bench_lopdf_parsing(&test_pdfs);

    // Save and print results
    save_results(&[oxidize_result.clone(), lopdf_result.clone()]);
    print_summary(&oxidize_result, &lopdf_result);
}

fn find_test_pdfs() -> Vec<PathBuf> {
    let test_dir = PathBuf::from("benches/lopdf_comparison/test_pdfs");

    if !test_dir.exists() {
        std::fs::create_dir_all(&test_dir).ok();
        return Vec::new();
    }

    std::fs::read_dir(&test_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("pdf"))
                .collect()
        })
        .unwrap_or_default()
}

fn bench_oxidize_parsing(pdfs: &[PathBuf]) -> ParsingResult {
    let mut successful = 0;
    let mut failed = 0;

    let start = Instant::now();

    for (idx, pdf_path) in pdfs.iter().enumerate() {
        match oxidize_pdf::Document::load(pdf_path) {
            Ok(_doc) => {
                successful += 1;
                if (idx + 1) % 10 == 0 {
                    print!(".");
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                }
            }
            Err(e) => {
                failed += 1;
                println!(
                    "\n  ⚠️  Failed to parse {}: {}",
                    pdf_path.display(),
                    e
                );
            }
        }
    }

    let duration = start.elapsed();
    println!();

    ParsingResult {
        library: "oxidize-pdf".to_string(),
        pdf_count: pdfs.len(),
        successful,
        failed,
        total_duration_ms: duration.as_millis(),
        pdfs_per_second: pdfs.len() as f64 / duration.as_secs_f64(),
    }
}

fn bench_lopdf_parsing(pdfs: &[PathBuf]) -> ParsingResult {
    let mut successful = 0;
    let mut failed = 0;

    let start = Instant::now();

    for (idx, pdf_path) in pdfs.iter().enumerate() {
        match lopdf::Document::load(pdf_path) {
            Ok(_doc) => {
                successful += 1;
                if (idx + 1) % 10 == 0 {
                    print!(".");
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                }
            }
            Err(e) => {
                failed += 1;
                println!(
                    "\n  ⚠️  Failed to parse {}: {}",
                    pdf_path.display(),
                    e
                );
            }
        }
    }

    let duration = start.elapsed();
    println!();

    ParsingResult {
        library: "lopdf".to_string(),
        pdf_count: pdfs.len(),
        successful,
        failed,
        total_duration_ms: duration.as_millis(),
        pdfs_per_second: pdfs.len() as f64 / duration.as_secs_f64(),
    }
}

fn save_results(results: &[ParsingResult]) {
    std::fs::create_dir_all("benches/lopdf_comparison/results").ok();

    let json = serde_json::to_string_pretty(&results).expect("Failed to serialize results");
    std::fs::write(
        "benches/lopdf_comparison/results/parsing_benchmark.json",
        json,
    )
    .expect("Failed to write results");
}

fn print_summary(oxidize: &ParsingResult, lopdf: &ParsingResult) {
    println!("\n📊 PARSING SUMMARY");
    println!("==================\n");

    println!("oxidize-pdf:");
    println!("  ✅ Successful: {}/{}", oxidize.successful, oxidize.pdf_count);
    println!("  ❌ Failed:     {}/{}", oxidize.failed, oxidize.pdf_count);
    println!("  ⚡ Speed:      {:.2} PDFs/second", oxidize.pdfs_per_second);
    println!("  ⏱️  Duration:   {:.2}ms\n", oxidize.total_duration_ms);

    println!("lopdf:");
    println!("  ✅ Successful: {}/{}", lopdf.successful, lopdf.pdf_count);
    println!("  ❌ Failed:     {}/{}", lopdf.failed, lopdf.pdf_count);
    println!("  ⚡ Speed:      {:.2} PDFs/second", lopdf.pdfs_per_second);
    println!("  ⏱️  Duration:   {:.2}ms\n", lopdf.total_duration_ms);

    let speedup = oxidize.pdfs_per_second / lopdf.pdfs_per_second;
    println!(
        "📈 oxidize-pdf is {:.2}x {} than lopdf",
        speedup.abs(),
        if speedup > 1.0 { "faster" } else { "slower" }
    );

    let success_diff = oxidize.successful as f64 / oxidize.pdf_count as f64
        - lopdf.successful as f64 / lopdf.pdf_count as f64;
    if success_diff.abs() > 0.01 {
        println!(
            "📊 Success rate difference: {:.1}%",
            success_diff * 100.0
        );
    }
}
