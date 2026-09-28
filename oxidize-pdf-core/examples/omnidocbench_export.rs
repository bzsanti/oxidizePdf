//! Deterministic native-text prediction exporter for OmniDocBench.

#[path = "support/omnidocbench_serialization.rs"]
mod serialization;
use serialization::Serialization;

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
    serialization: serde_json::Value,
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

fn write_predictions(
    jobs: &[Job],
    staging: &Path,
    serialization: Serialization,
) -> Result<Vec<Failure>, String> {
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
                Ok(text) => fs::write(destination, serialization.serialize(&text).as_bytes())
                    .map_err(|error| error.to_string())?,
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

fn run(
    jobs_path: &Path,
    output: &Path,
    report_path: &Path,
    serialization: Serialization,
) -> Result<(), String> {
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
    let mut failures = match write_predictions(&jobs, &staging, serialization) {
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
        serialization: serialization.configuration(),
    };
    let mut rendered = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
    rendered.push(b'\n');
    fs::write(report_path, rendered).map_err(|error| error.to_string())?;
    fs::rename(staging, output).map_err(|error| error.to_string())
}

fn main() {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 4 && !(args.len() == 6 && args[4] == "--serialization") {
        eprintln!("usage: omnidocbench_export <jobs.json> <predictions-dir> <report.json> [--serialization <contract-id>]");
        std::process::exit(2);
    }
    let serialization = match args.get(5) {
        Some(id) => Serialization::parse(&id.to_string_lossy()),
        None => Ok(Serialization::default()),
    }
    .unwrap_or_else(|error| {
        eprintln!("error: {error}");
        std::process::exit(2)
    });
    if let Err(error) = run(
        Path::new(&args[1]),
        Path::new(&args[2]),
        Path::new(&args[3]),
        serialization,
    ) {
        eprintln!("error: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_export_matches_historical_normalized_bytes() {
        use oxidize_pdf::{Document, Font, Page};
        let root = tempfile::tempdir().unwrap();
        let pdf = root.path().join("two-lines.pdf");
        let mut document = Document::new();
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(72.0, 720.0)
            .write("Alpha")
            .unwrap();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(72.0, 690.0)
            .write("Beta")
            .unwrap();
        document.add_page(page);
        document.save(&pdf).unwrap();
        let jobs = root.path().join("jobs.json");
        fs::write(
            &jobs,
            serde_json::to_vec(&serde_json::json!([
                {"prediction_name":"page.md", "pdf_path":pdf, "page_index":0}
            ]))
            .unwrap(),
        )
        .unwrap();
        let output = root.path().join("predictions");
        run(
            &jobs,
            &output,
            &root.path().join("report.json"),
            Serialization::default(),
        )
        .unwrap();
        assert_eq!(fs::read(output.join("page.md")).unwrap(), b"Alpha Beta");
        let preserved = root.path().join("preserved");
        let report = root.path().join("preserved-report.json");
        run(&jobs, &preserved, &report, Serialization::Preserved).unwrap();
        assert_eq!(fs::read(preserved.join("page.md")).unwrap(), b"Alpha\nBeta");
        let report: serde_json::Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
        assert_eq!(
            report["serialization"],
            Serialization::Preserved.configuration()
        );
    }

    #[test]
    fn empty_pages_and_failed_pages_remain_distinct_in_report() {
        use oxidize_pdf::{Document, Page};
        let root = tempfile::tempdir().unwrap();
        let pdf = root.path().join("empty.pdf");
        let mut document = Document::new();
        document.add_page(Page::a4());
        document.save(&pdf).unwrap();
        let jobs = root.path().join("jobs.json");
        fs::write(
            &jobs,
            serde_json::to_vec(&serde_json::json!([
                {"prediction_name":"empty.md", "pdf_path":pdf, "page_index":0},
                {"prediction_name":"failed.md", "pdf_path":pdf, "page_index":1}
            ]))
            .unwrap(),
        )
        .unwrap();
        let output = root.path().join("predictions");
        let report = root.path().join("report.json");
        run(&jobs, &output, &report, Serialization::default()).unwrap();
        assert_eq!(fs::read(output.join("empty.md")).unwrap(), b"");
        assert_eq!(fs::read(output.join("failed.md")).unwrap(), b"");
        let report: serde_json::Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
        assert_eq!(
            report["counts"],
            serde_json::json!({"attempted":2,"written":2,"failed":1})
        );
        assert_eq!(report["failures"][0]["prediction_name"], "failed.md");
        assert_eq!(
            report["serialization"],
            Serialization::default().configuration()
        );
    }

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
