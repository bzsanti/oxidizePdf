//! Independent PDF syntax fixtures: no oxidize writer or placeholder oracle.
use oxidize_pdf::memory::stream_processor::{
    ProcessingAction, StreamProcessor, StreamingOptions as ProcessorOptions,
};
use oxidize_pdf::streaming::page_streamer::PageIterator;
use oxidize_pdf::streaming::{IncrementalParser, ParseEvent, StreamingDocument, StreamingOptions};
use std::io::Cursor;

fn fixture(pages: usize) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        format!(
            "<< /Type /Pages /Count {pages} /Kids [{}] >>",
            (0..pages)
                .map(|i| format!("{} 0 R", 3 + i * 2))
                .collect::<Vec<_>>()
                .join(" ")
        )
        .into_bytes(),
    ];
    for i in 0..pages {
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [10 20 {} {}] /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /Contents {} 0 R >>",310+i*10,420+i*10,4+i*2).into_bytes());
        let content = format!("BT /F1 12 Tf 30 40 Td (Source-{i}) Tj ET");
        objects.push(
            format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            )
            .into_bytes(),
        );
    }
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0];
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
    for offset in &offsets[1..] {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF",
            offsets.len()
        )
        .as_bytes(),
    );
    out
}

#[test]
fn invalid_inputs_never_produce_pages() {
    for bytes in [b"".as_slice(), b"not a PDF", b"%PDF-1.7\n"] {
        let mut callbacks = 0;
        let result = StreamingDocument::new(Cursor::new(bytes), StreamingOptions::default())
            .and_then(|mut d| {
                d.process_pages(|_| {
                    callbacks += 1;
                    Ok(())
                })
            });
        assert!(result.is_err(), "invalid document accepted");
        assert_eq!(callbacks, 0);
        let mut iterator = PageIterator::new(Cursor::new(bytes));
        assert!(iterator.next().unwrap().is_err());
        let mut processor = StreamProcessor::new(Cursor::new(bytes), ProcessorOptions::default());
        assert!(processor
            .process_pages(|_, _| {
                callbacks += 1;
                Ok(ProcessingAction::Continue)
            })
            .is_err());
        assert_eq!(callbacks, 0);
    }
}

