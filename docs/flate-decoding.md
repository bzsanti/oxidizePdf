# Flate decoding integrity

The stream APIs `decode_stream`, `decode_stream_with_limit`, `PdfStream::decode`
and the document decoding/content-stream methods require a complete zlib stream
for FlateDecode. A valid empty compressed stream returns empty bytes. Corruption,
truncation (including a missing checksum) and checksum errors return
`ParseError::StreamDecodeError`; failed decoding is never reported as `Ok([])`.

This contract applies to strict, tolerant and skip-error parsing options.
Those APIs return only bytes, so they cannot truthfully signal partial or
recovered content. They no longer guess raw DEFLATE/gzip encodings, skip damaged
headers, truncate footers, ignore checksum failures or substitute compressed
input for decoded bytes when DecodeParms is present. No implicit tolerant Flate
recovery is retained. Structural PDF recovery and content-operator parsing have
their own policies; this change does not make them universally strict.

As before, decoding a zlib member stops at its verified end; bytes after that
member are not decoded. A second concatenated member is not concatenated into
the result. This is not recovery of a damaged member.

Both Flate paths enforce the 256 MiB output cap. The bounded API additionally
enforces the caller's smaller limit. Neither a size nor a compression-ratio
failure triggers fallback. Small highly compressible images remain supported;
the existing ratio guard applies above 64 MiB at ratios greater than 1000:1.
Limit failures remain `StreamDecodeError` with a size-limit or compression-ratio
diagnostic, distinct from corruption and incomplete-stream diagnostics.

Page-content decoding and text extraction propagate these errors. Applications
must handle them explicitly and must not infer that a failed page is empty.
Some damaged files that previously appeared to decode successfully will now
fail. This is an intentional correction of silent data loss, not a guarantee
that all extraction paths or other filters reject every malformed PDF.

Predictor postprocessing and its error policy are tracked separately in
[#638](https://github.com/bzsanti/oxidizePdf/issues/638); #637 only prevents
DecodeParms from bypassing Flate integrity checks. No predictor-validation
guarantee is added by this change.

## Explicit recovery of damaged input

Recovery is opt-in through a different return type:

- `filters::decode_stream_with_recovery(data, dict, options, max_bytes)` and
  `PdfDocument::decode_stream_with_recovery(stream, max_bytes)` return
  `RecoveredStream { data, diagnostics }`. `Unverified` means DEFLATE reached
  its end but the zlib integrity checks did not pass. `Incomplete` means only
  a prefix was decoded. Both carry the original error and filter index.
- `TextExtractor::extract_from_page_with_recovery` returns `RecoveredText`
  with the page index, text and diagnostics identifying each Contents index
  or Form name/object. `PdfDocument::extract_text_with_recovery` collects the
  same results across pages, returning an error if any page fails fatally.
- If a damaged Flate stream yields no bytes, stream recovery returns
  `StreamRecoveryErrorKind::InvalidFlate`. Explicit text recovery may omit it
  and continue with healthy content, but records `TextRecoveryAction::Omitted`.
  An empty recovered page with an omission is not a verified empty page.
- Resource limits never permit fallback or omission. Predictor and other
  filter errors propagate. For Flate, size/ratio failures have the typed
  `ResourceLimit` classification; errors from other bounded filters remain
  `Other` and also propagate.

The implementation retries raw DEFLATE at the payload of a syntactically valid
zlib header (or from byte zero for raw input). It does not guess offsets, gzip,
preset dictionaries or predictors. Recovery is limited to content/Forms during
text extraction; it does not repair the PDF structure or promise recovery of
fonts, images or compressed object dictionaries. The per-stream bound applies
to decoded content/Forms, not total document memory. All Flate attempts retain
the hard 256 MiB cap and the ratio guard.

Adjacent verified Contents streams retain their shared operand state. A
recovered or omitted stream breaks that continuity so healthy bytes on either
side cannot be combined into an invented operator. Graphics/text state may
still be uncertain after missing content; diagnostics accompany the result.

Inspect diagnostics before using recovered text. Empty diagnostics mean no
Flate recovery was needed, **not** that every PDF structure, content operator,
font or other filter has been validated. `ExtractedText::truncated` separately
reports the configured text-output limit; it is not the Flate recovery signal.

```rust,no_run
# use oxidize_pdf::parser::{PdfDocument, PdfReader};
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let doc = PdfDocument::new(PdfReader::open("damaged.pdf")?);
for page in doc.extract_text_with_recovery(16 * 1024 * 1024)? {
    if !page.diagnostics.is_empty() {
        eprintln!("Page {} has recovery limitations: {:?}",
                  page.page_index, page.diagnostics);
    }
    println!("{}", page.text.text);
}
# Ok(())
# }
```

## Corpus measurement

T3 records strict success and explicit recovery separately in
`t3-explicit-recovery.json`. The 90% gate measures availability under the
explicit recovery contract; it does not assert 90% verified integrity. The
reading-order gates use that same explicit path and log every recovery/omission.
Their historical baselines already included implicit recovery; numerical rate,
file-count and content-coverage limits are unchanged. This protocol change was
authorized after independently checking damaged input streams (#637).
