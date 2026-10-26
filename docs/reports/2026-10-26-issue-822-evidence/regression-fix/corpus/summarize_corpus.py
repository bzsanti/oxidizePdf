import pathlib,json,statistics,collections,csv,random,gzip
E=pathlib.Path('/home/santi/repos/BelowZero/oxidizePdf/oxidize-pdf/docs/reports/2026-10-26-issue-822-evidence/regression-fix/corpus')
def load(name):
 p=E/name
 data=p.read_text() if p.exists() else gzip.open(str(p)+'.gz','rt').read()
 return [json.loads(x) for x in data.splitlines()]
def paired(rows,phase):
 population=json.loads((E/('unique-population.json' if phase=='full' else 'stratified-sample.json')).read_text())
 expected={(r['sha256'],m,b,v) for r in population for m in (['rewrite'] if phase=='full' else ['rewrite','ttf','cjk']) for b in range(1 if phase=='full' else 5) for v in ['control','candidate']}
 seen=set();g={}
 for r in rows:
  key=(r['sha256'],r['mode'],r['block'],r['variant'])
  assert key not in seen, ('duplicate',key)
  seen.add(key)
  if r['status']=='ok':assert len(r['serialize_ns'])==(1 if phase=='full' else 3),key
  g.setdefault(key[:3],{})[r['variant']]=r
 assert seen==expected,('incomplete or unexpected rows',len(expected-seen),len(seen-expected))
 return g
full=paired(load('full-results.jsonl'),'full');population={r['sha256']:r for r in json.loads((E/'unique-population.json').read_text())}
fullrows=[];tiers=collections.defaultdict(collections.Counter)
for (h,mode,b),v in full.items():
 a=v['control'];c=v['candidate'];status=a['status']+'/'+c['status'];row={'sha256':h,'path':a['path'],'status':status,'aliases':len(population[h]['aliases']),'tiers':','.join(a['tiers'])}
 for tier in a['tiers']:tiers[tier][status]+=1
 if status=='ok/ok':row.update(pages=a['pages'],control_ns=a['serialize_ns'][0],candidate_ns=c['serialize_ns'][0],reduction_percent=100*(1-c['serialize_ns'][0]/a['serialize_ns'][0]),output_size_equal=a['output_bytes']==c['output_bytes'])
 fullrows.append(row)
fields=sorted(set().union(*(r.keys() for r in fullrows)))
with (E/'full-per-document.csv').open('w') as f:
 w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(fullrows)
