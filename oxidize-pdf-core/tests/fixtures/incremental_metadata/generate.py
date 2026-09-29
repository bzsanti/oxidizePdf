from pathlib import Path
root=Path(__file__).resolve().parent
def stream(data, attrs=''):
 return f'<< /Length {len(data)} {attrs} >>\nstream\n'.encode()+data+b'\nendstream'
def assemble(name,nested=False,size=10,rich=False,generation=0):
 xmp=b'<?xpacket begin=""?><x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/" dc:title="Original title"/></rdf:RDF></x:xmpmeta><?xpacket end="w"?>'
 objects={1:b'<< /Type /Catalog /Pages 2 0 R /Metadata 8 0 R /Lang (es-ES) >>',
 2:b'<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /MediaBox [0 0 600 800] /Resources << /Font << /F1 5 0 R >> >> >>',
 3:b'<< /Type /Page /Parent 2 0 R /Contents 6 0 R /Metadata 8 0 R >>',
 4:b'<< /Type /Page /Parent 2 0 R /Contents 7 0 R /Metadata 8 0 R >>',
 5:b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',
 6:stream(b'BT /F1 12 Tf 50 700 Td (OriginalOne) Tj ET'),
 7:stream(b'BT /F1 12 Tf 50 700 Td (OriginalTwo) Tj ET'),
 8:stream(xmp,'/Type /Metadata /Subtype /XML'),
 9:b'<< /Title (Original title) /Author (Original author) /CustomCustomerField (Keep this value) >>'}
 if nested:
  objects[2]=b'<< /Type /Pages /Kids [10 0 R] /Count 2 /MediaBox [0 0 600 800] /Resources << /Font << /F1 5 0 R >> >> >>'
  objects[10]=b'<< /Type /Pages /Parent 2 0 R /Kids [3 0 R 4 0 R] /Count 2 >>'
  for i in [3,4]: objects[i]=objects[i].replace(b'/Parent 2 0 R',b'/Parent 10 0 R')
  size=max(size,11)
 if rich:
  objects[1]=objects[1][:-2]+b' /CustomerCatalog <feff00450073> /Customer#20Field /Value#23name /Raw#ff /N#ff /OpenAction [3 0 R /Fit] >>'
  objects[2]=objects[2].replace(b'[0 0 600 800]',b'[10 20 610 820] /CropBox [20 30 600 800] /Rotate 90')
  objects[3]=objects[3][:-2]+b' /Annots 11 0 R /CustomerPage <00ff80> >>'
  objects[11]=b'[12 0 R]'
  objects[12]=b'<< /Type /Annot /Subtype /Text /Rect [30 40 50 60] /Contents (Existing note) /P 3 0 R >>'
  size=13
 data=bytearray(b'%PDF-1.4\n'); offsets={}
 for i,obj in objects.items(): offsets[i]=len(data); data+=f'{i} {generation if i==9 else 0} obj\n'.encode()+obj+b'\nendobj\n'
 xref=len(data); data+=b'xref\n0 1\n0000000000 65535 f \n'
 for i,offset in offsets.items():data+=f'{i} 1\n{offset:010} {generation if i==9 else 0:05} n \n'.encode()
 data+=f'trailer\n<< /Size {size} /Root 1 0 R /Info 9 {generation} R >>\nstartxref\n{xref}\n%%EOF\n'.encode()
 (root/f'{name}.pdf').write_bytes(data)
assemble('flat');assemble('nested',True);assemble('maxsize',size=4294967295)

assemble('rich', rich=True); assemble('generation', generation=7)
