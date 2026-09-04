//! Deterministic native-text prediction exporter for OmniDocBench.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::plaintext::{PlainTextConfig, PlainTextExtractor};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct Job {
    prediction_name: String,
    pdf_path: PathBuf,
    page_index: u32,
}

#[derive(Debug, Serialize)]
struct Counts {
    attempted: usize,
    written: usize,
    failed: usize,
}

#[derive(Debug, Serialize)]
struct Failure {
    prediction_name: String,
    error: String,
}

#[derive(Debug, Serialize)]
struct ExtractionConfig {
    api: &'static str,
    preserve_layout: bool,
    line_break_mode: &'static str,
    space_threshold: f64,
    tj_space_threshold: f64,
    newline_threshold: f64,
}

#[derive(Debug, Serialize)]
struct Report {
    counts: Counts,
    failures: Vec<Failure>,
    extraction_config: ExtractionConfig,
}

fn prediction_path(output: &Path, name: &str) -> Result<PathBuf, String> {
    let path = Path::new(name);
    if path.components().count() != 1
        || path.extension().and_then(|part| part.to_str()) != Some("md")
    {
        return Err(format!("invalid prediction filename: {name}"));
    }
    Ok(output.join(path))
}

fn extraction_config() -> ExtractionConfig {
    let config = PlainTextConfig::preserve_layout();
    ExtractionConfig {
        api: "PlainTextExtractor::preserve_layout",
        preserve_layout: config.preserve_layout,
        line_break_mode: "PreserveAll",
        space_threshold: config.space_threshold,
        tj_space_threshold: config.tj_space_threshold,
        newline_threshold: config.newline_threshold,
    }
}

fn temporary_output_path(output: &Path) -> Result<PathBuf, String> {
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("invalid prediction directory: {}", output.display()))?;
    Ok(output.with_file_name(format!(".{name}.tmp-{}", std::process::id())))
}

fn write_predictions(jobs: &[Job], staging: &Path) -> Result<Vec<Failure>, String> {
    let mut jobs_by_pdf: BTreeMap<&Path, Vec<&Job>> = BTreeMap::new();
    for job in jobs {
        jobs_by_pdf.entry(&job.pdf_path).or_default().push(job);
    }
    let mut failures = Vec::new();
    for (pdf_path, pdf_jobs) in jobs_by_pdf {
        let document = PdfReader::open(pdf_path)
            .map(PdfDocument::new)
            .map_err(|error| error.to_string());
        let page_count = document
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|document| document.page_count().map_err(|error| error.to_string()));
        let mut extractor = PlainTextExtractor::with_config(PlainTextConfig::preserve_layout());
        for job in pdf_jobs {
            let destination = prediction_path(staging, &job.prediction_name)?;
            let extracted = match (&document, &page_count) {
                (Ok(document), Ok(count)) if job.page_index < *count => extractor
                    .extract(document, job.page_index)
                    .map(|result| result.text)
                    .map_err(|error| error.to_string()),
                (Ok(_), Ok(count)) => Err(format!(
                    "page index {} outside document with {} pages",
                    job.page_index, count
                )),
                (Err(error), _) | (_, Err(error)) => Err(error.clone()),
            };
            match extracted {
                Ok(text) => {
                    fs::write(destination, text.as_bytes()).map_err(|error| error.to_string())?
                }
                Err(error) => {
                    fs::write(destination, []).map_err(|write_error| write_error.to_string())?;
                    failures.push(Failure {
                        prediction_name: job.prediction_name.clone(),
                        error,
                    });
                }
            }
        }
    }
    Ok(failures)
}

fn run(jobs_path: &Path, output: &Path, report_path: &Path) -> Result<(), String> {
    let jobs: Vec<Job> = serde_json::from_slice(
        &fs::read(jobs_path).map_err(|error| format!("read jobs: {error}"))?,
    )
    .map_err(|error| format!("parse jobs: {error}"))?;
    if jobs.is_empty() {
        return Err("job population is empty".to_string());
    }
    if output.exists() {
        return Err(format!(
            "prediction directory already exists: {}",
            output.display()
        ));
    }
    let staging = temporary_output_path(output)?;
    if staging.exists() {
        return Err(format!(
            "staging directory already exists: {}",
            staging.display()
        ));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::create_dir(&staging).map_err(|error| error.to_string())?;
    let mut failures = match write_predictions(&jobs, &staging) {
        Ok(failures) => failures,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };
    failures.sort_by(|left, right| left.prediction_name.cmp(&right.prediction_name));
    let report = Report {
        counts: Counts {
            attempted: jobs.len(),
            written: jobs.len(),
            failed: failures.len(),
        },
        failures,
        extraction_config: extraction_config(),
    };
    let mut rendered = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
    rendered.push(b'\n');
    fs::write(report_path, rendered).map_err(|error| error.to_string())?;
    fs::rename(staging, output).map_err(|error| error.to_string())
}

fn main() {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 4 {
        eprintln!("usage: omnidocbench_export <jobs.json> <predictions-dir> <report.json>");
        std::process::exit(2);
    }
    if let Err(error) = run(
        Path::new(&args[1]),
        Path::new(&args[2]),
        Path::new(&args[3]),
    ) {
        eprintln!("error: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prediction_names_are_flat_markdown_files() {
        let output = Path::new("predictions");
        assert_eq!(
            prediction_path(output, "source.pdf_7.md").unwrap(),
            output.join("source.pdf_7.md")
        );
        assert!(prediction_path(output, "../escape.md").is_err());
        assert!(prediction_path(output, "page.txt").is_err());
    }

    #[test]
    fn effective_configuration_is_reportable() {
        let config = extraction_config();
        assert!(config.preserve_layout);
        assert_eq!(config.line_break_mode, "PreserveAll");
        assert_eq!(config.space_threshold, 0.3);
        assert_eq!(config.tj_space_threshold, 0.2);
        assert_eq!(config.newline_threshold, 10.0);
    }
}