ok=[r for r in fullrows if r['status']=='ok/ok']
out={'population_paths':sum(len(r['aliases']) for r in population.values()),'unique_inputs':len(population),'full_status':dict(collections.Counter(r['status'] for r in fullrows)),'full_alias_status':{s:sum(r['aliases'] for r in fullrows if r['status']==s) for s in set(r['status'] for r in fullrows)},'tiers':dict(tiers),'full_pages':sum(r['pages'] for r in ok),'full_median_reduction_percent':statistics.median(r['reduction_percent'] for r in ok),'full_total_time_reduction_percent':100*(1-sum(r['candidate_ns'] for r in ok)/sum(r['control_ns'] for r in ok)),'full_size_mismatches':sum(not r['output_size_equal'] for r in ok),'full_timing_limitation':'one cold serialization per revision/input; descriptive only, no per-document significance'}
if (E/'sample-results.jsonl').exists() or (E/'sample-results.jsonl.gz').exists():
 groups=paired(load('sample-results.jsonl'),'sample');bydoc=collections.defaultdict(list)
 for (h,mode,b),v in groups.items():
  if len(v)!=2:continue
  a=v['control'];c=v['candidate']
  if a['status']==c['status']=='ok':
   av=statistics.mean(a['serialize_ns'][1:]);cv=statistics.mean(c['serialize_ns'][1:]);ar=int(a['process_peak_rss'].split()[1]);cr=int(c['process_peak_rss'].split()[1])
   bydoc[h,mode].append({'block':b,'path':a['path'],'generation_reduction':100*(1-(c['author_ns']+c['serialize_ns'][0])/(a['author_ns']+a['serialize_ns'][0])),'control_generation_ns':a['author_ns']+a['serialize_ns'][0],'candidate_generation_ns':c['author_ns']+c['serialize_ns'][0],'cold_reduction':100*(1-c['serialize_ns'][0]/a['serialize_ns'][0]),'warm_reduction':100*(1-cv/av),'control_warm_ns':av,'candidate_warm_ns':cv,'rss_delta_kib':cr-ar,'rss_reduction':100*(1-cr/ar)})
 perdoc=[]
 for (h,mode),r in bydoc.items():
  perdoc.append({'sha256':h,'mode':mode,'path':r[0]['path'],'paired_blocks':len(r),'generation_reduction_percent':statistics.median(x['generation_reduction'] for x in r),'control_generation_ns':statistics.median(x['control_generation_ns'] for x in r),'candidate_generation_ns':statistics.median(x['candidate_generation_ns'] for x in r),'cold_reduction_percent':statistics.median(x['cold_reduction'] for x in r),'warm_reduction_percent':statistics.median(x['warm_reduction'] for x in r),'control_warm_ns':statistics.median(x['control_warm_ns'] for x in r),'candidate_warm_ns':statistics.median(x['candidate_warm_ns'] for x in r),'rss_delta_kib':statistics.median(x['rss_delta_kib'] for x in r),'rss_reduction_percent':statistics.median(x['rss_reduction'] for x in r),'candidate_slower_all_blocks':all(x['warm_reduction']<0 for x in r)})
 with (E/'sample-per-document.csv').open('w') as f:
  w=csv.DictWriter(f,fieldnames=list(perdoc[0]) if perdoc else []);w.writeheader();w.writerows(perdoc)
 out['sample']={}
 rng=random.Random(82208)
 for mode in ['rewrite','ttf','cjk']:
  rs=[r for r in perdoc if r['mode']==mode and r['paired_blocks']==5]
  stats=collections.Counter('/'.join(v[x]['status'] for x in ['control','candidate']) for (h,m,b),v in groups.items() if m==mode and len(v)==2)
  if not rs:
   out['sample'][mode]={'paired_status_across_blocks':dict(stats),'documents_complete_five_pairs':0};continue
  genblocks=[statistics.median(next(x['generation_reduction'] for x in bydoc[r['sha256'],mode] if x['block']==b) for r in rs) for b in range(5)]
  genboot=sorted(statistics.median(rng.choices(genblocks,k=5)) for _ in range(10000))
  blocks=[statistics.median(next(x['warm_reduction'] for x in bydoc[r['sha256'],mode] if x['block']==b) for r in rs) for b in range(5)]
  boot=sorted(statistics.median(rng.choices(blocks,k=5)) for _ in range(10000))
  out['sample'][mode]={'paired_status_across_blocks':dict(stats),'documents_complete_five_pairs':len(rs),'median_document_generation_reduction_percent':statistics.median(r['generation_reduction_percent'] for r in rs),'median_block_generation_reduction_percent':statistics.median(genblocks),'block_bootstrap95_generation_reduction_percent':[genboot[249],genboot[9749]],'total_median_generation_time_reduction_percent':100*(1-sum(r['candidate_generation_ns'] for r in rs)/sum(r['control_generation_ns'] for r in rs)),'median_document_cold_reduction_percent':statistics.median(r['cold_reduction_percent'] for r in rs),'median_document_warm_reduction_percent':statistics.median(r['warm_reduction_percent'] for r in rs),'median_block_warm_reduction_percent':statistics.median(blocks),'block_bootstrap95_warm_reduction_percent':[boot[249],boot[9749]],'total_median_warm_time_reduction_percent':100*(1-sum(r['candidate_warm_ns'] for r in rs)/sum(r['control_warm_ns'] for r in rs)),'median_document_peak_rss_delta_kib':statistics.median(r['rss_delta_kib'] for r in rs),'median_document_peak_rss_reduction_percent':statistics.median(r['rss_reduction_percent'] for r in rs),'regression_flags':[r for r in rs if r['candidate_slower_all_blocks'] and r['warm_reduction_percent'] < -5 and r['candidate_warm_ns']-r['control_warm_ns']>50000]}
(E/'summary.json').write_text(json.dumps(out,indent=2));print(json.dumps(out,indent=2))
