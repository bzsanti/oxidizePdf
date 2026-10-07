import subprocess, json, random, statistics, hashlib, platform, os
from pathlib import Path
root=Path(__file__).resolve().parent
out=root/'results'
out.mkdir(exist_ok=False)
rng=random.Random(661)
variants=['control','shared']
records=[]
for block in range(16):
    cases=[(variant,mode,pages) for variant in variants for mode in ['default','raw'] for pages in [1,10,100]]
    rng.shuffle(cases)
    for variant,mode,pages in cases:
        binary=root/f'{variant}-timing'
        pdf=out/f'{variant}-{mode}-{pages}.pdf'
        r=json.loads(subprocess.check_output([str(binary),mode,str(pages),str(max(10,1000//pages)),str(pdf)]))
        r.update(variant=variant,block=block)
        records.append(r)
        with (out/'samples.jsonl').open('a') as f: f.write(json.dumps(r)+'\n')
summary=[]
for mode in ['default','raw']:
    for pages in [1,10,100]:
        groups={v:[r['elapsed_ns']/r['documents']/1000 for r in records if r['variant']==v and r['mode']==mode and r['pages']==pages] for v in variants}
        summary.append(dict(mode=mode,pages=pages,us={v:dict(median=statistics.median(xs),min=min(xs),max=max(xs),stdev=statistics.stdev(xs)) for v,xs in groups.items()},improvement_pct=100*(1-statistics.median(groups['shared'])/statistics.median(groups['control']))))
validation=[]
allocations=[]
for variant in variants:
    for mode in ['default','raw']:
        for pages in [1,10,100]:
            pdf=out/f'{variant}-{mode}-{pages}.pdf'
            subprocess.run(['qpdf','--check',str(pdf)],check=True,capture_output=True)
            assert int(subprocess.check_output(['qpdf','--show-npages',str(pdf)]))==pages
            text=subprocess.check_output(['pdftotext','-layout',str(pdf),'-']).decode()
            expected=[]
            for page in range(1,pages+1):
                expected += [f'Invoice benchmark | Page {page:04}/{pages:04}']+[f'Item {item:02} | Qty 2 | Unit 12.50 | Total 25.00' for item in range(1,21)]+['TOTAL 500.00 EUR']
            assert [s.strip() for s in text.replace('\f','\n').splitlines() if s.strip()]==expected
            validation.append(dict(variant=variant,mode=mode,pages=pages,bytes=pdf.stat().st_size,qpdf='pass',exact_lines='pass',sha256=hashlib.sha256(pdf.read_bytes()).hexdigest()))
            r=json.loads(subprocess.check_output([str(root/f'{variant}-alloc'),mode,str(pages),'10',str(out/f'{variant}-{mode}-{pages}-alloc.pdf')]))
            r.update(variant=variant);r.pop('elapsed_ns');allocations.append(r)
for name,value in [('summary',summary),('validation',validation),('allocations',allocations)]:
    (out/f'{name}.json').write_text(json.dumps(value,indent=2)+'\n')
env=dict(platform=platform.platform(),cpu=Path('/proc/cpuinfo').read_text(),affinity=sorted(os.sched_getaffinity(0)),rustc=subprocess.check_output(['rustc','-Vv']).decode(),binaries={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in root.iterdir() if p.name.endswith(('-timing','-alloc'))})
(out/'environment.json').write_text(json.dumps(env,indent=2)+'\n')
print(json.dumps(summary,indent=2))
