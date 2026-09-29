# Certificate recipient encryption — implementation candidate

**Not approved for supported use yet (#642).** The implementation and independent
interoperability tests are present, but security acceptance is blocked by
[RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html) affecting
`rsa 0.9.10`. Upstream reports no patched release. OAEP and explicit RSA blinding
are used here, but do not establish exemption from that advisory. Do not expose
this candidate as a decryption service for attacker-controlled documents.
Closure requires a suitable pure-Rust provider or conclusive independent review
of the exact private-key path; passing functional tests is insufficient.

The optional `recipient-encryption` feature implements a bounded Adobe.PubSec
profile through `Document::set_recipient_encryption` and
`PdfReader::unlock_with_recipient`. It is separate from password encryption
and digital signatures. PAdES is outside the scope of oxidize-pdf; anyone can
implement an independent external extension without incorporating it into this library.

## Supported profile

| Component | Supported value |
| --- | --- |
| PDF security handler | `/Adobe.PubSec`, `/adbe.pkcs7.s5`, `/V 5`, `/Length 256` |
| Crypt filter | One AESV3 filter, DocOpen; same filter for streams, strings and embedded files |
| PDF content | AES-256-CBC, random 16-byte IV, PKCS#7 padding; metadata encrypted |
| Recipient container | DER CMS EnvelopedData v0, issuer/serial KeyTransRecipientInfo v0 |
| Key transport | RSA-OAEP with SHA-256, MGF1-SHA-256 and empty label |
| CMS content | AES-256-CBC, fresh key/IV; 20-byte seed and four big-endian permission bytes |
| File key | SHA-256 over the seed and every original recipient container in array order |
| Certificates | X.509 DER; rsaEncryption public keys, 2048–4096 bits; at most 32 recipients |
| Private keys | Unencrypted PKCS#8 DER RSA, matching the supplied certificate |
| Writer | Fresh PDF 2.0 with classic xref; default WriterConfig is supported |
| Reader | Classic xref or xref streams, including encrypted object streams |

Mixed/Identity filters, explicit `/Crypt` stream filters, unencrypted metadata,
other CMS algorithms/recipient forms, PDF MAC, encrypted PKCS#8/PEM input,
modern writer object/xref streams and incremental encryption are unsupported.
Conflicting password and recipient policies return an error. Unsupported writer
configuration is rejected before writing bytes; discard output on any write error.

## Public workflow

```rust,no_run
use oxidize_pdf::{Document, Page};
use oxidize_pdf::encryption::{Permissions, RecipientEncryption};
use oxidize_pdf::parser::PdfReader;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let certificate = std::fs::read("recipient.cert.der")?;
let recipients = RecipientEncryption::new(&certificate, Permissions::all())?;
let mut document = Document::new();
document.add_page(Page::a4());
document.set_recipient_encryption(recipients);
document.save("encrypted.pdf")?;

let private_key = std::fs::read("recipient.key.der")?;
let mut reader = PdfReader::open("encrypted.pdf")?;
reader.unlock_with_recipient(&certificate, &private_key)?;
let parsed = reader.into_document();
assert_eq!(parsed.page_count()?, 1);
# Ok(())
# }
```

Add additional certificates with `RecipientEncryption::add_recipient` before
setting the document policy. Each can have different permissions. Duplicate
issuer/serial identities are rejected. Certificates must be within their validity
period when configuring and writing; KeyUsage, if present, must allow
keyEncipherment. These checks are **not certificate trust, chain, hostname or
revocation validation**: the caller selects and authenticates its recipients.
Expired certificates can still decrypt archived documents with the correct key.

The reader reports protected recipient permissions after successful unlock.
Permission enforcement depends on the reader and does not prevent a private-key
holder from processing plaintext. A failed recipient unlock clears the previous
key, permissions and plaintext caches. Password unlock cannot authenticate a
recipient-encrypted PDF. Caller-owned private-key buffers remain the caller's
responsibility; use a zeroizing buffer where appropriate. Internal seed, CMS key,
plaintext key-recovery buffers and retained file keys are cleared on drop; this
is not a guarantee against all copies in process memory or crash dumps.

AES-CBC provides confidentiality, **not authenticated integrity**. Invalid DER,
unsupported profiles, wrong keys, malformed ciphertext lengths and padding return
errors; some modified ciphertext can still have valid padding. Successful opening
does not prove that a PDF is authentic or unmodified. This profile has no PDF MAC.

## Legacy migration

`PublicKeySecurityHandler` remains an unsupported compatibility shell. Its
cryptographic methods return errors and permission checks deny access, as required
by #640. Its raw dictionary helpers do not protect documents. Do not suppress
those errors and write plaintext. Use the new `RecipientEncryption` API instead;
outputs from the old simulation must not be treated as confidential.

Password-based RC4/AES profiles and the `signatures` feature remain separate.
This feature does not add AES-256 R6 password output, certificate issuance, key
custody or PKI services.

## Independent evidence and reproduction

Fixtures in `oxidize-pdf-core/tests/fixtures/recipient_encryption` are generated
by `oracle.py` with Python 3.14.6, pyHanko 0.29.1, cryptography 50.0.1 and
asn1crypto 1.5.1. Keys are deliberately public **test-only** RSA-2048 keys.
`SHA256SUMS` records the committed DER/PDF fixtures. Regeneration changes keys,
seeds and ciphertext. Fixtures contain text, a 2×2 RGB image, Info and stream
dictionary strings, two recipients, and classic/xref/object-stream variants.

```sh
cargo test -p oxidize-pdf --features recipient-encryption --test recipient_encryption_test
python scripts/check_recipient_product.py --target x86_64-unknown-linux-gnu --output /tmp/recipient.pdf
# In a separate test environment: pip install pyhanko==0.29.1 cryptography==50.0.1 asn1crypto==1.5.1
python oxidize-pdf-core/tests/fixtures/recipient_encryption/oracle.py verify /tmp/recipient.pdf
```

The external Rust consumer has no project dev-dependencies and builds with
`CC=false CXX=false`. CI runs it on Linux, Windows and macOS. The independent
Python consumer verifies both recipients, content and permissions, and checks
recovered seeds/file keys differ between repeated outputs. Python/OpenSSL is a
test oracle, not a product dependency. See #642 and its review report for results.
