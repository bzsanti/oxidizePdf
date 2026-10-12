use oxidize_pdf::parser::{PdfDictionary, PdfName, PdfObject, PdfReader, PdfString};
use oxidize_pdf::signatures::{
    prepare_document_timestamp, prepare_incremental_signature, CertificationPermission,
    DocumentTimestampOptions, SignaturePreparationOptions,
};
use oxidize_pdf::writer::IncrementalObjectEditor;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;
use std::process::Command;

fn base(stream: bool) -> Vec<u8> {
    let mut doc = Document::new();
    doc.enable_xref_streams(stream);
    doc.add_page(Page::a4());
    doc.to_bytes().unwrap()
}

// Only an envelope probe, deliberately not a cryptographic token.
const DER: &[u8] = b"\x30\x03\x02\x01\x01";
fn text(value: &str) -> PdfObject {
    PdfObject::String(PdfString(value.as_bytes().to_vec()))
}

#[test]
fn catalog_and_object_updates_preserve_revisions_and_unrelated_entries() {
    for stream in [false, true] {
        let original = base(stream);
        assert_eq!(
            IncrementalObjectEditor::new(&original)
                .unwrap()
                .finish()
                .unwrap(),
            original
        );
        let mut edit = IncrementalObjectEditor::new(&original).unwrap();
        let pages = edit.catalog().get("Pages").unwrap().clone();
        let id = edit.allocate_id().unwrap();
        edit.replace(id, text("first evidence")).unwrap();
        edit.set_catalog_entry("Evidence", PdfObject::Reference(id.0, id.1))
            .unwrap();
        edit.set_catalog_entry("UnrelatedExtension", text("keep me"))
            .unwrap();
        let first = edit.finish().unwrap();
        assert!(first.starts_with(&original));
        let mut edit = IncrementalObjectEditor::new(&first).unwrap();
        edit.replace(id, text("second evidence")).unwrap();
        edit.set_catalog_entry("NewExtension", PdfObject::Integer(17))
            .unwrap();
        let second = edit.finish().unwrap();
        assert!(second.starts_with(&first));
        let mut reader = PdfReader::new(Cursor::new(&second)).unwrap();
        assert_eq!(
            reader.get_object(id.0, id.1).unwrap(),
            &text("second evidence")
        );
        let catalog = reader.catalog().unwrap();
        assert_eq!(catalog.get("Pages"), Some(&pages));
        assert_eq!(catalog.get("UnrelatedExtension"), Some(&text("keep me")));
        assert_eq!(
            catalog.get("Evidence"),
            Some(&PdfObject::Reference(id.0, id.1))
        );
        assert_eq!(catalog.get("NewExtension"), Some(&PdfObject::Integer(17)));
        let mut edit = IncrementalObjectEditor::new(&second).unwrap();
        edit.remove_catalog_entry("NewExtension").unwrap();
        let third = edit.finish().unwrap();
        assert!(third.starts_with(&second));
        assert!(!PdfReader::new(Cursor::new(third))
            .unwrap()
            .catalog()
            .unwrap()
            .contains_key("NewExtension"));
    }
}

#[test]
fn rejects_invalid_references_unfilled_reservations_and_structural_overrides() {
    let source = base(false);
    let mut edit = IncrementalObjectEditor::new(&source).unwrap();
    let id = edit.allocate_id().unwrap();
    assert!(edit.replace((id.0, 1), text("wrong generation")).is_err());
    assert!(edit.replace((0, 65535), PdfObject::Null).is_err());
    assert!(edit
        .set_catalog_entry("Unknown", PdfObject::Reference(u32::MAX, 0))
        .is_err());
    assert!(edit
        .set_catalog_entry("NaN", PdfObject::Real(f64::NAN))
        .is_err());
    assert!(edit
        .set_catalog_entry("Type", PdfObject::Name(PdfName("Pages".into())))
        .is_err());
    assert!(edit.remove_catalog_entry("Pages").is_err());
    assert!(edit
        .set_catalog_entry("Perms", PdfObject::Dictionary(PdfDictionary::new()))
        .is_err());
    assert!(edit.finish().is_err());
    let mut edit = IncrementalObjectEditor::new(&source).unwrap();
    let id = edit.allocate_id().unwrap();
    edit.replace(id, text("first")).unwrap();
    assert!(edit.replace(id, text("duplicate")).is_err());
    let result = edit.finish().unwrap();
    assert_eq!(
        PdfReader::new(Cursor::new(&result))
            .unwrap()
            .get_object(id.0, id.1)
            .unwrap(),
        &text("first")
    );
}

