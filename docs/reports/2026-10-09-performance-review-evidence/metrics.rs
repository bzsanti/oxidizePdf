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
use std::hint::black_box;
use oxidize_pdf::{Font};
use oxidize_pdf::text::metrics::{measure_text_with, FontMetricsStore, FontMetrics};
fn main() {
 let store=FontMetricsStore::new();
 let chars: Vec<_>=(32u32..10032).filter_map(char::from_u32).map(|c|(c,500u16)).collect();
 store.register("Large",FontMetrics::new(500).with_widths(&chars));
 for (label,font,scope) in [("standard",Font::Helvetica,None),("custom10000",Font::Custom("Large".into()),Some(&store))] {
  black_box(measure_text_with("Hello world",&font,12.,scope));
  allocs::COUNT.store(0,std::sync::atomic::Ordering::Relaxed); allocs::BYTES.store(0,std::sync::atomic::Ordering::Relaxed);
  let start=std::time::Instant::now(); let mut total=0.;
  for _ in 0..1000 { total+=black_box(measure_text_with(black_box("Hello world"),&font,12.,scope)); }
  let elapsed=start.elapsed().as_nanos();
  let calls=allocs::COUNT.load(std::sync::atomic::Ordering::Relaxed); let bytes=allocs::BYTES.load(std::sync::atomic::Ordering::Relaxed);
  println!("{}",serde_json::json!({"case":label,"iterations":1000,"allocation_calls":calls,"requested_bytes":bytes,"elapsed_ns_instrumented_not_benchmark":elapsed,"total_width":total}));
 }
}
