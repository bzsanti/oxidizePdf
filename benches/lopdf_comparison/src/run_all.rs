//! Run all benchmarks and generate comparison report

use std::process::Command;
use std::time::Instant;

fn main() {
    println!("🚀 Running Complete Benchmark Suite: oxidize-pdf vs lopdf");
    println!("==========================================================\n");

    let total_start = Instant::now();

    // Create results directory
    std::fs::create_dir_all("benches/lopdf_comparison/results")
        .expect("Failed to create results directory");

    // Run all benchmarks
    run_benchmark("Creation", "benchmark_creation");
    run_benchmark("Parsing", "benchmark_parsing");
    run_benchmark("Compression", "benchmark_compression");

    let total_duration = total_start.elapsed();

    println!(
        "\n✅ All benchmarks completed in {:.2}s",
        total_duration.as_secs_f64()
    );
    println!("\n📁 Results saved to: benches/lopdf_comparison/results/");
    println!("   - creation_benchmark.json");
    println!("   - parsing_benchmark.json");
    println!("   - compression_benchmark.json");

    // Generate summary report
    generate_summary_report();
}

fn run_benchmark(name: &str, binary: &str) {
    println!("\n{'='*60}");
    println!("Running {} Benchmark", name);
    println!("{'='*60}\n");

    let status = Command::new("cargo")
        .args(&[
            "run",
            "--release",
            "--bin",
            binary,
            "--manifest-path",
            "benches/lopdf_comparison/Cargo.toml",
        ])
        .status();

    match status {
        Ok(exit) => {
            if !exit.success() {
                eprintln!("⚠️  {} benchmark exited with error", name);
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to run {} benchmark: {}", name, e);
        }
    }
}

fn generate_summary_report() {
    println!("\n📊 Generating Summary Report...");

    let mut report = String::new();
    report.push_str("# oxidize-pdf vs lopdf Benchmark Results\n\n");
    report.push_str(&format!(
        "**Date**: {}\n\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    ));

    // Read and summarize creation benchmark
    if let Ok(data) =
        std::fs::read_to_string("benches/lopdf_comparison/results/creation_benchmark.json")
    {
        if let Ok(results) = serde_json::from_str::<Vec<serde_json::Value>>(&data) {
            report.push_str("## 📄 Creation Performance\n\n");
            report.push_str("| Test | Library | Pages/Sec | File Size | Duration |\n");
            report.push_str("|------|---------|-----------|-----------|----------|\n");

            for result in results {
                let lib = result["library"].as_str().unwrap_or("?");
                let test = result["test_name"].as_str().unwrap_or("?");
                let pps = result["pages_per_second"].as_f64().unwrap_or(0.0);
                let size = result["file_size_bytes"].as_u64().unwrap_or(0);
                let dur = result["duration_ms"].as_u64().unwrap_or(0);

                report.push_str(&format!(
                    "| {} | {} | {:.2} | {} | {}ms |\n",
                    test, lib, pps, size, dur
                ));
            }
            report.push_str("\n");
        }
    }

    // Read and summarize parsing benchmark
    if let Ok(data) =
        std::fs::read_to_string("benches/lopdf_comparison/results/parsing_benchmark.json")
    {
        if let Ok(results) = serde_json::from_str::<Vec<serde_json::Value>>(&data) {
            report.push_str("## 🔍 Parsing Performance\n\n");
            report.push_str("| Library | Success Rate | PDFs/Sec | Duration |\n");
            report.push_str("|---------|--------------|----------|----------|\n");

            for result in results {
                let lib = result["library"].as_str().unwrap_or("?");
                let success = result["successful"].as_u64().unwrap_or(0);
                let total = result["pdf_count"].as_u64().unwrap_or(1);
                let pps = result["pdfs_per_second"].as_f64().unwrap_or(0.0);
                let dur = result["total_duration_ms"].as_u64().unwrap_or(0);

                let rate = (success as f64 / total as f64) * 100.0;
                report.push_str(&format!(
                    "| {} | {}/{} ({:.1}%) | {:.2} | {}ms |\n",
                    lib, success, total, rate, pps, dur
                ));
            }
            report.push_str("\n");
        }
    }

    // Read and summarize compression benchmark
    if let Ok(data) =
        std::fs::read_to_string("benches/lopdf_comparison/results/compression_benchmark.json")
    {
        if let Ok(results) = serde_json::from_str::<Vec<serde_json::Value>>(&data) {
            report.push_str("## 🗜️  Compression Performance\n\n");
            report.push_str("| Mode | Library | File Size | Duration |\n");
            report.push_str("|------|---------|-----------|----------|\n");

            for result in results {
                let lib = result["library"].as_str().unwrap_or("?");
                let mode = result["mode"].as_str().unwrap_or("?");
                let size = result["file_size_bytes"].as_u64().unwrap_or(0);
                let dur = result["duration_ms"].as_u64().unwrap_or(0);

                report.push_str(&format!(
                    "| {} | {} | {} bytes | {}ms |\n",
                    mode, lib, size, dur
                ));
            }
            report.push_str("\n");
        }
    }

    report.push_str("---\n\n");
    report.push_str("**Environment**:\n");
    report.push_str(&format!("- Rust: {}\n", rustc_version()));
    report.push_str(&format!("- oxidize-pdf: {}\n", env!("CARGO_PKG_VERSION")));
    report.push_str("- lopdf: 0.37\n");

    // Save report
    std::fs::write(
        "benches/lopdf_comparison/results/BENCHMARK_REPORT.md",
        &report,
    )
    .expect("Failed to write report");

    println!("✅ Summary report: benches/lopdf_comparison/results/BENCHMARK_REPORT.md");
}

fn rustc_version() -> String {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string()
}
