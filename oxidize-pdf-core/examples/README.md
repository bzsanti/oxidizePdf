# oxidize-pdf examples

Runnable examples for the `oxidize-pdf` crate. Run any of them with:

```bash
cargo run --example <name>
```

Some examples need a feature flag (noted below). The full list lives in this
directory; this README highlights the RAG / chunk-extraction path, which is the
most common starting point.

## RAG chunk extraction

### `rag_pipeline`

The one-liner API end to end: open a PDF, call `rag_chunks()`, iterate the
chunks and their metadata.

```bash
cargo run --example rag_pipeline -- path/to/document.pdf
```

```rust
use oxidize_pdf::parser::{PdfDocument, PdfReader};

let doc = PdfDocument::new(PdfReader::open("document.pdf")?);
let chunks = doc.rag_chunks()?; // default: 512-token target, heading propagation on

for chunk in &chunks {
    // chunk.full_text       — heading context prepended; embed this
    // chunk.heading_context — nearest enclosing section heading
    // chunk.page_numbers    — for citation / visual grounding
    // chunk.element_types   — "paragraph", "title", "list_item", "table", ...
    // chunk.token_estimate  — word-count proxy (see note below)
    // chunk.is_oversized    — a single element exceeded the token budget
}
```

### `rag_realworld`

Downloads five real government and academic PDFs (cached after the first run),
chunks each with the default pipeline, and writes RAG-ready JSONL to `./out/`.

```bash
cargo run --example rag_realworld
# → ./corpus_cache/<sha1>.pdf   (downloaded inputs, kept across runs)
# → ./out/<slug>.jsonl          (one chunk per line, RAG-ready)
```

A committed sample of the output shape (clean Spanish government text) lives at
[`sample_output/rag_chunks_sample.jsonl`](sample_output/rag_chunks_sample.jsonl)
so you can see a chunk without running the example or downloading anything:

```json
{
  "id": "ens-0077",
  "text": "El marco organizativo está constituido por un conjunto de medidas relacionadas con \nla organización global de la seguridad.",
  "metadata": {
    "page_numbers": [31],
    "heading_context": "3. Marco organizativo [ORG]",
    "element_types": ["paragraph"],
    "token_estimate": 18,
    "is_oversized": false
  }
}
```

The `heading_context` is inherited from the nearest enclosing section heading
even when that heading appears earlier in the document — this is what lets a
retriever disambiguate near-identical opening sentences from different sections.

## Extraction fidelity — read this before trusting the text

Chunking is only as good as the text extraction underneath it, and that quality
**varies with the source layout and fonts**:

- **Clean single-column text** (e.g. the BOE Spanish government documents in
  `rag_realworld`) extracts faithfully.
- **Dense two-column scientific PDFs** (e.g. the ATLAS Higgs paper) can reorder
  fragments within a line where subscripts/superscripts shift the baseline, and
  the inter-word space heuristic can both drop and insert spurious spaces.
- **PDFs with typographic quotes/dingbats** can surface `U+FFFD` replacement
  characters where the font lacks a ToUnicode mapping.

These are tracked in
[issue #302](https://github.com/bzsanti/oxidizePdf/issues/302). The pipeline is
robust (no document aborts the batch), but if you hit garbled text, filing an
issue with the source PDF attached is the most useful thing you can do.

## Notes

- `token_estimate` is a word-count proxy. BPE/WordPiece tokenisers produce
  roughly 1.3–1.7× more sub-word tokens than raw words, so the default
  `max_tokens: 512` lands around ~300–390 actual model tokens. Tune with
  `rag_chunks_with(HybridChunkConfig { max_tokens: 256, ..Default::default() })`.
- For multi-column reading order, `rag_chunks_with_profile(ExtractionProfile::Rag)`
  applies XY-Cut. Note it does **not** fix the sub/superscript reordering above
  (see #302).
