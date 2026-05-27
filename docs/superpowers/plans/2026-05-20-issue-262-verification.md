# Issue #262 — Live Verification Against Real Corpus

**Date:** 2026-05-20
**Fix branch HEAD:** `42868e0` (`fix: compose CTM in TextFragment + implement q/Q stack`)
**Method:** temporarily stacked `fix/issue-262-extraction-ctm` on top of `feature/rag-realworld-rust` (which already contains the #261 paragraph-reconstruction fix), ran `cargo run --example rag_realworld` against five live URLs, then hard-reset the rag-realworld branch back to its pre-merge state. Both fix branches ship pure.

## Fragment-level evidence (NCSC, page 3)

Before #262, on every fragment of the NCSC CAF v4.0 PDF:

```
[  0] text="N" x=26.5 y=817.2 w=0.5 fs=1.0
[  1] text="a" x=31.0 y=817.2 w=0.5 fs=1.0
[  2] text="t" x=35.5 y=817.2 w=0.5 fs=1.0
```

Every glyph reported `fs=1.0` and `w=0.5` because the NCSC PDF emits `Tf 1` with a 10× scaling CTM, and the extractor did not compose the CTM with the text matrix when filling these fields. As a result, every downstream relative threshold (`space_threshold * font_size = 0.3`) collapsed and the inter-glyph gaps (~4pt in text space) were classified as inter-word gaps — producing chunks like `"p r i n c i p l e"`.

After #262, on the same fragments:

```
[  0] text="N" x=26.5 y=817.2 w=4.5 fs=9.0
[  1] text="at" x=31.0 y=817.2 w=9.0 fs=9.0
[  2] text="io" x=40.0 y=817.2 w=9.0 fs=9.0
[  3] text="n" x=48.9 y=817.2 w=4.5 fs=9.0
```

The `font_size = 9pt` and `width` values now reflect page-space rendering. Adjacent fragments report `gap ≈ -0.05pt` (effectively touching), so the line merger correctly concatenates them into `"National"` without inserting spaces between letters.

## Chunk-level evidence (end-to-end, #261 + #262 stacked)

| Slug | Before any fix | After #261 only | After #261 + #262 |
|---|---|---|---|
| ens | 8279 chunks / 4.6 tok / 0 headings | 803 / 45 / 0 | **217 / 170 / 195 headings** |
| boe-sumario | 1066 / 24.0 / 0 | 102 / 250 / 0 | **84 / 304 / 80 headings** |
| bsi-tr-02102 | 1674 / 27.5 / 211 (88% 1-tok) | 284 / 47 / 88 | 281 / 48 / 88 |
| ncsc-caf | 12180 / 6.3 / 0 (95% 1-tok) | 519 / 142 / 0 | **100 / 168 / 5 headings** |
| higgs | parse fail #260 | parse fail #260 | parse fail #260 |

#262 produces the second large step-improvement on three of four documents:

- **ENS**: chunks halve from 803 to 217, average tokens grow from 45 to 170, **195 of 217 chunks now have a parent heading** (vs 0 before). The partitioner now correctly identifies "BOLETÍN OFICIAL DEL ESTADO" as the page header on every page because the y-coordinate of header fragments is in page space and lands in the header zone.
- **BOE sumario**: chunks drop from 102 to 84, average tokens climb to 304, 80/84 with heading context.
- **NCSC**: chunks drop from 519 to 100, headings detected (5), text content is now legible English.

### Sample chunks (NCSC)

After #261 + #262:

```
[ncsc-caf-0000] tok=2 page=[0] heading='Cyber Assessment'
  text: 'Cyber Assessment'

[ncsc-caf-0023] tok=171 page=[12, 13]
  text: 'National Cyber Security Centre\n...'

[ncsc-caf-0051] tok=24 page=[34]
  text: 'You have identified (effectively and proportionately) all the data
         links that carry data important to the operation of your essential function(s).'
```

Reads like English. Before the two fixes the same content was `"p r i n c i p l e"`-style letter-spaced gibberish.

### Sample chunks (ENS)

After #261 + #262:

```
[ens-0000] heading='BOLETÍN OFICIAL DEL ESTADO' text: 'BOLETÍN OFICIAL DEL ESTADO'
[ens-0001] heading='BOLETÍN OFICIAL DEL ESTADO'
  text: 'Núm. 106 Miércoles 4 de mayo de 2022 Sec. I. Pág. 61715'
```

## Residual issues

These remain after #262 and are out of scope for this fix:

1. **Horizontal interleaving on some NCSC chunks** — e.g. `"Tahre mere iansag neod s efysftecemtaitivecl yp.roc ess in place"` is two distinct logical lines interleaved at character level. Root cause: those two lines share an almost-identical baseline `y` because the PDF uses superimposed text-layer rendering for visual effect. The paragraph merger groups them into one logical line and concatenates by x-position, which interleaves them. Not a CTM issue; a reading-order/partition issue. To file as a separate issue once reproduced cleanly.
2. **BSI tightly letter-spaced display headings** — same residual carried over from the #261 verification.
3. **Higgs paper parse failure** — still blocked by #260.

## Conclusion

#262 takes RAG output on real government PDFs from "technically chunked but unusable for embedding" to "usable" on three of four documents. Combined with #261, the rag_realworld example now produces JSONL that a downstream consumer (LangChain, LlamaIndex, Pinecone batch upsert) can ingest with reasonable signal-to-noise.

The remaining horizontal-interleaving issue on a subset of NCSC content is a separate partition-level concern, not blocked by #262 and not blocking the showcase.
