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

## PAdES extension

PAdES is outside the scope of oxidize-pdf. Anyone can implement it as an
independent external extension using the existing signing interfaces,
without incorporating it into this library. The extension author is responsible
for implementing and validating the chosen profile, including timestamps and
long-term validation data where required. Embedding CMS alone does not establish
profile compliance or certification.

Certificate-based **recipient encryption** is different from signing and
certificate verification. The legacy handler now rejects
cryptographic operations explicitly (#640); it does not protect documents.
Actual recipient encryption is tracked separately in #642.

## Participant signature slots (5.2.0, #646)

`create_signature_slot` appends an unsigned root signature widget with portable
application metadata. `list_signature_slots` and `read_signature_slot` inspect
these widgets; `remove_signature_slot` removes an unsigned, uncompleted slot.
Each operation preserves the original PDF bytes as an incremental prefix.
Encrypted inputs and prohibited DocMDP/FieldMDP changes are rejected.

Use `complete_signature_slot(..., true)` for handwriting-only completion. It
creates a visible appearance and marks the slot read-only; it does **not** create
a CMS signature. For combined handwriting and cryptographic signing, call
`draw_signature_slot(..., false, label)` (or `complete_signature_slot(..., false)`),
then `prepare_incremental_signature` with
`SignaturePreparationOptions::existing(field_name)`, sign the covered bytes with
your external signer, and finalize. The existing appearance is preserved.

A slot's `digitally_signed` flag reports a non-null `/V` entry, not successful
cryptographic verification. Validate the resulting signature separately.

Slot names are 1–128 ASCII alphanumeric/hyphen bytes, metadata is UTF-8 up to
4096 bytes, and at most 100 slots can be prepared. Rectangles use unrotated PDF
page coordinates within the crop/media box; appearance rotation supports
0/90/180/270 degrees. Strokes use normalized coordinates in a 3:1 input pad,
with at most 1000 strokes and 20,000 points. Optional labels must fit at 8 pt
and be representable in WinAnsi. This API does not manage participants' identity,
consent, private keys, or a signing service.
