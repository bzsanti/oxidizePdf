//! CLI Performance Benchmarks
//!
//! Benchmarks for the oxidize-pdf CLI to measure command performance,
//! argument parsing, and PDF processing operations from the user interface perspective.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use oxidize_pdf_test_suite::generators::test_pdf_builder::TestPdfBuilder;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::TempDir;

/// Get the path to the CLI binary
fn get_cli_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove the test binary name
    path.pop(); // Remove target type (debug/release)
    path.pop(); // Remove target
    path.push("debug");
    path.push("oxidize-pdf");

    // If debug doesn't exist, try release
    if !path.exists() {
        path.pop();
        path.push("release");
        path.push("oxidize-pdf");
    }

    path
}

/// Setup test PDFs of various sizes for CLI benchmarking
fn setup_test_pdfs(temp_dir: &TempDir) -> Vec<(String, PathBuf)> {
    let mut test_files = Vec::new();

    // Minimal PDF
    let minimal_pdf = TestPdfBuilder::minimal().build();
    let minimal_path = temp_dir.path().join("minimal.pdf");
    fs::write(&minimal_path, minimal_pdf).unwrap();
    test_files.push(("minimal".to_string(), minimal_path));

    // Single page with text
    let mut text_builder = TestPdfBuilder::new();
    text_builder.add_text_page("Sample text for benchmarking CLI operations", 12.0);
    let text_pdf = text_builder.build();
    let text_path = temp_dir.path().join("text.pdf");
    fs::write(&text_path, text_pdf).unwrap();
    test_files.push(("text".to_string(), text_path));

    // Multiple pages
    let mut multi_builder = TestPdfBuilder::new();
    for i in 0..10 {
        multi_builder.add_text_page(&format!("Page {} content for benchmarking", i + 1), 14.0);
    }
    let multi_pdf = multi_builder.build();
    let multi_path = temp_dir.path().join("multi_page.pdf");
    fs::write(&multi_path, multi_pdf).unwrap();
    test_files.push(("10_pages".to_string(), multi_path));

    // Complex PDF with graphics
    let mut complex_builder = TestPdfBuilder::new();
    complex_builder.add_graphics_page();
    complex_builder.add_text_page("Complex document with graphics", 16.0);
    let complex_pdf = complex_builder.build();
    let complex_path = temp_dir.path().join("complex.pdf");
    fs::write(&complex_path, complex_pdf).unwrap();
    test_files.push(("complex".to_string(), complex_path));

    test_files
}

/// Execute a CLI command and measure its performance
fn run_cli_command(args: &[&str]) -> Result<std::process::Output, std::io::Error> {
    Command::new(get_cli_path())
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
}

/// Benchmark CLI command parsing and initialization
fn benchmark_cli_initialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("cli_initialization");

    // Benchmark help command (fastest)
    group.bench_function("help", |b| {
        b.iter(|| {
            let output = run_cli_command(&["--help"]);
            black_box(output)
        });
    });

    // Benchmark version command
    group.bench_function("version", |b| {
        b.iter(|| {
            let output = run_cli_command(&["--version"]);
            black_box(output)
        });
    });

    // Benchmark invalid command (error handling)
    group.bench_function("invalid_command", |b| {
        b.iter(|| {
            let output = run_cli_command(&["nonexistent-command"]);
            black_box(output)
        });
    });

    group.finish();
}

/// Benchmark PDF info command performance
fn benchmark_info_command(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let test_files = setup_test_pdfs(&temp_dir);
    let mut group = c.benchmark_group("info_command");

    for (name, path) in test_files {
        group.bench_with_input(BenchmarkId::new("info", &name), &path, |b, pdf_path| {
            b.iter(|| {
                let path_str = pdf_path.to_str().unwrap();
                let output = run_cli_command(&["info", path_str]);
                black_box(output)
            });
        });
    }

    group.finish();
}

