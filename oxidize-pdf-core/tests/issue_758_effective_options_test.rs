//! Public contracts for effective memory and streaming options (#758).
use oxidize_pdf::memory::{LazyDocument, MemoryOptions};
use oxidize_pdf::objects::ObjectId;
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::io::Cursor;

fn fixture(extra: &[u8]) -> Vec<u8> {
    let objects: [&[u8]; 4] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 321 456] >>",
        extra,
    ];
    encode_objects(&objects)
}

fn encode_objects(objects: &[&[u8]]) -> Vec<u8> {
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        bytes.extend_from_slice(object);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF",
            objects.len() + 1
        )
        .as_bytes(),
    );
    bytes
}

#[test]
fn eager_loading_reports_an_invalid_indexed_object_during_construction() {
    let bytes = fixture(b"not-a-pdf-object");
    let reader =
        || PdfReader::new_with_options(Cursor::new(bytes.clone()), ParseOptions::strict()).unwrap();
    let lazy = LazyDocument::new(reader(), MemoryOptions::default()).unwrap();
    assert_eq!(lazy.page_count(), 1);
    assert_eq!(lazy.get_page(0).unwrap().width(), 321.0);
    assert!(lazy.get_object(&ObjectId::new(4, 0)).is_err());
    assert!(
        LazyDocument::new(reader(), MemoryOptions::default().with_lazy_loading(false)).is_err(),
        "eager loading must inspect indexed objects before returning a document"
    );
}

#[test]
fn cache_statistics_follow_real_object_retention_and_clear() {
    let reader = PdfReader::new(Cursor::new(fixture(b"(independent object)"))).unwrap();
    let lazy = LazyDocument::new(reader, MemoryOptions::default().with_cache_size(1)).unwrap();
    assert_eq!(lazy.memory_stats().cached_objects, 0);
    let object = lazy.get_object(&ObjectId::new(4, 0)).unwrap();
    assert_eq!(lazy.memory_stats().cached_objects, 1);
    assert_eq!(*lazy.get_object(&ObjectId::new(4, 0)).unwrap(), *object);
    assert_eq!(lazy.memory_stats().cache_hits, 1);
    lazy.get_object(&ObjectId::new(3, 0)).unwrap();
    assert_eq!(lazy.memory_stats().cached_objects, 1);
    lazy.clear_cache();
    assert_eq!(lazy.memory_stats().cached_objects, 0);
    assert_eq!(*lazy.get_object(&ObjectId::new(4, 0)).unwrap(), *object);
}

#[derive(Clone)]
struct ObservedReader {
    bytes: Cursor<Vec<u8>>,
    reads: std::sync::Arc<std::sync::Mutex<Vec<usize>>>,
}
impl std::io::Read for ObservedReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reads.lock().unwrap().push(buf.len());
        std::io::Read::read(&mut self.bytes, buf)
    }
}
impl std::io::Seek for ObservedReader {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.bytes, pos)
    }
}

#[test]
fn eager_loading_and_buffer_size_change_actual_io() {
    let payload = format!("({})", "independent payload ".repeat(1000));
    let mut calls = Vec::new();
    for (lazy_loading, buffer_size) in [(true, 1), (true, 65536), (false, 65536)] {
        let reads = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let reader = PdfReader::new(ObservedReader {
            bytes: Cursor::new(fixture(payload.as_bytes())),
            reads: reads.clone(),
        })
        .unwrap();
        let doc = LazyDocument::new(
            reader,
            MemoryOptions {
                lazy_loading,
                buffer_size,
                cache_size: 0,
                ..MemoryOptions::default()
            },
        )
        .unwrap();
        reads.lock().unwrap().clear();
        let value = doc.get_object(&ObjectId::new(4, 0)).unwrap();
        assert!(matches!(&*value, oxidize_pdf::parser::PdfObject::String(_)));
        calls.push(reads.lock().unwrap().len());
        assert_eq!(doc.configuration().effective.buffer_size, buffer_size);
    }
    assert!(
        calls[0] > calls[1],
        "buffering must reduce underlying reads: {calls:?}"
    );
    assert!(calls[1] > 0, "lazy object must perform deferred I/O");
    assert_eq!(calls[2], 0, "eager object must already be parsed");
}

#[test]
fn requested_options_presets_and_unsupported_modes_are_inspectable() {
    use oxidize_pdf::memory::UnsupportedMemoryOption;
    for options in [
        MemoryOptions::default(),
        MemoryOptions::small_file(),
        MemoryOptions::large_file(),
    ] {
        let reader = PdfReader::new(Cursor::new(fixture(b"42"))).unwrap();
        let doc = LazyDocument::new(reader, options.clone()).unwrap();
        let config = doc.configuration();
        assert_eq!(config.requested, options);
        assert_eq!(config.effective.lazy_loading, options.lazy_loading);
        assert_eq!(config.effective.cache_size, options.cache_size);
        assert!(!config.effective.memory_mapping);
        assert!(!config.effective.streaming);
        assert_eq!(config.effective.mmap_threshold, usize::MAX);
        assert_eq!(
            config
                .unsupported
                .contains(&UnsupportedMemoryOption::MemoryMapping),
            options.memory_mapping
        );
        assert_eq!(
            config
                .unsupported
                .contains(&UnsupportedMemoryOption::Streaming),
            options.streaming
        );
        assert_eq!(doc.get_page(0).unwrap().height(), 456.0);
    }
}

