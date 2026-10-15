//! #728: final output must contain a real, complete page graph and exact xref offsets.
#![cfg(feature = "performance")]
use oxidize_pdf::{
    parser::{ParseOptions, PdfReader},
    performance::streaming_writer::{
        ContentStream, PageResources, StreamingOptions, StreamingPageContent, StreamingPdfWriter,
        WriteStrategy,
    },
};
use std::{io::Cursor, process::Command};

fn page(streams: &[&[u8]]) -> StreamingPageContent {
    StreamingPageContent {
        width: 612.,
        height: 792.,
        content_streams: streams
            .iter()
            .map(|s| ContentStream::new(s.to_vec()))
            .collect(),
        resources: PageResources::default(),
    }
}

fn fixtures(counts: &[usize]) -> Vec<StreamingPageContent> {
    counts
        .iter()
        .enumerate()
        .map(|(p, &n)| StreamingPageContent {
            width: 612.,
            height: 792.,
            resources: PageResources::default(),
            content_streams: (0..n)
                .map(|i| {
                    ContentStream::new(if i == 0 {
                        Vec::new()
                    } else {
                        format!("%PAGE{p}STREAM{i}\nq 0 0 10 10 re f Q\n").into_bytes()
                    })
                })
                .collect(),
        })
        .collect()
}

fn write_fixture(
    path: &std::path::Path,
    pages: &[StreamingPageContent],
    compression: bool,
    xref: bool,
    strategy: WriteStrategy,
) {
    let options = StreamingOptions {
        xref_streaming: xref,
        ..StreamingOptions::default()
            .with_compression(compression)
            .with_buffer_size(17)
            .with_write_strategy(strategy)
    };
    let mut writer = StreamingPdfWriter::create(path, options).unwrap();
    for p in pages {
        writer.write_page_streaming(p).unwrap();
        writer.flush().unwrap();
    }
    writer.finalize().unwrap();
    assert_eq!(
        writer.stats().bytes_written,
        std::fs::metadata(path).unwrap().len()
    );
}

