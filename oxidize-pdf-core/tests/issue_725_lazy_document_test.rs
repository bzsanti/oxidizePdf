//! Public regression contracts for #725, built from independent PDF syntax.
use oxidize_pdf::memory::lazy_loader::LazyPageIterator;
use oxidize_pdf::memory::{LazyDocument, MemoryOptions};
use oxidize_pdf::objects::ObjectId;
use oxidize_pdf::parser::{PdfDocument, PdfObject, PdfReader};
use oxidize_pdf::PdfError;
use std::io::Cursor;
use std::sync::Arc;

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
    encode_objects(objects)
}

fn encode_objects(objects: Vec<Vec<u8>>) -> Vec<u8> {
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
fn real_page_counts_and_pages() {
    for count in [0, 1, 3] {
        let bytes = fixture(count);
        let ordinary = PdfDocument::new(PdfReader::new(Cursor::new(bytes.clone())).unwrap());
        let lazy = Arc::new(
            LazyDocument::new(
                PdfReader::new(Cursor::new(bytes)).unwrap(),
                MemoryOptions::default(),
            )
            .unwrap(),
        );
        assert_eq!(lazy.page_count(), count as u32);
        for i in 0..count as u32 {
            let page = lazy.get_page(i).unwrap();
            let expected = ordinary.get_page(i).unwrap();
            assert_eq!(page.width(), 300. + i as f64 * 10.);
            assert_eq!(page.height(), 400. + i as f64 * 10.);
            assert_eq!(page.media_box, expected.media_box);
            assert_eq!(page.get_contents(), expected.get_contents());
            assert_eq!(page.get_resources(), expected.get_resources());
            lazy.preload_page(i).unwrap();
        }
        assert!(
            matches!(lazy.get_page(count as u32), Err(PdfError::InvalidPageNumber(n)) if n == count as u32)
        );
        let pages = LazyPageIterator::new(lazy)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(pages.len(), count);
    }
}

#[test]
fn actual_objects_survive_cache_clear_and_eviction() {
    for capacity in [0, 1, 10] {
        let bytes = fixture(3);
        let mut ordinary = PdfReader::new(Cursor::new(bytes.clone())).unwrap();
        let lazy = LazyDocument::new(
            PdfReader::new(Cursor::new(bytes)).unwrap(),
            MemoryOptions::default().with_cache_size(capacity),
        )
        .unwrap();
        for _ in 0..2 {
            for n in 1..=8 {
                let actual = lazy.get_object(&ObjectId::new(n, 0)).unwrap();
                assert_eq!(&*actual, ordinary.get_object(n, 0).unwrap());
                assert_ne!(*actual, PdfObject::Null);
            }
            let stream = lazy.get_object(&ObjectId::new(4, 0)).unwrap();
            assert_eq!(
                stream.as_stream().unwrap().data,
                b"BT /F1 12 Tf 30 40 Td (Source-0) Tj ET"
            );
            lazy.clear_cache();
        }
    }
}

#[test]
fn missing_objects_remain_errors_on_repeated_access() {
    let lazy = LazyDocument::new(
        PdfReader::new(Cursor::new(fixture(1))).unwrap(),
        MemoryOptions::default(),
    )
    .unwrap();
    for _ in 0..2 {
        assert!(matches!(
            lazy.get_object(&ObjectId::new(999, 0)),
            Err(PdfError::InvalidObjectReference(999, 0))
        ));
        assert!(lazy
            .get_object(&ObjectId::new(1, 0))
            .unwrap()
            .as_dict()
            .is_some());
    }
}

#[test]
fn shared_document_returns_real_values_concurrently() {
    let lazy = Arc::new(
        LazyDocument::new(
            PdfReader::new(Cursor::new(fixture(3))).unwrap(),
            MemoryOptions::default(),
        )
        .unwrap(),
    );
    let handles: Vec<_> = (0..3)
        .map(|i| {
            let doc = lazy.clone();
            std::thread::spawn(move || {
                for _ in 0..10 {
                    assert_eq!(doc.get_page(i).unwrap().width(), 300. + i as f64 * 10.);
                    assert!(doc
                        .get_object(&ObjectId::new(1, 0))
                        .unwrap()
                        .as_dict()
                        .is_some());
                    doc.clear_cache();
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn content_loading_is_deferred_until_object_access() {
    use std::io::{Read, Seek, SeekFrom};
    use std::sync::Mutex;
    struct ObservedReader {
        bytes: Cursor<Vec<u8>>,
        seeks: Arc<Mutex<Vec<u64>>>,
    }
    impl Read for ObservedReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.bytes.read(buf)
        }
    }
    impl Seek for ObservedReader {
        fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
            let offset = self.bytes.seek(from)?;
            self.seeks.lock().unwrap().push(offset);
            Ok(offset)
        }
    }
    let bytes = fixture(1);
    let content_offset = bytes.windows(7).position(|w| w == b"4 0 obj").unwrap() as u64;
    let seeks = Arc::new(Mutex::new(Vec::new()));
    let reader = PdfReader::new(ObservedReader {
        bytes: Cursor::new(bytes),
        seeks: seeks.clone(),
    })
    .unwrap();
    seeks.lock().unwrap().clear();
    let lazy = LazyDocument::new(reader, MemoryOptions::default()).unwrap();
    assert_eq!(lazy.page_count(), 1);
    lazy.get_page(0).unwrap();
    assert!(
        !seeks.lock().unwrap().contains(&content_offset),
        "content parsed eagerly"
    );
    assert!(lazy
        .get_object(&ObjectId::new(4, 0))
        .unwrap()
        .as_stream()
        .is_some());
    assert!(
        seeks.lock().unwrap().contains(&content_offset),
        "control: content was never read"
    );
}

#[test]
fn actual_null_is_distinct_from_absent_or_wrong_generation() {
    let bytes = encode_objects(vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [] /Count 0 >>".to_vec(),
        b"null".to_vec(),
    ]);
    let lazy = LazyDocument::new(
        PdfReader::new(Cursor::new(bytes)).unwrap(),
        MemoryOptions::default(),
    )
    .unwrap();
    assert_eq!(
        *lazy.get_object(&ObjectId::new(3, 0)).unwrap(),
        PdfObject::Null
    );
    for (n, g) in [(0, 65535), (1, 7), (4, 0)] {
        assert!(
            matches!(lazy.get_object(&ObjectId::new(n,g)), Err(PdfError::InvalidObjectReference(a,b)) if (a,b)==(n,g))
        );
    }
}

#[test]
fn cache_hit_and_clear_preserve_values() {
    let lazy = LazyDocument::new(
        PdfReader::new(Cursor::new(fixture(1))).unwrap(),
        MemoryOptions::default(),
    )
    .unwrap();
    let first = lazy.get_object(&ObjectId::new(1, 0)).unwrap();
    let second = lazy.get_object(&ObjectId::new(1, 0)).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(lazy.memory_stats().cache_hits, 1);
    lazy.clear_cache();
    let third = lazy.get_object(&ObjectId::new(1, 0)).unwrap();
    assert_eq!(*first, *third);
    assert!(!Arc::ptr_eq(&first, &third));
    assert_eq!(lazy.memory_stats().cache_misses, 2);
}

#[test]
fn invalid_catalog_is_rejected_at_construction() {
    let bytes = encode_objects(vec![b"42".to_vec()]);
    let reader = PdfReader::new(Cursor::new(bytes)).unwrap();
    assert!(LazyDocument::new(reader, MemoryOptions::default()).is_err());
}

#[test]
fn malformed_object_errors_do_not_poison_future_access() {
    let bytes = encode_objects(vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [] /Count 0 >>".to_vec(),
        b"@invalid".to_vec(),
    ]);
    let lazy = LazyDocument::new(
        PdfReader::new(Cursor::new(bytes)).unwrap(),
        MemoryOptions::default(),
    )
    .unwrap();
    for _ in 0..2 {
        assert!(matches!(
            lazy.get_object(&ObjectId::new(3, 0)),
            Err(PdfError::ParseError(_))
        ));
        assert!(lazy
            .get_object(&ObjectId::new(1, 0))
            .unwrap()
            .as_dict()
            .is_some());
    }
}

#[test]
fn preload_reports_missing_font_reference() {
    let bytes = encode_objects(vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 200] /Resources << /Font << /F1 99 0 R >> >> >>".to_vec(),
    ]);
    let lazy = LazyDocument::new(
        PdfReader::new(Cursor::new(bytes)).unwrap(),
        MemoryOptions::default(),
    )
    .unwrap();
    assert!(matches!(
        lazy.preload_page(0),
        Err(PdfError::InvalidObjectReference(99, 0))
    ));
}

