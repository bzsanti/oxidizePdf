"""Generate synthetic certificates around a public distrusted SPKI (no real private key).

Usage: python3 generate.py /path/to/rustls-webpki/src/data/distrusted/EMDVTLSCAG3B.spki.der
Requires cryptography. The input contains the SPKI sequence contents, as stored
by rustls-webpki 0.103.16. Only synthetic certificates are written; keys stay in memory.
"""
from datetime import datetime
from pathlib import Path
import sys
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.x509.oid import NameOID

out = Path(__file__).parent
spki = Path(sys.argv[1]).read_bytes()
assert len(spki) < 128
blocked = serialization.load_der_public_key(bytes([0x30, len(spki)]) + spki)
root_key = ec.generate_private_key(ec.SECP256R1())
root_name = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, "Synthetic distrust test root")])
leaf_name = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, "Synthetic PDF signer")])


def certificate(subject, key, serial, ca):
    return (
        x509.CertificateBuilder()
        .subject_name(subject).issuer_name(root_name)
        .public_key(key).serial_number(serial)
        .not_valid_before(datetime(2025, 1, 1))
        .not_valid_after(datetime(2035, 1, 1))
        .add_extension(x509.BasicConstraints(ca=ca, path_length=None), critical=True)
        .add_extension(x509.KeyUsage(
            digital_signature=True, content_commitment=False, key_encipherment=False,
            data_encipherment=False, key_agreement=False, key_cert_sign=ca,
            crl_sign=ca, encipher_only=False, decipher_only=False,
        ), critical=True)
        .sign(root_key, hashes.SHA256())
    )


for name, cert in [
    ("root", certificate(root_name, root_key.public_key(), 1, True)),
    ("allowed", certificate(leaf_name, ec.generate_private_key(ec.SECP384R1()).public_key(), 2, False)),
    ("distrusted", certificate(leaf_name, blocked, 3, False)),
]:
    (out / (name + ".der")).write_bytes(cert.public_bytes(serialization.Encoding.DER))