#[test]
fn real_pages_and_content_through_all_adapters() {
    for count in [1, 3] {
        let bytes = fixture(count);
        let mut d =
            StreamingDocument::new(Cursor::new(&bytes), StreamingOptions::default()).unwrap();
        for i in 0..count {
            let page = d.next_page().unwrap().unwrap();
            assert_eq!(page.number(), i as u32);
            assert_eq!(
                page.media_box(),
                [10., 20., 310. + i as f64 * 10., 420. + i as f64 * 10.]
            );
            assert_eq!(
                page.extract_text_streaming().unwrap().trim(),
                format!("Source-{i}")
            );
            let mut content = Vec::new();
            page.process_content(|b| {
                content.extend_from_slice(b);
                Ok(())
            })
            .unwrap();
            assert_eq!(
                content,
                format!("BT /F1 12 Tf 30 40 Td (Source-{i}) Tj ET").as_bytes()
            );
        }
        assert!(d.next_page().unwrap().is_none());
        let pages: Vec<_> = PageIterator::new(Cursor::new(&bytes))
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(pages.len(), count);
        for (i, page) in pages.iter().enumerate() {
            assert_eq!(
                page.extract_text_streaming().unwrap().trim(),
                format!("Source-{i}")
            );
        }
        let mut processor = StreamProcessor::new(Cursor::new(&bytes), ProcessorOptions::default());
        let mut texts = vec![];
        processor
            .process_pages(|i, p| {
                assert_eq!(p.number, i);
                assert!(!p.operations.is_empty());
                texts.push(p.text.unwrap().trim().to_owned());
                Ok(ProcessingAction::Continue)
            })
            .unwrap();
        assert_eq!(
            texts,
            (0..count)
                .map(|i| format!("Source-{i}"))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn incremental_objects_are_real_across_partitions_and_line_endings() {
    for eol in ["\n", "\r", "\r\n"] {
        let raw = format!(
            "%PDF-1.7{eol}1 4 obj << /Type /Catalog /Value [42 true (actual)] >> endobj{eol}"
        );
        for size in [1, 2, 7, 4096] {
            let mut parser = IncrementalParser::new();
            let mut found = 0;
            for chunk in raw.as_bytes().chunks(size) {
                parser.feed(chunk).unwrap();
                for e in parser.take_events() {
                    if let ParseEvent::ObjectEnd {
                        id,
                        generation,
                        object,
                    } = e
                    {
                        assert_eq!((id, generation), (1, 4));
                        let dict = object.as_dict().expect("source dictionary, not Null");
                        assert_eq!(dict.get_type(), Some("Catalog"));
                        let a = dict.get("Value").unwrap().as_array().unwrap();
                        assert_eq!(a.get(0).unwrap().as_integer(), Some(42));
                        assert_eq!(a.get(1).unwrap().as_bool(), Some(true));
                        assert_eq!(a.get(2).unwrap().as_string().unwrap().as_bytes(), b"actual");
                        found += 1;
                    }
                }
            }
            assert_eq!(found, 1, "chunk {size}, EOL {eol:?}");
        }
    }
}

#[test]
fn incremental_stream_bytes_are_exact() {
    let payload = b" \0\xc3\xa9\xff\r\nendobj\n%%EOF\n ";
    let mut raw = format!(
        "%PDF-1.7\n8 9 obj << /Length {} >>\nstream\n",
        payload.len()
    )
    .into_bytes();
    raw.extend_from_slice(payload);
    raw.extend_from_slice(b"\nendstream\nendobj\n");
    for size in [1, 2, 7, 4096] {
        let mut parser = IncrementalParser::new();
        let mut bytes = vec![];
        let mut objects = 0;
        for chunk in raw.chunks(size) {
            parser.feed(chunk).unwrap();
            for event in parser.take_events() {
                match event {
                    ParseEvent::StreamData { object_id, data } => {
                        assert_eq!(object_id, 8);
                        bytes.extend(data);
                    }
                    ParseEvent::ObjectEnd {
                        id,
                        generation,
                        object,
                    } => {
                        assert_eq!((id, generation), (8, 9));
                        assert_eq!(object.as_stream().unwrap().raw_data(), payload);
                        objects += 1;
                    }
                    _ => {}
                }
            }
        }
        assert_eq!(bytes, payload);
        assert_eq!(objects, 1);
    }
}

#[test]
fn eof_without_newline_is_recognized() {
    for count in [1, 3] {
        let raw = fixture(count);
        for size in [1, 17, 4096] {
            let mut parser = IncrementalParser::new();
            for bytes in raw.chunks(size) {
                parser.feed(bytes).unwrap();
            }
            assert!(parser.is_complete());
            parser.finish().unwrap();
            let mut page_count = 0;
            let mut trees = 0;
            let mut eof = 0;
            for event in parser.take_events() {
                match event {
                    ParseEvent::EndOfFile => eof += 1,
                    ParseEvent::ObjectEnd { object, .. } => {
                        if let Some(dict) = object.as_dict() {
                            match dict.get_type() {
                                Some("Pages") => {
                                    assert_eq!(
                                        dict.get("Count").unwrap().as_integer(),
                                        Some(count as i64)
                                    );
                                    assert_eq!(
                                        dict.get("Kids").unwrap().as_array().unwrap().len(),
                                        count
                                    );
                                    trees += 1;
                                }
                                Some("Page") => page_count += 1,
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            assert_eq!((page_count, trees, eof), (count, 1, 1));
        }
    }
}

#[test]
fn iterator_errors_once_and_synthetic_geometry_has_no_content() {
    let mut it = PageIterator::new(Cursor::new(b"broken"));
    assert!(it.next().unwrap().is_err());
    assert!(it.next().is_none());
    assert!(it.next().is_none());
    let page = oxidize_pdf::streaming::StreamingPage::new_for_test(3, 100., 200., 0, 0);
    assert!(page.extract_text_streaming().is_err());
    let mut called = false;
    assert!(page
        .process_content(|_| {
            called = true;
            Ok(())
        })
        .is_err());
    assert!(!called);
}

#[test]
fn seeking_content_limits_and_cache_accounting() {
    use oxidize_pdf::streaming::PageStreamer;
    let raw = fixture(3);
    let mut streamer = PageStreamer::new(Cursor::new(&raw));
    streamer.seek_to_page(2).unwrap();
    assert_eq!(streamer.total_pages(), Some(3));
    assert_eq!(
        streamer
            .next()
            .unwrap()
            .unwrap()
            .extract_text_streaming()
            .unwrap()
            .trim(),
        "Source-2"
    );
    assert!(streamer.next().unwrap().is_none());
    streamer.seek_to_page(0).unwrap();
    assert_eq!(streamer.next().unwrap().unwrap().number(), 0);
    assert!(streamer.seek_to_page(4).is_err());
    let options = StreamingOptions {
        max_content_stream_size: 3,
        ..StreamingOptions::default()
    };
    let mut doc = StreamingDocument::new(Cursor::new(&raw), options).unwrap();
    assert!(doc.next_page().is_err());
    let options = StreamingOptions::default()
        .with_memory_limit(100)
        .with_page_cache_size(1);
    let mut doc = StreamingDocument::new(Cursor::new(&raw), options).unwrap();
    let retained = doc.next_page().unwrap().unwrap();
    assert!(doc.memory_usage() > 0 && doc.memory_usage() <= 100);
    doc.next_page().unwrap();
    assert!(doc.memory_usage() <= 100);
    doc.clear_cache();
    assert_eq!(doc.memory_usage(), 0);
    assert_eq!(
        retained.extract_text_streaming().unwrap().trim(),
        "Source-0"
    );
}

#[test]
fn callbacks_stop_skip_and_propagate_errors() {
    use oxidize_pdf::memory::stream_processor::ProcessingEvent;
    use oxidize_pdf::PdfError;
    let raw = fixture(3);
    for stop_at in ["start", "header", "page"] {
        let mut processor = StreamProcessor::new(Cursor::new(&raw), ProcessorOptions::default());
        let mut seen = vec![];
        processor
            .process_with(|event| {
                let name = match event {
                    ProcessingEvent::Start => "start",
                    ProcessingEvent::Header { .. } => "header",
                    ProcessingEvent::Page(_) => "page",
                    ProcessingEvent::End => "end",
                    _ => "other",
                };
                seen.push(name);
                Ok(if name == stop_at {
                    ProcessingAction::Stop
                } else {
                    ProcessingAction::Continue
                })
            })
            .unwrap();
        assert_eq!(seen.last(), Some(&stop_at));
        assert!(!seen.contains(&"end"));
    }
    let mut processor = StreamProcessor::new(Cursor::new(&raw), ProcessorOptions::default());
    let mut ids = vec![];
    processor
        .process_pages(|i, _| {
            ids.push(i);
            Ok(ProcessingAction::Skip)
        })
        .unwrap();
    assert_eq!(ids, vec![0, 1, 2]);
    let mut processor = StreamProcessor::new(Cursor::new(&raw), ProcessorOptions::default());
    let mut calls = 0;
    assert!(matches!(
        processor.process_pages(|_, _| {
            calls += 1;
            Err(PdfError::OperationCancelled)
        }),
        Err(PdfError::OperationCancelled)
    ));
    assert_eq!(calls, 1);
    let page = PageIterator::new(Cursor::new(raw)).next().unwrap().unwrap();
    assert!(matches!(
        page.process_content(|_| Err(PdfError::OperationCancelled)),
        Err(PdfError::OperationCancelled)
    ));
}

#[test]
fn incremental_finish_rejects_invalid_truncated_and_unsupported_input() {
    use oxidize_pdf::streaming::process_incrementally;
    for raw in [
        b"".as_slice(),
        b"garbage",
        b"%PDF-1.7\n",
        b"%PDF-1.7\n1 0 obj << /Type /Catalog >>",
        b"%PDF-1.7\n1 0 obj << /Length 2 0 R >>\nstream\nabc\nendstream\nendobj\n",
        b"%PDF-1.7\n1 0 obj << /Length 9223372036854775807 >>\nstream\nx",
        b"%PDF-1.7\n1 0 obj << /Type /ObjStm /Length 0 >>\nstream\n\nendstream\nendobj\n",
    ] {
        let mut objects = 0;
        assert!(process_incrementally(Cursor::new(raw), |event| {
            if matches!(event, ParseEvent::ObjectEnd { .. }) {
                objects += 1;
            }
            Ok(())
        })
        .is_err());
        assert_eq!(objects, 0);
    }
    let mut raw = fixture(1);
    raw.truncate(raw.len() - 5);
    assert!(process_incrementally(Cursor::new(raw), |_| Ok(())).is_err());
}

#[test]
fn incremental_xref_trailer_and_short_reads_preserve_values() {
    use oxidize_pdf::streaming::process_incrementally;
    use std::io::Read;
    struct Short<'a>(&'a [u8]);
    impl Read for Short<'_> {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            let n = out.len().min(1).min(self.0.len());
            out[..n].copy_from_slice(&self.0[..n]);
            self.0 = &self.0[n..];
            Ok(n)
        }
    }
    let raw = fixture(1);
    let mut xrefs = 0;
    let mut trailers = 0;
    let mut eof = 0;
    process_incrementally(Short(&raw), |event| {
        match event {
            ParseEvent::XRef { entries } => {
                assert_eq!(entries.len(), 5);
                assert!(!entries[0].in_use);
                assert_eq!(entries[0].generation, 65535);
                let entry = &entries[1];
                assert_eq!(entry.object_number, 1);
                assert_eq!(&raw[entry.offset as usize..][..7], b"1 0 obj");
                xrefs += 1;
            }
            ParseEvent::Trailer { dict } => {
                assert_eq!(dict.get("Size").unwrap().as_integer(), Some(5));
                assert_eq!(dict.get("Root").unwrap().as_reference(), Some((1, 0)));
                trailers += 1;
            }
            ParseEvent::EndOfFile => eof += 1,
            _ => {}
        }
        Ok(())
    })
    .unwrap();
    assert_eq!((xrefs, trailers, eof), (1, 1, 1));
}

#[test]
fn incremental_delimiters_in_strings_comments_and_names_are_not_structure() {
    let raw=b"%PDF-1.7\r1 2 obj << /Name /endobj /Text (endobj \\( stream %%EOF) /Hex <656e646f626a> % endobj\r /Array [null 7] >> endobj\r";
    for size in [1, 3, 4096] {
        let mut parser = IncrementalParser::new();
        let mut objects = 0;
        for chunk in raw.chunks(size) {
            parser.feed(chunk).unwrap();
        }
        for event in parser.take_events() {
            if let ParseEvent::ObjectEnd { object, .. } = event {
                let dict = object.as_dict().unwrap();
                assert_eq!(
                    dict.get("Name").unwrap().as_name().unwrap().as_str(),
                    "endobj"
                );
                assert_eq!(
                    dict.get("Hex").unwrap().as_string().unwrap().as_bytes(),
                    b"endobj"
                );
                objects += 1;
            }
        }
        assert_eq!(objects, 1);
    }
}

#[test]
fn incremental_io_and_callback_errors_are_not_success() {
    use oxidize_pdf::{streaming::process_incrementally, PdfError};
    struct Bad;
    impl std::io::Read for Bad {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("source failed"))
        }
    }
    assert!(matches!(
        process_incrementally(Bad, |_| Ok(())),
        Err(PdfError::Io(_))
    ));
    let mut calls = 0;
    assert!(matches!(
        process_incrementally(Cursor::new(fixture(1)), |_| {
            calls += 1;
            Err(PdfError::OperationCancelled)
        }),
        Err(PdfError::OperationCancelled)
    ));
    assert_eq!(calls, 1);
}

#[test]
fn nested_and_oversized_objects_fail_before_parsing_or_allocating_payloads() {
    // A nested stream is invalid PDF; do not pass its untrusted Length to the recursive parser.
    let raw=b"%PDF-1.7\n1 0 obj << /Nested << /Length 9223372036854775807 >> stream\nx\nendstream >> endobj\n";
    for size in [1, 4096] {
        let mut parser = IncrementalParser::new();
        let error = raw
            .chunks(size)
            .try_for_each(|chunk| parser.feed(chunk))
            .unwrap_err();
        assert!(error.to_string().contains("top-level"));
        assert!(!parser
            .take_events()
            .iter()
            .any(|e| matches!(e, ParseEvent::ObjectEnd { .. })));
        assert!(parser.finish().is_err());
        assert!(parser.feed(b" ").is_err());
    }
    let raw = format!("%PDF-1.7\n1 0 obj {}", "[".repeat(129));
    let mut parser = IncrementalParser::new();
    assert!(parser
        .feed(raw.as_bytes())
        .unwrap_err()
        .to_string()
        .contains("nesting"));
}

#[test]
fn real_null_objects_are_preserved_instead_of_rejected() {
    for size in [1, 4096] {
        let mut parser = IncrementalParser::new();
        for chunk in b"%PDF-1.4\n7 2 obj null endobj\n".chunks(size) {
            parser.feed(chunk).unwrap();
        }
        let objects: Vec<_> = parser
            .take_events()
            .into_iter()
            .filter_map(|e| match e {
                ParseEvent::ObjectEnd {
                    id,
                    generation,
                    object,
                } => Some((id, generation, object)),
                _ => None,
            })
            .collect();
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].0, 7);
        assert_eq!(objects[0].1, 2);
        assert!(objects[0].2.is_null());
    }
}

#[test]
fn comments_before_endobj_do_not_change_objects() {
    for body in [
        "(source) % comment\n",
        "42 % comment\r",
        "/Source % comment\r\n",
    ] {
        let raw = format!("%PDF-1.7\n1 0 obj {body}endobj\n");
        for size in [1, 4096] {
            let mut parser = IncrementalParser::new();
            for chunk in raw.as_bytes().chunks(size) {
                parser.feed(chunk).unwrap();
            }
            let objects: Vec<_> = parser
                .take_events()
                .into_iter()
                .filter_map(|e| match e {
                    ParseEvent::ObjectEnd { object, .. } => Some(object),
                    _ => None,
                })
                .collect();
            assert_eq!(objects.len(), 1);
            assert!(!objects[0].is_null());
        }
    }
}

#[test]
fn processor_resume_keeps_source_page_numbers() {
    let raw = fixture(3);
    let mut processor = StreamProcessor::new(Cursor::new(raw), ProcessorOptions::default());
    let mut ids = vec![];
    processor
        .process_pages(|i, page| {
            assert_eq!(i, page.number);
            ids.push(i);
            Ok(ProcessingAction::Stop)
        })
        .unwrap();
    processor
        .process_pages(|i, page| {
            assert_eq!(i, page.number);
            ids.push(i);
            Ok(ProcessingAction::Continue)
        })
        .unwrap();
    assert_eq!(ids, [0, 1, 2]);
}

#[test]
fn blank_source_pages_have_empty_text_and_content() {
    let mut source = oxidize_pdf::Document::new();
    source.add_page(oxidize_pdf::Page::new(300., 400.));
    let raw = source.to_bytes().unwrap();
    let pages: Vec<_> = PageIterator::new(Cursor::new(&raw))
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].extract_text_streaming().unwrap(), "");
    let mut content = vec![];
    pages[0]
        .process_content(|b| {
            content.extend_from_slice(b);
            Ok(())
        })
        .unwrap();
    assert!(content.iter().all(u8::is_ascii_whitespace));
    let mut processor = StreamProcessor::new(Cursor::new(raw), ProcessorOptions::default());
    let mut count = 0;
    processor
        .process_pages(|i, p| {
            assert_eq!(i, 0);
            assert_eq!(p.text.as_deref(), Some(""));
            assert!(p.operations.is_empty());
            count += 1;
            Ok(ProcessingAction::Continue)
        })
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn page_adapters_preserve_send_and_sync_for_owned_readers() {
    fn require_send_sync<T: Send + Sync + std::panic::RefUnwindSafe + std::panic::UnwindSafe>() {}
    require_send_sync::<StreamingDocument<Cursor<Vec<u8>>>>();
    require_send_sync::<StreamProcessor<Cursor<Vec<u8>>>>();
    require_send_sync::<oxidize_pdf::streaming::PageStreamer<Cursor<Vec<u8>>>>();
    require_send_sync::<PageIterator<Cursor<Vec<u8>>>>();
    require_send_sync::<oxidize_pdf::streaming::StreamingPage>();
}

#[test]
fn page_adapters_preserve_sync_without_requiring_send() {
    // MutexGuard is Sync but not Send; this reader intentionally preserves that distinction.
    struct SyncReader(std::marker::PhantomData<std::sync::MutexGuard<'static, ()>>);
    impl std::io::Read for SyncReader {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Ok(0)
        }
    }
    impl std::io::Seek for SyncReader {
        fn seek(&mut self, _: std::io::SeekFrom) -> std::io::Result<u64> {
            Ok(0)
        }
    }
    fn require_sync<T: Sync>() {}
    require_sync::<StreamingDocument<SyncReader>>();
    require_sync::<StreamProcessor<SyncReader>>();
    require_sync::<oxidize_pdf::streaming::PageStreamer<SyncReader>>();
    require_sync::<PageIterator<SyncReader>>();
}
