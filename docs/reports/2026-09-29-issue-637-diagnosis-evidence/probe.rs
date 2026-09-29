use oxidize_pdf::parser::{PdfReader,PdfDocument,PdfObject};
use oxidize_pdf::text::TextExtractor;
use std::{path::Path,fs};
fn main(){
 let a:Vec<_>=std::env::args().collect();
 if a[1]=="--stream" {
  let b=fs::read(&a[2]).unwrap(); let mut dict=oxidize_pdf::parser::PdfDictionary::new();
  dict.insert("Filter".into(),PdfObject::Name(oxidize_pdf::parser::PdfName::new("FlateDecode".into())));
  let r=oxidize_pdf::parser::filters::decode_stream(&b,&dict,&oxidize_pdf::parser::ParseOptions::strict());
  match r{Ok(v)=>{fs::write(&a[3],&v).unwrap();println!("{}",serde_json::json!({"bytes":v.len()}));},Err(e)=>println!("{}",serde_json::json!({"error":e.to_string()}))};return;
 }
 let mut reader=match PdfReader::open(&a[1]){Ok(r)=>r,Err(e)=>{println!("{}",serde_json::json!({"parsed":false,"error":e.to_string()}));return}};
 if a.get(2).map(String::as_str)==Some("--tree") {
  println!("catalog: {:?}",reader.catalog());println!("object 113: {:?}",reader.get_object(113,0));
  let pages=reader.pages().unwrap().clone();println!("pages: {pages:?}");
  if let Some(PdfObject::Array(kids))=pages.get("Kids") {for kid in &kids.0{if let Some((id,generation))=kid.as_reference(){println!("kid {id} {generation}: {:?}",reader.get_object(id,generation));}}}return;
 }
 let doc=PdfDocument::new(reader);
 let pages=doc.page_count().unwrap_or(0);
 if let Some(out)=a.get(2){
  fs::create_dir_all(out).unwrap(); let mut records=Vec::new(); let mut ex=TextExtractor::new();
  for n in 0..pages{
   if let Some(sel)=a.get(3){if sel!="failed" && !sel.split(',').any(|v|v==n.to_string()){continue;}}

   let text=ex.extract_from_page(&doc,n);
   if a.get(3).map(String::as_str)==Some("failed") && text.is_ok(){continue;}
   let mut entry=serde_json::json!({"page":n});
   match text{Ok(t)=>{fs::write(Path::new(out).join(format!("page-{n}.txt")),&t.text).unwrap();entry["text_bytes"]=t.text.len().into()},Err(e)=>entry["error"]=e.to_string().into()}
   if let Ok(page)=doc.get_page(n){if let Some(c)=page.dict.get("Contents"){
    let mut pending=vec![c.clone()];let mut streams=Vec::new();
    while let Some(obj)=pending.pop(){if let Ok(res)=doc.resolve(&obj){match res{PdfObject::Array(a)=>pending.extend(a.0),PdfObject::Stream(s)=>{
     let name=format!("page-{n}-stream-{}.bin",streams.len());fs::write(Path::new(out).join(&name),&s.data).unwrap();
     streams.push(serde_json::json!({"file":name,"dict":format!("{:?}",s.dict),"error":doc.decode_stream(&s).err().map(|e|e.to_string())}));
    },_=>()}}}entry["streams"]=streams.into();
   }}records.push(entry);
  }
  fs::write(Path::new(out).join("pages.json"),serde_json::to_vec_pretty(&records).unwrap()).unwrap();
 }
 let result=doc.extract_text();
 let value=match result{Ok(t)=>serde_json::json!({"parsed":true,"pages":pages,"extracted":true,"text_bytes":t.iter().map(|p|p.text.len()).sum::<usize>()}),Err(e)=>serde_json::json!({"parsed":true,"pages":pages,"extracted":false,"error":e.to_string()})};
 println!("{value}");
}
