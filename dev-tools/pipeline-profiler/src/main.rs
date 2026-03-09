//! Pipeline profiler for oxidize-pdf
//!
//! Processes PDF files and measures timing for each pipeline stage:
//! 1. t_load — PdfReader::new_with_options()
//! 2. t_page_tree — page_count() (flatten)
//! 3. t_get_page — get_page(0)
//! 4. t_decompress — content stream decompression
//! 5. t_content_parse — ContentParser::parse_content()
//! 6. t_text_extract — extract_text_from_page(0) (full pipeline)
//!
//! With `--verbose`, also captures internal tracing spans from text extraction:
//!   font_resources, stream_decompress, content_parse, text_ops_loop, layout_finalize
//!
//! Usage:
//!   pipeline-profiler --corpus-dir <path> --top 20 --output results.json
//!   pipeline-profiler --file single.pdf --verbose

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use oxidize_pdf::parser::{ContentParser, ParseOptions, PdfDocument, PdfReader};
use serde::Serialize;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Parser)]
#[command(
    name = "pipeline-profiler",
    about = "Profile oxidize-pdf pipeline stages"
)]
struct Cli {
    /// Path to a directory of PDFs (corpus mode)
    #[arg(long)]
    corpus_dir: Option<PathBuf>,

    /// Path to a single PDF file
    #[arg(long)]
    file: Option<PathBuf>,

    /// Number of slowest PDFs to report
    #[arg(long, default_value_t = 20)]
    top: usize,

    /// Output JSON file path
    #[arg(long)]
    output: Option<PathBuf>,

    /// Verbose per-stage output for single file mode (includes tracing sub-stages)
    #[arg(long)]
    verbose: bool,
}

#[derive(Debug, Clone, Serialize)]
struct PdfTiming {
    file: String,
    file_size_bytes: u64,
    t_load_us: u64,
    t_page_tree_us: u64,
    t_get_page_us: u64,
    t_decompress_us: u64,
    t_content_parse_us: u64,
    t_text_extract_us: u64,
    total_us: u64,
    dominant_stage: String,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct AggregateStats {
    count: usize,
    errors: usize,
    mean_us: f64,
    p50_us: u64,
    p95_us: u64,
    p99_us: u64,
    max_us: u64,
}

#[derive(Debug, Serialize)]
struct StageBreakdown {
    stage: String,
    mean_pct: f64,
    mean_us: f64,
}

#[derive(Debug, Serialize)]
struct ProfileReport {
    total_pdfs: usize,
    successful: usize,
    errors: usize,
    aggregate: AggregateStats,
    stage_breakdown: Vec<StageBreakdown>,
    top_slowest: Vec<PdfTiming>,
}

// ---------------------------------------------------------------------------
// Span timing layer — captures enter/exit durations for named tracing spans
// ---------------------------------------------------------------------------

/// Accumulated timing data for a named span, keyed by span name.
type SpanTimings = Arc<Mutex<HashMap<String, u64>>>;

/// Per-span data stored in the registry extensions.
struct SpanTiming {
    entered_at: Option<Instant>,
}

/// Custom tracing layer that accumulates span durations by name.
struct TimingLayer {
    timings: SpanTimings,
}

impl<S> tracing_subscriber::Layer<S> for TimingLayer
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        _attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            let mut extensions = span.extensions_mut();
            extensions.insert(SpanTiming { entered_at: None });
        }
    }

    fn on_enter(&self, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let mut extensions = span.extensions_mut();
            if let Some(timing) = extensions.get_mut::<SpanTiming>() {
                timing.entered_at = Some(Instant::now());
            }
        }
    }

    fn on_exit(&self, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let name = span.name().to_string();
            let mut extensions = span.extensions_mut();
            if let Some(timing) = extensions.get_mut::<SpanTiming>() {
                if let Some(entered_at) = timing.entered_at.take() {
                    let elapsed_us = entered_at.elapsed().as_micros() as u64;
                    let mut map = self.timings.lock().unwrap();
                    *map.entry(name).or_insert(0) += elapsed_us;
                }
            }
        }
    }
}

/// Initialize a tracing subscriber with the timing layer and return the shared timings map.
fn init_timing_subscriber() -> SpanTimings {
    let timings: SpanTimings = Arc::new(Mutex::new(HashMap::new()));
    let layer = TimingLayer {
        timings: timings.clone(),
    };
    tracing_subscriber::registry().with(layer).init();
    timings
}

/// Reset accumulated span timings (call before each profiling run).
fn reset_timings(timings: &SpanTimings) {
    timings.lock().unwrap().clear();
}

