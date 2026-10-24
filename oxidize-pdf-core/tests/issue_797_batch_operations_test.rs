use oxidize_pdf::batch::{BatchJob, BatchOptions, BatchProcessor};
use oxidize_pdf::{Document, Font, Page};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn source(path: &Path) -> Vec<u8> {
    let mut document = Document::new();
    for (index, rotation) in [0, 90, 270].into_iter().enumerate() {
        let mut page = Page::new(300.0, 400.0);
        page.set_rotation(rotation);
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(20.0, 40.0)
            .write(&format!("Batch page {index}"))
            .unwrap();
        document.add_page(page);
    }
    document.save(path).unwrap();
    std::fs::read(path).unwrap()
}

fn rotate(input: &Path, output: &Path, rotation: i32, pages: Option<Vec<usize>>) -> BatchJob {
    BatchJob::Rotate {
        input: input.into(),
        output: output.into(),
        rotation,
        pages,
    }
}

#[test]
fn rotation_reads_back_all_selected_negative_and_duplicate_page_requests() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let original = source(&input);
    for (angle, pages, expected) in [
        (90, None, [90, 180, 0]),
        (-90, Some(vec![0, 2]), [270, 90, 180]),
        (180, Some(vec![1, 1]), [0, 270, 270]),
        (90, Some(vec![]), [0, 90, 270]),
    ] {
        let output = dir.path().join("rotated.pdf");
        let mut batch = BatchProcessor::new(BatchOptions::default().with_parallelism(1));
        batch.add_job(rotate(&input, &output, angle, pages));
        let summary = batch.execute().unwrap();
        assert!(summary.results[0].is_success(), "{:?}", summary.results[0]);
        assert_eq!(
            summary.results[0].output_files().unwrap(),
            std::slice::from_ref(&output)
        );
        let doc = oxidize_pdf::PdfReader::open_document(&output).unwrap();
        assert_eq!(doc.page_count().unwrap(), 3);
        for (i, rotation) in expected.into_iter().enumerate() {
            let page = doc.get_page(i as u32).unwrap();
            assert_eq!(page.rotation, rotation, "page {i}, angle {angle}");
            assert_eq!(page.media_box, [0.0, 0.0, 300.0, 400.0]);
            let displayed = if rotation == 90 || rotation == 270 {
                (400.0, 300.0)
            } else {
                (300.0, 400.0)
            };
            assert_eq!((page.width(), page.height()), displayed);
        }
        let text = doc.extract_text().unwrap();
        for (i, page) in text.iter().enumerate() {
            assert!(
                page.text.contains(&format!("Batch page {i}")),
                "{:?}",
                page.text
            );
        }
        assert_eq!(std::fs::read(&input).unwrap(), original);
    }
}

#[test]
fn invalid_rotation_and_page_do_not_create_or_overwrite_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let original = source(&input);
    for (angle, pages) in [(45, None), (90, Some(vec![0, 3]))] {
        for existing in [false, true] {
            let output = dir.path().join(format!("bad-{angle}-{existing}.pdf"));
            if existing {
                std::fs::write(&output, b"existing output").unwrap();
            }
            let mut batch = BatchProcessor::new(BatchOptions::default());
            batch.add_job(rotate(&input, &output, angle, pages.clone()));
            let summary = batch.execute().unwrap();
            assert!(summary.results[0].is_failed());
            assert!(summary.results[0].output_files().is_none());
            if existing {
                assert_eq!(std::fs::read(output).unwrap(), b"existing output");
            } else {
                assert!(!output.exists());
            }
            assert_eq!(std::fs::read(&input).unwrap(), original);
        }
    }
}

