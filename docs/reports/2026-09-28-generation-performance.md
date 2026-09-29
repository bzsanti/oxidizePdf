# PDF generation performance

Synthetic text-only workload on one shared host; defaults differ in compression/metadata. Batch-average p95 is not individual-request p95. Bootstrap describes run-level samples, not other hardware or real workloads. No PDFSharp, QuestPDF or IronPDF comparison, leak, image/font or deployment-size claim.

| Library | Version | Workload | Documents/s median | Batch-average ms/doc p95 | Process peak MiB | Bytes/doc |
|---|---|---|---:|---:|---:|---:|
| oxidize-pdf | 5.1.5 | invoice-1p | 6311.4 | 0.197 | 5.0 | 5015 |
| oxidize-pdf | 5.1.5 | report-10p | 732.4 | 1.587 | 5.4 | 20452 |
| lopdf | 0.44.0 | invoice-1p | 25780.3 | 0.047 | 3.6 | 2124 |
| lopdf | 0.44.0 | report-10p | 2999.0 | 0.472 | 3.8 | 17649 |
| pdf-writer | 0.15.0 | invoice-1p | 116236.9 | 0.015 | 3.4 | 2206 |
| pdf-writer | 0.15.0 | report-10p | 13064.2 | 0.134 | 3.5 | 18924 |
| printpdf | 0.12.7 | invoice-1p | 8707.0 | 0.175 | 4.4 | 1551 |
| printpdf | 0.12.7 | report-10p | 957.8 | 1.445 | 4.9 | 7307 |
| pdf_oxide | 0.3.78 | invoice-1p | 6630.9 | 0.215 | 4.2 | 3816 |
| pdf_oxide | 0.3.78 | report-10p | 923.2 | 1.494 | 4.5 | 22781 |

Measured 2026-09-28 on Intel Core i7-3770 @ 3.40 GHz, Linux x86_64,
Rust 1.96.0, release build, shared host. Twenty samples per engine/workload;
1,000 one-page documents or 100 ten-page documents per sample. All five engines
were rerun together in shuffled complete blocks: 200 samples, 110,000 timed
documents and 200,000 timed pages. Each child warms up with three documents.
Adapter default serialization/compression differs. Timing includes construction,
serialization to memory and buffer destruction, excluding filesystem IO.
One representative PDF per sample passed qpdf and exact pdftotext page/text checks.
These are synthetic text-only layouts, not image-heavy invoices or a real corpus.
All adapters share one harness binary; process RSS is not standalone app size.

pdf_oxide 0.3.78 is the unmodified registry release, with default features disabled.
Its office_oxide dependency is pinned to 0.1.9, matching the lockfile shipped in
that release: a fresh resolution to 0.1.12 failed to compile (missing defined_names
in DocumentIR). The failed build log and successful pinned check are retained.
A preliminary offline attempt stopped before measurement because a dependency
was not cached; it is not included in these samples.

These results are a new run, not an optimization claim against the earlier
four-engine run. Shared-host load, cohort order and the dependency/binary identity
changed. Do not pool the populations. In the one-page workload the descriptive
bootstrap intervals for oxidize-pdf and pdf_oxide overlap; their median ordering
alone is not evidence of a statistically established performance difference.

Reproduction in the oxidize-stats workspace:

```sh
python3 benchmark/scripts/generation_benchmark.py --offline \
  --output benchmark/results/generation/NEW-RUN --samples 20
```

Implementation: benchmark/generation/{Cargo.toml,Cargo.lock,src/main.rs} and
benchmark/scripts/generation_benchmark.py. Full protocol: docs/generation-performance.md.
Manifest, raw samples, source snapshots and 200 representative PDFs are preserved
under benchmark/results/generation/2026-09-28-v1-five-engines-attempt-2 in that
workspace. The earlier four-engine run remains unchanged at 2026-09-28-v1.
The [reproduction archive](2026-09-28-generation-evidence.tar.gz) contains the
benchmark source, pinned dependencies, protocol, both completed runs and build
evidence. Extract it and run the reproduction command from its root.

Archive SHA-256: `0bec09dd09556dc8faaf9e3038f13652c5ed12ed46331df9eb9b8605c6507519`.

Higher documents/second is better; lower milliseconds/document is better.
pdf-writer achieves 18.42× / 17.84× our median throughput in these two workloads.
pdf_oxide achieves about 5% / 26% more throughput; one-page descriptive intervals
overlap, ten-page intervals do not. This does not establish causes or generalize
to parsing/extraction. The pdf_oxide comparison was requested out of curiosity:
**beating pdf_oxide is not a project objective or acceptance criterion**.

Manifest SHA-256: `9eebf7cfd8d2fe744ade650414a6be44d595b23a2926308ba1db8dc24f016d78`.

Raw samples SHA-256: `b8a807daaa073a10a6008d30d9ad786bdfbe2fc11f538d831ccb4d079e9b4f09`.
