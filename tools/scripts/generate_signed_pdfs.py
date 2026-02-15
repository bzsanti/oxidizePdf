#!/usr/bin/env python3
"""
Generate signed PDF test fixtures for oxidize-pdf signature verification tests.

This script creates various signed PDFs for testing:
1. Self-signed certificate (basic case)
2. PAdES signatures (ETSI.CAdES.detached)
3. Multiple signatures
4. Edge cases (expired, modified after signing)

Requirements:
    pip install pyhanko[crypto] reportlab
"""

import os
import datetime
from io import BytesIO

# Certificate generation
from cryptography import x509
from cryptography.x509.oid import NameOID
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.hazmat.backends import default_backend

# PDF generation
from reportlab.lib.pagesizes import letter
from reportlab.pdfgen import canvas

# PDF signing
from pyhanko.sign import signers, fields
from pyhanko.pdf_utils.incremental_writer import IncrementalPdfFileWriter


OUTPUT_DIR = os.path.join(
    os.path.dirname(__file__),
    "../../oxidize-pdf-core/tests/fixtures/signatures"
)


def generate_key_pair():
    """Generate RSA key pair for testing."""
    private_key = rsa.generate_private_key(
        public_exponent=65537,
        key_size=2048,
        backend=default_backend()
    )
    return private_key


def create_self_signed_cert(
    private_key,
    common_name="Test Signer",
    valid_from=None,
    valid_to=None,
    is_ca=False,
    key_usage_digital_signature=True,
):
    """Create a self-signed X.509 certificate."""
    if valid_from is None:
        valid_from = datetime.datetime.now(datetime.timezone.utc)
    if valid_to is None:
        valid_to = valid_from + datetime.timedelta(days=365)

    subject = issuer = x509.Name([
        x509.NameAttribute(NameOID.COUNTRY_NAME, "US"),
        x509.NameAttribute(NameOID.STATE_OR_PROVINCE_NAME, "California"),
        x509.NameAttribute(NameOID.LOCALITY_NAME, "San Francisco"),
        x509.NameAttribute(NameOID.ORGANIZATION_NAME, "Test Organization"),
        x509.NameAttribute(NameOID.COMMON_NAME, common_name),
    ])

    builder = x509.CertificateBuilder().subject_name(
        subject
    ).issuer_name(
        issuer
    ).public_key(
        private_key.public_key()
    ).serial_number(
        x509.random_serial_number()
    ).not_valid_before(
        valid_from
    ).not_valid_after(
        valid_to
    )

    # Add key usage extension
    if key_usage_digital_signature:
        builder = builder.add_extension(
            x509.KeyUsage(
                digital_signature=True,
                content_commitment=True,  # Non-repudiation
                key_encipherment=False,
                data_encipherment=False,
                key_agreement=False,
                key_cert_sign=is_ca,
                crl_sign=is_ca,
                encipher_only=False,
                decipher_only=False,
            ),
            critical=True,
        )

    # Add basic constraints for CA
    builder = builder.add_extension(
        x509.BasicConstraints(ca=is_ca, path_length=0 if is_ca else None),
        critical=True,
    )

    # Sign the certificate
    certificate = builder.sign(private_key, hashes.SHA256(), default_backend())
    return certificate


def create_simple_pdf(content="This is a test PDF document."):
    """Create a simple PDF document for signing."""
    buffer = BytesIO()
    c = canvas.Canvas(buffer, pagesize=letter)
    c.setFont("Helvetica", 12)
    c.drawString(72, 720, content)
    c.drawString(72, 700, "This PDF is for digital signature testing.")
    c.drawString(72, 680, f"Created: {datetime.datetime.now().isoformat()}")
    c.save()
    buffer.seek(0)
    return buffer.read()


def sign_pdf_simple(pdf_bytes, private_key, certificate, field_name="Signature1"):
    """Sign a PDF with a simple PKCS#7 detached signature."""
    # Create signer
    signer = signers.SimpleSigner(
        signing_cert=certificate,
        signing_key=private_key,
        cert_registry=None,
    )

    # Read PDF
    pdf_writer = IncrementalPdfFileWriter(BytesIO(pdf_bytes))

    # Add signature field
    fields.append_signature_field(
        pdf_writer,
        sig_field_spec=fields.SigFieldSpec(
            sig_field_name=field_name,
            box=(72, 72, 300, 120),
        )
    )

    # Sign
    meta = signers.PdfSignatureMetadata(
        field_name=field_name,
        reason="Testing signature verification",
        location="Test Environment",
        name=certificate.subject.get_attributes_for_oid(NameOID.COMMON_NAME)[0].value,
    )

    output = BytesIO()
    signers.sign_pdf(
        pdf_writer,
        meta,
        signer=signer,
        output=output,
    )
    output.seek(0)
    return output.read()


