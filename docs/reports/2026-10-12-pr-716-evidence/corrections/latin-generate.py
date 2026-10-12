import sys
from pathlib import Path
sys.path.insert(0, '/tmp/pr716-fix/tools')
from text_contracts.fixture_pdf import assemble, stream
out=Path('/tmp/pr716-latin-final');out.mkdir(exist_ok=True)
raw=Path('/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/docs/reports/2026-10-12-issue-715-evidence/latin-symbol.ttf').read_bytes()
for mode in ['symbolic', 'winansi', 'nonsymbolic']:
    widths = ' '.join(['500'] * 95)
    enc='/Encoding /WinAnsiEncoding' if mode=='winansi' else ''
    objects=[b'<< /Type /Catalog /Pages 2 0 R >>', b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
        b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>',
        f'<< /Type /Font /Subtype /TrueType /BaseFont /ABCDEF+DejaVuSans {enc} /FirstChar 32 /LastChar 126 /Widths [{widths}] /FontDescriptor 6 0 R >>'.encode(),
        stream(b'BT /F1 12 Tf 72 700 Td (Hello World 123) Tj ET'),
        f'<< /Type /FontDescriptor /FontName /ABCDEF+DejaVuSans /Flags {32 if mode=="nonsymbolic" else 4} /FontBBox [-1100 -500 1800 1500] /Ascent 928 /Descent -236 /CapHeight 750 /ItalicAngle 0 /StemV 80 /FontFile2 7 0 R >>'.encode(),
        stream(raw, f'/Length1 {len(raw)}')]
    (out/(mode+'.pdf')).write_bytes(assemble(objects))