/// Print sub-stage breakdown from accumulated span timings.
fn print_substage_breakdown(timings: &SpanTimings, text_extract_us: u64) {
    let map = timings.lock().unwrap();

    // Ordered list of expected sub-stages
    let sub_stages = [
        "font_resources",
        "stream_decompress",
        "content_parse",
        "text_ops_loop",
        "layout_finalize",
    ];

    let mut found_any = false;
    for (i, name) in sub_stages.iter().enumerate() {
        if let Some(&us) = map.get(*name) {
            found_any = true;
            let connector = if i == sub_stages.len() - 1 {
                "\u{2514}\u{2500}"
            } else {
                "\u{251c}\u{2500}"
            };
            let pct = if text_extract_us > 0 {
                (us as f64 / text_extract_us as f64) * 100.0
            } else {
                0.0
            };
            println!(
                "    {} {:<20} {:>8} us  ({:>5.1}%)",
                connector, name, us, pct
            );
        }
    }

    if !found_any {
        println!("    (no sub-stage spans captured)");
    }
}

fn profile_pdf(path: &std::path::Path, verbose: bool, timings: Option<&SpanTimings>) -> PdfTiming {
    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            return PdfTiming {
                file: file_name,
                file_size_bytes: file_size,
                t_load_us: 0,
                t_page_tree_us: 0,
                t_get_page_us: 0,
                t_decompress_us: 0,
                t_content_parse_us: 0,
                t_text_extract_us: 0,
                total_us: 0,
                dominant_stage: String::new(),
                error: Some(format!("Read error: {e}")),
            };
        }
    };

    // Reset span timings before profiling
    if let Some(t) = timings {
        reset_timings(t);
    }

    let total_start = Instant::now();

    // Stage 1: Load (parse xref, trailer, header)
    let t0 = Instant::now();
    let reader = match PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::lenient()) {
        Ok(r) => r,
        Err(e) => {
            return PdfTiming {
                file: file_name,
                file_size_bytes: file_size,
                t_load_us: t0.elapsed().as_micros() as u64,
                t_page_tree_us: 0,
                t_get_page_us: 0,
                t_decompress_us: 0,
                t_content_parse_us: 0,
                t_text_extract_us: 0,
                total_us: total_start.elapsed().as_micros() as u64,
                dominant_stage: "load".to_string(),
                error: Some(format!("Parse error: {e}")),
            };
        }
    };
    let t_load = t0.elapsed();

    let doc = PdfDocument::new(reader);

    // Stage 2: Page tree flatten
    let t1 = Instant::now();
    let page_count = match doc.page_count() {
        Ok(c) => c,
        Err(e) => {
            return PdfTiming {
                file: file_name,
                file_size_bytes: file_size,
                t_load_us: t_load.as_micros() as u64,
                t_page_tree_us: t1.elapsed().as_micros() as u64,
                t_get_page_us: 0,
                t_decompress_us: 0,
                t_content_parse_us: 0,
                t_text_extract_us: 0,
                total_us: total_start.elapsed().as_micros() as u64,
                dominant_stage: "page_tree".to_string(),
                error: Some(format!("Page tree error: {e}")),
            };
        }
    };
    let t_page_tree = t1.elapsed();

    if page_count == 0 {
        return PdfTiming {
            file: file_name,
            file_size_bytes: file_size,
            t_load_us: t_load.as_micros() as u64,
            t_page_tree_us: t_page_tree.as_micros() as u64,
            t_get_page_us: 0,
            t_decompress_us: 0,
            t_content_parse_us: 0,
            t_text_extract_us: 0,
            total_us: total_start.elapsed().as_micros() as u64,
            dominant_stage: String::new(),
            error: Some("Zero pages".to_string()),
        };
    }

    // Stage 3: Get page 0
    let t2 = Instant::now();
    let page = match doc.get_page(0) {
        Ok(p) => p,
        Err(e) => {
            return PdfTiming {
                file: file_name,
                file_size_bytes: file_size,
                t_load_us: t_load.as_micros() as u64,
                t_page_tree_us: t_page_tree.as_micros() as u64,
                t_get_page_us: t2.elapsed().as_micros() as u64,
                t_decompress_us: 0,
                t_content_parse_us: 0,
                t_text_extract_us: 0,
                total_us: total_start.elapsed().as_micros() as u64,
                dominant_stage: "get_page".to_string(),
                error: Some(format!("Get page error: {e}")),
            };
        }
    };
    let t_get_page = t2.elapsed();

    // Stage 4: Decompress content streams
    let t3 = Instant::now();
    let streams = match doc.get_page_content_streams(&page) {
        Ok(s) => s,
        Err(e) => {
            return PdfTiming {
                file: file_name,
                file_size_bytes: file_size,
                t_load_us: t_load.as_micros() as u64,
                t_page_tree_us: t_page_tree.as_micros() as u64,
                t_get_page_us: t_get_page.as_micros() as u64,
                t_decompress_us: t3.elapsed().as_micros() as u64,
                t_content_parse_us: 0,
                t_text_extract_us: 0,
                total_us: total_start.elapsed().as_micros() as u64,
                dominant_stage: "decompress".to_string(),
                error: Some(format!("Decompress error: {e}")),
            };
        }
    };
    let t_decompress = t3.elapsed();

    // Stage 5: Parse content operations
    let content_bytes: Vec<u8> = streams.into_iter().flatten().collect();
    let t4 = Instant::now();
    let _ops = ContentParser::parse_content(&content_bytes);
    let t_content_parse = t4.elapsed();

    // Stage 6: Full text extraction from page 0 (separate measurement, end-to-end)
    // Reset span timings so we only capture sub-stages from text extraction
    if let Some(t) = timings {
        reset_timings(t);
    }
    let t5 = Instant::now();
    let _text = doc.extract_text_from_page(0);
    let t_text_extract = t5.elapsed();

    let total = total_start.elapsed();

    // Determine dominant stage
    let stages = [
        ("load", t_load),
        ("page_tree", t_page_tree),
        ("get_page", t_get_page),
        ("decompress", t_decompress),
        ("content_parse", t_content_parse),
        ("text_extract", t_text_extract),
    ];
    let dominant = stages
        .iter()
        .max_by_key(|(_, d)| d.as_micros())
        .map(|(name, _)| name.to_string())
        .unwrap_or_default();

    let timing = PdfTiming {
        file: file_name,
        file_size_bytes: file_size,
        t_load_us: t_load.as_micros() as u64,
        t_page_tree_us: t_page_tree.as_micros() as u64,
        t_get_page_us: t_get_page.as_micros() as u64,
        t_decompress_us: t_decompress.as_micros() as u64,
        t_content_parse_us: t_content_parse.as_micros() as u64,
        t_text_extract_us: t_text_extract.as_micros() as u64,
        total_us: total.as_micros() as u64,
        dominant_stage: dominant,
        error: None,
    };

    if verbose {
        println!("  load:          {:>8} us", timing.t_load_us);
        println!("  page_tree:     {:>8} us", timing.t_page_tree_us);
        println!("  get_page:      {:>8} us", timing.t_get_page_us);
        println!("  decompress:    {:>8} us", timing.t_decompress_us);
        println!("  content_parse: {:>8} us", timing.t_content_parse_us);
        println!("  text_extract:  {:>8} us", timing.t_text_extract_us);
        if let Some(t) = timings {
            print_substage_breakdown(t, timing.t_text_extract_us);
        }
        println!("  TOTAL:         {:>8} us", timing.total_us);
        println!("  dominant:      {}", timing.dominant_stage);
    }

    timing
}

