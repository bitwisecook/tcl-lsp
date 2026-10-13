from pathlib import Path
import gzip, hashlib, json, subprocess, sys
root=Path('/workspace/name-implementation-worktrees/runtime'); out=Path('/tmp/engine-wasm-replay-purpose-20261013')
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
meta=json.loads((out/'metadata.json').read_text()); paths=set()
for p in meta['packages']:
 d=Path(p['manifest_path']).parent
 for f in d.rglob('*'):
  if f.is_file() and not f.is_relative_to(root/'target') and '.git' not in f.parts: paths.add(f)
for f in subprocess.check_output(['git','ls-files','-z'],cwd=root).decode().split('\0'):
 p=root/f
 if p.is_file(): paths.add(p)
for e in ['TCL_LSP_TCLSH84','TCL_LSP_TCLSH85','TCL_LSP_TCLSH86','TCL_LSP_TCLSH90','TCL_LSP_TCLSH91','TCL_LSP_JIMSH','TCL_TOMMATH_DIR']:
 import os
 p=Path(os.environ[e])
 if p.is_file(): paths.add(p)
 elif p.is_dir(): paths.update(f for f in p.rglob('*') if f.is_file())
paths.update(p for p in (root/'experiments/name-handling/namespace-origin-query').rglob('*') if p.is_file())
paths.update(p for p in (root/'experiments/name-handling/runtime-command-query-string-observer').rglob('*') if p.is_file())
for row in json.loads((root/'experiments/name-handling/runtime-command-query-string-observer/records.json').read_text()):
 paths.update(Path(p) for key in ['linked_inputs','library_script_inputs'] for p in row[key])
rows={str(p):sha(p) for p in sorted(paths)}
with gzip.open(out/(sys.argv[1]+'.json.gz'),'wt') as f: json.dump(rows,f,sort_keys=True)
print(json.dumps({'stage':sys.argv[1],'paths':len(rows)}))
