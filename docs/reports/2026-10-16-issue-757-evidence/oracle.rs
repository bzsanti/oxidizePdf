use oxidize_pdf::parser::{PdfDocument, PdfReader};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut doc = PdfDocument::new(PdfReader::open(&args[1]).unwrap());
    let pages = doc.page_count().unwrap();
    eprintln!("pages={pages}");
    let mut bytes = 0;
    let mut output = std::fs::File::create(&args[2]).unwrap();
    use std::io::Write;
    for i in 0..pages {
        if i != 0 && i % 20 == 0 { doc = PdfDocument::new(PdfReader::open(&args[1]).unwrap()); }
        let p = doc.extract_text_from_page(i).unwrap();
        bytes += p.text.len();
        output.write_all(p.text.as_bytes()).unwrap();
        if i % 100 == 0 {
            eprintln!("page={i} bytes={bytes}");
        }
    }

    eprintln!("bytes={bytes}");
}