#[test]
fn compression_is_explicitly_unsupported_without_touching_files_at_any_quality() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let original = source(&input);
    for quality in [0, 75, 100, 255] {
        for existing in [false, true] {
            let output = dir
                .path()
                .join(format!("compressed-{quality}-{existing}.pdf"));
            if existing {
                std::fs::write(&output, b"existing output").unwrap();
            }
            let mut batch = BatchProcessor::new(BatchOptions::default());
            batch.add_job(BatchJob::Compress {
                input: input.clone(),
                output: output.clone(),
                quality,
            });
            let summary = batch.execute().unwrap();
            let error = summary.results[0]
                .error()
                .expect("unsupported compression must fail");
            assert!(
                error.contains("Compress") && error.contains("unsupported"),
                "{error}"
            );
            assert!(summary.results[0].output_files().is_none());
            if existing {
                assert_eq!(std::fs::read(output).unwrap(), b"existing output");
            } else {
                assert!(!output.exists());
            }
            assert_eq!(std::fs::read(&input).unwrap(), original);
        }
    }
    // An unsupported operation must not even overwrite its input.
    let mut batch = BatchProcessor::new(BatchOptions::default());
    batch.add_job(BatchJob::Compress {
        input: input.clone(),
        output: input.clone(),
        quality: 75,
    });
    assert!(batch.execute().unwrap().results[0].is_failed());
    assert_eq!(std::fs::read(input).unwrap(), original);
}

#[test]
fn mixed_builtin_outcomes_keep_indices_and_honor_stop_on_error() {
    for stop in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.pdf");
        source(&input);
        let output = dir.path().join("rotated.pdf");
        let mut batch = BatchProcessor::new(
            BatchOptions::default()
                .with_parallelism(1)
                .stop_on_error(stop),
        );
        let jobs = [
            BatchJob::Custom {
                name: "before".into(),
                operation: Box::new(|| Ok(())),
            },
            BatchJob::Compress {
                input: input.clone(),
                output: dir.path().join("unsupported.pdf"),
                quality: 75,
            },
            rotate(&input, &output, 90, None),
        ];
        let names: Vec<_> = jobs.iter().map(BatchJob::display_name).collect();
        for job in jobs {
            batch.add_job(job);
        }
        let summary = batch.execute().unwrap();
        assert_eq!(summary.results.len(), 3);
        for (result, name) in summary.results.iter().zip(names) {
            assert_eq!(result.job_name(), name);
        }
        assert!(summary.results[0].is_success());
        assert!(summary.results[1].is_failed());
        assert_eq!(summary.results[2].is_cancelled(), stop);
        assert_eq!(summary.results[2].is_success(), !stop);
        assert_eq!(output.exists(), !stop);
        assert!(!dir.path().join("unsupported.pdf").exists());
    }
}

#[test]
fn zero_timeout_prevents_rotation_from_writing() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    source(&input);
    let output: PathBuf = dir.path().join("never.pdf");
    let mut batch = BatchProcessor::new(BatchOptions::default().with_job_timeout(Duration::ZERO));
    batch.add_job(rotate(&input, &output, 90, None));
    let summary = batch.execute().unwrap();
    assert!(summary.results[0].error().unwrap().contains("timeout"));
    assert!(!output.exists());
}

#[test]
fn malformed_input_and_unwritable_destination_leave_no_partial_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("invalid.pdf");
    std::fs::write(&input, b"not a PDF").unwrap();
    let output = dir.path().join("existing.pdf");
    std::fs::write(&output, b"keep me").unwrap();
    let mut batch = BatchProcessor::new(BatchOptions::default());
    batch.add_job(rotate(&input, &output, 90, None));
    assert!(batch.execute().unwrap().results[0].is_failed());
    assert_eq!(std::fs::read(&output).unwrap(), b"keep me");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);

    source(&input);
    let directory = dir.path().join("directory.pdf");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(directory.join("sentinel"), b"keep directory").unwrap();
    let mut batch = BatchProcessor::new(BatchOptions::default());
    batch.add_job(rotate(&input, &directory, 90, None));
    assert!(batch.execute().unwrap().results[0].is_failed());
    assert_eq!(
        std::fs::read(directory.join("sentinel")).unwrap(),
        b"keep directory"
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 3);
}

