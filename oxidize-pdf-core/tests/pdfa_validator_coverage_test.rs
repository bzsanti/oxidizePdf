//! Behaviour coverage for the PDF/A validation engine (`pdfa::validator`).
//!
//! The pre-existing `pdfa_integration_test.rs` only feeds a single clean
//! `Document` + `PdfWriter` PDF, which returns early from every deep check, so
//! the `<R: Read + Seek>` methods (transparency, LZW, fonts, colour spaces,
//! JavaScript, external references, embedded files) were never exercised. These
//! tests hand-build PDFs whose object graph triggers each specific violation and
//! assert the exact `ValidationError` variant — plus a negative control per
//! group proving the validator does **not** flag conforming input.

#[path = "common/mod.rs"]
mod common;

use common::pdf_assembler::{assemble_pdf_with_version, stream_obj};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::pdfa::{PdfALevel, PdfAValidator, ValidationError};
use std::io::Cursor;

/// Default content stream body (a no-op text block, Flate-free).
fn plain_contents() -> Vec<u8> {
    stream_obj("", b"BT ET")
}

/// Assemble a one-page PDF with full control over catalog extras, page
/// resources, the content stream body, and any trailing objects (5, 6, ...).
///
/// Layout: 1=Catalog, 2=Pages, 3=Page, 4=Contents, then `extra_objs`.
fn doc(
    version: &str,
    catalog_extra: &str,
    resources: &str,
    contents_body: Vec<u8>,
    extra_objs: &[Vec<u8>],
) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = vec![
        format!("<< /Type /Catalog /Pages 2 0 R {} >>", catalog_extra).into_bytes(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
             /Resources << {} >> /Contents 4 0 R >>",
            resources
        )
        .into_bytes(),
        contents_body,
    ];
    objects.extend_from_slice(extra_objs);
    assemble_pdf_with_version(version, &objects)
}

/// Run the validator and return its error list, panicking if the fixture is not
/// parseable or validation itself errors (both indicate a broken fixture).
fn errors_of(pdf: &[u8], level: PdfALevel) -> Vec<ValidationError> {
    validate_result(pdf, level).expect("validation must not raise a parse error")
}

/// Run the validator and return the raw `Result`, so tests can assert that a
/// broken object graph (e.g. a dangling reference) surfaces as a parse error
/// rather than being silently accepted.
fn validate_result(
    pdf: &[u8],
    level: PdfALevel,
) -> Result<Vec<ValidationError>, oxidize_pdf::pdfa::PdfAError> {
    let mut reader = PdfReader::new(Cursor::new(pdf.to_vec())).expect("fixture must be parseable");
    PdfAValidator::new(level)
        .validate(&mut reader)
        .map(|r| r.errors().to_vec())
}

fn has_version_err(errs: &[ValidationError]) -> bool {
    errs.iter()
        .any(|e| matches!(e, ValidationError::IncompatiblePdfVersion { .. }))
}

// ---------------------------------------------------------------------------
// A. PDF version compatibility
// ---------------------------------------------------------------------------

#[test]
fn a1b_rejects_pdf_1_7() {
    let pdf = doc("1.7", "", "", plain_contents(), &[]);
    let errs = errors_of(&pdf, PdfALevel::A1b);
    assert!(
        errs.iter().any(|e| matches!(
            e,
            ValidationError::IncompatiblePdfVersion { actual, .. } if actual.starts_with("1.7")
        )),
        "PDF/A-1 requires version 1.4; 1.7 must be flagged. got: {:?}",
        errs
    );
}

#[test]
fn a1b_accepts_pdf_1_4_version() {
    let pdf = doc("1.4", "", "", plain_contents(), &[]);
    assert!(
        !has_version_err(&errors_of(&pdf, PdfALevel::A1b)),
        "PDF 1.4 is the exact version PDF/A-1 requires; no version error expected"
    );
}

#[test]
fn a2b_accepts_pdf_1_7_version() {
    let pdf = doc("1.7", "", "", plain_contents(), &[]);
    assert!(
        !has_version_err(&errors_of(&pdf, PdfALevel::A2b)),
        "PDF/A-2 permits 1.4..=1.7; 1.7 must not be flagged"
    );
}

#[test]
fn a2b_rejects_pdf_1_3() {
    let pdf = doc("1.3", "", "", plain_contents(), &[]);
    assert!(
        has_version_err(&errors_of(&pdf, PdfALevel::A2b)),
        "PDF/A-2 lower bound is 1.4; 1.3 must be flagged"
    );
}

// ---------------------------------------------------------------------------
// B. XMP metadata
// ---------------------------------------------------------------------------

