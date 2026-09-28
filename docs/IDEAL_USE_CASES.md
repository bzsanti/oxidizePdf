# Choosing oxidize-pdf

oxidize-pdf is a Rust library for generating PDFs, extracting content and applying
supported document operations. The [claims inventory](CLAIMS.md) records checked
versions, evidence and limits. Evaluate your actual documents before adoption.

## Supported workflows and boundaries

| Workflow | Available capability | Boundary |
|---|---|---|
| Generate invoices and reports | Text, graphics, images and tables through `Document` and `Page` | Measure throughput, memory and deployment size for your workload; no comparison with other libraries is established here. |
| Rust backend and AI/RAG ingestion | Native extraction and structured chunks with page references | Complex layouts, malformed streams and scanned content require evaluation; see the known filter defects in CLAIMS. |
| Existing-document operations | Split, merge, rotate and selected incremental updates | Reconstruction is not a lossless round trip. Check each operation's preservation report and permission handling. |
| Searchable scanned documents | Incremental OCR layers from supplied recognition results | Recognition and rendering are separate dependencies; there is no built-in page rasterizer. |
| Archival inspection | PDF/A conformance validation for eight levels | PDF/A authoring or certification, remediation and legal assurance are not provided. |
| Basic signatures | Detection, PKCS#7 verification, certificate validation and incremental preparation | Read the operation-level contract below; preparation is not a completed cryptographic signature. |

## Signatures and enterprise scope

PDF/A conformance validation, signature detection, PKCS#7
verification, certificate validation and an incremental-signature preparation
API are available. With caller-supplied CMS, finalization embeds that container
in the prepared revision. The [signature contract](signatures.md) documents
trust anchors, revocation results, failure states and configuration hooks.

Advanced PAdES is outside the open-source core scope and reserved for commercial
extensions. This policy does not assert that a commercial implementation exists,
or that preparing/embedding CMS satisfies a complete PAdES profile.

Managed-key, certificate-issuing or signing-service workflows are not provided.
Neither are PDF/UA compliance, archival certification or managed enterprise
support. Certificate-based recipient encryption is a separate, unsupported
workflow in 5.1.5; its public handler has known simulated-success defects (#640).
Do not use that handler to protect documents.

## Deployment and evaluation

Use the [dependency architecture](architecture/no-native-dependencies.md) for
supported Rust configurations and the optional Tesseract integration. Rust's
memory-safety model does not guarantee zero leaks, zero crashes or safe processing
of every hostile PDF. Apply resource limits and evaluate failure behavior.

The previous throughput comparisons, fuzzing comparisons, container sizes, ROI
figures and alternative-product prices have been removed: this document had no
versioned protocol and artifacts supporting them. They are not adoption evidence.
The README's historical single-library timings identify their baseline and are
not comparative results or current-workload guarantees.

Start with the [README](../README.md) and repository examples. Record the input
cohort, version, protocol, denominator and observed outcomes before making an
adoption or performance claim. An empty monitoring baseline is not a calibrated
probability of adoption.
