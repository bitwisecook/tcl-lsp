#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
from pathlib import Path
import hashlib,json,os,re,shutil,subprocess,tempfile,time
root=Path('/workspace/tcl-lsp-owned');base=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence');out=base/'current-b6ea6dc5-clean6-isolated-recovery';out.mkdir(exist_ok=False);deep='interprocedural::tests::deeply_nested_if_survives_interprocedural_scan';filters=['spectcl_source','original_name_value','formal_call_advice','realm','interprocedural','w216','w143','incomplete_body']
def process(pid):
 try:
  s=(Path('/proc')/str(pid)/'stat').read_text();v=s[s.rfind(')')+2:].split();return v[0],v[19]
 except OSError:return None
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  while data:=f.read(1024*1024):h.update(data)
 return h.hexdigest()
rows=[json.loads(line) for line in Path('/tmp/perf-compiler-test-build.jsonl').read_text().splitlines()];found=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('executable') and r['target']['name']=='tcl_compiler'];assert len(found)==1;original=Path(found[0]['executable']);worker=process(47490);record={'started_unix':time.time(),'pid':os.getpid(),'source_binary':str(original),'sha256':sha(original),'scope':'Preserve original incomplete suite. Run original nondeep tests together and unchanged1000-depth test alone, both modes, exact ELF. No assertions, inputs or thread limits changed. Incomplete/failed runs remain failed.','runs':[]}
with tempfile.TemporaryDirectory(prefix='perf-b6-clean6-isolated-') as td:
 exe=Path(td)/original.name;shutil.copy2(original,exe);assert sha(exe)==record['sha256'];(out/'inputs.json').write_text(json.dumps(record,indent=2)+'\n')
 while worker:
  now=process(47490)
  if not now or now[0]=='Z' or now[1]!=worker[1]:break
  time.sleep(5)
 controls=base/'current-b6ea6dc5-empty-allocated-controls/final.json'
 while not controls.exists():time.sleep(5)
 env=dict(os.environ,CARGO_MANIFEST_DIR=str(root/'rust/tcl-compiler'));env.pop('ZZ_OLD_ENV',None)
 outcomes={}
 for suite,selected in [('nondeep',[*filters,'--skip',deep]),('deep',[deep,'--exact'])]:
  listed=subprocess.check_output([str(exe),*selected,'--list'],cwd=root/'rust/tcl-compiler',env=env,text=True);expected=set(re.findall(r'^(.+): test$',listed,re.M));assert len(expected)==(227 if suite=='nondeep' else 1);(out/(suite+'-list.out')).write_text(listed)
  for mode in ['new','old']:
   start=time.monotonic();peak=0;log=out/(suite+'-'+mode+'.out');mode_env=env|({'ZZ_OLD_ENV':'1'} if mode=='old' else {})
   with log.open('w') as f:
    child=subprocess.Popen([str(exe),*selected,'--test-threads=2'],cwd=root/'rust/tcl-compiler',env=mode_env,stdout=f,stderr=subprocess.STDOUT)
    while child.poll() is None:
     try:
      s=(Path('/proc')/str(child.pid)/'status').read_text()
      if 'VmRSS:' in s:peak=max(peak,int(s.split('VmRSS:')[1].split()[0]))
     except OSError:pass
     time.sleep(.5)
    code=child.wait()
   found=dict(re.findall(r'^test (.+?) \.\.\. (ok|FAILED|ignored)$',log.read_text(),re.M));assert set(found)<=expected;outcomes[suite+'/'+mode]=found;record['runs'].append({'suite':suite,'mode':mode,'exit':code,'wall_s':time.monotonic()-start,'peak_rss_kib':peak,'declared':len(expected),'completed':len(found),'missing':sorted(expected-set(found)),'outcomes':found});(out/'progress.json').write_text(json.dumps(record,indent=2)+'\n');print(suite,mode,code,len(found),'peakRSSKiB',peak,flush=True)
 record.update(complete=all(r['completed']==r['declared'] for r in record['runs']),pass_to_fail=[suite+'/'+name for suite in ['nondeep','deep'] for name,state in outcomes[suite+'/old'].items() if state=='ok' and outcomes[suite+'/new'].get(name)!='ok'],finished_unix=time.time(),copy_removed_on_exit=True);(out/'final.json').write_text(json.dumps(record,indent=2)+'\n')
print('Isolated recovery ended; exact private ELF removed',flush=True)
