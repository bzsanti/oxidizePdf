use oxidize_pdf::encryption::{EncryptionKey, SecurityHandler, StandardSecurityHandler, Permissions};
use oxidize_pdf::objects::ObjectId;
use oxidize_pdf::document::{DocumentEncryption, EncryptionStrength};
use oxidize_pdf::{Document, Page, PdfError};
fn check(stream: bool) {
 for (handler,size) in [(StandardSecurityHandler::aes_128_r4(),16),(StandardSecurityHandler::aes_256_r5(),32)] {
  let security:&dyn SecurityHandler=&handler;
  let key=EncryptionKey::new(vec![0x42;size]);let id=ObjectId::new(1,0);
  let outcome=if stream {security.encrypt_stream(b"secret",&key,&id)}else{security.encrypt_string(b"secret",&key,&id)};
  assert!(matches!(outcome,Err(PdfError::EncryptionError(message)) if message=="injected secure entropy failure"));
 }
}
#[test] fn entropy_error_reaches_trait_string(){check(false);}
#[test] fn entropy_error_reaches_trait_stream(){check(true);}
#[test] fn entropy_error_reaches_writer_and_rc4_still_works(){
 for strength in [EncryptionStrength::Aes128,EncryptionStrength::Aes256,EncryptionStrength::Rc4_128bit] {
  let mut doc=Document::new();doc.add_page(Page::a4());
  doc.set_encryption(DocumentEncryption::new("user","owner",Permissions::new(),strength));
  let result=doc.to_bytes();
  if matches!(strength,EncryptionStrength::Rc4_128bit){assert!(result.unwrap().starts_with(b"%PDF-"));}
  else {assert!(matches!(result,Err(PdfError::EncryptionError(message)) if message.contains("injected secure entropy failure")));}
 }
}
