# Certificate verification without a native crypto dependency

Issue: [#627](https://github.com/bzsanti/oxidizePdf/issues/627).

Certificate verification is required product functionality. Its Cargo feature
must not be disabled to satisfy the no-C requirement. `rustls-webpki` retains
certificate path construction, trust anchors, validity, constraints, PDF EKU,
and CRL verification. `oxidize-webpki`, the independent
`SignatureVerificationAlgorithm` adapter crate published on crates.io, delegates to
RustCrypto `rsa`, `p256`, `p384`, `sha2`, and `ed25519-dalek`; it does not implement
cryptographic arithmetic or replace certificate validation.

The previous ring-backed algorithm set is preserved: P-256/P-384 with both
SHA-256/SHA-384, Ed25519, RSA PKCS#1 v1.5 with SHA-256/384/512 (NULL or absent
signature parameters), and RSA-PSS with the same digests. RSA moduli remain
2048–8192 bits, odd exponents 3 through 2^33−1; PSS requires matching MGF1 and
digest-length salt. RSA-PSS public-key identifiers remain unsupported, as in
the previous legacy-key provider. Compressed EC public keys remain rejected.
Ed25519 uses `verify_strict`, rejecting small-order keys and noncanonical
signatures. Certificate revocation behavior is unchanged: without usable CRLs,
a certificate does not become fully valid. No provider fallback is allowed.

## Provenance and limits

The dependency lock pins each registry version and checksum. The adapter uses
existing RustCrypto dependencies plus Ed25519-Dalek 2.2.0 / Curve25519-Dalek
4.1.3. The adapter introduces no unsafe code or FFI. Dependencies can use unsafe
Rust, CPU intrinsics and inline assembly: Rust source is not a proof of
cryptographic correctness, memory safety, constant-time behavior or FIPS
certification. Only public-key verification is added here, not private-key RSA
operations. Existing CMS verification continues to use its existing code.

The full `rustls-rustcrypto` TLS provider was evaluated and not selected because
its README warns against production use. WebPKI's public verifier interface
allows reusing the existing X.509 validation with narrowly scoped dispatch.

Primary references:

- https://docs.rs/rustls-pki-types/1.14.0/rustls_pki_types/trait.SignatureVerificationAlgorithm.html
- https://github.com/rustls/webpki
- https://github.com/RustCrypto/RSA
- https://github.com/dalek-cryptography/curve25519-dalek
- https://github.com/RustCrypto/rustls-rustcrypto

## Reproducible gate

Run `python3 scripts/check_no_native_dependencies.py --product-features --build`
on Linux. Set `--target` for other platforms. Python 3.11+ is required. The minimal compiled configuration is `compression,signatures`; disabling
compression currently exposes a pre-existing feature-gating defect recorded
in TASKS.md. Graph inspection alone does not assert buildability. The CI
checks Linux x86_64/aarch64, Windows x86_64 MSVC, macOS x86_64/aarch64 graphs,
and builds on native Linux/Windows/macOS hosts. `--product-features` selects
every declared feature except `ocr-tesseract` and its `ocr-full` alias; it includes
`signatures`, image support, performance, token counting and semantic features.

Tesseract is an optional external integration, separate from mandatory
certificate verification. `--all-features` inventories it as well and is
expected to report its additional image/AVIF native assembly sources. The gate
does not turn off this feature or claim an OCR-inclusive build is Rust-only.
The `--target all` inventory also includes unsupported platforms such as Haiku,
whose timezone dependency contains C++; it is not a supported-build pass.

The gate examines normal/build transitive edges, native compiler packages,
`links` metadata, C/C++/assembly sources and native archives, and direct compiler
invocations in build scripts. The library build additionally disables C/C++
compilers. This is a regression gate, not a proof against malicious build scripts;
dependency updates still require source/provenance review.

`python3 scripts/check_certificate_product.py --target TARGET` additionally
runs a consumer with only `compression,signatures`, a copy of the pinned lock,
and C/C++ compilers disabled. It checks the consumer graph, accepts the trusted
fixture with a valid CRL, and rejects its revoked variant. Development-only
feature unification cannot supply a hidden WebPKI provider to this test.

Narrow reviewed distinctions:

- Windows import archives represent the operating-system ABI. `libc`, `winapi`
  and Rust's standard library also access OS services; these are inventoried
  platform interfaces, not a substituted crypto implementation.
- `rayon-core` 1.13.0 uses Cargo `links` solely as a singleton marker; its build
  script only emits a rerun directive.
- Two hash-pinned C files in `fancy-regex` 0.17.0 are textual test inputs parsed
  by Rust via `include_str!`, never compiled. Other C files remain rejected.
- For an actual Python binding manifest, pass `--python-binding --package
  oxidize-python --features '' --manifest-path PATH`. This recognizes PyO3-FFI
  0.28.3's host Python ABI. The Python interpreter remains the host runtime;
  the extension must not pull native cryptography.
- Development-only `ureq -> rustls -> ring` and `criterion -> alloca -> cc` are
  separately inventoried and are not included in the library or wheel graph.
  Their presence in Cargo.lock is not evidence that the product links them.

## Python integration validation

A clean snapshot of binding commit `81a74e6b` is tested against the candidate
core via a local path dependency. This preserves the sibling repository's
uncommitted work. Wheel/source build results must identify that override and
must not be presented as verification of already published wheels. After the
core is released, the binding must pin the compliant release and repeat this
gate on its final lockfile before publication.

Security advisory review (2026-09-25):
[RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html)
still lists `rsa` 0.9.10 as unpatched for private-key timing leakage. This
adapter handles only public keys, messages and signatures; it exposes no
private-key operation. That scope analysis is not a claim that the crate is
advisory-free. The locked Dalek versions include the fixes for
[RUSTSEC-2024-0344](https://rustsec.org/advisories/RUSTSEC-2024-0344.html)
and [RUSTSEC-2022-0093](https://rustsec.org/advisories/RUSTSEC-2022-0093.html).
