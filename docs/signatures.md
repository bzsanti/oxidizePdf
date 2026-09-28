# Basic signatures and external signing

Checked against 5.1.5 (`fb4042fd5239b0951979770bab4945c2614a0db8`) on
2026-09-28. The contract applies to the Rust API; bindings must expose and
interpret its results correctly. See [CLAIMS](CLAIMS.md) for evidence.

| Operation | Core behavior | Caller responsibility and limit |
|---|---|---|
| Inspect | Detect fields; parse signature dictionaries, CMS and byte ranges | Finding a field is not validation. |
| Verify | Check covered-document digest and supported CMS cryptographic signatures | Inspect hash/signature results, errors and post-signature modifications separately. An `Ok` result alone is not a valid signature. |
| Validate certificates | Check chains, validity, signature usage and supplied CRLs with the `signatures` feature | Supply appropriate trust anchors, validation time and revocation data. Missing/indeterminate revocation is not checked-valid. No automatic online revocation retrieval or legal assurance is claimed. |
| Prepare | Append an incremental revision, reserve `/Contents` and calculate `/ByteRange` | No private key is used and no cryptographic signature is created. Encrypted PDFs, disallowed modifications, invalid targets and invalid options are rejected. |
| Finalize | Embed caller-supplied DER CMS without rewriting covered bytes | DER envelope acceptance is not cryptographic verification, signer trust or proof that it signs these bytes. Validate the resulting document. |

`FullSignatureValidationResult::is_valid()` requires valid hash and signature,
an acceptable certificate result, no errors and no later modifications. Detailed
certificate results expose `RevocationStatus`; inspect warnings as well. Signature
cryptography requires the `signatures` feature. Use a supported feature
configuration, including compression, as described in the dependency architecture.

## External signing flow

1. Call `prepare_incremental_signature(base, options)` (or the appearance variant).
2. Obtain `prepared.bytes_to_digest()`: these are the exact concatenated covered
   bytes, **not an already-computed digest**.
3. Have your external signer create a detached DER CMS container over those bytes,
   with suitable algorithms, signed attributes and certificate material.
4. Call `prepared.finalize(&cms)`. Empty input, invalid outer DER SEQUENCE lengths and containers exceeding
   the reserved slot fail. This is only an outer-envelope check: finalization
   does not parse/validate CMS SignedData or prove the presence of a signer. Reserve enough space before preparing; if
   you prepare again, sign the new bytes rather than reusing the previous CMS.
5. Verify the final PDF using the intended trust and revocation policy before
   treating it as signed and trusted.

`SignaturePreparationOptions` selects a new or existing field, page/widget,
rectangle, placeholder size, filter/sub-filter, reason/location/contact/time,
DocMDP certification and FieldMDP locks. Additional dictionary entries cannot
override engine-owned structural entries. The appearance variant accepts text
and watermark artwork that is included in the covered revision. Setting a
sub-filter or adding dictionary entries does not establish profile compliance.

The executable preparation example is in the public
[`signatures` module](../oxidize-pdf-core/src/signatures/mod.rs). Existing
`issue_540_incremental_signing_test.rs` covers the external CMS workflow and its
failure cases; its interoperability tests use an external test signer.

## Commercial boundary

Advanced PAdES is outside the open-source core scope and reserved for commercial
extensions. No complete PAdES profile, timestamp-service integration, long-term
validation/archival service or compliance certification is promised by the
prepare/embed APIs. This boundary does not claim a commercial implementation
already exists. External providers can use the neutral byte/CMS interfaces;
this documentation adds no advanced signing implementation.

Certificate-based **recipient encryption** is different from signing and
certificate verification. In 5.1.5 its legacy handler simulates success and must
not be used to protect documents. Containment is tracked in #640 and actual
recipient encryption in #642.
