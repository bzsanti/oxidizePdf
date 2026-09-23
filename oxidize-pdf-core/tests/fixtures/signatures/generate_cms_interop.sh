#!/usr/bin/env bash
set -euo pipefail

fixture_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
temporary_dir=$(mktemp -d)
trap 'rm -rf "$temporary_dir"' EXIT

printf 'oxidize-pdf CMS interoperability fixture\n' >"$fixture_dir/cms_content.bin"

openssl req -x509 -newkey rsa:2048 -sha256 -nodes -days 7300 \
  -subj '/CN=oxidize-pdf fixture root' \
  -addext 'basicConstraints=critical,CA:TRUE,pathlen:1' \
  -addext 'keyUsage=critical,keyCertSign,cRLSign' \
  -keyout "$temporary_dir/root.key" -out "$temporary_dir/root.pem" >/dev/null 2>&1
openssl x509 -in "$temporary_dir/root.pem" -outform DER \
  -out "$fixture_dir/cms_root.der"

openssl req -newkey rsa:2048 -nodes \
  -subj '/CN=oxidize-pdf fixture intermediate' \
  -keyout "$temporary_dir/intermediate.key" \
  -out "$temporary_dir/intermediate.csr" >/dev/null 2>&1
printf '%s\n' \
  'basicConstraints=critical,CA:TRUE,pathlen:0' \
  'keyUsage=critical,keyCertSign,cRLSign' \
  'subjectKeyIdentifier=hash' \
  'authorityKeyIdentifier=keyid,issuer' >"$temporary_dir/intermediate.ext"
openssl x509 -req -sha256 -days 6000 \
  -in "$temporary_dir/intermediate.csr" \
  -CA "$temporary_dir/root.pem" -CAkey "$temporary_dir/root.key" -CAcreateserial \
  -extfile "$temporary_dir/intermediate.ext" \
  -out "$temporary_dir/intermediate.pem" >/dev/null 2>&1

touch "$temporary_dir/index.txt"
mkdir "$temporary_dir/newcerts"
printf '1000\n' >"$temporary_dir/serial"
printf '1000\n' >"$temporary_dir/crlnumber"
cat >"$temporary_dir/ca.cnf" <<EOF
[ ca ]
default_ca = fixture_ca
[ fixture_ca ]
database = $temporary_dir/index.txt
new_certs_dir = $temporary_dir/newcerts
serial = $temporary_dir/serial
crlnumber = $temporary_dir/crlnumber
certificate = $temporary_dir/intermediate.pem
private_key = $temporary_dir/intermediate.key
default_md = sha256
default_days = 5000
default_crl_days = 5000
policy = fixture_policy
x509_extensions = signer
crl_extensions = crl_extensions
[ fixture_policy ]
commonName = supplied
[ signer ]
basicConstraints = critical,CA:FALSE
keyUsage = critical,digitalSignature
extendedKeyUsage = emailProtection
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid,issuer
[ no_signature_usage ]
basicConstraints = critical,CA:FALSE
keyUsage = critical,keyEncipherment
extendedKeyUsage = emailProtection
[ crl_extensions ]
authorityKeyIdentifier = keyid:always
EOF

openssl req -newkey rsa:2048 -nodes \
  -subj '/CN=oxidize-pdf RSA fixture' \
  -keyout "$temporary_dir/rsa.key" -out "$temporary_dir/rsa.csr" >/dev/null 2>&1
openssl ca -batch -config "$temporary_dir/ca.cnf" \
  -in "$temporary_dir/rsa.csr" -out "$temporary_dir/rsa.pem" >/dev/null 2>&1
openssl cms -sign -binary -md sha256 -nosmimecap \
  -in "$fixture_dir/cms_content.bin" \
  -signer "$temporary_dir/rsa.pem" -inkey "$temporary_dir/rsa.key" \
  -certfile "$temporary_dir/intermediate.pem" \
  -outform DER -out "$fixture_dir/cms_rsa_sha256.der"

openssl req -newkey rsa:2048 -nodes \
  -subj '/CN=oxidize-pdf no signature usage fixture' \
  -keyout "$temporary_dir/no-usage.key" -out "$temporary_dir/no-usage.csr" >/dev/null 2>&1
openssl ca -batch -config "$temporary_dir/ca.cnf" \
  -extensions no_signature_usage \
  -in "$temporary_dir/no-usage.csr" -out "$temporary_dir/no-usage.pem" >/dev/null 2>&1
openssl x509 -in "$temporary_dir/no-usage.pem" -outform DER \
  -out "$fixture_dir/cms_no_signature_usage.der"

openssl ca -gencrl -config "$temporary_dir/ca.cnf" \
  -out "$temporary_dir/valid.crl.pem" >/dev/null 2>&1
openssl crl -in "$temporary_dir/valid.crl.pem" -outform DER \
  -out "$fixture_dir/cms_valid.crl"
