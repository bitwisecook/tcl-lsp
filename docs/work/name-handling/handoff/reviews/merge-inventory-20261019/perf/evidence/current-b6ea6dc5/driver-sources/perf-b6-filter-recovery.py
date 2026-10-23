#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
from pathlib import Path
import hashlib,json,os,re,shutil,subprocess,tempfile,time
root=Path('/workspace/tcl-lsp-perf-latest');base=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence');original=base/'current-b6ea6dc5-controls/compiler-artifact-checks';out=base/'current-b6ea6dc5-filter-resource-recovery';out.mkdir(exist_ok=False)
def process(pid):
 try:
  s=(Path('/proc')/str(pid)/'stat').read_text();v=s[s.rfind(')')+2:].split();return v[0],v[19]
 except OSError:return None
worker=process(41020);r=json.loads((original/'binary.json').read_text());expected=set(re.findall(r'^(.+): test$',(original/'filters-list.out').read_text(),re.M));assert len(expected)==229
with tempfile.TemporaryDirectory(prefix='perf-b6-filter-recovery-') as td:
 exe=Path(td)/Path(r['copy']).name;shutil.copy2(r['copy'],exe);h=hashlib.sha256()
 with exe.open('rb') as f:
  while data:=f.read(1024*1024):h.update(data)
 assert h.hexdigest()==r['sha256']
 receipt={'started_unix':time.time(),'pid':os.getpid(),'original_worker':41020,'binary_sha256':h.hexdigest(),'original_binary_receipt':str(original/'binary.json'),'reason':'Keep exact ELF for a missing OLD run if the original selected-filter driver exits after an incomplete NEW inventory. Preserve failures, no assertion or budget change.','head':'b6ea6dc5ebaa9589f6e1c1747875127fecdbb573'};(out/'inputs.json').write_text(json.dumps(receipt,indent=2)+'\n');print('Exact private ELF retained; waiting for original driver',flush=True)
 while worker:
  now=process(41020)
  if not now or now[0]=='Z' or now[1]!=worker[1]:break
  time.sleep(5)
 observed={}
 for mode in ['new','old']:
  p=original/('filters-'+mode+'.out');s=p.read_text() if p.exists() else '';observed[mode]=dict(re.findall(r'^test (.+?) \.\.\. (ok|FAILED|ignored)$',s,re.M))
 if not observed['old']:
  env=dict(os.environ,ZZ_OLD_ENV='1',CARGO_MANIFEST_DIR=str(root/'rust/tcl-compiler'));filters=['spectcl_source','original_name_value','formal_call_advice','realm','interprocedural','w216','w143','incomplete_body','codegen::statements::tests::native_array_indices_emit_iteratively_from_the_full_original_arena'];start=time.time()
  with (out/'filters-old.out').open('w') as log:code=subprocess.run([str(exe),*filters,'--test-threads=2'],cwd=root/'rust/tcl-compiler',env=env,stdout=log,stderr=subprocess.STDOUT).returncode
  observed['old']=dict(re.findall(r'^test (.+?) \.\.\. (ok|FAILED|ignored)$',(out/'filters-old.out').read_text(),re.M));receipt['recovered_old']={'exit':code,'wall_s':time.time()-start}
 receipt['inventories']={mode:{'completed':len(v),'missing':sorted(expected-set(v)),'outcomes':v} for mode,v in observed.items()};receipt['pass_to_fail']=[n for n,v in observed['old'].items() if v=='ok' and observed['new'].get(n)!='ok'];receipt['finished_unix']=time.time();receipt['complete']=all(set(v)==expected for v in observed.values());receipt['copy_removed_on_exit']=True;(out/'final.json').write_text(json.dumps(receipt,indent=2)+'\n')
print('Recovery driver finished; private ELF removed',flush=True)
