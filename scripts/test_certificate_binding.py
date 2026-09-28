"""Run against an installed candidate wheel; uses only the Python stdlib."""
from pathlib import Path
import unittest
import oxidize_pdf

FIXTURES = Path(__file__).resolve().parents[1] / "oxidize-pdf-core/tests/fixtures/signatures"


class CertificateBindingTests(unittest.TestCase):
    def test_certificate_parser_is_enabled_and_untrusted_root_fails(self):
        cert = (FIXTURES / "cms_root.der").read_bytes()
        result = oxidize_pdf.validate_pdf_certificate(cert, oxidize_pdf.TrustStore.empty())
        self.assertEqual(result.subject, "oxidize-pdf fixture root")
        self.assertFalse(result.is_trusted)
        self.assertFalse(result.is_valid())

    def test_malformed_certificate_raises(self):
        with self.assertRaisesRegex(RuntimeError, "Failed to parse certificate"):
            oxidize_pdf.validate_pdf_certificate(b"invalid DER", oxidize_pdf.TrustStore.empty())

    def test_signed_pdf_parsing_is_enabled(self):
        results = oxidize_pdf.verify_pdf_signatures((FIXTURES / "signed_rsa.pdf").read_bytes())
        self.assertEqual(len(results), 1)
        self.assertEqual(results[0]["name"], "Signature1")
        self.assertIsNone(results[0]["error"])
        # Do not treat this wrapper's `valid` as a certificate-validation oracle:
        # its pre-existing is_ok() bug is recorded separately in TASKS.md.


if __name__ == "__main__":
    unittest.main()