#[test]
fn zero_buffer_disabled_cache_and_extreme_inputs_are_explicit() {
    for capacity in [0, 1, usize::MAX] {
        let doc = LazyDocument::new(
            PdfReader::new(Cursor::new(fixture(b"42"))).unwrap(),
            MemoryOptions {
                buffer_size: 0,
                cache_size: capacity,
                ..MemoryOptions::default()
            },
        )
        .unwrap();
        assert_eq!(doc.configuration().requested.buffer_size, 0);
        assert_eq!(doc.configuration().effective.buffer_size, 1);
        for _ in 0..2 {
            doc.get_object(&ObjectId::new(4, 0)).unwrap();
        }
        assert_eq!(doc.memory_stats().cache_hits, usize::from(capacity > 0));
        assert_eq!(doc.memory_stats().cached_objects, usize::from(capacity > 0));
    }
    let result = LazyDocument::new(
        PdfReader::new(Cursor::new(fixture(b"42"))).unwrap(),
        MemoryOptions {
            buffer_size: usize::MAX,
            ..MemoryOptions::default()
        },
    );
    assert!(matches!(
        result,
        Err(oxidize_pdf::PdfError::InvalidOperation(_))
    ));
}

fn streamed_fixture(compressed: bool) -> (Vec<u8>, usize) {
    use std::io::Write;
    // Valid content of known decoded size, much larger than its compressed form.
    let content = b"q Q\n".repeat(4096);
    let encoded = if compressed {
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&content).unwrap();
        encoder.finish().unwrap()
    } else {
        content.clone()
    };
    let mut stream = format!(
        "<< /Length {} {} >>\nstream\n",
        encoded.len(),
        if compressed {
            "/Filter /FlateDecode"
        } else {
            ""
        }
    )
    .into_bytes();
    stream.extend_from_slice(&encoded);
    stream.extend_from_slice(b"\nendstream");
    let bytes = encode_objects(&[
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 321 456] /Contents 5 0 R >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 222 333] /Contents 5 0 R >>",
        &stream,
    ]);
    (bytes, content.len())
}

#[test]
fn streaming_decoded_content_limits_include_compressed_input_and_zero() {
    use oxidize_pdf::streaming::{StreamingDocument, StreamingOptions};
    for compressed in [false, true] {
        let (bytes, size) = streamed_fixture(compressed);
        for limit in [0, size - 1, size] {
            let mut doc = StreamingDocument::new(
                Cursor::new(bytes.clone()),
                StreamingOptions {
                    max_content_stream_size: limit,
                    ..StreamingOptions::default()
                },
            )
            .unwrap();
            let result = doc.next_page();
            if limit < size {
                assert!(
                    result.is_err(),
                    "decoded content exceeds {limit}, compression={compressed}"
                );
                assert!(
                    doc.next_page().is_err(),
                    "error must not silently advance the page"
                );
                assert_eq!(doc.cached_pages(), 0);
            } else {
                assert_eq!(result.unwrap().unwrap().width(), 321.0);
                assert_eq!(doc.next_page().unwrap().unwrap().width(), 222.0);
                assert!(doc.next_page().unwrap().is_none());
            }
        }
    }
    let mut empty = StreamingDocument::new(
        Cursor::new(fixture(b"42")),
        StreamingOptions {
            max_content_stream_size: 0,
            ..StreamingOptions::default()
        },
    )
    .unwrap();
    assert!(empty.next_page().unwrap().is_some());
}

#[test]
fn streaming_cache_budgets_eviction_clear_and_requested_options() {
    use oxidize_pdf::streaming::{StreamingDocument, StreamingOptions};
    let (bytes, size) = streamed_fixture(true);
    for (cache_size, budget, expected_pages) in [
        (0, size, 0),
        (2, 0, 0),
        (2, size - 1, 0),
        (2, size, 1),
        (1, size * 2, 1),
        (2, size * 2, 2),
    ] {
        let options = StreamingOptions {
            buffer_size: 0,
            page_cache_size: cache_size,
            memory_limit: budget,
            max_content_stream_size: size,
            ..StreamingOptions::default()
        };
        let mut doc = StreamingDocument::new(Cursor::new(bytes.clone()), options.clone()).unwrap();
        assert_eq!(doc.configuration().requested, options);
        assert_eq!(doc.configuration().effective.buffer_size, 1);
        assert!(!doc.configuration().effective.progressive_hints);
        assert!(doc.configuration().progressive_hints_unsupported);
        let retained = doc.next_page().unwrap().unwrap();
        doc.next_page().unwrap().unwrap();
        assert_eq!(doc.cached_pages(), expected_pages);
        assert_eq!(doc.memory_usage(), expected_pages * size);
        doc.clear_cache();
        assert_eq!(doc.cached_pages(), 0);
        assert_eq!(doc.memory_usage(), 0);
        assert_eq!(retained.width(), 321.0);
    }
}

