#!/usr/bin/env python3
"""
Generate encrypted PDFs using pypdf for cross-validation with oxidize-pdf.

This script creates R5 and R6 encrypted PDFs to verify compatibility
between different PDF encryption implementations.
"""

import sys
from pathlib import Path

try:
    from pypdf import PdfWriter, PdfReader
    from pypdf.generic import NameObject
except ImportError:
    print("ERROR: pypdf not installed. Run: pip install pypdf")
    sys.exit(1)

OUTPUT_DIR = Path(__file__).parent.parent.parent / "oxidize-pdf-core" / "tests" / "fixtures"


def create_simple_pdf():
    """Create a simple PDF with text content."""
    writer = PdfWriter()

    # Add a blank page
    writer.add_blank_page(width=612, height=792)  # Letter size

    return writer


def generate_r5_pdf(password: str, output_name: str):
    """Generate AES-256 R5 encrypted PDF using pypdf."""
    writer = create_simple_pdf()

    # AES-256 encryption (pypdf uses R5/R6 automatically for AES-256)
    writer.encrypt(
        user_password=password,
        owner_password=password + "_owner",
        algorithm="AES-256",  # This should use R5 or R6
    )

    output_path = OUTPUT_DIR / output_name
    with open(output_path, "wb") as f:
        writer.write(f)

    print(f"Generated: {output_path}")
    return output_path


def generate_r6_pdf(password: str, output_name: str):
    """Generate AES-256-R6 encrypted PDF using pypdf if supported."""
    writer = create_simple_pdf()

    # Try to use R6 explicitly if pypdf supports it
    try:
        writer.encrypt(
            user_password=password,
            owner_password=password + "_owner",
            algorithm="AES-256-R6",
        )
    except ValueError:
        # Fall back to AES-256 (may be R5 or R6 depending on pypdf version)
        writer.encrypt(
            user_password=password,
            owner_password=password + "_owner",
            algorithm="AES-256",
        )

    output_path = OUTPUT_DIR / output_name
    with open(output_path, "wb") as f:
        writer.write(f)

    print(f"Generated: {output_path}")
    return output_path


def verify_encryption(pdf_path: Path, password: str):
    """Verify the encryption of a PDF."""
    try:
        reader = PdfReader(pdf_path)

        if reader.is_encrypted:
            # Try to decrypt
            if reader.decrypt(password):
                # Get encryption info
                encrypt_dict = reader.trailer.get("/Encrypt")
                if encrypt_dict:
                    v = encrypt_dict.get("/V", "unknown")
                    r = encrypt_dict.get("/R", "unknown")
                    print(f"  Encrypted: V={v}, R={r}")
                else:
                    print("  Encrypted (details unavailable)")
                return True
            else:
                print("  Failed to decrypt!")
                return False
        else:
            print("  Not encrypted!")
            return False
    except Exception as e:
        print(f"  Error: {e}")
        return False


def main():
    print(f"pypdf version: {__import__('pypdf').__version__}")
    print(f"Output directory: {OUTPUT_DIR}")
    print()

    # Ensure output directory exists
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    # Generate test PDFs
    test_cases = [
        ("pypdf_test", "encrypted_pypdf_aes256_user.pdf"),
        ("", "encrypted_pypdf_aes256_empty.pdf"),
        ("Contraseña123", "encrypted_pypdf_aes256_spanish.pdf"),
    ]

    print("Generating encrypted PDFs with pypdf...")
    print("-" * 50)

    for password, filename in test_cases:
        print(f"\nGenerating: {filename}")
        print(f"  Password: '{password}'")

        path = generate_r5_pdf(password, filename)
        verify_encryption(path, password)

    print("\n" + "=" * 50)
    print("Cross-validation complete!")
    print("Run: cargo test --test encryption_cross_validation_test")


if __name__ == "__main__":
    main()
