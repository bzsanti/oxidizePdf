import subprocess,json,random,statistics,hashlib
from pathlib import Path
root=Path(__file__).resolve().parent
out=root/'empty-results';out.mkdir(exist_ok=False)
rng=random.Random(662);records=[]
for block in range(24):
    cases=[(v,m,p) for v in ['control','compression'] for m in ['empty'] for p in [1,10]]
    rng.shuffle(cases)
    for v,m,p in cases:
        r=json.loads(subprocess.check_output([str(root/f'extended-{v}-timing'),m,str(p),str(max(10,1000//p)),str(out/f'{v}-{m}-{p}.pdf')]))
        r.update(variant=v,block=block);records.append(r)
        with (out/'samples.jsonl').open('a') as f:f.write(json.dumps(r)+'\n')
summary=[];validation=[];alloc=[]
for m in ['empty']:
    for p in [1,10]:
        med={v:statistics.median(r['elapsed_ns']/r['documents']/1000 for r in records if r['variant']==v and r['mode']==m and r['pages']==p) for v in ['control','compression']}
        summary.append(dict(mode=m,pages=p,median_us=med,improvement_pct=100*(1-med['compression']/med['control'])))
        rasters={}
        for v in ['control','compression']:
            pdf=out/f'{v}-{m}-{p}.pdf'
            subprocess.run(['qpdf','--check',str(pdf)],check=True,capture_output=True)
            assert int(subprocess.check_output(['qpdf','--show-npages',str(pdf)]))==p
            result=subprocess.run(['pdftotext','-layout',str(pdf),'-'],capture_output=True,check=True)
            assert not result.stderr, result.stderr
            lines=[''.join(s.split()) for s in result.stdout.decode().replace('\f','\n').splitlines() if s.strip()]
            expected=[] if m=='empty' else ['中文测试']*p if m=='cjk' else [f'Page{i}' for i in range(1,p+1)]
            assert lines==expected,(m,p,lines)
            validation.append(dict(variant=v,mode=m,pages=p,bytes=pdf.stat().st_size,qpdf='pass',text='exact',sha256=hashlib.sha256(pdf.read_bytes()).hexdigest()))
            r=json.loads(subprocess.check_output([str(root/f'extended-{v}-alloc'),m,str(p),'2',str(out/f'{v}-{m}-{p}-alloc.pdf')]))
            r.pop('elapsed_ns');r['variant']=v;alloc.append(r)
            for page in sorted({1,p}):
                result=subprocess.run(['pdftoppm','-f',str(page),'-l',str(page),'-singlefile','-scale-to','1000','-gray',str(pdf)],capture_output=True,check=True)
                assert not result.stderr,result.stderr
                digest=hashlib.sha256(result.stdout).hexdigest()
                if v=='control':rasters[page]=digest
                else:assert rasters[page]==digest,(m,p,page)
for name,data in [('summary',summary),('validation',validation),('allocations',alloc)]:
    (out/f'{name}.json').write_text(json.dumps(data,indent=2)+'\n')
print(json.dumps(summary,indent=2))