/// Metadata stream object carrying the given XMP packet body.
fn metadata_obj(xmp: &str) -> Vec<u8> {
    stream_obj("/Type /Metadata /Subtype /XML", xmp.as_bytes())
}

fn xmp_with(part: &str, conformance: &str) -> String {
    format!(
        "<?xpacket?><x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF>\
         <rdf:Description xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">\
         <pdfaid:part>{}</pdfaid:part>\
         <pdfaid:conformance>{}</pdfaid:conformance>\
         </rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end?>",
        part, conformance
    )
}

#[test]
fn metadata_missing_is_flagged() {
    let pdf = doc("1.4", "", "", plain_contents(), &[]);
    assert!(
        errors_of(&pdf, PdfALevel::A1b)
            .iter()
            .any(|e| matches!(e, ValidationError::XmpMetadataMissing)),
        "a catalog without /Metadata must raise XmpMetadataMissing"
    );
}

#[test]
fn metadata_present_without_pdfa_identifier_is_flagged() {
    let xmp = "<?xpacket?><x:xmpmeta><rdf:RDF><rdf:Description/></rdf:RDF></x:xmpmeta>";
    let pdf = doc(
        "1.4",
        "/Metadata 5 0 R",
        "",
        plain_contents(),
        &[metadata_obj(xmp)],
    );
    let errs = errors_of(&pdf, PdfALevel::A1b);
    assert!(
        errs.iter()
            .any(|e| matches!(e, ValidationError::XmpMissingPdfAIdentifier)),
        "XMP without pdfaid must raise XmpMissingPdfAIdentifier. got: {:?}",
        errs
    );
}

#[test]
fn metadata_part_mismatch_is_flagged() {
    // part 2 declared while validating against PDF/A-1
    let pdf = doc(
        "1.4",
        "/Metadata 5 0 R",
        "",
        plain_contents(),
        &[metadata_obj(&xmp_with("2", "B"))],
    );
    let errs = errors_of(&pdf, PdfALevel::A1b);
    assert!(
        errs.iter().any(|e| matches!(
            e,
            ValidationError::XmpInvalidPdfAIdentifier { details } if details.contains("Part mismatch")
        )),
        "declaring part 2 under A1b must raise a Part mismatch. got: {:?}",
        errs
    );
}

#[test]
fn metadata_conformance_mismatch_is_flagged() {
    // part 1 but conformance A while validating against A1b (conformance B)
    let pdf = doc(
        "1.4",
        "/Metadata 5 0 R",
        "",
        plain_contents(),
        &[metadata_obj(&xmp_with("1", "A"))],
    );
    let errs = errors_of(&pdf, PdfALevel::A1b);
    assert!(
        errs.iter().any(|e| matches!(
            e,
            ValidationError::XmpInvalidPdfAIdentifier { details } if details.contains("Conformance mismatch")
        )),
        "declaring conformance A under A1b must raise a Conformance mismatch. got: {:?}",
        errs
    );
}

#[test]
fn metadata_matching_identifier_passes() {
    let pdf = doc(
        "1.4",
        "/Metadata 5 0 R",
        "",
        plain_contents(),
        &[metadata_obj(&xmp_with("1", "B"))],
    );
    let errs = errors_of(&pdf, PdfALevel::A1b);
    assert!(
        !errs.iter().any(|e| matches!(
            e,
            ValidationError::XmpMetadataMissing
                | ValidationError::XmpMissingPdfAIdentifier
                | ValidationError::XmpInvalidPdfAIdentifier { .. }
        )),
        "a matching part-1 conformance-B identifier must produce no metadata errors. got: {:?}",
        errs
    );
}

#[test]
fn metadata_non_stream_object_is_flagged() {
    // /Metadata points to a plain dictionary, not a stream
    let pdf = doc(
        "1.4",
        "/Metadata 5 0 R",
        "",
        plain_contents(),
        &[b"<< /Type /Metadata >>".to_vec()],
    );
    assert!(
        errors_of(&pdf, PdfALevel::A1b)
            .iter()
            .any(|e| matches!(e, ValidationError::XmpMetadataMissing)),
        "a non-stream /Metadata object must be treated as missing metadata"
    );
}

// ---------------------------------------------------------------------------
// C. JavaScript (forbidden in all levels)
// ---------------------------------------------------------------------------

fn has_js_at(errs: &[ValidationError], loc: &str) -> bool {
    errs.iter()
        .any(|e| matches!(e, ValidationError::JavaScriptForbidden { location } if location == loc))
}

