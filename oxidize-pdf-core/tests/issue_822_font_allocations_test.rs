//! Repeated standard-font text must not allocate a name for each Tf operator.
use oxidize_pdf::{text::TextContext, Font};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

struct Counter;
thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
}
fn record() {
    if ENABLED.try_with(Cell::get).unwrap_or(false) {
        let _ = CALLS.try_with(|calls| calls.set(calls.get() + 1));
    }
}
// SAFETY: System receives the original pointer, layout and size unchanged;
// the thread-local counters neither allocate nor access the allocated memory.
unsafe impl GlobalAlloc for Counter {
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
static ALLOCATOR: Counter = Counter;

#[test]
fn standard_font_writes_allocate_text_buffers_without_repeated_font_names() {
    for font in [
        Font::Helvetica,
        Font::HelveticaBold,
        Font::HelveticaOblique,
        Font::HelveticaBoldOblique,
        Font::TimesRoman,
        Font::TimesBold,
        Font::TimesItalic,
        Font::TimesBoldItalic,
        Font::Courier,
        Font::CourierBold,
        Font::CourierOblique,
        Font::CourierBoldOblique,
        Font::Symbol,
        Font::ZapfDingbats,
    ] {
        let mut context = TextContext::new();
        context.set_font(font.clone(), 12.0).write("AB").unwrap();
        CALLS.with(|calls| calls.set(0));
        ENABLED.with(|enabled| enabled.set(true));
        for _ in 0..100 {
            context.write(std::hint::black_box("AB")).unwrap();
        }
        ENABLED.with(|enabled| enabled.set(false));
        let calls = CALLS.with(Cell::get);
        assert!(
            calls <= 110,
            "{font:?}: {calls} allocations; budget is one text buffer per write plus vector growth"
        );
    }
}
