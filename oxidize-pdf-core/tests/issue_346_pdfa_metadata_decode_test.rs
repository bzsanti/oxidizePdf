//! Issue #346: `PdfAValidator` reads the **raw** `/Metadata` stream bytes
//! instead of the **decoded** bytes, so a PDF/A document whose XMP metadata
//! stream is `/Filter /FlateDecode`-compressed is wrongly reported as
//! non-conformant (`XmpMetadataMissing` / `XmpMissingPdfAIdentifier`), even
//! though the same document with an uncompressed `/Metadata` stream validates.
//!
//! Fixtures: `issue_346_compressed_xmp.pdf` and `issue_346_uncompressed_xmp.pdf`
//! are the two PDF/A-3b documents attached by the reporter to
//! <https://github.com/bzsanti/oxidizePdf/issues/346>; they differ *only* in
//! whether the `/Metadata` XMP stream is FlateDecode-compressed. Compressing the
//! metadata stream must not change the validation outcome.
//! `issue_346_unsupported_filter_xmp.pdf` is the compressed fixture with its
//! `/Metadata` filter rewritten to an unsupported name, to exercise the
//! decode-failure branch (the stream cannot be decoded → metadata unreadable).

use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::pdfa::{PdfALevel, PdfAValidator, ValidationError};

const COMPRESSED: &str = "tests/fixtures/issue_346_compressed_xmp.pdf";
const UNCOMPRESSED: &str = "tests/fixtures/issue_346_uncompressed_xmp.pdf";
const UNSUPPORTED_FILTER: &str = "tests/fixtures/issue_346_unsupported_filter_xmp.pdf";

fn validate(path: &str) -> Vec<ValidationError> {
    let mut reader = PdfReader::open(path).expect("open fixture");
    PdfAValidator::new(PdfALevel::A3b)
        .validate(&mut reader)
        .expect("validate")
        .errors()
        .to_vec()
}

#[test]
fn flate_compressed_metadata_validates_like_uncompressed() {
    let compressed = validate(COMPRESSED);
    let uncompressed = validate(UNCOMPRESSED);

    // Compressing the /Metadata stream is a representation detail; it must not
    // change which validation errors are reported. Before the fix the validator
    // reads the raw (still Flate-compressed) bytes for the compressed document,
    // so it diverges from the uncompressed one with a spurious XMP error.
    assert_eq!(
        compressed, uncompressed,
        "FlateDecode /Metadata changed the validation result: compressed={compressed:?} \
         uncompressed={uncompressed:?}"
    );
}

#[test]
fn flate_compressed_metadata_does_not_spuriously_report_missing_xmp() {
    let errors = validate(COMPRESSED);

    // The XMP packet is present and carries a valid PDF/A identifier — it is
    // merely Flate-compressed. Neither "missing metadata" nor "missing
    // identifier" must be reported once the stream is decoded before parsing.
    assert!(
        !errors.contains(&ValidationError::XmpMetadataMissing),
        "spurious XmpMetadataMissing on a FlateDecode XMP stream: {errors:?}"
    );
    assert!(
        !errors.contains(&ValidationError::XmpMissingPdfAIdentifier),
        "spurious XmpMissingPdfAIdentifier on a FlateDecode XMP stream: {errors:?}"
    );
}

#[test]
fn undecodable_metadata_stream_reports_missing_without_erroring() {
    // The /Metadata stream declares a filter that cannot be applied, so its XMP
    // packet is genuinely unreadable. The validator must classify this as
    // XmpMetadataMissing and still return Ok — not propagate a parse error nor
    // panic. This covers the decode-failure branch of check_metadata_from_data.
    let mut reader = PdfReader::open(UNSUPPORTED_FILTER).expect("open fixture");
    let result = PdfAValidator::new(PdfALevel::A3b)
        .validate(&mut reader)
        .expect("validate must return Ok even when the metadata stream is undecodable");

    assert!(
        result
            .errors()
            .contains(&ValidationError::XmpMetadataMissing),
        "an undecodable /Metadata stream must report XmpMetadataMissing, got: {:?}",
        result.errors()
    );
}
