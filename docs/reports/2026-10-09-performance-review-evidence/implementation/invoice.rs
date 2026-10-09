use std::{hint::black_box,time::Instant};
use oxidize_pdf::{Document,Page,Font,BuildIdentification};
type Result<T> = std::result::Result<T,Box<dyn std::error::Error>>;
#[cfg(feature="alloc-count")]
mod allocs {
 use std::{alloc::{GlobalAlloc,Layout,System},sync::atomic::{AtomicU64,Ordering}};
 pub static COUNT:AtomicU64=AtomicU64::new(0);pub static BYTES:AtomicU64=AtomicU64::new(0);
 pub struct Counter;
 unsafe impl GlobalAlloc for Counter {
 unsafe fn alloc(&self,l:Layout)->*mut u8{COUNT.fetch_add(1,Ordering::Relaxed);BYTES.fetch_add(l.size() as u64,Ordering::Relaxed);System.alloc(l)}
 unsafe fn alloc_zeroed(&self,l:Layout)->*mut u8{COUNT.fetch_add(1,Ordering::Relaxed);BYTES.fetch_add(l.size() as u64,Ordering::Relaxed);System.alloc_zeroed(l)}
 unsafe fn realloc(&self,p:*mut u8,l:Layout,n:usize)->*mut u8{COUNT.fetch_add(1,Ordering::Relaxed);BYTES.fetch_add(n as u64,Ordering::Relaxed);System.realloc(p,l,n)}
 unsafe fn dealloc(&self,p:*mut u8,l:Layout){System.dealloc(p,l)}
 }
}
#[cfg(feature="alloc-count")]
#[global_allocator]static ALLOC:allocs::Counter=allocs::Counter;
fn lines(page: usize, pages: usize) -> Vec<String> {
    let mut lines = vec![format!("Invoice benchmark | Page {page:04}/{pages:04}")];
    for item in 1..=20 {
        lines.push(format!("Item {item:02} | Qty 2 | Unit 12.50 | Total 25.00"));
    }
    lines.push("TOTAL 500.00 EUR".into());
    lines
}


fn build(pages:usize,mode:&str)->Result<Document>{
 let mut d=Document::new();
 if mode.contains("raw"){d.set_compress(false);}
 if mode.contains("noid"){d.set_build_identification(BuildIdentification::Disabled);}
 for page in 1..=pages{let mut p=Page::a4();for(row,text)in lines(page,pages).iter().enumerate(){p.text().set_font(Font::Helvetica,10.).at(50.,790.-row as f64*20.).write(text)?;}d.add_page(p);}Ok(d)
}
fn pdfwriter(pages:usize)->Result<Vec<u8>>{
            use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str};
            let mut pdf = Pdf::new();
            let catalog = Ref::new(1);
            let tree = Ref::new(2);
            let font = Ref::new(3);
            let ids: Vec<_> = (0..pages).map(|i| Ref::new(4 + i as i32 * 2)).collect();
            pdf.catalog(catalog).pages(tree);
            pdf.pages(tree)
                .kids(ids.iter().copied())
                .count(pages as i32);
            pdf.type1_font(font).base_font(Name(b"Helvetica"));
            for (index, id) in ids.into_iter().enumerate() {
                let content = Ref::new(5 + index as i32 * 2);
                let mut page = pdf.page(id);
                page.media_box(Rect::new(0.0, 0.0, 595.276, 841.89));
                page.parent(tree).contents(content);
                page.resources().fonts().pair(Name(b"F1"), font);
                page.finish();
                let mut ops = Content::new();
                for (row, text) in lines(index + 1, pages).iter().enumerate() {
                    ops.begin_text();
                    ops.set_font(Name(b"F1"), 10.0);
                    ops.next_line(50.0, 790.0 - row as f32 * 20.0);
                    ops.show(Str(text.as_bytes()));
                    ops.end_text();
                }
                pdf.stream(content, &ops.finish());
            }
            Ok(pdf.finish())

}
fn main()->Result<()>{
 let a:Vec<String>=std::env::args().collect();let mode=&a[1];let pages:usize=a[2].parse()?;let n:usize=a[3].parse()?;
 let run=||->Result<Vec<u8>>{if mode=="pdf-writer"{pdfwriter(pages)}else{build(pages,mode)?.to_bytes().map_err(Into::into)}};
 for _ in 0..3{black_box(run()?);}
 #[cfg(feature="alloc-count")] {allocs::COUNT.store(0,std::sync::atomic::Ordering::Relaxed);allocs::BYTES.store(0,std::sync::atomic::Ordering::Relaxed);}
 let start=Instant::now();let mut construction=0u128;let mut serialization=0u128;
 for _ in 0..n{if mode.starts_with("stages") {let t=Instant::now();let mut d=build(pages,mode)?;construction+=t.elapsed().as_nanos();let t=Instant::now();let bytes=d.to_bytes()?;serialization+=t.elapsed().as_nanos();black_box(bytes);}else{black_box(run()?);}}
 let elapsed=start.elapsed().as_nanos();
 #[cfg(feature="alloc-count")] let allocation={let calls=allocs::COUNT.load(std::sync::atomic::Ordering::Relaxed);let requested_bytes=allocs::BYTES.load(std::sync::atomic::Ordering::Relaxed);serde_json::json!({"calls":calls,"requested_bytes":requested_bytes})};
 #[cfg(not(feature="alloc-count"))] let allocation=serde_json::Value::Null;
 let bytes=run()?;std::fs::write(&a[4],&bytes)?;
 println!("{}",serde_json::json!({"mode":mode,"pages":pages,"documents":n,"elapsed_ns":elapsed,"construction_ns":construction,"serialization_ns":serialization,"bytes":bytes.len(),"allocations":allocation}));Ok(())
}
