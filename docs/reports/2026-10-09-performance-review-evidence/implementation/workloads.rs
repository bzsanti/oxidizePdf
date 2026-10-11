use oxidize_pdf::{Document, Font, Page};
use oxidize_pdf::text::{measure_text_block_with, metrics::{FontMetrics, FontMetricsStore}};
use std::{hint::black_box, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
 let a:Vec<_>=std::env::args().collect(); let mode=&a[1]; let n:usize=a[2].parse()?;
 let store=FontMetricsStore::new();
 let widths:Vec<_>=(32..10032).filter_map(char::from_u32).map(|c|(c,500u16)).collect();
 store.register("Large",FontMetrics::new(500).with_widths(&widths));
 let paragraph="A short paragraph with repeated words, café and punctuation. ".repeat(40);
 let custom=Font::Custom("Large".into());
 let run=|| -> Result<Vec<u8>,Box<dyn std::error::Error>> {
  if mode.starts_with("layout") {
   let font=if mode=="layout-custom" {&custom} else {&Font::Helvetica};
   let v=measure_text_block_with(&paragraph,font,12.,1.2,480.,Some(&store));
   black_box(&v); return Ok(format!("{} {} {}",v.width,v.height,v.line_count).into_bytes());
  }
  let mut d=Document::new();
  if mode.starts_with("cjk") {d.add_font_from_bytes("CJK",include_bytes!("../../../issue690-review/oxidize-pdf-core/tests/fixtures/writer_resources/WriterCjkTest-Regular.otf").to_vec())?;}
  if mode.ends_with("-raw") {d.set_compress(false);}
  for _ in 0..10 {let mut p=Page::a4();
   if mode.starts_with("cjk") {for i in 0..22 {p.text().set_font(Font::Custom("CJK".into()),10.).at(50.,790.-i as f64*20.).write("中文 测试 中文 测试")?;}}
   else if mode.starts_with("flow") {let mut f=p.text_flow(); f.write_wrapped(&paragraph)?;p.add_text_flow(&f);}
   else if mode.starts_with("graphics") {for i in 0..200 {let offset=if mode.contains("fractional") {0.125} else {0.}; p.graphics().move_to(20.+i as f64+offset,20.+offset).line_to(20.+i as f64+offset,720.+offset).stroke();}}
   else {for i in 0..22 {p.text().set_font(Font::Helvetica,10.).at(50.,790.-i as f64*20.).write("Café déjà vu (éèêë) — € 12,50; München, España \\ test")?;}}
   d.add_page(p);
  }
  Ok(d.to_bytes()?)
 };
 for _ in 0..3 {black_box(run()?);}
 let start=Instant::now();for _ in 0..n {black_box(run()?);} let elapsed=start.elapsed().as_nanos();
 let out=run()?;if let Some(path)=a.get(3){std::fs::write(path,&out)?;}
 println!("{}",serde_json::json!({"mode":mode,"iterations":n,"elapsed_ns":elapsed,"output_bytes":out.len()}));Ok(())
}
