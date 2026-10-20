from pathlib import Path
import subprocess, shutil, os, time, json
root=Path('/tmp/release551-aggregate-race-evidence');root.mkdir(exist_ok=True)
exe='/tmp/oxidize-bridges-release-20261018/target/debug/deps/issue_729_external_validators_test-0815096d75d86836'
results=[]
for warm in (False, True):
 p=root/('warm' if warm else 'cold');p.mkdir();(p/'bin').symlink_to('/tmp/release551-verapdf/bin',target_is_directory=True);shutil.copy2('/tmp/release551-verapdf/verapdf',p/'verapdf')
 if warm:
  init=subprocess.run([str(p/'verapdf'),'--version'],text=True,capture_output=True,check=True)
  (p/'initialization.log').write_text(init.stdout+init.stderr)
 marker=p/'observed-missing'
 env=dict(os.environ,PATH=str(p)+':'+os.environ['PATH'],LD_PRELOAD='/tmp/release551-delay-stat.so',RELEASE551_STAT_MARKER=str(marker))
 with (p/'aggregate.log').open('w') as out:
  first=subprocess.Popen([exe,'real_aggregate_preserves_each_validation_scope','--ignored','--exact','--nocapture'],env=env,stdout=out,stderr=subprocess.STDOUT)
  deadline=time.monotonic()+10
  while not marker.exists() and first.poll() is None and time.monotonic()<deadline:time.sleep(.005)
  second=subprocess.run([str(p/'verapdf'),'--format','xml','--flavour','1b','--nonpdfext','/tmp/oxidize-release551/oxidize-pdf-core/tests/fixtures/issue_729/pdfa-1b.pdf'],text=True,capture_output=True)
  (p/'direct.xml').write_text(second.stdout);(p/'direct.err').write_text(second.stderr)
  result={'warm':warm,'missing_config_observed':marker.exists(),'aggregate_exit':first.wait(),'direct_exit':second.returncode}
 results.append(result);print(result,flush=True);print((p/'aggregate.log').read_text(),flush=True)
 assert result['aggregate_exit']==(0 if warm else 101) and result['direct_exit']==0
(root/'results.json').write_text(json.dumps(results,indent=2))
