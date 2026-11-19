import json,subprocess
from pathlib import Path
files={}
for line in Path('/tmp/bd-fs-c-artifacts.jsonl').read_text().splitlines():
 a=json.loads(line)
 if a.get('reason')=='compiler-artifact':
  for f in a['filenames']:
   if f.endswith('.rlib'):files[a['target']['name']]=f
top=Path('/workspace/bd-build/word-rules/debug/build');cmd=['rustc','--edition=2024','/tmp/bd-fs-c-walk.rs','-C','link-arg=-s','-o','/tmp/bd-fs-c-walk']
for p in {f.parent for ext in ['*.so','*.rlib'] for f in top.glob('*/*/out/'+ext)}:cmd+=['-L','dependency='+str(p)]
for p in {f.parent for f in top.rglob('*.a')}:cmd+=['-L','native='+str(p)]
for n in ['tcl_cmd_core','tcl_dialect','tcl_host_native','tcl_platform']:cmd+=['--extern',n+'='+files[n]]
subprocess.run(cmd,check=True)
