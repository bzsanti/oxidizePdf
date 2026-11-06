from pathlib import Path
import hashlib,json
root=Path(__file__).parent

def write_pdf(path,objects):
    with path.open('wb') as f:
        f.write(b'%PDF-1.7\n'); offsets=[0]
        for i,obj in enumerate(objects,1):
            offsets.append(f.tell());f.write(f'{i} 0 obj\n'.encode());f.write(obj);f.write(b'\nendobj\n')
        start=f.tell();f.write(f'xref\n0 {len(offsets)}\n0000000000 65535 f \n'.encode())
        for offset in offsets[1:]:f.write(f'{offset:010d} 00000 n \n'.encode())
        f.write(f'trailer\n<< /Root 1 0 R /Size {len(offsets)} >>\nstartxref\n{start}\n%%EOF\n'.encode())
def stream(data):return f'<< /Length {len(data)} >>\nstream\n'.encode()+data+b'\nendstream'
def fixture(name,pages,large):
    kids=' '.join(f'{6+2*i} 0 R' for i in range(pages))
    entries=b'\n'.join(f'<{i:04X}> <{i:04X}>'.encode() for i in range(32,2032))
    lines=entries.splitlines()
    blocks=[b'100 beginbfchar\n'+b'\n'.join(lines[i:i+100])+b'\nendbfchar' for i in range(0,len(lines),100)]
    cmap=b'begincmap 1 begincodespacerange <0000> <FFFF> endcodespacerange\n'+b'\n'.join(blocks)+b'\nendcmap'
    objects=[b'<< /Type /Catalog /Pages 2 0 R >>',f'<< /Type /Pages /Kids [{kids}] /Count {pages} >>'.encode(),b'<< /Type /Font /Subtype /Type0 /BaseFont /Shared /Encoding /Identity-H /ToUnicode 5 0 R /DescendantFonts [4 0 R] >>',b'<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Shared /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /DW 600 >>',stream(cmap)]
    content=b'BT /F1 12 Tf 30 750 Td <00480065006c006c006f002003a900200416> Tj ET\n'
    if large:content+=b'10 10 500 700 re S\n'*14500
    for i in range(pages):
        objects.append(f'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> /Contents {7+2*i} 0 R >>'.encode());objects.append(stream(content))
    write_pdf(root/name,objects)
fixture('shared-font-256p.pdf',256,False)
fixture('vectors-256p.pdf',256,True)
fixture('vectors-32p.pdf',32,True)
print(json.dumps({p.name:{'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in root.glob('*.pdf')},indent=2))
