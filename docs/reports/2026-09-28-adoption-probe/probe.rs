use oxidize_pdf::parser::{filters::{decode_stream,decode_stream_with_limit},ParseOptions,PdfDictionary,PdfName,PdfObject};
use oxidize_pdf::encryption::{PublicKeySecurityHandler,Permissions};
use std::io::Write;
fn main(){
 let mut dict=PdfDictionary::new();dict.insert("Filter".into(),PdfObject::Name(PdfName::new("FlateDecode".into())));
 println!("strict_invalid_flate={:?}",decode_stream(&[0xff;8],&dict,&ParseOptions::strict()));
 println!("bounded_invalid_flate={:?}",decode_stream_with_limit(&[0xff;8],&dict,&ParseOptions::strict(),1024));
 let mut e=flate2::write::ZlibEncoder::new(Vec::new(),flate2::Compression::default());e.write_all(&[0,42,43]).unwrap();let compressed=e.finish().unwrap();
 let mut params=PdfDictionary::new();params.insert("Predictor".into(),PdfObject::Integer(12));params.insert("Columns".into(),PdfObject::Integer(1));dict.insert("DecodeParms".into(),PdfObject::Dictionary(params));
 println!("strict_invalid_predictor={:?}",decode_stream(&compressed,&dict,&ParseOptions::strict()));
 println!("bounded_invalid_predictor={:?}",decode_stream_with_limit(&compressed,&dict,&ParseOptions::strict(),1024));
 let mut h=PublicKeySecurityHandler::new_sha256();println!("invalid_certificate_accepted={}",h.add_recipient(vec![0u8;100],Permissions::default()).is_ok());
 println!("unrelated_keys_return_same_seed={}",h.decrypt_seed(&h.recipients[0].encrypted_seed,b"a").unwrap()==h.decrypt_seed(&h.recipients[0].encrypted_seed,b"b").unwrap());
 let mut doc=oxidize_pdf::Document::new();doc.add_page(oxidize_pdf::Page::a4());let bytes=doc.to_bytes().unwrap();println!("output_contains_build_marker={}",bytes.windows(b"oxidize-pdf-build".len()).any(|w|w==b"oxidize-pdf-build"));
}