#[test]
fn javascript_in_names_tree_is_flagged() {
    let pdf = doc(
        "1.4",
        "/Names << /JavaScript << /Names [] >> >>",
        "",
        plain_contents(),
        &[],
    );
    assert!(
        has_js_at(&errors_of(&pdf, PdfALevel::A1b), "Names/JavaScript"),
        "a /Names /JavaScript entry must raise JavaScriptForbidden(Names/JavaScript)"
    );
}

#[test]
fn javascript_in_open_action_is_flagged() {
    let pdf = doc(
        "1.4",
        "/OpenAction << /S /JavaScript /JS (app.alert\\(1\\);) >>",
        "",
        plain_contents(),
        &[],
    );
    assert!(
        has_js_at(&errors_of(&pdf, PdfALevel::A1b), "OpenAction"),
        "an /OpenAction with /S /JavaScript must raise JavaScriptForbidden(OpenAction)"
    );
}

#[test]
fn javascript_in_additional_actions_is_flagged() {
    let pdf = doc(
        "1.4",
        "/AA << /WC << /S /JavaScript /JS (close\\(\\);) >> >>",
        "",
        plain_contents(),
        &[],
    );
    assert!(
        has_js_at(&errors_of(&pdf, PdfALevel::A1b), "Catalog/AA"),
        "an /AA dictionary holding a JavaScript action must raise JavaScriptForbidden(Catalog/AA)"
    );
}

#[test]
fn javascript_in_names_tree_via_reference_is_flagged() {
    // /Names is an indirect object resolving to a dict with a JavaScript tree
    let pdf = doc(
        "1.4",
        "/Names 5 0 R",
        "",
        plain_contents(),
        &[b"<< /JavaScript << /Names [] >> >>".to_vec()],
    );
    assert!(
        has_js_at(&errors_of(&pdf, PdfALevel::A1b), "Names/JavaScript"),
        "an indirect /Names dict with /JavaScript must raise JavaScriptForbidden(Names/JavaScript)"
    );
}

#[test]
fn javascript_in_open_action_via_reference_is_flagged() {
    let pdf = doc(
        "1.4",
        "/OpenAction 5 0 R",
        "",
        plain_contents(),
        &[b"<< /S /JavaScript /JS (app.alert\\(1\\);) >>".to_vec()],
    );
    assert!(
        has_js_at(&errors_of(&pdf, PdfALevel::A1b), "OpenAction"),
        "an indirect /OpenAction JavaScript action must raise JavaScriptForbidden(OpenAction)"
    );
}

#[test]
fn additional_actions_via_reference_is_flagged() {
    // /AA is an indirect dict, and the per-event action is itself an indirect ref
    let pdf = doc(
        "1.4",
        "/AA 5 0 R",
        "",
        plain_contents(),
        &[
            b"<< /WC 6 0 R >>".to_vec(),
            b"<< /S /JavaScript /JS (x) >>".to_vec(),
        ],
    );
    assert!(
        has_js_at(&errors_of(&pdf, PdfALevel::A1b), "Catalog/AA"),
        "an indirect /AA holding an indirect JavaScript action must raise JavaScriptForbidden(Catalog/AA)"
    );
}

#[test]
fn additional_actions_without_javascript_is_not_flagged() {
    let pdf = doc(
        "1.4",
        "/AA << /O << /S /GoTo /D [0 /Fit] >> >>",
        "",
        plain_contents(),
        &[],
    );
    assert!(
        !errors_of(&pdf, PdfALevel::A1b)
            .iter()
            .any(|e| matches!(e, ValidationError::JavaScriptForbidden { .. })),
        "an /AA dictionary with only non-JavaScript actions must not be flagged"
    );
}

#[test]
fn open_action_gotoe_is_flagged_external() {
    let pdf = doc(
        "1.4",
        "/OpenAction << /S /GoToE /T << /R /C >> >>",
        "",
        plain_contents(),
        &[],
    );
    assert!(
        errors_of(&pdf, PdfALevel::A1b).iter().any(|e| matches!(
            e,
            ValidationError::ExternalReferenceForbidden { reference_type } if reference_type == "GoToE"
        )),
        "an embedded-go-to /GoToE action must raise ExternalReferenceForbidden(GoToE)"
    );
}

#[test]
fn dangling_metadata_reference_denies_conformance() {
    // /Metadata points to an object number that does not exist. The lenient
    // parser resolves the missing object to an empty placeholder rather than
    // erroring, so the validator must treat it as missing metadata (fail-safe
    // denial) — never a silent pass.
    let pdf = doc("1.4", "/Metadata 99 0 R", "", plain_contents(), &[]);
    let result =
        validate_result(&pdf, PdfALevel::A1b).expect("lenient parse resolves to no stream");
    assert!(
        result
            .iter()
            .any(|e| matches!(e, ValidationError::XmpMetadataMissing)),
        "a dangling /Metadata reference must be treated as missing metadata, not accepted. got: {:?}",
        result
    );
}

