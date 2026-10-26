use oxidize_pdf::{Document, Font, Page};
use std::{hint::black_box, time::Instant};

#[cfg(feature = "alloc-count")]
mod allocator {
    use std::{alloc::{GlobalAlloc, Layout, System}, sync::atomic::{AtomicU64, Ordering}};
    pub static CALLS: AtomicU64 = AtomicU64::new(0);
    pub static BYTES: AtomicU64 = AtomicU64::new(0);
    pub struct Counter;
    // SAFETY: pointers, layouts and sizes are forwarded unchanged to System.
    unsafe impl GlobalAlloc for Counter {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            CALLS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
            unsafe { System.alloc(layout) }
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            CALLS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
            unsafe { System.alloc_zeroed(layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
            CALLS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(size as u64, Ordering::Relaxed);
            unsafe { System.realloc(ptr, layout, size) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) }
        }
    }
}
#[cfg(feature = "alloc-count")]
#[global_allocator]
static ALLOCATOR: allocator::Counter = allocator::Counter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let kind = &args[1];
    let count: usize = args[2].parse()?;
    let iterations: usize = args[3].parse()?;
    let text: String = match kind.as_str() {
        "unique" => (0..count).map(|i| char::from_u32(0x4e00 + i as u32).unwrap()).collect(),
        "latin" => "é".repeat(count),
        _ => "A".repeat(count),
    };
    let run = || -> Result<Vec<u8>, oxidize_pdf::PdfError> {
        let mut doc = Document::new();
        doc.set_compress(false);
        let mut page = Page::a4();
        page.text().set_font(Font::Helvetica, 0.04).at(20.0, 790.0).write(&text)?;
        doc.add_page(page);
        doc.to_bytes()
    };
    for _ in 0..3 { black_box(run()?); }
    #[cfg(feature = "alloc-count")]
    {
        allocator::CALLS.store(0, std::sync::atomic::Ordering::Relaxed);
        allocator::BYTES.store(0, std::sync::atomic::Ordering::Relaxed);
    }
    let start = Instant::now();
    for _ in 0..iterations { black_box(run()?); }
    let elapsed = start.elapsed().as_nanos();
    #[cfg(feature = "alloc-count")]
    let allocations = serde_json::json!({"calls":allocator::CALLS.load(std::sync::atomic::Ordering::Relaxed),"requested_bytes":allocator::BYTES.load(std::sync::atomic::Ordering::Relaxed)});
    #[cfg(not(feature = "alloc-count"))]
    let allocations = serde_json::Value::Null;
    let pdf = run()?;
    std::fs::write(&args[4], &pdf)?;
    println!("{}",serde_json::json!({"kind":kind,"characters":count,"documents":iterations,"elapsed_ns":elapsed,"pdf_bytes":pdf.len(),"allocations":allocations}));
    Ok(())
}
