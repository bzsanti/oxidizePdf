use oxidize_pdf::text::cid_to_unicode::CidCollection;
fn name(c:CidCollection)->&'static str { match c { CidCollection::Cns1=>"CNS1",CidCollection::Gb1=>"GB1",CidCollection::Japan1=>"Japan1",CidCollection::Korea1=>"Korea1" }}
fn main(){println!("{}",name(CidCollection::Cns1));}
