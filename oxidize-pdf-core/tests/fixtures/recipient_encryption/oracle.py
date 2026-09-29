"""Independent fixture generator/verifier. TEST KEYS ONLY, never use in production.
Run with Python 3.14.6, pyHanko 0.29.1, cryptography 50.0.1, asn1crypto 1.5.1.
  python oracle.py generate
  python oracle.py verify /path/to/rust-output.pdf
Certificates valid 2025-01-01..2050-01-01, randomly generated RSA-2048.
"""
from pathlib import Path
import sys, datetime, json, hashlib
from cryptography import x509
from cryptography.x509.oid import NameOID
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa, ec
from asn1crypto import x509 as asn_x509
from pyhanko.pdf_utils import writer, reader, generic as g
from pyhanko.pdf_utils.crypt.pubkey import RecipientEncryptionPolicy, SimpleEnvelopeKeyDecrypter
from pyhanko.pdf_utils.crypt.permissions import PubKeyPermissions
HERE = Path(__file__).parent
TEXT = b'BT /F1 12 Tf 40 100 Td (Recipient interoperability 642) Tj ET\nq 2 0 0 2 20 20 cm /Im1 Do Q\n'
PIXELS = bytes([255,0,0,0,255,0,0,0,255,255,255,255])
def n(s): return g.pdf_name('/'+s)
def dic(**kw): return g.DictionaryObject({n(k):v for k,v in kw.items()})
def certs(): return [asn_x509.Certificate.load((HERE/f'{who}.cert.der').read_bytes()) for who in ['alice','bob']]
def generate():
    for i, who in enumerate(['alice','bob','outsider','signing-only','expired']):
        key = rsa.generate_private_key(public_exponent=65537,key_size=2048)
        name=x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, 'oxidize-pdf TEST ONLY '+who)])
        cert=(x509.CertificateBuilder().subject_name(name).issuer_name(name).public_key(key.public_key()).serial_number(64200+i)
              .not_valid_before(datetime.datetime(2025,1,1)).not_valid_after(datetime.datetime(2025 if who=='expired' else 2050,12,31))
              .add_extension(x509.KeyUsage(digital_signature=True,content_commitment=False,key_encipherment=who!='signing-only',data_encipherment=False,key_agreement=False,key_cert_sign=False,crl_sign=False,encipher_only=None,decipher_only=None),critical=True).sign(key,hashes.SHA256()))
        (HERE/f'{who}.cert.der').write_bytes(cert.public_bytes(serialization.Encoding.DER))
        (HERE/f'{who}.key.der').write_bytes(key.private_bytes(serialization.Encoding.DER,serialization.PrivateFormat.PKCS8,serialization.NoEncryption()))
    for compressed in [False, True]:
        w=writer.PdfFileWriter(stream_xrefs=compressed)
        image=w.add_object(g.StreamObject(dic(Type=n('XObject'),Subtype=n('Image'),Width=g.NumberObject(2),Height=g.NumberObject(2),ColorSpace=n('DeviceRGB'),BitsPerComponent=g.NumberObject(8),Note=g.TextStringObject('encrypted stream dictionary')),stream_data=PIXELS))
        font=w.add_object(dic(Type=n('Font'),Subtype=n('Type1'),BaseFont=n('Helvetica')))
        content=w.add_object(g.StreamObject(stream_data=TEXT))
        resources=dic(Font=dic(F1=font), XObject=dic(Im1=image))
        w.insert_page(writer.PageObject(content,[0,0,200,200],resources))
        w.set_info(dic(Title=g.TextStringObject('Recipient title 642')))
        if compressed:
            objects=w.prepare_object_stream()
            hidden=w.add_object(dic(Message=g.TextStringObject('object stream secret')),obj_stream=objects)
            w.root[n('RecipientTest')]=hidden
        w.encrypt_pubkey(certs(),policy=RecipientEncryptionPolicy(prefer_oaep=True),pdf_mac=False,perms=PubKeyPermissions.ALLOW_PRINTING|PubKeyPermissions.TOLERATE_MISSING_PDF_MAC)
        with (HERE/f'pyhanko-{"xref-stream" if compressed else "classic"}.pdf').open('wb') as out: w.write(out)
    extra_fixtures()
    (HERE/'SHA256SUMS').write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest()+'  '+p.name+'\n' for p in sorted(HERE.iterdir()) if p.suffix in ['.der','.pdf']))