#[test]
fn font_resources_via_reference_are_checked() {
    // /Font is an indirect dict; the font entry is itself an indirect ref
    let pdf = doc(
        "1.4",
        "",
        "/Font 5 0 R",
        plain_contents(),
        &[
            b"<< /F1 6 0 R >>".to_vec(),
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        ],
    );
    assert!(
        has_not_embedded(&errors_of(&pdf, PdfALevel::A1b), "F1"),
        "fonts reached through an indirect /Font resource dict must still be checked for embedding"
    );
}

#[test]
fn extgstate_via_reference_is_checked() {
    let pdf = doc(
        "1.4",
        "",
        "/ExtGState 5 0 R",
        plain_contents(),
        &[b"<< /GS1 << /ca 0.5 >> >>".to_vec()],
    );
    assert!(
        has_transparency(&errors_of(&pdf, PdfALevel::A1b)),
        "an indirect /ExtGState resource dict must still be scanned for transparency"
    );
}

#[test]
fn colorspace_dict_via_reference_is_checked() {
    let pdf = doc(
        "1.4",
        "",
        "/ColorSpace 5 0 R",
        plain_contents(),
        &[b"<< /CS0 /DeviceRGB >>".to_vec()],
    );
    assert!(
        has_invalid_cs(&errors_of(&pdf, PdfALevel::A1b)),
        "an indirect /ColorSpace resource dict must still be validated"
    );
}

#[test]
fn colorspace_value_via_reference_is_resolved() {
    // the colour space entry value is an indirect reference to a /DeviceRGB name
    let pdf = doc(
        "1.4",
        "",
        "/ColorSpace << /CS0 5 0 R >>",
        plain_contents(),
        &[b"/DeviceRGB".to_vec()],
    );
    assert!(
        errors_of(&pdf, PdfALevel::A1b).iter().any(|e| matches!(
            e,
            ValidationError::InvalidColorSpace { color_space, .. } if color_space == "DeviceRGB"
        )),
        "a colour-space value given as an indirect reference must be resolved and validated"
    );
}

#[test]
fn output_intents_via_reference_is_resolved() {
    // /OutputIntents is an indirect reference to the array
    let pdf = doc(
        "1.4",
        "/OutputIntents 5 0 R",
        "/ColorSpace << /CS0 /DeviceRGB >>",
        plain_contents(),
        &[
            b"[6 0 R]".to_vec(),
            b"<< /Type /OutputIntent /S /GTS_PDFA1 >>".to_vec(),
        ],
    );
    assert!(
        !has_invalid_cs(&errors_of(&pdf, PdfALevel::A1b)),
        "an OutputIntent reached through an indirect /OutputIntents array must satisfy device colour"
    );
}

#[test]
fn non_javascript_open_action_is_not_flagged_as_js() {
    let pdf = doc(
        "1.4",
        "/OpenAction << /S /GoTo /D [0 /Fit] >>",
        "",
        plain_contents(),
        &[],
    );
    let errs = errors_of(&pdf, PdfALevel::A1b);
    assert!(
        !errs
            .iter()
            .any(|e| matches!(e, ValidationError::JavaScriptForbidden { .. })),
        "an internal /GoTo action is not JavaScript. got: {:?}",
        errs
    );
}

// ---------------------------------------------------------------------------
// D. External references (forbidden in all levels)
// ---------------------------------------------------------------------------

#[test]
fn open_action_gotor_is_flagged_external() {
    let pdf = doc(
        "1.4",
        "/OpenAction << /S /GoToR /F (other.pdf) /D [0 /Fit] >>",
        "",
        plain_contents(),
        &[],
    );
    assert!(
        errors_of(&pdf, PdfALevel::A1b).iter().any(|e| matches!(
            e,
            ValidationError::ExternalReferenceForbidden { reference_type } if reference_type == "GoToR"
        )),
        "a remote /GoToR open action must raise ExternalReferenceForbidden(GoToR)"
    );
}

#[test]
fn open_action_launch_via_reference_is_flagged_external() {
    // OpenAction is an indirect reference to a Launch action
    let pdf = doc(
        "1.4",
        "/OpenAction 5 0 R",
        "",
        plain_contents(),
        &[b"<< /S /Launch /F (calc.exe) >>".to_vec()],
    );
    assert!(
        errors_of(&pdf, PdfALevel::A1b).iter().any(|e| matches!(
            e,
            ValidationError::ExternalReferenceForbidden { reference_type } if reference_type == "Launch"
        )),
        "an indirect /Launch open action must raise ExternalReferenceForbidden(Launch)"
    );
}

