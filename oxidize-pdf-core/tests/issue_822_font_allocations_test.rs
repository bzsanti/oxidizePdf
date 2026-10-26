//! Repeated standard-font text must not allocate a name for each Tf operator.
use oxidize_pdf::{text::TextContext, Document, Font, Page};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

struct Counter;
thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
    static FONT_SIZE: Cell<usize> = const { Cell::new(0) };
    static FONT_COPIES: Cell<usize> = const { Cell::new(0) };
}
fn record(bytes: usize) {
    if ENABLED.try_with(Cell::get).unwrap_or(false) {
        let _ = CALLS.try_with(|calls| calls.set(calls.get() + 1));
        let _ = BYTES.try_with(|total| total.set(total.get() + bytes));
        if FONT_SIZE.try_with(Cell::get).unwrap_or(0) == bytes {
            let _ = FONT_COPIES.try_with(|copies| copies.set(copies.get() + 1));
        }
    }
}
// SAFETY: System receives the original pointer, layout and size unchanged;
// the thread-local counters neither allocate nor access the allocated memory.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size);
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[test]
fn repeated_text_does_not_reserve_a_character_set_for_every_occurrence() {
    let text = "A".repeat(16_384);
    let mut context = TextContext::new();
    BYTES.with(|bytes| bytes.set(0));
    ENABLED.with(|enabled| enabled.set(true));
    context.write(&text).unwrap();
    ENABLED.with(|enabled| enabled.set(false));
    let bytes = BYTES.with(Cell::get);
    assert!(
        bytes < 2 * text.len(),
        "{bytes} cumulative bytes for a {}-byte text with one distinct character",
        text.len()
    );
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;

#[test]
fn serializing_a_cjk_font_reuses_the_original_parse_for_widths() {
    let data = include_bytes!("fixtures/writer_resources/WriterCjkTest-Regular.otf");
    let mut document = Document::new();
    document.add_font_from_bytes("CJK", data.to_vec()).unwrap();
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("CJK".into()), 12.0)
        .at(40.0, 700.0)
        .write("中文测试")
        .unwrap();
    document.add_page(page);
    FONT_SIZE.with(|size| size.set(data.len()));
    FONT_COPIES.with(|copies| copies.set(0));
    ENABLED.with(|enabled| enabled.set(true));
    let pdf = document.to_bytes().unwrap();
    ENABLED.with(|enabled| enabled.set(false));
    let copies = FONT_COPIES.with(Cell::get);
    FONT_SIZE.with(|size| size.set(0));
    assert!(pdf.starts_with(b"%PDF-"));
    assert!(copies <= 3, "{copies} full-font-sized allocations; default and per-character widths must share one original-font parse");
}

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
