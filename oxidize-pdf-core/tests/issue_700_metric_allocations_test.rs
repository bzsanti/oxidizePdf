//! Width queries must not allocate a copy of an entire font table.
use oxidize_pdf::{
    text::metrics::{measure_text_with, FontMetrics, FontMetricsStore},
    Font,
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
};
struct CountingAllocator;
thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
}
fn record() {
    if COUNTING.try_with(Cell::get).unwrap_or(false) {
        let _ = CALLS.try_with(|c| c.set(c.get() + 1));
    }
}
// SAFETY: all allocations and deallocations are delegated to System unchanged;
// thread-local counters neither allocate nor touch the allocated memory.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record();
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
fn check_queries(font: &Font, store: Option<&FontMetricsStore>, expected: f64) {
    assert!((measure_text_with("AB", font, 10., store) - expected).abs() < 1e-10);
    CALLS.with(|c| c.set(0));
    COUNTING.with(|c| c.set(true));
    for _ in 0..100 {
        black_box(measure_text_with(black_box("AB"), font, 10., store));
    }
    COUNTING.with(|c| c.set(false));
    assert_eq!(
        CALLS.with(Cell::get),
        0,
        "width queries must borrow/share metrics, not clone maps"
    );
}
#[test]
#[allow(deprecated)]
fn warmed_queries_do_not_clone_maps_and_replacements_remain_visible() {
    use oxidize_pdf::text::metrics::{get_custom_font_metrics, register_custom_font_metrics};
    check_queries(&Font::Helvetica, None, 13.34);
    let widths: Vec<_> = (32..10032)
        .filter_map(char::from_u32)
        .map(|c| (c, 500u16))
        .collect();
    let first = FontMetricsStore::new();
    let second = FontMetricsStore::new();
    first.register("Scoped", FontMetrics::new(500).with_widths(&widths));
    second.register("Scoped", FontMetrics::new(700));
    let font = Font::Custom("Scoped".into());
    check_queries(&font, Some(&first), 10.);
    check_queries(&font, Some(&second), 14.);
    first.register("Scoped", FontMetrics::new(800));
    check_queries(&font, Some(&first), 16.);
    check_queries(&font, Some(&second), 14.);
    register_custom_font_metrics("Scoped".into(), FontMetrics::new(900));
    check_queries(&font, None, 18.);
    check_queries(&font, Some(&first), 16.);
    let snapshot = get_custom_font_metrics("Scoped").unwrap();
    register_custom_font_metrics("Scoped".into(), FontMetrics::new(300));
    assert_eq!(snapshot.char_width('A'), 900);
    check_queries(&font, None, 6.);
}