def sign_pdf_pades(pdf_bytes, private_key, certificate, field_name="Signature1"):
    """Sign a PDF with PAdES (ETSI.CAdES.detached) signature."""
    signer = signers.SimpleSigner(
        signing_cert=certificate,
        signing_key=private_key,
        cert_registry=None,
    )

    pdf_writer = IncrementalPdfFileWriter(BytesIO(pdf_bytes))

    fields.append_signature_field(
        pdf_writer,
        sig_field_spec=fields.SigFieldSpec(
            sig_field_name=field_name,
            box=(72, 72, 300, 120),
        )
    )

    # Use PAdES signature subfilter
    meta = signers.PdfSignatureMetadata(
        field_name=field_name,
        reason="Testing PAdES signature verification",
        location="Test Environment",
        name=certificate.subject.get_attributes_for_oid(NameOID.COMMON_NAME)[0].value,
        subfilter=fields.SigSeedSubFilter.PADES,
    )

    output = BytesIO()
    signers.sign_pdf(
        pdf_writer,
        meta,
        signer=signer,
        output=output,
    )
    output.seek(0)
    return output.read()


def modify_pdf_after_signing(signed_pdf_bytes):
    """Add an incremental update to a signed PDF (simulates modification)."""
    from pyhanko.pdf_utils import generic

    pdf_writer = IncrementalPdfFileWriter(BytesIO(signed_pdf_bytes))

    # Get the first page
    root = pdf_writer.root
    pages = root.raw_get('/Pages').get_object()
    first_page_ref = pages['/Kids'][0]
    first_page = first_page_ref.get_object()

    # Add an annotation (comment)
    annot_dict = generic.DictionaryObject({
        generic.pdf_name('/Type'): generic.pdf_name('/Annot'),
        generic.pdf_name('/Subtype'): generic.pdf_name('/Text'),
        generic.pdf_name('/Rect'): generic.ArrayObject([
            generic.FloatObject(400),
            generic.FloatObject(700),
            generic.FloatObject(450),
            generic.FloatObject(750),
        ]),
        generic.pdf_name('/Contents'): generic.pdf_string('Added after signing'),
        generic.pdf_name('/Open'): generic.BooleanObject(False),
    })

    annot_ref = pdf_writer.add_object(annot_dict)

    # Add to page annotations
    if '/Annots' in first_page:
        annots = first_page.raw_get('/Annots')
        if isinstance(annots, generic.IndirectObject):
            annots = annots.get_object()
        annots.append(annot_ref)
    else:
        first_page[generic.pdf_name('/Annots')] = generic.ArrayObject([annot_ref])

    pdf_writer.update_container(first_page_ref)

    output = BytesIO()
    pdf_writer.write(output)
    output.seek(0)
    return output.read()


def save_certificate(cert, path):
    """Save certificate to PEM file."""
    with open(path, "wb") as f:
        f.write(cert.public_bytes(serialization.Encoding.PEM))


def save_private_key(key, path):
    """Save private key to PEM file (unencrypted, for testing only)."""
    with open(path, "wb") as f:
        f.write(key.private_bytes(
            encoding=serialization.Encoding.PEM,
            format=serialization.PrivateFormat.PKCS8,
            encryption_algorithm=serialization.NoEncryption(),
        ))