// ---------------------------------------------------------------------------
// E. Transparency (forbidden in PDF/A-1, allowed in PDF/A-2+)
// ---------------------------------------------------------------------------

fn has_transparency(errs: &[ValidationError]) -> bool {
    errs.iter()
        .any(|e| matches!(e, ValidationError::TransparencyForbidden { .. }))
}

#[test]
fn extgstate_fill_alpha_below_one_is_transparency() {
    let pdf = doc(
        "1.4",
        "",
        "/ExtGState << /GS1 << /ca 0.5 >> >>",
        plain_contents(),
        &[],
    );
    assert!(
        errors_of(&pdf, PdfALevel::A1b).iter().any(|e| matches!(
            e,
            ValidationError::TransparencyForbidden { location } if location.contains("ExtGState/GS1/ca")
        )),
        "fill alpha (ca) != 1.0 is transparency forbidden in PDF/A-1"
    );
}

#[test]
fn extgstate_stroke_alpha_below_one_is_transparency() {
    let pdf = doc(
        "1.4",
        "",
        "/ExtGState << /GS1 << /CA 0.25 >> >>",
        plain_contents(),
        &[],
    );
    assert!(
        errors_of(&pdf, PdfALevel::A1b).iter().any(|e| matches!(
            e,
            ValidationError::TransparencyForbidden { location } if location.contains("ExtGState/GS1/CA")
        )),
        "stroke alpha (CA) != 1.0 is transparency forbidden in PDF/A-1"
    );
}

#[test]
fn extgstate_soft_mask_is_transparency() {
    let pdf = doc(
        "1.4",
        "",
        "/ExtGState << /GS1 << /SMask << /S /Alpha /G 5 0 R >> >> >>",
        plain_contents(),
        &[stream_obj("/Type /XObject /Subtype /Form", b"")],
    );
    assert!(
        errors_of(&pdf, PdfALevel::A1b).iter().any(|e| matches!(
            e,
            ValidationError::TransparencyForbidden { location } if location.contains("ExtGState/GS1/SMask")
        )),
        "a non-/None SMask in ExtGState is transparency forbidden in PDF/A-1"
    );
}

#[test]
fn extgstate_non_normal_blend_mode_is_transparency() {
    let pdf = doc(
        "1.4",
        "",
        "/ExtGState << /GS1 << /BM /Multiply >> >>",
        plain_contents(),
        &[],
    );
    assert!(
        errors_of(&pdf, PdfALevel::A1b).iter().any(|e| matches!(
            e,
            ValidationError::TransparencyForbidden { location } if location.contains("BM=Multiply")
        )),
        "a non-Normal blend mode is transparency forbidden in PDF/A-1"
    );
}

#[test]
fn xobject_transparency_group_is_flagged() {
    let pdf = doc(
        "1.4",
        "",
        "/XObject << /Fm0 5 0 R >>",
        plain_contents(),
        &[stream_obj(
            "/Type /XObject /Subtype /Form /Group << /S /Transparency >> /BBox [0 0 10 10]",
            b"",
        )],
    );
    assert!(
        errors_of(&pdf, PdfALevel::A1b).iter().any(|e| matches!(
            e,
            ValidationError::TransparencyForbidden { location } if location.contains("transparency group")
        )),
        "a Form XObject with a /Transparency group is forbidden in PDF/A-1"
    );
}

#[test]
fn image_xobject_with_smask_is_flagged() {
    let pdf = doc(
        "1.4",
        "",
        "/XObject << /Im0 5 0 R >>",
        plain_contents(),
        &[stream_obj(
            "/Type /XObject /Subtype /Image /Width 1 /Height 1 /SMask 6 0 R \
             /ColorSpace /DeviceGray /BitsPerComponent 8",
            b"\x00",
        )],
    );
    assert!(
        errors_of(&pdf, PdfALevel::A1b).iter().any(|e| matches!(
            e,
            ValidationError::TransparencyForbidden { location } if location.contains("has SMask")
        )),
        "an Image XObject carrying an /SMask is forbidden in PDF/A-1"
    );
}

#[test]
fn conforming_extgstate_has_no_transparency_error() {
    let pdf = doc(
        "1.4",
        "",
        "/ExtGState << /GS1 << /ca 1.0 /CA 1 /SMask /None /BM /Normal >> >>",
        plain_contents(),
        &[],
    );
    assert!(
        !has_transparency(&errors_of(&pdf, PdfALevel::A1b)),
        "opaque alphas, /SMask /None and /BM /Normal are PDF/A-1 conforming"
    );
}