#[test]
fn finalized_pages_and_streams_round_trip_without_recovery() {
    for compression in [false, true] {
        for xref in [false, true] {
            for strategy in [WriteStrategy::Buffered, WriteStrategy::Direct] {
                for counts in [vec![], vec![0], vec![1], vec![2], vec![0, 3, 1]] {
                    let dir = tempfile::tempdir().unwrap();
                    let path = dir.path().join("output.pdf");
                    let pages = fixtures(&counts);
                    write_fixture(&path, &pages, compression, xref, strategy);
                    let bytes = std::fs::read(path).unwrap();
                    let doc =
                        PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict())
                            .unwrap()
                            .into_document();
                    assert_eq!(doc.page_count().unwrap() as usize, counts.len());
                    for (index, expected) in pages.iter().enumerate() {
                        let p = doc.get_page(index as u32).unwrap();
                        let streams = doc.get_page_content_streams(&p).unwrap();
                        assert_eq!(
                            streams,
                            expected
                                .content_streams
                                .iter()
                                .map(|s| s.data.clone())
                                .collect::<Vec<_>>(),
                            "page {index}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn byte_statistics_match_final_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("output.pdf");
    let mut writer = StreamingPdfWriter::create(&path, StreamingOptions::default()).unwrap();
    writer.write_page_streaming(&page(&[b"", b"q Q"])).unwrap();
    writer.finalize().unwrap();
    assert_eq!(
        writer.stats().bytes_written,
        std::fs::metadata(path).unwrap().len()
    );
}

#[test]
fn finalized_writer_rejects_additional_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("output.pdf");
    let mut writer = StreamingPdfWriter::create(&path, StreamingOptions::default()).unwrap();
    writer.write_page_streaming(&page(&[])).unwrap();
    writer.finalize().unwrap();
    let original = std::fs::read(&path).unwrap();
    assert!(writer.finalize().is_err());
    assert!(writer.write_page_streaming(&page(&[])).is_err());
    assert_eq!(std::fs::read(path).unwrap(), original);
}

#[test]
fn unsupported_resources_and_strategy_are_explicit_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("output.pdf");
    let mut writer = StreamingPdfWriter::create(&path, StreamingOptions::default()).unwrap();
    let mut invalid = page(&[]);
    invalid.resources.fonts.insert("F1".into(), 1);
    assert!(writer.write_page_streaming(&invalid).is_err());
    assert!(StreamingPdfWriter::create(
        dir.path().join("mmap.pdf"),
        StreamingOptions::default().with_write_strategy(WriteStrategy::MemoryMapped)
    )
    .is_err());
}

#[test]
fn invalid_input_does_not_write_and_valid_retry_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("output.pdf");
    let mut writer = StreamingPdfWriter::create(&path, StreamingOptions::default()).unwrap();
    writer.flush().unwrap();
    let original = std::fs::read(&path).unwrap();
    for width in [f64::NAN, f64::INFINITY, 0., -1.] {
        let mut invalid = page(&[]);
        invalid.width = width;
        assert!(writer.write_page_streaming(&invalid).is_err());
    }
    let mut invalid = page(&[]);
    invalid.resources.images.insert("Image".into(), 999);
    assert!(writer.write_page_streaming(&invalid).is_err());
    writer.flush().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original);
    writer.write_page_streaming(&page(&[])).unwrap();
    writer.finalize().unwrap();
    let mut reader = PdfReader::new_with_options(
        Cursor::new(std::fs::read(path).unwrap()),
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(reader.page_count().unwrap(), 1);
}

#[test]
fn invalid_options_do_not_truncate_existing_destination() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("keep.pdf");
    std::fs::write(&path, b"KEEP").unwrap();
    for options in [
        StreamingOptions::default().with_buffer_size(0),
        StreamingOptions {
            compression_level: 99,
            ..Default::default()
        },
        StreamingOptions {
            progress_callbacks: true,
            ..Default::default()
        },
        StreamingOptions::default().with_write_strategy(WriteStrategy::MemoryMapped),
    ] {
        assert!(StreamingPdfWriter::create(&path, options).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"KEEP");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn io_failure_prevents_later_successful_finalization() {
    let mut writer = StreamingPdfWriter::create("/dev/full", StreamingOptions::default()).unwrap();
    writer.write_page_streaming(&page(&[b"q Q"])).unwrap();
    assert!(writer.flush().is_err());
    assert!(writer.finalize().is_err());
    assert!(writer.write_page_streaming(&page(&[])).is_err());
}

fn qpdf_check(path: &std::path::Path) -> std::process::Output {
    Command::new("qpdf")
        .arg("--check")
        .arg(path)
        .output()
        .expect("qpdf required")
}

#[test]
#[ignore = "requires qpdf; explicitly executed in interoperability CI"]
fn independent_output_oracle() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("output.pdf");
    for compression in [false, true] {
        for xref in [false, true] {
            for counts in [vec![], vec![0], vec![1], vec![2], vec![0, 3, 1]] {
                write_fixture(
                    &path,
                    &fixtures(&counts),
                    compression,
                    xref,
                    WriteStrategy::Buffered,
                );
                let check = qpdf_check(&path);
                assert!(
                    check.status.success(),
                    "{} {}",
                    String::from_utf8_lossy(&check.stdout),
                    String::from_utf8_lossy(&check.stderr)
                );
                let count = Command::new("qpdf")
                    .arg("--show-npages")
                    .arg(&path)
                    .output()
                    .unwrap();
                assert!(count.status.success());
                assert_eq!(
                    String::from_utf8(count.stdout)
                        .unwrap()
                        .trim()
                        .parse::<usize>()
                        .unwrap(),
                    counts.len()
                );
            }
        }
    }
    // An independently generated ordinary Document is a positive control for the validator.
    let mut ordinary = oxidize_pdf::Document::new();
    ordinary.add_page(oxidize_pdf::Page::a4());
    std::fs::write(&path, ordinary.to_bytes().unwrap()).unwrap();
    assert!(qpdf_check(&path).status.success());
    // A missing catalog reference must not be accepted by the validator invocation.
    write_fixture(
        &path,
        &fixtures(&[1]),
        false,
        false,
        WriteStrategy::Buffered,
    );
    let bytes = String::from_utf8(std::fs::read(&path).unwrap()[15..].to_vec()).unwrap();
    let mut damaged = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    damaged.extend(bytes.replace("/Root 1 0 R", "/Root 999999 0 R").as_bytes());
    std::fs::write(&path, damaged).unwrap();
    assert!(
        !qpdf_check(&path).status.success(),
        "qpdf must reject the missing catalog control"
    );
}
