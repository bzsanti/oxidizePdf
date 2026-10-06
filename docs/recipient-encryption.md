# Recipient encryption support and migration

Certificate-based PDF recipient encryption/decryption is unsupported. Before
the #640 containment change, `PublicKeySecurityHandler` simulated recipient
protection: it stored the seed in cleartext and recovered it with unrelated
nonempty keys. Successful calls through that handler did not establish
confidentiality. This finding does not describe the password-based handler or
signature certificate verification.

The constructors, public fields and method signatures remain available for
source compatibility. Behavior changes deliberately:

- `add_recipient` and `decrypt_seed` return `PdfError::EncryptionError` with an
  explicit unsupported-recipient-encryption message for every input. No seed is
  generated and failed insertion leaves existing recipient data unchanged.
- All eight `SecurityHandler` encryption/decryption methods return the same
  error, including explicit AES methods and calls routed through crypt filters.
  Supplying a key or changing public configuration fields cannot enable them.
- `verify_permission` always returns `false`: caller-supplied recipient data
  does not authenticate or authorize anyone.
- `build_recipients_dict`, `PublicKeyEncryptionDict::new` and `to_dict` remain
  raw metadata helpers. They copy caller-supplied data without validating
  certificates, encrypting seeds, deriving a file key or producing a supported
  encryption dictionary. Never treat their return value as proof of protection.

Handle the unsupported error at the application boundary. Do not catch it and
silently write an unencrypted document. Existing outputs produced using the
simulation must not be treated as confidential; regenerate protected originals
using an independently supported encryption path when needed.

The basic encryption surface is password-based document encryption through
`DocumentEncryption`/`StandardSecurityHandler` and password decryption through
the parser. RC4, AES-128 and AES-256 primitives exist; exact revision support is
specific to the reader/writer API. This change does not expand password-based
R6 writing or recommend legacy RC4 for new protection.

Signature detection, PKCS#7 verification and certificate validation belong to
the separate `signatures` feature. Certificate validation there does not
provide recipient encryption.

`SecurityHandler` allows custom low-level crypt-filter implementations; it does
not itself supply certificate-aware writer/reader integration. Real recipient
encryption, key derivation, randomness, CMS envelopes and independent
interoperability are tracked in [#642](https://github.com/bzsanti/oxidizePdf/issues/642).
PAdES is outside the scope of oxidize-pdf. Anyone can implement it as an
independent external extension without incorporating it into this library.
