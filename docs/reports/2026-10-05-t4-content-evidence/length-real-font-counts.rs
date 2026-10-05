use oxidize_pdf::parser::content::{ContentParser, ContentOperation as Op, TextElement};
fn main() {
 let bytes=std::fs::read("target/t4-content-work/preserve_447403-content.bin").unwrap();
 let ops=ContentParser::parse_strict(&bytes).unwrap();
 let mut font=String::new(); let mut stack=Vec::new(); let mut counts=std::collections::BTreeMap::<String,usize>::new();
 for op in ops {match op {
 Op::SaveGraphicsState=>stack.push(font.clone()), Op::RestoreGraphicsState=>{if let Some(f)=stack.pop(){font=f;}},
 Op::SetFont(f,_)=>font=f,
 Op::ShowText(s)|Op::NextLineShowText(s)|Op::SetSpacingNextLineShowText(_,_,s)=>*counts.entry(font.clone()).or_default()+=s.len(),
 Op::ShowTextArray(v)=>for x in v {if let TextElement::Text(s)=x {*counts.entry(font.clone()).or_default()+=s.len();}},
 _=>{}}}
 println!("{counts:#?}");
}