def main():
    """Generate all test fixtures."""
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    print(f"Generating signed PDF fixtures in: {OUTPUT_DIR}")
    print()

    # 1. Generate keys and certificates
    print("1. Generating certificates...")

    # Valid self-signed certificate
    valid_key = generate_key_pair()
    valid_cert = create_self_signed_cert(valid_key, common_name="Valid Test Signer")
    save_certificate(valid_cert, os.path.join(OUTPUT_DIR, "valid_cert.pem"))
    save_private_key(valid_key, os.path.join(OUTPUT_DIR, "valid_key.pem"))
    print("   - Valid certificate (365 days)")

    # Expired certificate
    expired_key = generate_key_pair()
    expired_cert = create_self_signed_cert(
        expired_key,
        common_name="Expired Test Signer",
        valid_from=datetime.datetime(2020, 1, 1, tzinfo=datetime.timezone.utc),
        valid_to=datetime.datetime(2021, 1, 1, tzinfo=datetime.timezone.utc),
    )
    save_certificate(expired_cert, os.path.join(OUTPUT_DIR, "expired_cert.pem"))
    print("   - Expired certificate (2020-2021)")

    # Certificate without digital signature key usage
    no_sig_key = generate_key_pair()
    no_sig_cert = create_self_signed_cert(
        no_sig_key,
        common_name="No Signature Key Usage",
        key_usage_digital_signature=False,
    )
    save_certificate(no_sig_cert, os.path.join(OUTPUT_DIR, "no_sig_usage_cert.pem"))
    print("   - Certificate without digital signature key usage")

    print()

    # 2. Generate base PDF
    print("2. Creating base PDF...")
    base_pdf = create_simple_pdf()
    base_path = os.path.join(OUTPUT_DIR, "unsigned.pdf")
    with open(base_path, "wb") as f:
        f.write(base_pdf)
    print(f"   - {base_path}")
    print()

    # 3. Generate signed PDFs
    print("3. Generating signed PDFs...")

    # 3.1 Simple PKCS#7 detached signature (self-signed)
    try:
        signed_simple = sign_pdf_simple(base_pdf, valid_key, valid_cert)
        path = os.path.join(OUTPUT_DIR, "signed_pkcs7_selfsigned.pdf")
        with open(path, "wb") as f:
            f.write(signed_simple)
        print(f"   - {os.path.basename(path)} ({len(signed_simple)} bytes)")
    except Exception as e:
        print(f"   - ERROR signing PKCS#7: {e}")
        signed_simple = None

    # 3.2 PAdES signature (ETSI.CAdES.detached)
    try:
        signed_pades = sign_pdf_pades(base_pdf, valid_key, valid_cert, "PAdESSignature")
        path = os.path.join(OUTPUT_DIR, "signed_pades_selfsigned.pdf")
        with open(path, "wb") as f:
            f.write(signed_pades)
        print(f"   - {os.path.basename(path)} ({len(signed_pades)} bytes)")
    except Exception as e:
        print(f"   - ERROR signing PAdES: {e}")

    # 3.3 Signature with expired certificate
    try:
        signed_expired = sign_pdf_simple(base_pdf, expired_key, expired_cert, "ExpiredSig")
        path = os.path.join(OUTPUT_DIR, "signed_expired_cert.pdf")
        with open(path, "wb") as f:
            f.write(signed_expired)
        print(f"   - {os.path.basename(path)} ({len(signed_expired)} bytes)")
    except Exception as e:
        print(f"   - ERROR signing with expired cert: {e}")

    # 3.4 Modified after signing (incremental update)
    if signed_simple:
        try:
            modified = modify_pdf_after_signing(signed_simple)
            path = os.path.join(OUTPUT_DIR, "signed_then_modified.pdf")
            with open(path, "wb") as f:
                f.write(modified)
            print(f"   - {os.path.basename(path)} ({len(modified)} bytes)")
        except Exception as e:
            print(f"   - ERROR modifying after signing: {e}")

    # 3.5 Multiple signatures
    if signed_simple:
        try:
            # Add second signature
            second_key = generate_key_pair()
            second_cert = create_self_signed_cert(second_key, common_name="Second Signer")
            multi_signed = sign_pdf_simple(signed_simple, second_key, second_cert, "Signature2")
            path = os.path.join(OUTPUT_DIR, "signed_multiple.pdf")
            with open(path, "wb") as f:
                f.write(multi_signed)
            print(f"   - {os.path.basename(path)} ({len(multi_signed)} bytes)")
            save_certificate(second_cert, os.path.join(OUTPUT_DIR, "second_cert.pem"))
        except Exception as e:
            print(f"   - ERROR adding second signature: {e}")

    print()
    print("Done! Generated test fixtures:")
    for f in sorted(os.listdir(OUTPUT_DIR)):
        fpath = os.path.join(OUTPUT_DIR, f)
        size = os.path.getsize(fpath)
        print(f"  {f}: {size:,} bytes")


if __name__ == "__main__":
    main()
