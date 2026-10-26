use oxidize_pdf::{Document, Font, Page};
use std::{hint::black_box, time::Instant};

#[cfg(feature = "alloc-count")]
mod allocator {
    use std::{alloc::{GlobalAlloc, Layout, System}, sync::atomic::{AtomicU64, Ordering}};
    pub static CALLS: AtomicU64 = AtomicU64::new(0);
    pub static BYTES: AtomicU64 = AtomicU64::new(0);
    pub static FONT_SIZE: AtomicU64 = AtomicU64::new(0);
    pub static FONT_COPIES: AtomicU64 = AtomicU64::new(0);
    fn record(size: usize) {
        CALLS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(size as u64, Ordering::Relaxed);
        if size as u64 == FONT_SIZE.load(Ordering::Relaxed) {
            FONT_COPIES.fetch_add(1, Ordering::Relaxed);
        }
    }
    pub struct Counter;
    // SAFETY: pointers, layouts and sizes are forwarded unchanged to System.
    unsafe impl GlobalAlloc for Counter {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 { record(layout.size()); unsafe { System.alloc(layout) } }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 { record(layout.size()); unsafe { System.alloc_zeroed(layout) } }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 { record(size); unsafe { System.realloc(ptr, layout, size) } }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) { unsafe { System.dealloc(ptr, layout) } }
    }
}
#[cfg(feature = "alloc-count")]
#[global_allocator]
static ALLOCATOR: allocator::Counter = allocator::Counter;

fn build(kind: &str, data: &[u8], pages: usize, compressed: bool) -> oxidize_pdf::Result<Document> {
    let mut doc = Document::new();
    doc.set_compress(compressed);
    let font = if kind == "latin" { Font::Helvetica } else {
        doc.add_font_from_bytes("UnicodeBench", data.to_vec())?;
        Font::Custom("UnicodeBench".into())
    };
    for _ in 0..pages {
        let mut page = Page::a4();
        for row in 0..18 {
            page.text().set_font(font.clone(), 10.0).at(40.0, 790.0 - row as f64 * 25.0).write(line(kind))?;
        }
        doc.add_page(page);
    }
    Ok(doc)
}
fn line(kind: &str) -> &'static str {
    if kind == "cjk" { "中文测试 中文测试 中文测试" }
    else { "Español: acción, mañana; Français: été, cœur; café 25 €." }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let kind = &args[1];
    let pages: usize = args[2].parse()?;
    let iterations: usize = args[3].parse()?;
    let phase = &args[5];
    let compressed = args[6] == "default";
    let path = match kind.as_str() {
        "cjk" => "/tmp/oxidize-issue581/test-pdfs/SourceHanSansSC-Regular.otf",
        "ttf" => "/tmp/oxidize-issue581/oxidize-pdf-core/tests/fixtures/text_contracts/fonts/SourceSans3-Regular.ttf",
        _ => "",
    };
    let data = if path.is_empty() { Vec::new() } else { std::fs::read(path)? };
    if kind != "latin" {
        let check = oxidize_pdf::fonts::Font::from_bytes("UnicodeBench", data.clone())?;
        assert!(check.missing_glyphs(line(kind)).is_empty());
    }
    let mut prepared = if phase == "serialize" { Some(build(kind, &data, pages, compressed)?) } else { None };
    let mut run = || -> oxidize_pdf::Result<Vec<u8>> {
        if let Some(doc) = &mut prepared { doc.to_bytes() }
        else { build(kind, &data, pages, compressed)?.to_bytes() }
    };
    for _ in 0..3 { black_box(run()?); }
    #[cfg(feature = "alloc-count")]
    {
        use std::sync::atomic::Ordering::Relaxed;
        allocator::CALLS.store(0, Relaxed); allocator::BYTES.store(0, Relaxed);
        allocator::FONT_SIZE.store(data.len() as u64, Relaxed); allocator::FONT_COPIES.store(0, Relaxed);
    }
    let start = Instant::now();
    for _ in 0..iterations { black_box(run()?); }
    let elapsed = start.elapsed().as_nanos();
    #[cfg(feature = "alloc-count")]
    let allocations = {
        use std::sync::atomic::Ordering::Relaxed;
        serde_json::json!({"calls":allocator::CALLS.load(Relaxed),"requested_bytes":allocator::BYTES.load(Relaxed),"font_sized_allocations":allocator::FONT_COPIES.load(Relaxed)})
    };
    #[cfg(not(feature = "alloc-count"))]
    let allocations = serde_json::Value::Null;
    let pdf = run()?;
    std::fs::write(&args[4], &pdf)?;
    println!("{}",serde_json::json!({"kind":kind,"pages":pages,"phase":phase,"compression":compressed,"font_bytes":data.len(),"documents":iterations,"elapsed_ns":elapsed,"pdf_bytes":pdf.len(),"allocations":allocations}));
    Ok(())
}
