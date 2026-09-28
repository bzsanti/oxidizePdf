//! Count cumulative allocation requests on this thread before the first callback.
//! This is not RSS or peak live memory; inputs are allocated before measurement.
use oxidize_pdf::streaming::stream_text;
use oxidize_pdf::PdfError;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct CountingAllocator;
thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}
fn record(size: usize) {
    let _ = COUNTING.try_with(|enabled| {
        if enabled.get() {
            let _ = BYTES.try_with(|n| n.set(n.get() + size));
        }
    });
}
// SAFETY: all requests are forwarded unchanged to System, including alignment
// and original layouts. Counters are thread-local and do not allocate.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size);
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn first_callback_allocations(count: usize, same_stream: bool) -> usize {
    let first = "BT /F1 12 Tf 100 700 Td (FIRST) Tj ";
    let tail = format!("{} ET", "(X) Tj ".repeat(count));
    let streams = if same_stream {
        vec![format!("{first}{tail}").into_bytes()]
    } else {
        vec![first.as_bytes().to_vec(), tail.into_bytes()]
    };
    BYTES.with(|n| n.set(0));
    COUNTING.with(|enabled| enabled.set(true));
    let mut observed = None;
    let result = stream_text(streams, |chunk| {
        COUNTING.with(|enabled| enabled.set(false));
        observed = Some(BYTES.with(Cell::get));
        assert_eq!(chunk.text, "FIRST");
        Err(PdfError::OperationCancelled)
    });
    COUNTING.with(|enabled| enabled.set(false));
    assert!(matches!(result, Err(PdfError::OperationCancelled)));
    observed.expect("first callback must run")
}

#[test]
fn first_callback_and_cancellation_do_not_materialize_the_tail() {
    for same_stream in [false, true] {
        let small = first_callback_allocations(1, same_stream);
        let large = first_callback_allocations(10_000, same_stream);
        eprintln!(
            "same_stream={same_stream}: first callback allocation requests {small} -> {large}"
        );
        assert!(
            large <= small + 4096,
            "tail allocation before callback: {small} -> {large}"
        );
    }
    let mut count = 0;
    stream_text(
        vec![
            b"BT /F1 12 Tf 100 700 Td (FIRST) Tj".to_vec(),
            format!("{} ET", "(X) Tj ".repeat(10_000)).into_bytes(),
        ],
        |chunk| {
            assert_eq!(chunk.text, if count == 0 { "FIRST" } else { "X" });
            assert_eq!((chunk.x, chunk.y, chunk.font_size), (100.0, 700.0, 12.0));
            count += 1;
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(count, 10_001);
}
