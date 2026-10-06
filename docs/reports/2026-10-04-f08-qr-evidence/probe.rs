#[path = "../../../oxidize-pdf-core/tests/common/text_contracts.rs"]
mod contract;
use contract::assembler::stream_obj;
use oxidize_pdf::fonts::{ResolvedFontResource,Type3Font};
use oxidize_pdf::parser::{PdfReader,ParseOptions,PdfObject};
use oxidize_pdf::text::{TextExtractor,ExtractionOptions};
use std::io::Cursor;
fn probe(label:&str,encoding:&str,charprocs:&str,widths:&str,matrix:&str,extra:Vec<Vec<u8>>,program:&[u8]) {
 let font=format!("<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] /FontMatrix [{matrix}] /CharProcs << {charprocs} >> /Encoding {encoding} /FirstChar 65 /LastChar 65 /Widths {widths} /Resources << >> >>");
 let mut objects=vec![stream_obj("",program),stream_obj("",b"500 0 d0")];objects.extend(extra);
 for options in [ParseOptions::strict(),ParseOptions::lenient()] {
 let strict=options.strict_mode;
 let bytes=contract::pdf(&font,b"BT /F1 10 Tf 100 700 Td (AA) Tj ET",objects.clone());
 let doc=PdfReader::new_with_options(Cursor::new(bytes),options).unwrap().into_document();
 match Type3Font::resolve(&PdfObject::Reference(4,0),&doc) {
 Ok(f)=>println!("{label} strict={strict} type3={:?}",f.glyph(65).map(|g|(&g.name,g.width,g.procedure_width))),Err(e)=>println!("{label} strict={strict} type3 ERR {e}")
 }
 match ResolvedFontResource::from_page(&doc,0,"F1") {
 Ok(f)=>println!("{label} glyphs={:?}",f.decode_glyphs(b"A")),Err(e)=>println!("{label} resolved ERR {e}")
 }
 let text=TextExtractor::with_options(ExtractionOptions{preserve_layout:true,sort_by_position:false,..Default::default()}).extract_from_page(&doc,0);
 println!("{label} extraction={:?}",text.map(|t|t.fragments.into_iter().map(|f|(f.text,f.x,f.y)).collect::<Vec<_>>()));
 }
}
fn main(){
 let matrix="0.001 0 0 0.001 0 0";
 probe("direct","<< /Differences [65 /B] >>","/A 6 0 R /B 7 0 R","[500]",matrix,vec![],b"500 0 d0");
 probe("indirect-diffs","<< /Differences 8 0 R >>","/A 6 0 R /B 7 0 R","[500]",matrix,vec![b"[65 /B]".to_vec()],b"500 0 d0");
 probe("indirect-base","<< /BaseEncoding 8 0 R /Differences [] >>","/A 6 0 R /B 7 0 R","[500]",matrix,vec![b"/BogusEncoding".to_vec()],b"500 0 d0");
 probe("missing-proc","<< /Differences [65 /fi] >>","/B 7 0 R","[500]",matrix,vec![],b"500 0 d0");
 probe("missing-proc-null-width","<< /Differences [65 /fi] >>","/B 7 0 R","[null]",matrix,vec![],b"500 0 d0");
 let huge=format!("1{}.0", "0".repeat(307));
 probe("overflow-matrix","<< /Differences [65 /A] >>","/A 6 0 R","[500]",&format!("{huge} 0 0 0.001 0 0"),vec![],b"500 0 d0");
 let program=format!("1{}.0 0 d0", "0".repeat(40));
 probe("overflow-procedure","<< /Differences [65 /A] >>","/A 6 0 R","[500]",matrix,vec![],program.as_bytes());
}
