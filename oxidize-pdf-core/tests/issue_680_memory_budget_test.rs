//! Recovery must not retain both expanded attempts simultaneously.
#![cfg(feature = "compression")]
use flate2::{write::ZlibEncoder, Compression};
use oxidize_pdf::parser::{
    filters::decode_stream_with_recovery, ParseOptions, PdfDictionary, PdfName, PdfObject,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
struct Counting;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
fn add(n: usize) {
    let value = LIVE.fetch_add(n, Ordering::SeqCst) + n;
    PEAK.fetch_max(value, Ordering::SeqCst);
}
// Safety: every operation delegates unchanged pointer/layout contracts to System;
// atomics only count requested live allocation sizes, never dereference pointers.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() {
            add(l.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l);
        LIVE.fetch_sub(l.size(), Ordering::SeqCst);
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = System.realloc(p, l, n);
        if !q.is_null() {
            if n >= l.size() {
                add(n - l.size());
            } else {
                LIVE.fetch_sub(l.size() - n, Ordering::SeqCst);
            }
        }
        q
    }
}
#[global_allocator]
static ALLOC: Counting = Counting;
#[test]
fn recovery_keeps_one_expanded_buffer_live() {
    let plain = vec![b'A'; 1024 * 1024];
    let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
    z.write_all(&plain).unwrap();
    let mut z = z.finish().unwrap();
    *z.last_mut().unwrap() ^= 1;
    let mut d = PdfDictionary::new();
    d.insert(
        "Filter".into(),
        PdfObject::Name(PdfName::new("FlateDecode".into())),
    );
    let start = LIVE.load(Ordering::SeqCst);
    PEAK.store(start, Ordering::SeqCst);
    let output = decode_stream_with_recovery(&z, &d, &ParseOptions::strict(), plain.len()).unwrap();
    let extra = PEAK.load(Ordering::SeqCst) - start;
    assert_eq!(output.data, plain);
    assert_eq!(output.diagnostics.len(), 1);
    println!(
        "decoded_bytes={} peak_live_requested_increment={extra}",
        plain.len()
    );
    assert!(
        extra <= plain.len() + 512 * 1024,
        "Recovery retained multiple expanded buffers: {extra} bytes for {} decoded bytes",
        plain.len()
    );
}
