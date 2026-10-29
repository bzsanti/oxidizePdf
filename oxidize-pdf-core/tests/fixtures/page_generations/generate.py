"""Synthetic PDFs with exact generation IDs; no third-party document content.

Run this script, then qpdf --check on each output. The xref stream is written
without object compression: compressed objects cannot have nonzero generations.
"""
from pathlib import Path
import struct
import re
import subprocess
import tempfile


def build(nested, stream, signed=False):
    objects = [
        (7, b'<< /Type /Catalog /Pages 2 4 R >>'),
        (4, b'<< /Type /Pages /Kids [4 9 R] /Count 1 >>' if nested else
            b'<< /Type /Pages /Kids [3 2 R] /Count 1 >>'),
        (2, b'<< /Type /Page /Parent 4 9 R /Resources << >> >>' if nested else
            b'<< /Type /Page /Parent 2 4 R /MediaBox [0 0 595 842] /Resources << >> >>'),
    ]
    if nested:
        objects.append((9, b'<< /Type /Pages /Parent 2 4 R /Kids [3 2 R] /Count 1 /MediaBox [0 0 595 842] >>'))
    if signed:
        assert nested
        objects[0] = (7, b'<< /Type /Catalog /Pages 2 4 R /AcroForm << /Fields [5 0 R] /SigFlags 3 >> >>')
        objects[2] = (2, b'<< /Type /Page /Parent 4 9 R /Resources << >> /Annots [5 0 R] >>')
        objects.extend([
            (0, b'<< /Type /Annot /Subtype /Widget /FT /Sig /T (Original) /Rect [0 0 0 0] /P 3 2 R /V 6 0 R >>'),
            (0, b'<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /ByteRange [0 0000000000 0000000000 0000000000] /Contents <' + b'0' * 8192 + b'> >>'),
        ])
    pdf = bytearray(b'%PDF-1.7\n%\xe2\xe3\xcf\xd3\n')
    entries = [(0, 0, 65535)]
    for number, (generation, body) in enumerate(objects, 1):
        entries.append((1, len(pdf), generation))
        pdf.extend(f'{number} {generation} obj\n'.encode() + body + b'\nendobj\n')
    offset = len(pdf)
    if stream:
        number = len(entries)
        entries.append((1, offset, 0))
        data = b''.join(struct.pack('>BIH', *entry) for entry in entries)
        pdf.extend(f'{number} 0 obj\n<< /Type /XRef /Size {len(entries)} /Root 1 7 R /W [1 4 2] /Length {len(data)} >>\nstream\n'.encode())
        pdf.extend(data + b'\nendstream\nendobj\n')
    else:
        pdf.extend(f'xref\n0 {len(entries)}\n0000000000 65535 f \n'.encode())
        for _, pos, generation in entries[1:]:
            pdf.extend(f'{pos:010} {generation:05} n \n'.encode())
        pdf.extend(f'trailer\n<< /Size {len(entries)} /Root 1 7 R >>\n'.encode())
    pdf.extend(f'startxref\n{offset}\n%%EOF\n'.encode())
    return pdf


def sign(pdf):
    """Sign exact PDF byte ranges with an ephemeral OpenSSL fixture key."""
    match = re.search(rb'/Contents (<[0]+>)', pdf)
    start, end = match.span(1)
    marker = b'/ByteRange [0 0000000000 0000000000 0000000000]'
    replacement = f'/ByteRange [0 {start:010} {end:010} {len(pdf) - end:010}]'.encode()
    pdf = pdf.replace(marker, replacement)
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        (root / 'content').write_bytes(pdf[:start] + pdf[end:])
        subprocess.run(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-sha256', '-nodes',
                        '-days', '3650', '-subj', '/CN=issue-836 synthetic fixture',
                        '-keyout', str(root / 'key.pem'), '-out', str(root / 'cert.pem')],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        subprocess.run(['openssl', 'cms', '-sign', '-binary', '-md', 'sha256', '-nosmimecap',
                        '-in', str(root / 'content'), '-signer', str(root / 'cert.pem'),
                        '-inkey', str(root / 'key.pem'), '-outform', 'DER', '-out', str(root / 'cms.der')], check=True)
        subprocess.run(['openssl', 'cms', '-verify', '-binary', '-inform', 'DER',
                        '-in', str(root / 'cms.der'), '-content', str(root / 'content'),
                        '-noverify', '-out', '/dev/null'], check=True)
        cms = (root / 'cms.der').read_bytes().hex().upper().encode()
        assert len(cms) <= end - start - 2
        pdf[start + 1:end - 1] = cms.ljust(end - start - 2, b'0')
    return pdf


if __name__ == '__main__':
    for nested in (False, True):
        for stream in (False, True):
            name = f'{"nested" if nested else "flat"}-{"stream" if stream else "table"}.pdf'
            Path(__file__).with_name(name).write_bytes(build(nested, stream))
    Path(__file__).with_name('signed-nested-stream.pdf').write_bytes(sign(build(True, True, signed=True)))