#[test]
fn streaming_buffer_changes_underlying_reads_and_presets_remain_usable() {
    use oxidize_pdf::streaming::{StreamingDocument, StreamingOptions};
    let (bytes, _) = streamed_fixture(false);
    let mut observations = Vec::new();
    for buffer_size in [1, 65536] {
        let reads = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut doc = StreamingDocument::new(
            ObservedReader {
                bytes: Cursor::new(bytes.clone()),
                reads: reads.clone(),
            },
            StreamingOptions {
                buffer_size,
                ..StreamingOptions::default()
            },
        )
        .unwrap();
        assert_eq!(doc.next_page().unwrap().unwrap().width(), 321.0);
        observations.push(reads.lock().unwrap().clone());
    }
    assert!(!observations[0].contains(&65536));
    assert!(
        observations[1].contains(&65536),
        "configured buffer must reach underlying Read"
    );
    for options in [
        StreamingOptions::minimal_memory(),
        StreamingOptions::fast_processing(),
    ] {
        let mut doc = StreamingDocument::new(Cursor::new(bytes.clone()), options.clone()).unwrap();
        assert_eq!(doc.configuration().requested, options);
        assert_eq!(
            doc.configuration().effective.memory_limit,
            options.memory_limit
        );
        assert_eq!(
            doc.configuration().progressive_hints_unsupported,
            options.progressive_hints
        );
        assert!(doc.next_page().unwrap().is_some());
    }
    let result = StreamingDocument::new(
        Cursor::new(bytes),
        StreamingOptions {
            buffer_size: usize::MAX,
            ..StreamingOptions::default()
        },
    );
    assert!(matches!(
        result,
        Err(oxidize_pdf::PdfError::InvalidOperation(_))
    ));
}

#[test]
fn opening_file_applies_options_and_mapping_threshold_is_explicitly_inactive() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source.pdf");
    std::fs::write(&path, fixture(b"42")).unwrap();
    for threshold in [0, 1, usize::MAX] {
        let doc = LazyDocument::open(
            &path,
            MemoryOptions {
                buffer_size: 17,
                mmap_threshold: threshold,
                ..MemoryOptions::default()
            },
        )
        .unwrap();
        assert_eq!(doc.configuration().effective.buffer_size, 17);
        assert_eq!(doc.configuration().requested.mmap_threshold, threshold);
        assert!(!doc.configuration().effective.memory_mapping);
        assert_eq!(doc.memory_stats().mapped_regions, 0);
        assert_eq!(doc.get_page(0).unwrap().width(), 321.0);
    }
}

#[test]
fn combined_page_limit_accounts_for_all_streams_and_separator() {
    use oxidize_pdf::streaming::{StreamingDocument, StreamingOptions};
    let bytes = encode_objects(&[
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 321 456] /Contents [4 0 R 5 0 R] >>",
        b"<< /Length 1 >>\nstream\nq\nendstream",
        b"<< /Length 1 >>\nstream\nQ\nendstream",
    ]);
    for limit in [1, 2, 3] {
        let mut doc = StreamingDocument::new(
            Cursor::new(bytes.clone()),
            StreamingOptions {
                max_content_stream_size: limit,
                ..StreamingOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            doc.next_page().is_ok(),
            limit == 3,
            "combined content is q + newline + Q, not a per-stream budget"
        );
    }
}

#[test]
fn adapter_lru_eviction_is_observable_independently_of_parser_cache() {
    let doc = LazyDocument::new(
        PdfReader::new(Cursor::new(fixture(b"42"))).unwrap(),
        MemoryOptions::default().with_cache_size(2),
    )
    .unwrap();
    for number in [4, 3, 4, 2] {
        doc.get_object(&ObjectId::new(number, 0)).unwrap();
    }
    assert_eq!(doc.memory_stats().cache_hits, 1);
    assert_eq!(doc.memory_stats().cache_misses, 3);
    doc.get_object(&ObjectId::new(4, 0)).unwrap(); // Most recently used survives.
    assert_eq!(doc.memory_stats().cache_hits, 2);
    doc.get_object(&ObjectId::new(3, 0)).unwrap(); // Evicted, despite parser retention.
    assert_eq!(doc.memory_stats().cache_misses, 4);
    assert_eq!(doc.memory_stats().cached_objects, 2);
}

#[test]
fn rebuffering_preserves_already_parsed_objects() {
    let reads = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut reader = PdfReader::new(ObservedReader {
        bytes: Cursor::new(fixture(b"(already parsed)")),
        reads: reads.clone(),
    })
    .unwrap();
    let before = reader.get_object(4, 0).unwrap().clone();
    let doc = LazyDocument::new(
        reader,
        MemoryOptions {
            buffer_size: 17,
            cache_size: 0,
            ..MemoryOptions::default()
        },
    )
    .unwrap();
    reads.lock().unwrap().clear();
    assert_eq!(*doc.get_object(&ObjectId::new(4, 0)).unwrap(), before);
    assert!(
        reads.lock().unwrap().is_empty(),
        "existing parser cache must survive rebuffering"
    );
}