#[test]
fn nonzero_generation_resolves_without_aliasing_generation_zero() {
    let bytes = encode_objects(vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [] /Count 0 >>".to_vec(),
        b"12345".to_vec(),
    ]);
    let source = String::from_utf8(bytes)
        .unwrap()
        .replace("3 0 obj", "3 7 obj");
    let offset = source.find("3 7 obj").unwrap();
    let bytes = source
        .replace(
            &format!("{offset:010} 00000 n"),
            &format!("{offset:010} 00007 n"),
        )
        .into_bytes();
    let mut ordinary = PdfReader::new(Cursor::new(bytes.clone())).unwrap();
    let lazy = LazyDocument::new(
        PdfReader::new(Cursor::new(bytes)).unwrap(),
        MemoryOptions::default(),
    )
    .unwrap();
    let obj = lazy.get_object(&ObjectId::new(3, 7)).unwrap();
    assert_eq!(*obj, PdfObject::Integer(12345));
    assert_eq!(&*obj, ordinary.get_object(3, 7).unwrap());
    assert!(matches!(
        lazy.get_object(&ObjectId::new(3, 0)),
        Err(PdfError::InvalidObjectReference(3, 0))
    ));
}

#[test]
fn preserves_public_auto_traits() {
    use std::io::{Read, Seek, SeekFrom};
    struct PinnedReader(Cursor<Vec<u8>>, std::marker::PhantomPinned);
    impl Read for PinnedReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.0.read(buf)
        }
    }
    impl Seek for PinnedReader {
        fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
            self.0.seek(from)
        }
    }
    fn assert_traits<
        T: Send + Sync + Unpin + std::panic::UnwindSafe + std::panic::RefUnwindSafe,
    >() {
    }
    assert_traits::<LazyDocument<Cursor<Vec<u8>>>>();
    assert_traits::<LazyDocument<PinnedReader>>();
}