openssl ca -config "$temporary_dir/ca.cnf" -revoke "$temporary_dir/rsa.pem" \
  -crl_reason keyCompromise >/dev/null 2>&1
openssl ca -gencrl -config "$temporary_dir/ca.cnf" \
  -out "$temporary_dir/revoked.crl.pem" >/dev/null 2>&1
openssl crl -in "$temporary_dir/revoked.crl.pem" -outform DER \
  -out "$fixture_dir/cms_revoked.crl"

openssl ecparam -name prime256v1 -genkey -noout -out "$temporary_dir/ec.key"
openssl req -x509 -new -sha256 -days 3650 \
  -subj '/CN=oxidize-pdf ECDSA fixture' \
  -key "$temporary_dir/ec.key" -out "$temporary_dir/ec.pem" >/dev/null 2>&1
openssl cms -sign -binary -md sha256 -nosmimecap \
  -in "$fixture_dir/cms_content.bin" \
  -signer "$temporary_dir/ec.pem" -inkey "$temporary_dir/ec.key" \
  -outform DER -out "$fixture_dir/cms_ecdsa_sha256.der"

openssl cms -verify -binary -inform DER \
  -in "$fixture_dir/cms_rsa_sha256.der" \
  -content "$fixture_dir/cms_content.bin" -noverify -out /dev/null
openssl cms -verify -binary -inform DER \
  -in "$fixture_dir/cms_ecdsa_sha256.der" \
  -content "$fixture_dir/cms_content.bin" -noverify -out /dev/null

python3 - "$fixture_dir/signed_rsa.pdf" "$temporary_dir/pdf-to-sign.bin" <<'PY'
import sys

pdf_path, signed_path = sys.argv[1:]
placeholder = b"0" * 32768
objects = [
    b"<< /Type /Catalog /Pages 4 0 R /AcroForm 2 0 R >>",
    b"<< /Fields [3 0 R] /SigFlags 3 >>",
    b"<< /Type /Annot /Subtype /Widget /FT /Sig /T (Signature1) /Rect [0 0 0 0] /V 6 0 R /P 5 0 R >>",
    b"<< /Type /Pages /Kids [5 0 R] /Count 1 >>",
    b"<< /Type /Page /Parent 4 0 R /MediaBox [0 0 612 792] /Annots [3 0 R] >>",
    b"<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached "
    b"/ByteRange [0 0000000000 0000000000 0000000000] /Contents <" + placeholder + b"> >>",
]
pdf = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
offsets = [0]
for number, body in enumerate(objects, 1):
    offsets.append(len(pdf))
    pdf += f"{number} 0 obj\n".encode() + body + b"\nendobj\n"
xref = len(pdf)
pdf += f"xref\n0 {len(objects) + 1}\n".encode()
pdf += b"0000000000 65535 f \n"
for offset in offsets[1:]:
    pdf += f"{offset:010d} 00000 n \n".encode()
pdf += f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
contents_start = pdf.index(b"<" + placeholder) + 1
contents_end = contents_start + len(placeholder)
range_start = contents_start - 1
range_end = contents_end + 1
byte_range = f"[0 {range_start:010d} {range_end:010d} {len(pdf) - range_end:010d}]".encode()
marker = b"[0 0000000000 0000000000 0000000000]"
assert len(byte_range) == len(marker)
pdf[pdf.index(marker):pdf.index(marker) + len(marker)] = byte_range
open(pdf_path, "wb").write(pdf)
open(signed_path, "wb").write(pdf[:range_start] + pdf[range_end:])
PY

openssl cms -sign -binary -md sha256 -nosmimecap \
  -in "$temporary_dir/pdf-to-sign.bin" \
  -signer "$temporary_dir/rsa.pem" -inkey "$temporary_dir/rsa.key" \
  -certfile "$temporary_dir/intermediate.pem" \
  -outform DER -out "$temporary_dir/pdf-signature.der"

python3 - "$fixture_dir/signed_rsa.pdf" "$temporary_dir/pdf-signature.der" \
  "$fixture_dir/signed_rsa_incremental.pdf" "$fixture_dir/signed_rsa_altered.pdf" <<'PY'
import sys

pdf_path, signature_path, incremental_path, altered_path = sys.argv[1:]
pdf = bytearray(open(pdf_path, "rb").read())
signature = open(signature_path, "rb").read().hex().encode()
start = pdf.index(b"/Contents <") + len(b"/Contents <")
end = pdf.index(b">", start)
assert len(signature) <= end - start
pdf[start:start + len(signature)] = signature
open(pdf_path, "wb").write(pdf)
open(incremental_path, "wb").write(pdf + b"\n% incremental update after signing\n")
altered = bytearray(pdf)
position = altered.index(b"/MediaBox")
altered[position] = ord("X")
open(altered_path, "wb").write(altered)
PY