#[test]
fn transparency_is_allowed_in_pdfa_2() {
    // identical translucent ExtGState, but A2 permits transparency → check skipped
    let pdf = doc(
        "1.7",
        "",
        "/ExtGState << /GS1 << /ca 0.5 >> >>",
        plain_contents(),
        &[],
    );
    assert!(
        !has_transparency(&errors_of(&pdf, PdfALevel::A2b)),
        "PDF/A-2 allows transparency, so ca 0.5 must not be flagged"
    );
}

// ---------------------------------------------------------------------------
// F. LZW compression (forbidden in PDF/A-1, allowed in PDF/A-2+)
// ---------------------------------------------------------------------------

fn has_lzw(errs: &[ValidationError]) -> bool {
    errs.iter()
        .any(|e| matches!(e, ValidationError::LzwCompressionForbidden { .. }))
}

#[test]
fn lzw_in_content_stream_is_flagged() {
    let pdf = doc(
        "1.4",
        "",
        "",
        stream_obj("/Filter /LZWDecode", b"\x80\x0b\x60"),
        &[],
    );
    assert!(
        has_lzw(&errors_of(&pdf, PdfALevel::A1b)),
        "an LZW-filtered content stream is forbidden in PDF/A-1"
    );
}

#[test]
fn lzw_in_xobject_stream_is_flagged() {
    let pdf = doc(
        "1.4",
        "",
        "/XObject << /Im0 5 0 R >>",
        plain_contents(),
        &[stream_obj(
            "/Type /XObject /Subtype /Image /Width 1 /Height 1 \
             /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /LZWDecode",
            b"\x80\x0b\x60",
        )],
    );
    assert!(
        has_lzw(&errors_of(&pdf, PdfALevel::A1b)),
        "an LZW-filtered XObject stream is forbidden in PDF/A-1"
    );
}

#[test]
fn lzw_in_filter_array_is_flagged() {
    let pdf = doc(
        "1.4",
        "",
        "",
        stream_obj("/Filter [/ASCII85Decode /LZWDecode]", b"data~>"),
        &[],
    );
    assert!(
        has_lzw(&errors_of(&pdf, PdfALevel::A1b)),
        "LZWDecode anywhere in a /Filter array is forbidden in PDF/A-1"
    );
}

#[test]
fn flate_content_stream_has_no_lzw_error() {
    let pdf = doc(
        "1.4",
        "",
        "",
        stream_obj("/Filter /FlateDecode", b"\x78\x9c\x03\x00\x00\x00\x00\x01"),
        &[],
    );
    assert!(
        !has_lzw(&errors_of(&pdf, PdfALevel::A1b)),
        "a FlateDecode stream must not be flagged as LZW"
    );
}

#[test]
fn lzw_is_allowed_in_pdfa_2() {
    let pdf = doc(
        "1.7",
        "",
        "",
        stream_obj("/Filter /LZWDecode", b"\x80\x0b\x60"),
        &[],
    );
    assert!(
        !has_lzw(&errors_of(&pdf, PdfALevel::A2b)),
        "PDF/A-2 permits LZW, so an LZW content stream must not be flagged"
    );
}

// ---------------------------------------------------------------------------
// G. Embedded files (forbidden in PDF/A-1 and -2, allowed in -3)
// ---------------------------------------------------------------------------

#[test]
fn embedded_files_forbidden_in_pdfa_1() {
    let pdf = doc(
        "1.4",
        "/Names << /EmbeddedFiles << /Names [] >> >>",
        "",
        plain_contents(),
        &[],
    );
    assert!(
        errors_of(&pdf, PdfALevel::A1b)
            .iter()
            .any(|e| matches!(e, ValidationError::EmbeddedFileForbidden)),
        "a /Names /EmbeddedFiles tree is forbidden in PDF/A-1"
    );
}

#[test]
fn embedded_files_allowed_in_pdfa_3() {
    let pdf = doc(
        "1.7",
        "/Names << /EmbeddedFiles << /Names [] >> >>",
        "",
        plain_contents(),
        &[],
    );
    assert!(
        !errors_of(&pdf, PdfALevel::A3b)
            .iter()
            .any(|e| matches!(e, ValidationError::EmbeddedFileForbidden)),
        "PDF/A-3 permits embedded files"
    );
}

// ---------------------------------------------------------------------------
// H. Fonts
// ---------------------------------------------------------------------------

