# Choosing oxidize-pdf

oxidize-pdf is a Rust library for generating PDFs, extracting content and applying
supported document operations. The [claims inventory](CLAIMS.md) records checked
versions, evidence and limits. Evaluate your actual documents before adoption.

## Supported workflows and boundaries

| Workflow | Available capability | Boundary |
|---|---|---|
| Generate invoices and reports | Text, graphics, images and tables through `Document` and `Page` | See the measured synthetic workloads below; validate performance for your actual documents. |
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

PAdES is outside the scope of oxidize-pdf. Anyone can implement it as an
independent external extension using the existing signing interfaces,
without incorporating it into this library. The extension author is responsible
for implementing and validating the chosen profile, including timestamps and
long-term validation data where required. Embedding CMS alone does not establish
profile compliance or certification.

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

## Measured generation performance

The new `pdf-generation-v1` measurements provide evidence instead of the previous
unverified throughput figures. For oxidize-pdf 5.1.5 the median is **6,311 one-page
documents/sec** or **732 ten-page reports/sec**, on Intel i7-3770, Linux and Rust
1.96.0 release builds. Each case contains twenty samples; one-page samples generate
1,000 separate documents, and ten-page samples generate 100 reports.

The [full comparison](reports/2026-09-28-generation-performance.md) includes lopdf,
pdf-writer, printpdf and pdf_oxide 0.3.78, measured together in a new run. All four
have higher median throughput here; the one-page uncertainty intervals for
oxidize-pdf and pdf_oxide overlap, so median ordering alone is not conclusive. It
records latency, process RSS, output size, versions and measurement scope.
Defaults differ by adapter; no general superiority or real-world throughput is
inferred. Structure, page count and exact text are independently checked on one
representative PDF per sample.

PDFSharp/QuestPDF/IronPDF comparisons, zero-leak guarantees, deployment sizes,
ROI and product-price claims remain unsupported by this experiment. They must
not be inferred from these generation measurements.

Start with the [README](../README.md) and repository examples. Record the input
cohort, version, protocol, denominator and observed outcomes before making an
adoption or performance claim. An empty monitoring baseline is not a calibrated
probability of adoption.
