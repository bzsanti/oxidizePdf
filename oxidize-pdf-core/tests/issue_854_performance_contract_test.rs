//! Behavioral guards for the storage optimizations measured by #854.
#[path = "common/pdf_assembler.rs"]
mod assembler;
use oxidize_pdf::parser::{PdfObject, PdfReader};
use oxidize_pdf::{Document, Font, Page};
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

struct CountedInput {
    bytes: Cursor<Vec<u8>>,
    reads: Arc<AtomicUsize>,
}
impl Read for CountedInput {
    fn read(&mut self, dst: &mut [u8]) -> std::io::Result<usize> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.bytes.read(dst)
    }
}
impl Seek for CountedInput {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.bytes.seek(pos)
    }
}

#[test]
fn object_cache_avoids_io_and_returns_independent_streams() {
    let payload = b"0 0 100 100 re S\n".repeat(10000);
    let pdf = assembler::assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>".to_vec(),
        assembler::stream_obj("", &payload),
    ]);
    let reads = Arc::new(AtomicUsize::new(0));
    let document = PdfReader::new(CountedInput {
        bytes: Cursor::new(pdf),
        reads: reads.clone(),
    })
    .unwrap()
    .into_document();
    let mut first = document.get_object(4, 0).unwrap();
    let count = reads.load(Ordering::Relaxed);
    if let PdfObject::Stream(ref mut stream) = first {
        stream.data.fill(b'x');
    } else {
        panic!("expected stream");
    }
    let second = document.get_object(4, 0).unwrap();
    assert_eq!(
        reads.load(Ordering::Relaxed),
        count,
        "cached lookup must not re-read input"
    );
    assert_eq!(second.as_stream().unwrap().data, payload);
    assert!(
        document.get_object(4, 1).is_err(),
        "generation is part of object identity"
    );
    assert_eq!(document.get_object(4, 0).unwrap(), second);
    drop(document);
    assert_eq!(
        second.as_stream().unwrap().data,
        payload,
        "returned objects own their bytes"
    );
}

#[test]
fn repeated_context_switches_preserve_order_and_repeatable_content() {
    let mut page = Page::a4();
    for index in 0..300 {
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(30.0, 700.0)
            .write(&format!("label-{index:03}"))
            .unwrap();
        if index % 50 == 0 {
            for offset in 0..200 {
                page.graphics()
                    .move_to(offset as f64, 0.0)
                    .line_to(offset as f64, 1.0);
            }
        }
        page.graphics()
            .move_to(index as f64, 10.0)
            .line_to(index as f64, 20.0)
            .stroke();
    }
    let mut document = Document::new();
    document.set_compress(false);
    document.add_page(page);
    let mut previous = None;
    for _ in 0..2 {
        let bytes = document.to_bytes().unwrap();
        let parsed = PdfReader::new(Cursor::new(bytes)).unwrap().into_document();
        let content = parsed
            .get_page_content_streams(&parsed.get_page(0).unwrap())
            .unwrap()
            .concat();
        let text = std::str::from_utf8(&content).unwrap();
        let mut cursor = 0;
        for index in 0..300 {
            let label = format!("(label-{index:03}) Tj");
            let label_pos = cursor + text[cursor..].find(&label).unwrap();
            let stroke = text[label_pos..].find("S\n").unwrap() + label_pos;
            cursor = stroke + 2;
            assert_eq!(text.matches(&label).count(), 1);
        }
        if let Some(prev) = &previous {
            assert_eq!(
                &content, prev,
                "serialization must not consume context operators"
            );
        }
        previous = Some(content);
    }
}