fn has_not_embedded(errs: &[ValidationError], name: &str) -> bool {
    errs.iter()
        .any(|e| matches!(e, ValidationError::FontNotEmbedded { font_name } if font_name == name))
}

#[test]
fn font_without_descriptor_is_not_embedded() {
    let pdf = doc(
        "1.4",
        "",
        "/Font << /F1 5 0 R >>",
        plain_contents(),
        &[b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec()],
    );
    assert!(
        has_not_embedded(&errors_of(&pdf, PdfALevel::A1b), "F1"),
        "a Type1 font with no FontDescriptor must be reported as not embedded"
    );
}

#[test]
fn font_descriptor_without_fontfile_is_not_embedded() {
    let pdf = doc(
        "1.4",
        "",
        "/Font << /F1 5 0 R >>",
        plain_contents(),
        &[
            b"<< /Type /Font /Subtype /TrueType /BaseFont /Arial /FontDescriptor 6 0 R >>".to_vec(),
            b"<< /Type /FontDescriptor /FontName /Arial /Flags 32 >>".to_vec(),
        ],
    );
    assert!(
        has_not_embedded(&errors_of(&pdf, PdfALevel::A1b), "F1"),
        "a FontDescriptor lacking FontFile/2/3 means the font is not embedded"
    );
}

#[test]
fn font_descriptor_with_fontfile_is_embedded() {
    let pdf = doc(
        "1.4",
        "",
        "/Font << /F1 5 0 R >>",
        plain_contents(),
        &[
            b"<< /Type /Font /Subtype /TrueType /BaseFont /Arial /FontDescriptor 6 0 R >>".to_vec(),
            b"<< /Type /FontDescriptor /FontName /Arial /Flags 32 /FontFile2 7 0 R >>".to_vec(),
            stream_obj("/Length1 4", b"FONT"),
        ],
    );
    assert!(
        !has_not_embedded(&errors_of(&pdf, PdfALevel::A1b), "F1"),
        "a FontDescriptor with FontFile2 means the font is embedded"
    );
}

#[test]
fn type0_font_without_descendants_is_not_embedded() {
    let pdf = doc(
        "1.4",
        "",
        "/Font << /F1 5 0 R >>",
        plain_contents(),
        &[b"<< /Type /Font /Subtype /Type0 /BaseFont /X /Encoding /Identity-H >>".to_vec()],
    );
    assert!(
        has_not_embedded(&errors_of(&pdf, PdfALevel::A1b), "F1"),
        "a Type0 font with no /DescendantFonts must be reported as not embedded"
    );
}

#[test]
fn type0_descendant_without_fontfile_is_not_embedded() {
    let pdf = doc(
        "1.4",
        "",
        "/Font << /F1 5 0 R >>",
        plain_contents(),
        &[
            b"<< /Type /Font /Subtype /Type0 /BaseFont /X /Encoding /Identity-H \
              /DescendantFonts [6 0 R] >>"
                .to_vec(),
            b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /X /FontDescriptor 7 0 R >>".to_vec(),
            b"<< /Type /FontDescriptor /FontName /X /Flags 4 >>".to_vec(),
        ],
    );
    assert!(
        has_not_embedded(&errors_of(&pdf, PdfALevel::A1b), "F1"),
        "a Type0 CIDFont descriptor lacking FontFile means not embedded"
    );
}

#[test]
fn type0_descendant_with_fontfile_is_embedded() {
    let pdf = doc(
        "1.4",
        "",
        "/Font << /F1 5 0 R >>",
        plain_contents(),
        &[
            b"<< /Type /Font /Subtype /Type0 /BaseFont /X /Encoding /Identity-H \
              /DescendantFonts [6 0 R] >>"
                .to_vec(),
            b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /X /FontDescriptor 7 0 R >>".to_vec(),
            b"<< /Type /FontDescriptor /FontName /X /Flags 4 /FontFile2 8 0 R >>".to_vec(),
            stream_obj("/Length1 4", b"FONT"),
        ],
    );
    assert!(
        !has_not_embedded(&errors_of(&pdf, PdfALevel::A1b), "F1"),
        "a Type0 CIDFont descriptor with FontFile2 means embedded"
    );
}

#[test]
fn level_a_type3_without_tounicode_lacks_unicode_mapping() {
    let pdf = doc(
        "1.4",
        "",
        "/Font << /F1 5 0 R >>",
        plain_contents(),
        &[b"<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1 1] \
           /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << >> /Encoding << >> >>"
            .to_vec()],
    );
    assert!(
        errors_of(&pdf, PdfALevel::A1a).iter().any(|e| matches!(
            e,
            ValidationError::FontMissingToUnicode { font_name } if font_name == "F1"
        )),
        "a Type3 font without /ToUnicode fails Level A (accessibility) conformance"
    );
}

