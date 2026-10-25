//! Native-PDF prediction adapter for OmniDocBench structural evaluation.
//!
//! Uses only the public partition API. It writes HTML table predictions for
//! TEDS plus typed, image-coordinate bounding boxes for the layout evaluator.
//! Formula CDM is intentionally unsupported: PDF glyph extraction is not a
//! LaTeX-recognition API, and guessed TeX would make the metric misleading.

use oxidize_pdf::parser::PdfDocument;
use oxidize_pdf::pipeline::{Element, ElementBBox, TableElementData};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
struct Job {
    prediction_name: String,
    pdf_path: PathBuf,
    page_index: u32,
    image_width: f64,
    image_height: f64,
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn table_html(table: &TableElementData) -> String {
    let mut html = String::from("<table>");
    if let Some(structure) = &table.structure {
        for row in 0..structure.num_rows {
            if row == 0 {
                html.push_str(if structure.header_rows > 0 {
                    "<thead>"
                } else {
                    "<tbody>"
                });
            } else if row == structure.header_rows {
                html.push_str("</thead><tbody>");
            }
            html.push_str("<tr>");
            for cell in structure.cells.iter().filter(|cell| cell.row == row) {
                let tag = if cell.is_header || row < structure.header_rows {
                    "th"
                } else {
                    "td"
                };
                html.push('<');
                html.push_str(tag);
                if cell.row_span > 1 {
                    html.push_str(&format!(" rowspan=\"{}\"", cell.row_span));
                }
                if cell.col_span > 1 {
                    html.push_str(&format!(" colspan=\"{}\"", cell.col_span));
                }
                html.push('>');
                html.push_str(&escape_html(&cell.text));
                html.push_str("</");
                html.push_str(tag);
                html.push('>');
            }
            html.push_str("</tr>");
        }
        if structure.num_rows > 0 {
            if structure.header_rows >= structure.num_rows {
                html.push_str("</thead>");
            } else {
                html.push_str("</tbody>");
            }
        }
    } else {
        html.push_str("<tbody>");
        for row in &table.rows {
            html.push_str("<tr>");
            for cell in row {
                html.push_str("<td>");
                html.push_str(&escape_html(cell));
                html.push_str("</td>");
            }
            html.push_str("</tr>");
        }
        html.push_str("</tbody>");
    }
    html.push_str("</table>");
    html
}

fn element_markdown(element: &Element) -> String {
    match element {
        Element::Table(table) => table_html(table),
        _ => String::new(),
    }
}

fn layout_category_id(element: &Element) -> u8 {
    match element {
        Element::Title(_) => 0,
        Element::Paragraph(_)
        | Element::ListItem(_)
        | Element::CodeBlock(_)
        | Element::KeyValue(_) => 1,
        Element::Header(_) | Element::Footer(_) => 2,
        Element::Image(_) => 3,
        Element::Table(_) => 4,
    }
}

/// Convert a lower-left-origin PDF box into top-left-origin image coordinates.
fn image_bbox(
    bbox: ElementBBox,
    frame: [f64; 4],
    rotation: i32,
    image_width: f64,
    image_height: f64,
) -> [f64; 4] {
    let [left, bottom, right, top] = frame;
    let rotation = rotation.rem_euclid(360);
    let (width, height) = if rotation == 90 || rotation == 270 {
        (top - bottom, right - left)
    } else {
        (right - left, top - bottom)
    };
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for (x, y) in [
        (bbox.x, bbox.y),
        (bbox.right(), bbox.y),
        (bbox.x, bbox.top()),
        (bbox.right(), bbox.top()),
    ] {
        let (x, y) = match rotation {
            90 => (y - bottom, x - left),
            180 => (right - x, y - bottom),
            270 => (top - y, right - x),
            _ => (x - left, top - y),
        };
        let (x, y) = (x * image_width / width, y * image_height / height);
        bounds[0] = bounds[0].min(x);
        bounds[1] = bounds[1].min(y);
        bounds[2] = bounds[2].max(x);
        bounds[3] = bounds[3].max(y);
    }
    bounds
}

fn validate_jobs(jobs: &mut [Job]) -> Result<(), String> {
    if jobs.is_empty() {
        return Err("job population is empty".into());
    }
    jobs.sort_by(|left, right| left.prediction_name.cmp(&right.prediction_name));
    let mut names = BTreeSet::new();
    for job in jobs {
        let candidate = Path::new(&job.prediction_name);
        if candidate.components().count() != 1
            || candidate.extension().and_then(|part| part.to_str()) != Some("md")
        {
            return Err(format!(
                "invalid prediction filename: {}",
                job.prediction_name
            ));
        }
        if !names.insert(&job.prediction_name) {
            return Err(format!(
                "duplicate prediction filename: {}",
                job.prediction_name
            ));
        }
        if !job.image_width.is_finite()
            || !job.image_height.is_finite()
            || job.image_width <= 0.0
            || job.image_height <= 0.0
        {
            return Err(format!("invalid image dimensions: {}", job.prediction_name));
        }
    }
    Ok(())
}

fn file_hash(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            return Ok(format!("{:x}", digest.finalize()));
        }
        digest.update(&buffer[..count]);
    }
}

