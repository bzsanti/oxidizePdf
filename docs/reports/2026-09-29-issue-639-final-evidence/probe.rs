use oxidize_pdf::{Document,Page,BuildIdentification};
use oxidize_pdf::writer::{PdfWriter,WriterConfig,IncrementalInfoPolicy};
fn main() {
 let root=std::path::Path::new("/tmp/issue639-final-validation");
 for name in ["flat","nested","maxsize","encrypted","compressed","generation","rich"] {
  for method in 0..3 {
   for replace in [false,true] {
    let mut output=Vec::new();
    let mut writer=PdfWriter::with_config(&mut output,WriterConfig::incremental());
    if replace {writer.set_incremental_info_policy(IncrementalInfoPolicy::Replace);}
    let mut doc=Document::new(); doc.set_title("Replacement title");
    doc.set_build_identification(BuildIdentification::Disabled); let mut page=Page::a4(); page.text().set_font(oxidize_pdf::text::Font::Helvetica,12.0).at(50.0,700.0).write("Replacement").unwrap(); doc.add_page(page);
    let base=root.join(format!("{name}.pdf"));
    let result=match method {
     0=>writer.write_incremental_update(&base,&mut doc),
     1=>writer.write_incremental_with_page_replacement(&base,&mut doc),
     _=>writer.write_incremental_with_overlay(&base,|page| { page.text().set_font(oxidize_pdf::text::Font::Helvetica,12.0).at(50.0,650.0).write("Overlay")?; Ok(()) }),
    };
    println!("{name} method={method} replace={replace}: {result:?}; output_bytes={}",output.len());
    if result.is_ok() {std::fs::write(root.join(format!("{name}-{method}-{replace}.pdf")),output).unwrap();}
   }
  }
 }
 for compressed in [false,true] {
  for xref in [false,true] {
   let mut doc=Document::new(); doc.add_page(Page::a4());
   let config=WriterConfig{compress_streams:compressed,use_xref_streams:xref,..WriterConfig::default()};
   std::fs::write(root.join(format!("config-{compressed}-{xref}.pdf")),doc.to_bytes_with_config(config).unwrap()).unwrap();
  }
 }
}