def extra_fixtures():
    for name,oaep,metadata in [('legacy-rsa',False,True),('clear-metadata',True,False)]:
        w=writer.PdfFileWriter(stream_xrefs=False)
        content=w.add_object(g.StreamObject(stream_data=TEXT))
        w.insert_page(writer.PageObject(content,[0,0,200,200]))
        w.encrypt_pubkey(certs(),policy=RecipientEncryptionPolicy(prefer_oaep=oaep),encrypt_metadata=metadata,pdf_mac=False)
        with (HERE/f'unsupported-{name}.pdf').open('wb') as out: w.write(out)
    key=ec.generate_private_key(ec.SECP256R1())
    name=x509.Name([x509.NameAttribute(NameOID.COMMON_NAME,'oxidize-pdf TEST ONLY EC')])
    cert=(x509.CertificateBuilder().subject_name(name).issuer_name(name).public_key(key.public_key()).serial_number(64299)
          .not_valid_before(datetime.datetime(2025,1,1)).not_valid_after(datetime.datetime(2050,1,1)).sign(key,hashes.SHA256()))
    (HERE/'ec.cert.der').write_bytes(cert.public_bytes(serialization.Encoding.DER))

def randomness(first, second):
    secrets=[]
    for path in [first,second]:
        with open(path,'rb') as inp:
            pdf=reader.PdfFileReader(inp)
            pdf.decrypt_pubkey(SimpleEnvelopeKeyDecrypter.load(HERE/'alice.key.der',HERE/'alice.cert.der'))
            cf=pdf.security_handler.get_stream_filter()
            secrets.append((cf._recp_key_seed,cf.shared_key))
    assert len(secrets[0][0])==20 and len(secrets[0][1])==32
    assert secrets[0][0]!=secrets[1][0], 'seed reused'
    assert secrets[0][1]!=secrets[1][1], 'file key reused'
    print('Independent seed and file-key randomization verified')

def verify(path):
    for who in ['alice','bob']:
        cred=SimpleEnvelopeKeyDecrypter.load(HERE/f'{who}.key.der',HERE/f'{who}.cert.der')
        with open(path,'rb') as inp:
            pdf=reader.PdfFileReader(inp)
            auth=pdf.decrypt_pubkey(cred)
            assert auth.status.value != 0, auth
            assert auth.permission_flags & PubKeyPermissions.ALLOW_PRINTING
            assert bool(auth.permission_flags & PubKeyPermissions.ALLOW_CONTENT_EXTRACTION) == (who == 'alice')
            page=pdf.root['/Pages']['/Kids'][0].get_object()
            contents=page['/Contents']; streams=contents if isinstance(contents,g.ArrayObject) else [contents]
            assert b'Recipient interoperability 642' in b''.join(s.get_object().data for s in streams)
            assert str(pdf.trailer['/Info']['/Title'])=='Recipient title 642'
            images=page['/Resources']['/XObject']
            assert any(obj.get_object().data==PIXELS for obj in images.values())
            print(json.dumps({'reader':'pyHanko 0.29.1','file':str(path),'recipient':who,'permissions':str(auth.permission_flags),'text_image_title':'verified'}))
if __name__=='__main__':
    if sys.argv[1]=='generate': generate()
    elif sys.argv[1]=='verify':
        verify(sys.argv[2])
        randomness(sys.argv[2],sys.argv[2]+'.second.pdf')
