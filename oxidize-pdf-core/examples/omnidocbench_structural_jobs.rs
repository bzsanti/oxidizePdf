//! Build the deterministic job manifest consumed by
//! `omnidocbench_structural_export` from a pinned OmniDocBench dataset.
//!
//! The dataset supplies image dimensions and an image name such as
//! `paper.pdf_7.jpg`; its corresponding original PDF is
//! `ori_pdfs/paper.pdf_7.pdf` and contains the annotated page as page zero.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct DatasetPage {
    page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    image_path: String,
    width: f64,
    height: f64,
}

#[derive(Debug, Serialize, PartialEq)]
struct Job {
    prediction_name: String,
    pdf_path: PathBuf,
    page_index: u32,
    image_width: f64,
    image_height: f64,
}

fn image_stem(image_path: &str) -> Result<&str, String> {
    let filename = Path::new(image_path)
        .file_name()
        .and_then(|part| part.to_str())
        .ok_or_else(|| format!("invalid image path: {image_path:?}"))?;
    filename
        .strip_suffix(".jpg")
        .or_else(|| filename.strip_suffix(".png"))
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| format!("unsupported image extension: {image_path:?}"))
}

fn jobs_from_pages(dataset: &Path, pages: Vec<DatasetPage>) -> Result<Vec<Job>, String> {
    if pages.is_empty() {
        return Err("dataset page population is empty".into());
    }
    let mut names = BTreeSet::new();
    let mut jobs = Vec::with_capacity(pages.len());
    for page in pages {
        let stem = image_stem(&page.page_info.image_path)?;
        let prediction_name = format!("{stem}.md");
        if !names.insert(prediction_name.clone()) {
            return Err(format!("duplicate prediction filename: {prediction_name}"));
        }
        if !page.page_info.width.is_finite()
            || !page.page_info.height.is_finite()
            || page.page_info.width <= 0.0
            || page.page_info.height <= 0.0
        {
            return Err(format!("invalid image dimensions: {prediction_name}"));
        }
        let pdf_path = dataset.join("ori_pdfs").join(format!("{stem}.pdf"));
        if !pdf_path.is_file() {
            return Err(format!("missing original PDF: {}", pdf_path.display()));
        }
        jobs.push(Job {
            prediction_name,
            pdf_path,
            page_index: 0,
            image_width: page.page_info.width,
            image_height: page.page_info.height,
        });
    }
    jobs.sort_by(|left, right| left.prediction_name.cmp(&right.prediction_name));
    Ok(jobs)
}

fn write_manifest(output: &Path, jobs: &[Job]) -> Result<(), String> {
    if output.exists() {
        return Err(format!("output path already exists: {}", output.display()));
    }
    let parent = output
        .parent()
        .ok_or_else(|| format!("output has no parent: {}", output.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let filename = output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("invalid output path: {}", output.display()))?;
    let staging = parent.join(format!(".{filename}.tmp-{}", std::process::id()));
    if staging.exists() {
        return Err(format!(
            "staging output path already exists: {}",
            staging.display()
        ));
    }
    let rendered = serde_json::to_vec_pretty(jobs).map_err(|error| error.to_string())?;
    if let Err(error) = fs::write(&staging, rendered) {
        return Err(error.to_string());
    }
    fs::rename(&staging, output).map_err(|error| error.to_string())
}

fn run(dataset: &Path, output: &Path) -> Result<(), String> {
    let dataset = fs::canonicalize(dataset).map_err(|error| error.to_string())?;
    let pages: Vec<DatasetPage> = serde_json::from_slice(
        &fs::read(dataset.join("OmniDocBench.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let jobs = jobs_from_pages(&dataset, pages)?;
    write_manifest(output, &jobs)
}

fn main() {
    let arguments: Vec<_> = env::args_os().collect();
    if arguments.len() != 3 {
        eprintln!("usage: omnidocbench_structural_jobs <dataset-dir> <jobs.json>");
        std::process::exit(2);
    }
    if let Err(error) = run(Path::new(&arguments[1]), Path::new(&arguments[2])) {
        eprintln!("error: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(name: &str, width: f64, height: f64) -> DatasetPage {
        DatasetPage {
            page_info: PageInfo {
                image_path: name.into(),
                width,
                height,
            },
        }
    }

    #[test]
    fn accepts_supported_image_extensions() {
        assert_eq!(image_stem("nested/paper.pdf_7.jpg").unwrap(), "paper.pdf_7");
        assert_eq!(image_stem("paper.png").unwrap(), "paper");
        assert!(image_stem("paper.webp").is_err());
    }

    #[test]
    fn rejects_duplicate_prediction_names_before_reading_pdfs() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("ori_pdfs")).unwrap();
        fs::write(directory.path().join("ori_pdfs/paper.pdf"), []).unwrap();
        let error = jobs_from_pages(
            directory.path(),
            vec![page("paper.jpg", 1.0, 1.0), page("paper.png", 1.0, 1.0)],
        )
        .unwrap_err();
        assert!(error.contains("duplicate prediction filename"));
    }
}