fn prediction_tree_hash(root: &Path) -> Result<String, String> {
    let mut paths = fs::read_dir(root)
        .map_err(|error| error.to_string())?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    let mut digest = Sha256::new();
    for path in paths {
        if path.extension().and_then(|part| part.to_str()) != Some("md") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("invalid prediction path: {}", path.display()))?;
        digest.update(name.as_bytes());
        digest.update([0]);
        digest.update(file_hash(&path)?.as_bytes());
        digest.update([b'\n']);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn git_sha(source_root: &Path) -> Option<String> {
    Command::new("git")
        .args(["-C", source_root.to_str()?, "rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
}

fn run(jobs_path: &Path, output: &Path) -> Result<(), String> {
    let jobs_bytes = fs::read(jobs_path).map_err(|e| format!("read jobs: {e}"))?;
    let mut jobs: Vec<Job> =
        serde_json::from_slice(&jobs_bytes).map_err(|e| format!("parse jobs: {e}"))?;
    validate_jobs(&mut jobs)?;
    if output.exists() {
        return Err("structural output directory must not already exist".into());
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let staging = tempfile::Builder::new()
        .prefix(".structural-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let staging_predictions = staging.path().join("predictions");
    let staging_layout = staging.path().join("layout.json");
    let staging_report = staging.path().join("report.json");
    fs::create_dir(&staging_predictions).map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    let mut failures = Vec::new();
    let attempted = jobs.len();
    let mut by_pdf: BTreeMap<PathBuf, Vec<Job>> = BTreeMap::new();
    for job in jobs {
        by_pdf.entry(job.pdf_path.clone()).or_default().push(job);
    }
    let export = (|| -> Result<(), String> {
        for (pdf_path, pdf_jobs) in by_pdf {
            let document = PdfDocument::open(&pdf_path).map_err(|e| e.to_string());
            let elements = document
                .as_ref()
                .map_err(Clone::clone)
                .and_then(|document| document.partition().map_err(|e| e.to_string()));
            for job in pdf_jobs {
                let target = staging_predictions.join(&job.prediction_name);
                let extracted = (|| -> Result<(String, Vec<serde_json::Value>), String> {
                    let document = document.as_ref().map_err(Clone::clone)?;
                    let page = document
                        .get_page(job.page_index)
                        .map_err(|e| e.to_string())?;
                    let mut page_elements = elements
                        .as_ref()
                        .map_err(Clone::clone)?
                        .iter()
                        .filter(|element| element.metadata().page == job.page_index)
                        .collect::<Vec<_>>();
                    page_elements.sort_by(|left, right| {
                        oxidize_pdf::pipeline::element_reading_order(left, right)
                    });
                    let markdown = page_elements
                        .iter()
                        .map(|element| element_markdown(element))
                        .filter(|text| !text.trim().is_empty())
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    let boxes = page_elements.iter().map(|element| json!({
                        "image_name": Path::new(&job.prediction_name).file_stem().and_then(|s| s.to_str()).unwrap_or_default(),
                        "bbox": image_bbox(*element.bbox(), page.crop_box.unwrap_or(page.media_box), page.rotation, job.image_width, job.image_height),
                        "category_id": layout_category_id(element), "score": element.metadata().confidence,
                    })).collect();
                    Ok((markdown, boxes))
                })();
                match extracted {
                    Ok((markdown, boxes)) => {
                        fs::write(target, markdown).map_err(|e| e.to_string())?;
                        results.extend(boxes);
                    }
                    Err(error) => {
                        fs::write(target, []).map_err(|e| e.to_string())?;
                        failures
                            .push(json!({"prediction_name": job.prediction_name, "error": error}));
                    }
                }
            }
        }
        let categories = [
            ("0", "title"),
            ("1", "plain text"),
            ("2", "abandon"),
            ("3", "figure"),
            ("4", "table"),
        ]
        .into_iter()
        .map(|(id, name)| (id.to_string(), json!(name)))
        .collect::<serde_json::Map<_, _>>();
        fs::write(
            &staging_layout,
            format!(
                "{}\n",
                serde_json::to_string_pretty(
                    &json!({"categories": categories, "results": results})
                )
                .map_err(|e| e.to_string())?
            ),
        )
        .map_err(|e| e.to_string())?;
        let source_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or("missing workspace root")?;
        let report = json!({
            "schema_version": 3, "attempted": attempted, "failed": failures.len(), "failures": failures,
            "formula_cdm": "not_applicable: public API exposes no LaTeX formula recognition",
            "text_and_reading_order": "not_reportable: structural predictions contain tables only",
            "artifacts": {
                "jobs_sha256": format!("{:x}", Sha256::digest(&jobs_bytes)),
                "predictions_sha256": prediction_tree_hash(&staging_predictions)?,
                "layout_sha256": file_hash(&staging_layout)?,
            },
            "identity": {
                "compiled_adapter_sha256": format!("{:x}", Sha256::digest(include_bytes!("omnidocbench_structural_export.rs"))),
                "compiled_lock_sha256": format!("{:x}", Sha256::digest(include_bytes!("../../Cargo.lock"))),
                "executable_sha256": file_hash(&env::current_exe().map_err(|e| e.to_string())?)?,
                "runtime_checkout_git_sha": git_sha(source_root),
                "crate_version": env!("CARGO_PKG_VERSION"), "partition_api": "PdfDocument::partition",
            },
        });
        fs::write(
            &staging_report,
            serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })();
    export?;
    fs::rename(staging.path(), output).map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 3 {
        eprintln!("usage: omnidocbench_structural_export <jobs.json> <output-directory>");
        std::process::exit(2);
    }
    if let Err(error) = run(Path::new(&args[1]), Path::new(&args[2])) {
        eprintln!("error: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxidize_pdf::pipeline::{ElementMetadata, RichCell, TableStructure};

    #[test]
    fn renders_merged_headers_as_valid_html() {
        let table = TableElementData::from_structure(
            TableStructure {
                cells: vec![RichCell {
                    row: 0,
                    col: 0,
                    row_span: 1,
                    col_span: 2,
                    text: "A&B".into(),
                    is_header: true,
                }],
                num_rows: 1,
                num_cols: 2,
                header_rows: 1,
            },
            ElementMetadata::default(),
        );
        assert_eq!(
            table_html(&table),
            "<table><thead><tr><th colspan=\"2\">A&amp;B</th></tr></thead></table>"
        );
    }

    #[test]
    fn converts_pdf_coordinates_to_image_coordinates() {
        assert_eq!(
            image_bbox(
                ElementBBox::new(10.0, 20.0, 30.0, 40.0),
                [0.0, 0.0, 100.0, 100.0],
                0,
                200.0,
                400.0
            ),
            [20.0, 160.0, 80.0, 320.0]
        );
    }

    #[test]
    fn transforms_offset_crop_in_all_page_rotations() {
        let bbox = ElementBBox::new(20.0, 40.0, 30.0, 20.0);
        let frame = [10.0, 20.0, 110.0, 220.0];
        for (rotation, expected) in [
            (0, [20.0, 80.0, 80.0, 90.0]),
            (90, [20.0, 10.0, 40.0, 40.0]),
            (180, [120.0, 10.0, 180.0, 20.0]),
            (270, [160.0, 60.0, 180.0, 90.0]),
        ] {
            assert_eq!(
                image_bbox(bbox, frame, rotation, 200.0, 100.0),
                expected,
                "rotation {rotation}"
            );
        }
    }

    #[test]
    fn literal_table_syntax_in_text_does_not_create_a_table_prediction() {
        use oxidize_pdf::pipeline::ElementData;
        let paragraph = Element::Paragraph(ElementData {
            text: "<table><tr><td>Literal markup</td></tr></table>".into(),
            metadata: ElementMetadata::default(),
        });
        assert!(element_markdown(&paragraph).is_empty());
    }

    #[test]
    fn renders_table_without_headers_with_balanced_body() {
        let table = TableElementData::from_structure(
            TableStructure {
                cells: vec![RichCell {
                    row: 0,
                    col: 0,
                    row_span: 1,
                    col_span: 1,
                    text: "A".into(),
                    is_header: false,
                }],
                num_rows: 1,
                num_cols: 1,
                header_rows: 0,
            },
            ElementMetadata::default(),
        );
        assert_eq!(
            table_html(&table),
            "<table><tbody><tr><td>A</td></tr></tbody></table>"
        );
    }

    fn job(name: &str) -> Job {
        Job {
            prediction_name: name.into(),
            pdf_path: PathBuf::from("fixture.pdf"),
            page_index: 0,
            image_width: 100.0,
            image_height: 200.0,
        }
    }

    #[test]
    fn rejects_duplicate_or_nested_prediction_names() {
        let mut duplicates = vec![job("page.md"), job("page.md")];
        assert!(validate_jobs(&mut duplicates)
            .unwrap_err()
            .contains("duplicate"));
        let mut nested = vec![job("subdir/page.md")];
        assert!(validate_jobs(&mut nested)
            .unwrap_err()
            .contains("invalid prediction filename"));
    }

    #[test]
    fn rejects_non_positive_image_dimensions() {
        let mut invalid = vec![job("page.md")];
        invalid[0].image_width = 0.0;
        assert!(validate_jobs(&mut invalid)
            .unwrap_err()
            .contains("invalid image dimensions"));
    }

    #[test]
    fn records_pdf_failures_as_empty_predictions() {
        let directory = tempfile::tempdir().unwrap();
        let jobs_path = directory.path().join("jobs.json");
        let output = directory.path().join("output");
        let predictions = output.join("predictions");
        let report = output.join("report.json");
        fs::write(
            &jobs_path,
            serde_json::to_vec(&json!([{
                "prediction_name": "missing.md",
                "pdf_path": directory.path().join("missing.pdf"),
                "page_index": 0,
                "image_width": 100.0,
                "image_height": 200.0,
            }]))
            .unwrap(),
        )
        .unwrap();

        run(&jobs_path, &output).unwrap();

        assert_eq!(fs::read(predictions.join("missing.md")).unwrap(), b"");
        let rendered: serde_json::Value =
            serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
        assert_eq!(rendered["attempted"], 1);
        assert_eq!(rendered["failed"], 1);
        assert!(rendered["artifacts"]["predictions_sha256"].is_string());
    }
}
