"""Generate synthetic #832 fixtures with independent qpdf (no third-party PDFs).

Run with Python 3 and qpdf 11+: python3 tests/fixtures/generate_issue_832.py
Each file has a one-page Helvetica marker, short/empty/long/nested strings,
and a sparse array. Encryption uses owner832 and an empty user password (user832 in the _user control).
qpdf chooses fresh encryption randomness; outputs are semantically reproducible.
"""
import pathlib
import subprocess
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
content = b'BT /F1 12 Tf 20 100 Td (ISSUE832 TEXT 12345) Tj ET'
objects = [
    b'<< /Type /Catalog /Pages 2 0 R /Probe 6 0 R /Sparse 7 0 R >>',
    b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
    b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>',
    b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Probe (Identity) >>',
    b'<< /Length ' + str(len(content)).encode() + b' >>\nstream\n' + content + b'\nendstream',
    b'<< /Short (Identity) /Empty () /Long (0123456789abcdef0123456789abcdef) /Nested [ (Adobe) << /Value (nested) >> ] >>',
    b'[null (Identity) null << /Label (Adobe) >> () (0123456789abcdef0123456789abcdef)]',
]
pdf = bytearray(b'%PDF-1.7\n'); offsets = [0]
for i, obj in enumerate(objects, 1):
    offsets.append(len(pdf)); pdf.extend(f'{i} 0 obj\n'.encode()+obj+b'\nendobj\n')
xref = len(pdf)
pdf.extend(f'xref\n0 {len(offsets)}\n0000000000 65535 f \n'.encode())
for offset in offsets[1:]: pdf.extend(f'{offset:010} 00000 n \n'.encode())
pdf.extend(f'trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n'.encode())
variants = {
    'plain': [],
    'rc4': ['--allow-weak-crypto', '--encrypt', '', 'owner832', '128', '--use-aes=n', '--'],
    'aes128': ['--encrypt', '', 'owner832', '128', '--use-aes=y', '--'],
    'aes256r5': ['--encrypt', '', 'owner832', '256', '--force-R5', '--'],
    'aes256r6': ['--encrypt', '', 'owner832', '256', '--'],
    'aes256r6_user': ['--encrypt', 'user832', 'owner832', '256', '--'],
    'aes256r6_uncompressed': ['--encrypt', '', 'owner832', '256', '--'],
}
with tempfile.TemporaryDirectory() as temp:
    source = pathlib.Path(temp) / 'base.pdf'; source.write_bytes(pdf)
    for name, flags in variants.items():
        mode = 'disable' if name.endswith('_uncompressed') else 'generate'
        output = HERE / f'issue_832_{name}.pdf'
        subprocess.run(['qpdf', f'--object-streams={mode}', *flags, str(source), str(output)], check=True)
        subprocess.run(['qpdf', '--password=user832' if name.endswith('_user') else '--password=', '--check', str(output)], check=True, stdout=subprocess.DEVNULL)