#[test]
fn level_a_type0_identity_encoding_without_tounicode_passes() {
    let pdf = doc(
        "1.4",
        "",
        "/Font << /F1 5 0 R >>",
        plain_contents(),
        &[
            b"<< /Type /Font /Subtype /Type0 /BaseFont /X /Encoding /Identity-H \
              /DescendantFonts [6 0 R] >>"
                .to_vec(),
            b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /X /FontDescriptor 7 0 R >>".to_vec(),
            b"<< /Type /FontDescriptor /FontName /X /Flags 4 /FontFile2 8 0 R >>".to_vec(),
            stream_obj("/Length1 4", b"FONT"),
        ],
    );
    assert!(
        !errors_of(&pdf, PdfALevel::A1a)
            .iter()
            .any(|e| matches!(e, ValidationError::FontMissingToUnicode { .. })),
        "Identity-H encoding is an acceptable Unicode mapping for Level A"
    );
}

// ---------------------------------------------------------------------------
// I. Colour spaces / OutputIntent
// ---------------------------------------------------------------------------

fn has_invalid_cs(errs: &[ValidationError]) -> bool {
    errs.iter()
        .any(|e| matches!(e, ValidationError::InvalidColorSpace { .. }))
}

#[test]
fn device_rgb_without_output_intent_is_invalid() {
    let pdf = doc(
        "1.4",
        "",
        "/ColorSpace << /CS0 /DeviceRGB >>",
        plain_contents(),
        &[],
    );
    assert!(
        errors_of(&pdf, PdfALevel::A1b).iter().any(|e| matches!(
            e,
            ValidationError::InvalidColorSpace { color_space, location }
                if color_space == "DeviceRGB" && location.contains("ColorSpace/CS0")
        )),
        "a device-dependent colour space without an OutputIntent is invalid for PDF/A"
    );
}

#[test]
fn device_rgb_with_output_intent_dest_profile_is_valid() {
    let pdf = doc(
        "1.4",
        "/OutputIntents [5 0 R]",
        "/ColorSpace << /CS0 /DeviceRGB >>",
        plain_contents(),
        &[
            b"<< /Type /OutputIntent /S /GTS_PDFA1 /DestOutputProfile 6 0 R >>".to_vec(),
            stream_obj("/N 3", b"ICCPROFILE"),
        ],
    );
    assert!(
        !has_invalid_cs(&errors_of(&pdf, PdfALevel::A1b)),
        "DeviceRGB is permitted when a valid OutputIntent (DestOutputProfile) is present"
    );
}

#[test]
fn output_intent_pdfa_subtype_satisfies_device_colour() {
    // OutputIntent identified only by its /S subtype containing PDFA
    let pdf = doc(
        "1.4",
        "/OutputIntents [5 0 R]",
        "/ColorSpace << /CS0 /DeviceCMYK >>",
        plain_contents(),
        &[b"<< /Type /OutputIntent /S /GTS_PDFA1 >>".to_vec()],
    );
    assert!(
        !has_invalid_cs(&errors_of(&pdf, PdfALevel::A1b)),
        "an OutputIntent whose /S subtype names PDFA satisfies device colour usage"
    );
}

#[test]
fn icc_based_array_colour_space_is_not_device_dependent() {
    let pdf = doc(
        "1.4",
        "",
        "/ColorSpace << /CS0 [/ICCBased 5 0 R] >>",
        plain_contents(),
        &[stream_obj("/N 3", b"ICCPROFILE")],
    );
    assert!(
        !has_invalid_cs(&errors_of(&pdf, PdfALevel::A1b)),
        "an ICCBased colour space is device-independent and must not be flagged"
    );
}

#[test]
fn image_xobject_device_cmyk_without_output_intent_is_invalid() {
    let pdf = doc(
        "1.4",
        "",
        "/XObject << /Im0 5 0 R >>",
        plain_contents(),
        &[stream_obj(
            "/Type /XObject /Subtype /Image /Width 1 /Height 1 \
             /ColorSpace /DeviceCMYK /BitsPerComponent 8",
            b"\x00\x00\x00\x00",
        )],
    );
    assert!(
        errors_of(&pdf, PdfALevel::A1b).iter().any(|e| matches!(
            e,
            ValidationError::InvalidColorSpace { color_space, location }
                if color_space == "DeviceCMYK" && location.contains("XObject/Im0")
        )),
        "a DeviceCMYK image without an OutputIntent is invalid for PDF/A"
    );
}
