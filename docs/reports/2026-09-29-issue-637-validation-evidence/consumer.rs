use oxidize_pdf::parser::{filters::{decode_stream,decode_stream_with_recovery,FlateRecoveryKind,StreamRecoveryErrorKind},ParseOptions,PdfDictionary,PdfName,PdfObject};
fn main(){
 let mut d=PdfDictionary::new();d.insert("Filter".into(),PdfObject::Name(PdfName::new("FlateDecode".into())));
 let input=b"\x78\x01\x01\x0a\x00\xf5\xffABCD";
 assert!(decode_stream(input,&d,&ParseOptions::tolerant()).is_err());
 let r=decode_stream_with_recovery(input,&d,&ParseOptions::strict(),4).unwrap();assert_eq!(r.data,b"ABCD");assert_eq!(r.diagnostics[0].kind,FlateRecoveryKind::Incomplete);
 assert_eq!(decode_stream_with_recovery(input,&d,&ParseOptions::strict(),3).unwrap_err().kind,StreamRecoveryErrorKind::ResourceLimit);
 assert_eq!(decode_stream_with_recovery(b"",&d,&ParseOptions::strict(),4).unwrap_err().kind,StreamRecoveryErrorKind::InvalidFlate);
 println!("compression-only external consumer passed");
}
