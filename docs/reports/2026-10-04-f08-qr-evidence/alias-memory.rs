#[path = "../../../oxidize-pdf-core/tests/common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use oxidize_pdf::fonts::Type3Font;
use oxidize_pdf::parser::{PdfReader,PdfObject};
use std::io::Cursor;
fn main(){
 for count in [1,32,128] {
 let font=format!("<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /A 6 0 R >> /Encoding << /Differences [0 {}] >> /FirstChar 0 /LastChar {} /Widths [{}] /Resources << >> >>","/A ".repeat(count),count-1,"500 ".repeat(count));
 let program=format!("500 0 d0 {}","0 0 m ".repeat(2048));
 let bytes=contract::pdf(&font,b"",vec![stream_obj("",program.as_bytes())]);let input=bytes.len();
 let doc=PdfReader::new(Cursor::new(bytes)).unwrap().into_document();
 let f=Type3Font::resolve(&PdfObject::Reference(4,0),&doc).unwrap();
 let capacity:usize=f.glyphs().map(|g|g.operations.capacity()*std::mem::size_of::<oxidize_pdf::parser::content::ContentOperation>()).sum();
 let pointers:std::collections::HashSet<_>=f.glyphs().map(|g|g.operations.as_ptr()).collect();
 println!("aliases={count} input_bytes={input} operations={} allocated_operation_capacity_bytes={capacity} distinct_buffers={}",f.glyphs().map(|g|g.operations.len()).sum::<usize>(),pointers.len());
 }
}