#[test]
fn rotation_can_replace_its_input_after_reading_and_writing_successfully() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in-place.pdf");
    source(&input);
    let mut batch = BatchProcessor::new(BatchOptions::default());
    batch.add_job(rotate(&input, &input, 90, Some(vec![0])));
    assert!(batch.execute().unwrap().results[0].is_success());
    let doc = oxidize_pdf::PdfReader::open_document(&input).unwrap();
    assert_eq!(doc.get_page(0).unwrap().rotation, 90);
    assert_eq!(doc.get_page(1).unwrap().rotation, 90);
    assert_eq!(doc.get_page(2).unwrap().rotation, 270);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn rotation_preserves_ordinary_permissions_and_existing_symlinks() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    source(&input);
    std::fs::set_permissions(&input, std::fs::Permissions::from_mode(0o640)).unwrap();
    for existing in [false, true] {
        let output = dir.path().join(format!("output-{existing}.pdf"));
        if existing {
            std::fs::write(&output, b"old").unwrap();
            std::fs::set_permissions(&output, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        let mut batch = BatchProcessor::new(BatchOptions::default());
        batch.add_job(rotate(&input, &output, 90, None));
        assert!(batch.execute().unwrap().results[0].is_success());
        assert_eq!(
            std::fs::metadata(&output).unwrap().permissions().mode() & 0o7777,
            if existing { 0o644 } else { 0o640 }
        );
    }
    let target = dir.path().join("target.pdf");
    std::fs::write(&target, b"old target").unwrap();
    let link = dir.path().join("link.pdf");
    symlink("target.pdf", &link).unwrap();
    let mut batch = BatchProcessor::new(BatchOptions::default());
    batch.add_job(rotate(&input, &link, 90, None));
    assert!(batch.execute().unwrap().results[0].is_success());
    assert!(std::fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(
        oxidize_pdf::PdfReader::open_document(&target)
            .unwrap()
            .get_page(0)
            .unwrap()
            .rotation,
        90
    );
    let dangling = dir.path().join("dangling.pdf");
    symlink("absent.pdf", &dangling).unwrap();
    let mut batch = BatchProcessor::new(BatchOptions::default());
    batch.add_job(rotate(&input, &dangling, 90, None));
    assert!(batch.execute().unwrap().results[0].is_failed());
    assert!(std::fs::symlink_metadata(&dangling)
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(!dir.path().join("absent.pdf").exists());
}

#[test]
fn positive_timeout_reports_completion_and_leaves_complete_rotated_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    source(&input);
    let mut batch =
        BatchProcessor::new(BatchOptions::default().with_job_timeout(Duration::from_nanos(1)));
    batch.add_job(rotate(&input, &input, 90, None));
    let summary = batch.execute().unwrap();
    assert!(summary.results[0]
        .error()
        .unwrap()
        .contains("after completion"));
    let doc = oxidize_pdf::PdfReader::open_document(&input).unwrap();
    assert_eq!(doc.page_count().unwrap(), 3);
    assert_eq!(doc.get_page(0).unwrap().rotation, 90);
}

#[cfg(unix)]
#[test]
fn rotation_does_not_transfer_special_permission_bits() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    source(&input);
    let output = dir.path().join("output.pdf");
    std::fs::write(&output, b"old").unwrap();
    std::fs::set_permissions(&output, std::fs::Permissions::from_mode(0o1755)).unwrap();
    assert_eq!(
        std::fs::metadata(&output).unwrap().permissions().mode() & 0o7777,
        0o1755
    );
    let mut batch = BatchProcessor::new(BatchOptions::default());
    batch.add_job(rotate(&input, &output, 90, None));
    assert!(batch.execute().unwrap().results[0].is_success());
    assert_eq!(
        std::fs::metadata(&output).unwrap().permissions().mode() & 0o7777,
        0o755
    );
}