#[test]
fn generic_updates_deny_all_certification_levels_and_timestamp_obeys_signing_policy() {
    for permission in [
        CertificationPermission::NoChanges,
        CertificationPermission::FormFillAndSign,
        CertificationPermission::FormFillSignAndAnnotate,
    ] {
        let mut options = SignaturePreparationOptions::invisible("certification");
        options.certification = Some(permission);
        let signed = prepare_incremental_signature(&base(false), &options)
            .unwrap()
            .finalize(DER)
            .unwrap();
        assert!(matches!(
            IncrementalObjectEditor::new(&signed),
            Err(oxidize_pdf::PdfError::PermissionDenied(_))
        ));
        let timestamp =
            prepare_document_timestamp(&signed, &DocumentTimestampOptions::invisible("timestamp"));
        assert_eq!(
            timestamp.is_ok(),
            permission != CertificationPermission::NoChanges
        );
        if let Ok(timestamp) = timestamp {
            assert!(timestamp.finalize(DER).unwrap().starts_with(&signed));
        }
    }
    let source = include_bytes!("fixtures/signatures/docmdp_p2_rsa.pdf");
    assert!(matches!(
        IncrementalObjectEditor::new(source),
        Err(oxidize_pdf::PdfError::PermissionDenied(_))
    ));
}

#[test]
fn timestamp_type_subfilter_and_byte_range_are_engine_owned() {
    for stream in [false, true] {
        let original = base(stream);
        let signed = prepare_incremental_signature(
            &original,
            &SignaturePreparationOptions::invisible("approval"),
        )
        .unwrap()
        .finalize(DER)
        .unwrap();
        let prepared =
            prepare_document_timestamp(&signed, &DocumentTimestampOptions::invisible("timestamp"))
                .unwrap();
        assert!(prepared.prepared_pdf().starts_with(&signed));
        let range = prepared.byte_range().ranges().to_vec();
        assert_eq!(range.len(), 2);
        assert_eq!(range[0].0, 0);
        assert_eq!(
            range[1].0 + range[1].1,
            prepared.prepared_pdf().len() as u64
        );
        let expected: Vec<u8> = range
            .iter()
            .flat_map(|&(s, n)| {
                prepared.prepared_pdf()[s as usize..(s + n) as usize]
                    .iter()
                    .copied()
            })
            .collect();
        assert_eq!(prepared.bytes_to_digest(), expected);
        let gap = range[0].1 as usize..range[1].0 as usize;
        assert_eq!(prepared.prepared_pdf()[gap.start], b'<');
        assert_eq!(prepared.prepared_pdf()[gap.end - 1], b'>');
        assert!(prepared.clone().finalize(b"not DER").is_err());
        let output = prepared.finalize(DER).unwrap();
        let final_covered: Vec<u8> = range
            .iter()
            .flat_map(|&(s, n)| output[s as usize..(s + n) as usize].iter().copied())
            .collect();
        assert_eq!(final_covered, expected);
        let mut reader = PdfReader::new(Cursor::new(&output)).unwrap();
        let fields = reader.signatures().unwrap();
        assert_eq!(fields.len(), 2);
        let timestamp = fields
            .iter()
            .find(|f| f.name.as_deref() == Some("timestamp"))
            .unwrap();
        assert_eq!(timestamp.sub_filter.as_deref(), Some("ETSI.RFC3161"));
        assert!(output[signed.len()..]
            .windows(b"/Type /DocTimeStamp".len())
            .any(|w| w == b"/Type /DocTimeStamp"));
    }
}

