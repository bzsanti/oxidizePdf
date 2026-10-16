use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::io::Write;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let doc = PdfDocument::new(PdfReader::open(&args[1]).unwrap());
    let pages = doc.page_count().unwrap();
    eprintln!("pages={pages}");
    let mode = args.get(3).map(String::as_str).unwrap_or("page");
    if mode == "all" {
        let text: String = doc
            .extract_text()
            .unwrap()
            .into_iter()
            .map(|p| p.text)
            .collect();
        std::fs::write(&args[2], &text).unwrap();
        eprintln!("bytes={}", text.len());
    } else if mode == "markdown" {
        let text = doc.to_markdown().unwrap();
        std::fs::write(&args[2], &text).unwrap();
        eprintln!("bytes={}", text.len());
    } else {
        let mut bytes = 0;
        let mut output = std::fs::File::create(&args[2]).unwrap();
        for i in 0..pages {
            let p = doc.extract_text_from_page(i).unwrap();
            bytes += p.text.len();
            output.write_all(p.text.as_bytes()).unwrap();
            if i % 100 == 0 {
                eprintln!("page={i} bytes={bytes}");
            }
        }
        eprintln!("bytes={bytes}");
    }
}
