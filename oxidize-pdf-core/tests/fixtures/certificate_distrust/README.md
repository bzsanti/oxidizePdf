# Synthetic certificate distrust regression

These certificates are synthetic test data, issued by an ephemeral test root.
No private keys are stored. They are valid from 2025-01-01 to 2035-01-01; the test
uses a fixed validation time. The allowed and distrusted signers have the same
subject, usage, issuer and validity, but different public keys and serials.

The distrusted signer contains the public EMDVTLSCAG3B SPKI listed in
rustls-webpki 0.103.16, `src/data/distrusted/EMDVTLSCAG3B.spki.der`.
Its outer DER SEQUENCE is restored by the generator. This is not a real emSign
certificate and does not require or contain an emSign private key.

Run `python3 generate.py /path/to/EMDVTLSCAG3B.spki.der` with Python
`cryptography` installed to regenerate all three certificates together. New
random keys/signatures change their bytes but preserve the tested properties.
The Rust test consumes the checked-in DER without Python or network access.

The positive control must validate under the explicitly trusted synthetic root;
the distrusted key must fail specifically with `CertDistrusted`. This exercises
core chain validation with the pure-Rust oxidize-webpki provider. It does not
claim complete revocation checks or test every distrusted intermediate.

References: https://rustsec.org/advisories/RUSTSEC-2026-0382.html and
https://github.com/rustls/webpki/pull/546.