#[test]
fn generic_update_preserves_approval_signature_dictionary() {
    let signed = prepare_incremental_signature(
        &base(false),
        &SignaturePreparationOptions::invisible("approval"),
    )
    .unwrap()
    .finalize(DER)
    .unwrap();
    let original_fields = PdfReader::new(Cursor::new(&signed))
        .unwrap()
        .signatures()
        .unwrap();
    let mut edit = IncrementalObjectEditor::new(&signed).unwrap();
    edit.set_catalog_entry("Evidence", text("external evidence"))
        .unwrap();
    let output = edit.finish().unwrap();
    assert!(output.starts_with(&signed));
    let fields = PdfReader::new(Cursor::new(&output))
        .unwrap()
        .signatures()
        .unwrap();
    assert_eq!(
        fields[0].byte_range.ranges(),
        original_fields[0].byte_range.ranges()
    );
    assert_eq!(fields[0].contents, original_fields[0].contents);
}

fn generated_pdf_with_generations() -> Vec<u8> {
    let objects = [
        (
            1,
            7,
            "<< /Type /Catalog /Pages 2 3 R /Evidence 4 9 R /Unrelated 42 >>",
        ),
        (2, 3, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (
            3,
            0,
            "<< /Type /Page /Parent 2 3 R /MediaBox [0 0 200 200] >>",
        ),
        (4, 9, "(old)"),
    ];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut entries = Vec::new();
    for (n, g, body) in objects {
        entries.push((bytes.len(), g));
        bytes.extend_from_slice(format!("{n} {g} obj\n{body}\nendobj\n").as_bytes());
    }
    let offset = bytes.len();
    bytes.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for (off, g) in entries {
        bytes.extend_from_slice(format!("{off:010} {g:05} n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size 5 /Root 1 7 R >>\nstartxref\n{offset}\n%%EOF\n").as_bytes(),
    );
    bytes
}

#[test]
fn replaces_nonzero_generations_without_renumbering_catalog_or_references() {
    let source = generated_pdf_with_generations();
    let mut edit = IncrementalObjectEditor::new(&source).unwrap();
    assert!(edit.replace((4, 0), text("bad")).is_err());
    edit.replace((4, 9), text("new")).unwrap();
    edit.set_catalog_entry("Added", PdfObject::Boolean(true))
        .unwrap();
    let output = edit.finish().unwrap();
    assert!(output.starts_with(&source));
    let mut reader = PdfReader::new(Cursor::new(output)).unwrap();
    assert_eq!(reader.trailer().root().unwrap(), (1, 7));
    assert_eq!(reader.get_object(4, 9).unwrap(), &text("new"));
    assert_eq!(
        reader.catalog().unwrap().get("Pages"),
        Some(&PdfObject::Reference(2, 3))
    );
    assert_eq!(
        reader.catalog().unwrap().get("Evidence"),
        Some(&PdfObject::Reference(4, 9))
    );
}

#[test]
#[ignore = "requires qpdf; run explicitly in interoperability CI"]
fn compressed_objects_and_encryption_are_checked_by_independent_consumer() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let compressed = dir.path().join("compressed.pdf");
    std::fs::write(&input, base(false)).unwrap();
    assert!(Command::new("qpdf")
        .arg("--object-streams=generate")
        .arg(&input)
        .arg(&compressed)
        .status()
        .unwrap()
        .success());
    let source = std::fs::read(&compressed).unwrap();
    assert!(source.windows(7).any(|w| w == b"/ObjStm"));
    let mut reader = PdfReader::new(Cursor::new(&source)).unwrap();
    let root = reader.trailer().root().unwrap();
    let old_catalog = reader.catalog().unwrap().clone();
    let mut edit = IncrementalObjectEditor::new(&source).unwrap();
    edit.set_catalog_entry("Added", text("compressed root survives"))
        .unwrap();
    let updated = edit.finish().unwrap();
    let stamped = prepare_document_timestamp(&updated, &DocumentTimestampOptions::invisible("ts"))
        .unwrap()
        .finalize(DER)
        .unwrap();
    assert!(stamped.starts_with(&source));
    let mut reader = PdfReader::new(Cursor::new(&stamped)).unwrap();
    assert_eq!(reader.trailer().root().unwrap(), root);
    assert_eq!(
        reader.catalog().unwrap().get("Pages"),
        old_catalog.get("Pages")
    );
    let out = dir.path().join("result.pdf");
    std::fs::write(&out, stamped).unwrap();
    assert!(Command::new("qpdf")
        .arg("--check")
        .arg(&out)
        .status()
        .unwrap()
        .success());
    let encrypted = dir.path().join("encrypted.pdf");
    assert!(Command::new("qpdf")
        .args(["--encrypt", "user", "owner", "256", "--"])
        .arg(&input)
        .arg(&encrypted)
        .status()
        .unwrap()
        .success());
    let encrypted = std::fs::read(encrypted).unwrap();
    assert!(IncrementalObjectEditor::new(&encrypted).is_err());
    assert!(
        prepare_document_timestamp(&encrypted, &DocumentTimestampOptions::invisible("ts")).is_err()
    );
}

#[test]
#[ignore = "requires openssl and qpdf; run explicitly in interoperability CI"]
fn real_external_timestamp_token_covers_final_pdf_and_survives_later_revision() {
    let dir = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        let output = Command::new("openssl")
            .args(args)
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "openssl {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    run(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        "key.pem",
        "-out",
        "cert.pem",
        "-subj",
        "/CN=Test Timestamp Authority",
        "-days",
        "1",
        "-addext",
        "extendedKeyUsage=critical,timeStamping",
        "-addext",
        "keyUsage=critical,digitalSignature",
    ]);
    std::fs::write(dir.path().join("serial"), "01\n").unwrap();
    std::fs::write(dir.path().join("tsa.cnf"),"[tsa]\ndefault_tsa = test\n[test]\nserial = serial\ncrypto_device = builtin\nsigner_cert = cert.pem\ncerts = cert.pem\nsigner_key = key.pem\nsigner_digest = sha256\ndefault_policy = 1.2.3.4\ndigests = sha256\naccuracy = secs:1\nordering = yes\ntsa_name = yes\ness_cert_id_chain = no\ness_cert_id_alg = sha256\n").unwrap();
    for stream in [false, true] {
        let source = base(stream);
        let prepared =
            prepare_document_timestamp(&source, &DocumentTimestampOptions::invisible("timestamp"))
                .unwrap();
        std::fs::write(dir.path().join("covered.bin"), prepared.bytes_to_digest()).unwrap();
        run(&[
            "ts",
            "-query",
            "-data",
            "covered.bin",
            "-sha256",
            "-cert",
            "-out",
            "request.tsq",
        ]);
        run(&[
            "ts",
            "-reply",
            "-config",
            "tsa.cnf",
            "-queryfile",
            "request.tsq",
            "-token_out",
            "-out",
            "token.der",
        ]);
        let token = std::fs::read(dir.path().join("token.der")).unwrap();
        let output = prepared.finalize(&token).unwrap();
        assert!(output.starts_with(&source));
        let mut edit = IncrementalObjectEditor::new(&output).unwrap();
        edit.set_catalog_entry("ExternalEvidence", text("caller-owned"))
            .unwrap();
        let final_pdf = edit.finish().unwrap();
        assert!(final_pdf.starts_with(&output));
        let fields = PdfReader::new(Cursor::new(&final_pdf))
            .unwrap()
            .signatures()
            .unwrap();
        let covered: Vec<u8> = fields[0]
            .byte_range
            .ranges()
            .iter()
            .flat_map(|&(s, n)| final_pdf[s as usize..(s + n) as usize].iter().copied())
            .collect();
        std::fs::write(dir.path().join("covered.bin"), &covered).unwrap();
        assert_eq!(&fields[0].contents[..token.len()], token);
        run(&[
            "ts",
            "-verify",
            "-token_in",
            "-in",
            "token.der",
            "-data",
            "covered.bin",
            "-CAfile",
            "cert.pem",
        ]);
        let mut altered = covered;
        altered[0] ^= 1;
        std::fs::write(dir.path().join("altered.bin"), altered).unwrap();
        assert!(!Command::new("openssl")
            .args([
                "ts",
                "-verify",
                "-token_in",
                "-in",
                "token.der",
                "-data",
                "altered.bin",
                "-CAfile",
                "cert.pem"
            ])
            .current_dir(dir.path())
            .output()
            .unwrap()
            .status
            .success());
        std::fs::write(dir.path().join("result.pdf"), final_pdf).unwrap();
        assert!(Command::new("qpdf")
            .args(["--check", "result.pdf"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success());
    }
}

#[test]
fn timestamp_existing_field_and_fieldmdp_boundaries() {
    use oxidize_pdf::parser::PdfArray;
    use oxidize_pdf::signatures::{FieldLock, SignatureRect, SignatureTarget};
    let source = base(false);
    let mut edit = IncrementalObjectEditor::new(&source).unwrap();
    let field_id = edit.allocate_id().unwrap();
    let mut field = PdfDictionary::new();
    field.insert("FT".into(), PdfObject::Name(PdfName("Sig".into())));
    field.insert("Subtype".into(), PdfObject::Name(PdfName("Widget".into())));
    field.insert("T".into(), text("reserved"));
    field.insert(
        "Rect".into(),
        PdfObject::Array(PdfArray(vec![PdfObject::Integer(0); 4])),
    );
    edit.replace(field_id, PdfObject::Dictionary(field))
        .unwrap();
    let mut form = PdfDictionary::new();
    form.insert(
        "Fields".into(),
        PdfObject::Array(PdfArray(vec![PdfObject::Reference(field_id.0, field_id.1)])),
    );
    edit.set_catalog_entry("AcroForm", PdfObject::Dictionary(form))
        .unwrap();
    let source = edit.finish().unwrap();
    let timestamp =
        prepare_document_timestamp(&source, &DocumentTimestampOptions::existing("reserved"))
            .unwrap()
            .finalize(DER)
            .unwrap();
    assert!(timestamp.starts_with(&source));
    assert!(prepare_document_timestamp(
        &timestamp,
        &DocumentTimestampOptions::existing("reserved")
    )
    .is_err());
    let mut signed_options = SignaturePreparationOptions::invisible("lock");
    signed_options.field_lock = Some(FieldLock::Include(vec!["reserved".into()]));
    let locked = prepare_incremental_signature(&source, &signed_options)
        .unwrap()
        .finalize(DER)
        .unwrap();
    assert!(
        prepare_document_timestamp(&locked, &DocumentTimestampOptions::existing("reserved"))
            .is_err()
    );
    assert!(matches!(
        IncrementalObjectEditor::new(&locked),
        Err(oxidize_pdf::PdfError::PermissionDenied(_))
    ));
    let mut options = DocumentTimestampOptions::invisible("visible");
    options.target = SignatureTarget::New {
        field_name: "visible".into(),
        page_index: 0,
        rect: Some(SignatureRect {
            left: 0.0,
            bottom: 0.0,
            right: 100.0,
            top: 30.0,
        }),
    };
    assert!(prepare_document_timestamp(&source, &options).is_err());
    let mut options = DocumentTimestampOptions::invisible("small");
    options.placeholder_bytes = 1;
    assert!(prepare_document_timestamp(&source, &options).is_err());
}

#[test]
fn rejects_orphan_certification_and_signature_replacements() {
    let mut options = SignaturePreparationOptions::invisible("certification");
    options.certification = Some(CertificationPermission::FormFillAndSign);
    let mut certified = prepare_incremental_signature(&base(false), &options)
        .unwrap()
        .finalize(DER)
        .unwrap();
    let offset = certified.windows(6).position(|w| w == b"/Perms").unwrap();
    certified[offset + 1] = b'X';
    assert!(matches!(
        IncrementalObjectEditor::new(&certified),
        Err(oxidize_pdf::PdfError::InvalidStructure(_))
    ));
    let signed = prepare_incremental_signature(
        &base(false),
        &SignaturePreparationOptions::invisible("approval"),
    )
    .unwrap()
    .finalize(DER)
    .unwrap();
    let mut reader = PdfReader::new(Cursor::new(&signed)).unwrap();
    let size = reader.trailer().size().unwrap();
    let signature = (1..size)
        .find(|&id| {
            reader
                .get_object(id, 0)
                .ok()
                .and_then(PdfObject::as_dict)
                .is_some_and(|d| d.get_type() == Some("Sig"))
        })
        .unwrap();
    let mut edit = IncrementalObjectEditor::new(&signed).unwrap();
    assert!(edit.replace((signature, 0), PdfObject::Null).is_err());
    assert_eq!(edit.finish().unwrap(), signed);
}

#[cfg(feature = "signatures")]
#[test]
fn real_approval_signature_remains_cryptographically_valid() {
    use oxidize_pdf::signatures::{parse_pkcs7_signature_detailed, verify_signature_detailed};
    let source = include_bytes!("fixtures/signatures/signed_rsa.pdf");
    let mut edit = IncrementalObjectEditor::new(source).unwrap();
    edit.set_catalog_entry("Evidence", text("external evidence"))
        .unwrap();
    let updated = edit.finish().unwrap();
    let stamped =
        prepare_document_timestamp(&updated, &DocumentTimestampOptions::invisible("timestamp"))
            .unwrap()
            .finalize(DER)
            .unwrap();
    for bytes in [source.as_slice(), updated.as_slice(), stamped.as_slice()] {
        let signatures = PdfReader::new(Cursor::new(bytes))
            .unwrap()
            .signatures()
            .unwrap();
        let parsed = parse_pkcs7_signature_detailed(&signatures[0].contents).unwrap();
        let result = verify_signature_detailed(bytes, &parsed, &signatures[0].byte_range).unwrap();
        assert!(result.hash_valid && result.signature_valid, "{result:?}");
    }
}

#[test]
fn timestamp_preserves_direct_and_indirect_catalog_extensions() {
    for indirect in [false, true] {
        let source = base(false);
        let mut edit = IncrementalObjectEditor::new(&source).unwrap();
        let mut vendor = PdfDictionary::new();
        vendor.insert("BaseVersion".into(), PdfObject::Name(PdfName("1.7".into())));
        vendor.insert("ExtensionLevel".into(), PdfObject::Integer(8));
        let mut extensions = PdfDictionary::new();
        extensions.insert("ADBE".into(), PdfObject::Dictionary(vendor));
        extensions.insert("PrivateVendor".into(), text("preserved"));
        let expected = PdfObject::Dictionary(extensions);
        let entry = if indirect {
            let id = edit.allocate_id().unwrap();
            edit.replace(id, expected.clone()).unwrap();
            PdfObject::Reference(id.0, id.1)
        } else {
            expected.clone()
        };
        edit.set_catalog_entry("Extensions", entry.clone()).unwrap();
        let source = edit.finish().unwrap();
        let output =
            prepare_document_timestamp(&source, &DocumentTimestampOptions::invisible("timestamp"))
                .unwrap()
                .finalize(DER)
                .unwrap();
        let mut reader = PdfReader::new(Cursor::new(output)).unwrap();
        let actual = reader.catalog().unwrap().get("Extensions").unwrap().clone();
        assert_eq!(actual, entry);
        let actual = match actual {
            PdfObject::Reference(n, g) => reader.get_object(n, g).unwrap().clone(),
            value => value,
        };
        assert_eq!(actual, expected);
    }
}