/// Benchmark PDF creation command performance
fn benchmark_create_command(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let mut group = c.benchmark_group("create_command");

    // Different PDF types to create
    let creation_types = vec![
        ("minimal", vec!["create", "--minimal"]),
        ("single_page", vec!["create", "--pages", "1"]),
        ("multi_page", vec!["create", "--pages", "5"]),
        ("with_text", vec!["create", "--pages", "3", "--sample-text"]),
    ];

    for (name, mut args) in creation_types {
        // Add output file
        let output_path = temp_dir.path().join(format!("{name}.pdf"));
        let output_str = output_path.to_str().unwrap();
        args.extend_from_slice(&["--output", output_str]);

        group.bench_with_input(
            BenchmarkId::new("create", name),
            &args,
            |b, command_args| {
                b.iter(|| {
                    let output = run_cli_command(command_args);
                    let _ = black_box(output);
                    // Clean up the file for next iteration
                    let _ = fs::remove_file(&output_path);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark text extraction command performance
fn benchmark_extract_text_command(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let test_files = setup_test_pdfs(&temp_dir);
    let mut group = c.benchmark_group("extract_text_command");

    for (name, path) in test_files {
        group.bench_with_input(
            BenchmarkId::new("extract_text", &name),
            &path,
            |b, pdf_path| {
                b.iter(|| {
                    let path_str = pdf_path.to_str().unwrap();
                    let output = run_cli_command(&["extract-text", path_str]);
                    black_box(output)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark PDF rotation command performance  
fn benchmark_rotate_command(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let test_files = setup_test_pdfs(&temp_dir);
    let mut group = c.benchmark_group("rotate_command");

    for (name, path) in test_files {
        // Create a copy for rotation (since it modifies the file)
        let rotate_path = temp_dir.path().join(format!("rotate_{name}.pdf"));

        group.bench_with_input(
            BenchmarkId::new("rotate_90", &name),
            &path,
            |b, source_path| {
                b.iter(|| {
                    // Copy source file
                    fs::copy(source_path, &rotate_path).unwrap();

                    let path_str = rotate_path.to_str().unwrap();
                    let output = run_cli_command(&["rotate", path_str, "90"]);
                    let _ = black_box(output);

                    // Clean up for next iteration
                    let _ = fs::remove_file(&rotate_path);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark demo command performance
fn benchmark_demo_command(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let mut group = c.benchmark_group("demo_command");

    // Different demo types
    let demo_types = vec![
        ("text", vec!["demo", "text"]),
        ("graphics", vec!["demo", "graphics"]),
        ("complex", vec!["demo", "complex"]),
    ];

    for (name, mut args) in demo_types {
        let output_path = temp_dir.path().join(format!("demo_{name}.pdf"));
        let output_str = output_path.to_str().unwrap();
        args.extend_from_slice(&["--output", output_str]);

        group.bench_with_input(BenchmarkId::new("demo", name), &args, |b, command_args| {
            b.iter(|| {
                let output = run_cli_command(command_args);
                let _ = black_box(output);
                // Clean up
                let _ = fs::remove_file(&output_path);
            });
        });
    }

    group.finish();
}

/// Benchmark argument parsing performance
fn benchmark_argument_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("argument_parsing");
    group.sample_size(100); // More samples for fast operations

    // Simple commands
    let simple_commands = vec![
        ("short_help", vec!["-h"]),
        ("long_help", vec!["--help"]),
        ("version_short", vec!["-V"]),
        ("version_long", vec!["--version"]),
    ];

    for (name, args) in simple_commands {
        group.bench_with_input(
            BenchmarkId::new("simple", name),
            &args,
            |b, command_args| {
                b.iter(|| {
                    let output = run_cli_command(command_args);
                    black_box(output)
                });
            },
        );
    }

    // Complex commands with many arguments
    let complex_commands = vec![
        (
            "create_full_args",
            vec![
                "create",
                "--pages",
                "10",
                "--sample-text",
                "--version",
                "1.7",
                "--output",
                "/tmp/test.pdf",
                "--verbose",
            ],
        ),
        (
            "info_verbose",
            vec!["info", "/tmp/nonexistent.pdf", "--verbose", "--json"],
        ),
    ];

    for (name, args) in complex_commands {
        group.bench_with_input(
            BenchmarkId::new("complex", name),
            &args,
            |b, command_args| {
                b.iter(|| {
                    let output = run_cli_command(command_args);
                    black_box(output)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark file I/O operations from CLI perspective
fn benchmark_file_operations(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let test_files = setup_test_pdfs(&temp_dir);
    let mut group = c.benchmark_group("file_operations");

    // Test file reading (info command as proxy)
    for (name, path) in &test_files {
        group.bench_with_input(BenchmarkId::new("read_file", name), path, |b, pdf_path| {
            b.iter(|| {
                let path_str = pdf_path.to_str().unwrap();
                let output = run_cli_command(&["info", path_str]);
                black_box(output)
            });
        });
    }

    // Test file writing (create command)
    let sizes = vec![1, 5, 10, 20];
    for size in sizes {
        let output_path = temp_dir.path().join(format!("output_{size}.pdf"));
        let output_str = output_path.to_str().unwrap();

        group.bench_with_input(
            BenchmarkId::new("write_file", size),
            &size,
            |b, &page_count| {
                b.iter(|| {
                    let output = run_cli_command(&[
                        "create",
                        "--pages",
                        &page_count.to_string(),
                        "--output",
                        output_str,
                    ]);
                    let _ = black_box(output);
                    // Clean up
                    let _ = fs::remove_file(&output_path);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark error handling performance
fn benchmark_error_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling");

    let error_scenarios = vec![
        ("missing_file", vec!["info", "/nonexistent/file.pdf"]),
        ("invalid_args", vec!["create", "--invalid-flag"]),
        ("bad_rotation", vec!["rotate", "/tmp/test.pdf", "invalid"]),
        ("missing_output", vec!["create", "--output"]),
    ];

    for (name, args) in error_scenarios {
        group.bench_with_input(BenchmarkId::new("error", name), &args, |b, command_args| {
            b.iter(|| {
                let output = run_cli_command(command_args);
                black_box(output)
            });
        });
    }

    group.finish();
}

criterion_group!(
    cli_benches,
    benchmark_cli_initialization,
    benchmark_info_command,
    benchmark_create_command,
    benchmark_extract_text_command,
    benchmark_rotate_command,
    benchmark_demo_command,
    benchmark_argument_parsing,
    benchmark_file_operations,
    benchmark_error_handling
);
criterion_main!(cli_benches);
