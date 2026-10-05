#[path="../tests/common/text_contracts.rs"] mod contract;
use oxidize_pdf::parser::ParseOptions;
fn main() {
 let dir=std::env::args().nth(1).expect("PDF directory");
 let mut results=Vec::new();
 for path in std::fs::read_dir(dir).unwrap() {
  let path=path.unwrap().path();
  if path.extension().and_then(|x|x.to_str())!=Some("pdf") {continue;}
  let result=contract::extract(std::fs::read(&path).unwrap(),ParseOptions::strict());
  assert_eq!(result.text,"fi","{}",path.display());
  results.push(serde_json::json!({"file":path.file_name().unwrap().to_str().unwrap(),"text":result.text}));
 }
 println!("{}",serde_json::to_string_pretty(&results).unwrap());
}