type StageGetter = (&'static str, fn(&&PdfTiming) -> u64);

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((p / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn compute_report(timings: &[PdfTiming], top_n: usize) -> ProfileReport {
    let successful: Vec<&PdfTiming> = timings.iter().filter(|t| t.error.is_none()).collect();
    let errors = timings.len() - successful.len();

    let mut totals: Vec<u64> = successful.iter().map(|t| t.total_us).collect();
    totals.sort_unstable();

    let sum_total: u64 = totals.iter().sum();
    let mean = if successful.is_empty() {
        0.0
    } else {
        sum_total as f64 / successful.len() as f64
    };

    let aggregate = AggregateStats {
        count: successful.len(),
        errors,
        mean_us: mean,
        p50_us: percentile(&totals, 50.0),
        p95_us: percentile(&totals, 95.0),
        p99_us: percentile(&totals, 99.0),
        max_us: totals.last().copied().unwrap_or(0),
    };

    // Stage breakdown — use fn pointers so all array elements have the same type
    let stage_names: &[StageGetter] = &[
        ("load", |t: &&PdfTiming| t.t_load_us),
        ("page_tree", |t: &&PdfTiming| t.t_page_tree_us),
        ("get_page", |t: &&PdfTiming| t.t_get_page_us),
        ("decompress", |t: &&PdfTiming| t.t_decompress_us),
        ("content_parse", |t: &&PdfTiming| t.t_content_parse_us),
        ("text_extract", |t: &&PdfTiming| t.t_text_extract_us),
    ];

    let stage_breakdown = stage_names
        .iter()
        .map(|(name, getter)| {
            let sum_stage: u64 = successful.iter().map(getter).sum();
            let stage_mean = if successful.is_empty() {
                0.0
            } else {
                sum_stage as f64 / successful.len() as f64
            };
            let pct = if sum_total == 0 {
                0.0
            } else {
                (sum_stage as f64 / sum_total as f64) * 100.0
            };
            StageBreakdown {
                stage: name.to_string(),
                mean_pct: pct,
                mean_us: stage_mean,
            }
        })
        .collect();

    // Top N slowest
    let mut all_timings: Vec<PdfTiming> = timings.to_vec();
    all_timings.sort_by(|a, b| b.total_us.cmp(&a.total_us));
    let top_slowest: Vec<PdfTiming> = all_timings.into_iter().take(top_n).collect();

    ProfileReport {
        total_pdfs: timings.len(),
        successful: successful.len(),
        errors,
        aggregate,
        stage_breakdown,
        top_slowest,
    }
}

fn print_report(report: &ProfileReport) {
    println!("\n=== Pipeline Profile Report ===\n");
    println!(
        "PDFs: {} total, {} successful, {} errors",
        report.total_pdfs, report.successful, report.errors
    );
    println!(
        "Timing: mean={:.0}us  p50={}us  p95={}us  p99={}us  max={}us",
        report.aggregate.mean_us,
        report.aggregate.p50_us,
        report.aggregate.p95_us,
        report.aggregate.p99_us,
        report.aggregate.max_us
    );

    println!("\n--- Stage Breakdown (% of total) ---");
    for stage in &report.stage_breakdown {
        println!(
            "  {:<15} {:>5.1}%  (mean {:.0} us)",
            stage.stage, stage.mean_pct, stage.mean_us
        );
    }

    println!("\n--- Top {} Slowest ---", report.top_slowest.len());
    for (i, t) in report.top_slowest.iter().enumerate() {
        let err = t
            .error
            .as_ref()
            .map(|e| format!(" [ERROR: {e}]"))
            .unwrap_or_default();
        println!(
            "  {:>3}. {:>8} us  {:<14}  {}{}",
            i + 1,
            t.total_us,
            t.dominant_stage,
            t.file,
            err
        );
    }
}

fn main() {
    let cli = Cli::parse();

    if cli.file.is_none() && cli.corpus_dir.is_none() {
        eprintln!("Error: must specify --file or --corpus-dir");
        std::process::exit(1);
    }

    // Initialize tracing subscriber with timing layer when verbose
    let span_timings = if cli.verbose {
        Some(init_timing_subscriber())
    } else {
        None
    };

    if let Some(ref file_path) = cli.file {
        // Single file mode
        println!("Profiling: {}", file_path.display());
        let timing = profile_pdf(file_path, cli.verbose, span_timings.as_ref());
        if let Some(ref err) = timing.error {
            eprintln!("Error: {err}");
        }
        if let Some(ref output_path) = cli.output {
            let json = serde_json::to_string_pretty(&timing).expect("JSON serialization failed");
            std::fs::write(output_path, json).expect("Failed to write output");
            println!("Written to {}", output_path.display());
        }
        return;
    }

    // Corpus mode
    let corpus_dir = cli.corpus_dir.as_ref().unwrap();
    if !corpus_dir.is_dir() {
        eprintln!("Error: {} is not a directory", corpus_dir.display());
        std::process::exit(1);
    }

    // Collect PDF files
    let mut pdf_files: Vec<PathBuf> = std::fs::read_dir(corpus_dir)
        .expect("Failed to read corpus directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("pdf") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    pdf_files.sort();

    println!(
        "Profiling {} PDFs from {}",
        pdf_files.len(),
        corpus_dir.display()
    );

    let pb = ProgressBar::new(pdf_files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .expect("Invalid progress bar template")
            .progress_chars("##-"),
    );

    let timings: Vec<PdfTiming> = pdf_files
        .iter()
        .map(|path| {
            let timing = profile_pdf(path, false, None);
            pb.inc(1);
            timing
        })
        .collect();

    pb.finish_with_message("done");

    let report = compute_report(&timings, cli.top);
    print_report(&report);

    if let Some(ref output_path) = cli.output {
        let json = serde_json::to_string_pretty(&report).expect("JSON serialization failed");
        std::fs::write(output_path, json).expect("Failed to write output");
        println!("\nFull report written to {}", output_path.display());
    }
}
