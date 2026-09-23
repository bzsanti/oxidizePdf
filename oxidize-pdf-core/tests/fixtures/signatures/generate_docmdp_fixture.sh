#!/usr/bin/env bash
set -euo pipefail

fixture_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
temporary_dir=$(mktemp -d)
trap 'rm -rf "$temporary_dir"' EXIT

openssl req -x509 -newkey rsa:2048 -sha256 -nodes -days 3650 \
  -subj '/CN=oxidize-pdf DocMDP fixture' \
  -addext 'basicConstraints=critical,CA:FALSE' \
  -addext 'keyUsage=critical,digitalSignature' \
  -keyout "$temporary_dir/signer.key" -out "$temporary_dir/signer.pem" >/dev/null 2>&1

python3 - "$temporary_dir/unsigned.pdf" "$temporary_dir/to-sign.bin" <<'PY'
import sys

pdf_path, signed_path = sys.argv[1:]
placeholder = b"0" * 32768
objects = [
    b"<< /Type /Catalog /Pages 4 0 R /AcroForm 2 0 R /Perms << /DocMDP 6 0 R >> >>",
    b"<< /Fields [3 0 R] /SigFlags 3 >>",
    b"<< /Type /Annot /Subtype /Widget /FT /Sig /T (Certification1) /Rect [0 0 0 0] /V 6 0 R /P 5 0 R >>",
    b"<< /Type /Pages /Kids [5 0 R] /Count 1 >>",
    b"<< /Type /Page /Parent 4 0 R /MediaBox [0 0 612 792] /Annots [3 0 R] >>",
    b"<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached "
    b"/ByteRange [0 0000000000 0000000000 0000000000] /Contents <" + placeholder +
    b"> /Reference [<< /Type /SigRef /TransformMethod /DocMDP "
    b"/TransformParams << /Type /TransformParams /V /1.2 /P 2 >> >>] >>",
]
pdf = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
offsets = [0]
for number, body in enumerate(objects, 1):
    offsets.append(len(pdf))
    pdf += f"{number} 0 obj\n".encode() + body + b"\nendobj\n"
xref = len(pdf)
pdf += f"xref\n0 {len(objects) + 1}\n".encode() + b"0000000000 65535 f \n"
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
start = pdf.index(marker)
pdf[start:start + len(marker)] = byte_range
open(pdf_path, "wb").write(pdf)
open(signed_path, "wb").write(pdf[:range_start] + pdf[range_end:])
PY

openssl cms -sign -binary -md sha256 -nosmimecap \
  -in "$temporary_dir/to-sign.bin" \
  -signer "$temporary_dir/signer.pem" -inkey "$temporary_dir/signer.key" \
  -outform DER -out "$temporary_dir/signature.der"

python3 - "$temporary_dir/unsigned.pdf" "$temporary_dir/signature.der" \
  "$fixture_dir/docmdp_p2_rsa.pdf" <<'PY'
import sys

pdf_path, signature_path, output_path = sys.argv[1:]
pdf = bytearray(open(pdf_path, "rb").read())
signature = open(signature_path, "rb").read().hex().encode()
start = pdf.index(b"/Contents <") + len(b"/Contents <")
end = pdf.index(b">", start)
assert len(signature) <= end - start
pdf[start:start + len(signature)] = signature
open(output_path, "wb").write(pdf)
PY
