//! Reproducible workload driver for issue #854; run timings outside compilation.
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::TextExtractor;
use oxidize_pdf::writer::{PdfWriter, WriterConfig};
use oxidize_pdf::{Document, Font, Page};
use std::{error::Error, hint::black_box, time::Instant};

fn hash(bytes: &[u8], state: &mut u64) {
    for byte in bytes {
        *state = (*state ^ u64::from(*byte)).wrapping_mul(1099511628211);
    }
}
fn generate(
    pages: usize,
    lines: usize,
    compressed: bool,
    mixed: bool,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut document = Document::new();
    for page_index in 0..pages {
        let mut page = Page::a4();
        for line in 0..lines {
            page.text().set_font(Font::Helvetica,10.0).at(36.0,800.0-line as f64*12.0)
                .write(&format!("Page {page_index:04} line {line:03}: Invoice 12345 - ABCDEFGHIJKLMNOPQRSTUVWXYZ abcdefghijklmnopqrstuvwxyz"))?;
            if mixed {
                page.graphics()
                    .move_to(36.0, 798.0 - line as f64 * 12.0)
                    .line_to(550.0, 798.0 - line as f64 * 12.0)
                    .stroke();
            }
        }
        document.add_page(page);
    }
    let mut bytes = Vec::new();
    PdfWriter::with_config(
        &mut bytes,
        WriterConfig {
            compress_streams: compressed,
            ..WriterConfig::default()
        },
    )
    .write_document(&mut document)?;
    Ok(bytes)
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    let usage = "usage: performance_854 generate PAGES LINES COUNT compressed|raw mixed|text [OUTPUT] | extract PDF COUNT | read-streams PDF";
    match args.get(1).map(String::as_str) {
        Some("generate")
            if (args.len() == 7 || args.len() == 8)
                && matches!(args[5].as_str(), "compressed" | "raw")
                && matches!(args[6].as_str(), "mixed" | "text") => {}
        Some("extract") if args.len() == 4 => {}
        Some("read-streams") if args.len() == 3 => {}
        _ => return Err(usage.into()),
    }
    let start = Instant::now();
    let mut digest = 14695981039346656037u64;
    let mut units = 0usize;
    match args[1].as_str() {
        "generate" => {
            let pages = args[2].parse()?;
            let lines = args[3].parse()?;
            let count: usize = args[4].parse()?;
            for _ in 0..count {
                let bytes = generate(pages, lines, args[5] == "compressed", args[6] == "mixed")?;
                units += bytes.len();
                // Generation equivalence is checked outside timing by comparing
                // decompressed streams; the digest only covers reading workloads.
                black_box(&bytes);
                if args.len() > 7 {
                    std::fs::write(&args[7], bytes)?;
                }
            }
        }
        "extract" => {
            let count: usize = args[3].parse()?;
            for _ in 0..count {
                let document = PdfReader::open_document(&args[2])?;
                let mut extractor = TextExtractor::new();
                for page in 0..document.page_count()? {
                    let text = extractor.extract_from_page(&document, page)?;
                    hash(text.text.as_bytes(), &mut digest);
                    for fragment in &text.fragments {
                        hash(&fragment.x.to_bits().to_le_bytes(), &mut digest);
                        hash(&fragment.y.to_bits().to_le_bytes(), &mut digest);
                        hash(&fragment.width.to_bits().to_le_bytes(), &mut digest);
                    }
                    units += text.text.len();
                    black_box(text);
                }
            }
        }
        "read-streams" => {
            let document = PdfReader::open_document(&args[2])?;
            for index in 0..document.page_count()? {
                let page = document.get_page(index)?;
                for bytes in document.get_page_content_streams(&page)? {
                    units += bytes.len();
                    hash(&bytes, &mut digest);
                    black_box(bytes);
                }
            }
            black_box(document);
        }
        _ => return Err("unknown workload".into()),
    }
    let digest = if args[1] == "generate" {
        "null".to_string()
    } else {
        format!("\"{digest:016x}\"")
    };
    println!(
        "{{\"elapsed_ns\":{},\"units\":{units},\"digest\":{digest}}}",
        start.elapsed().as_nanos()
    );
    Ok(())
}
