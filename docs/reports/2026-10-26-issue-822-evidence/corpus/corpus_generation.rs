use oxidize_pdf::{Document, Font, Page};
use oxidize_pdf::parser::PdfReader;
use std::{hint::black_box, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args:Vec<_>=std::env::args().collect();
    let mode=&args[2]; let iterations:usize=args[3].parse()?;
    let start=Instant::now();
    let source=PdfReader::open(&args[1])?.into_document();
    let count=source.page_count()?;
    if count==0 {return Err("zero pages".into());}
    let mut pages=Vec::new();
    for i in 0..count {pages.push(Page::from_parsed_with_content(&source.get_page(i)?, &source)?);}
    let prepare_ns=start.elapsed().as_nanos();
    let font_path=if mode=="cjk" {"/tmp/oxidize-issue581/test-pdfs/SourceHanSansSC-Regular.otf"} else {"/tmp/oxidize-issue581/oxidize-pdf-core/tests/fixtures/text_contracts/fonts/SourceSans3-Regular.ttf"};
    let font_data=if mode=="rewrite" {vec![]} else {std::fs::read(font_path)?};
    let start=Instant::now();
    let mut output=Document::new();
    if mode!="rewrite" {output.add_font_from_bytes("CorpusUnicode822",font_data)?;}
    for mut page in pages {
        if mode!="rewrite" {
            let text=if mode=="cjk" {"中文测试"} else {"Español: acción, mañana; Français: été, cœur; café 25 €."};
            page.text().set_font(Font::Custom("CorpusUnicode822".into()),10.0).at(24.0,24.0).write(text)?;
            page.text().set_font(Font::Helvetica,8.0).at(24.0,12.0).write("Corpus generation benchmark")?;
        }
        output.add_page(page);
    }
    let author_ns=start.elapsed().as_nanos();
    let mut times=Vec::new();let mut pdf=Vec::new();
    for _ in 0..iterations {
        drop(std::mem::take(&mut pdf));
        let start=Instant::now();pdf=black_box(output.to_bytes()?);times.push(start.elapsed().as_nanos());
    }
    if args[4]!="-" {std::fs::write(&args[4],&pdf)?;}
    let rss=std::fs::read_to_string("/proc/self/status").ok().and_then(|s|s.lines().find(|l|l.starts_with("VmHWM:")).map(str::to_owned));
    println!("{}",serde_json::json!({"pages":count,"mode":mode,"prepare_ns":prepare_ns,"author_ns":author_ns,"serialize_ns":times,"output_bytes":pdf.len(),"process_peak_rss":rss}));
    Ok(())
}
