import gzip,hashlib,json,pathlib,sys
root=pathlib.Path('/workspace/tcl-lsp');out=root/'experiments/name-handling/results/runtime-c86-real-suites/get-date-fields-runtime/validation-01';out.mkdir(parents=True,exist_ok=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
meta=pathlib.Path('/tmp/name-getdatefields-runtime-metadata.json');m=json.loads(meta.read_text());pkgs={p['id']:p for p in m['packages']};nodes={p['id']:p for p in m['resolve']['nodes']};rid=m['resolve']['root'];todo=[rid];seen=set()
while todo:
 p=todo.pop()
 if p in seen:continue
 seen.add(p)
 for d in nodes[p]['deps']:
  if p==rid or any(k['kind'] in (None,'build') for k in d['dep_kinds']):todo.append(d['pkg'])
selected={};local=[]
for pid in seen:
 p=pkgs[pid];directory=pathlib.Path(p['manifest_path']).parent
 if not directory.is_relative_to(root) or directory.is_relative_to(root/'tmp'):continue
 local.append(p['name'])
 for path in directory.rglob('*'):
  if path.is_file() and (path.suffix in ('.rs','.c','.h','.tcl','.txt','.json') or path.name in ('Cargo.toml','Cargo.lock')) and not any(x in ('target','.git') for x in path.relative_to(directory).parts):selected[str(path.relative_to(root))]=sha(path)
for rel in ['Cargo.toml','Cargo.lock','runtime/rust/Cargo.lock','experiments/name-handling/results/runtime-c86-real-suites/get-date-fields-question/input.tcl','experiments/name-handling/results/runtime-c86-real-suites/get-date-fields-question/capture-01/8.6.18/values/stdout.txt']:
 if (root/rel).is_file():selected[rel]=sha(root/rel)
phase=sys.argv[1]
with gzip.open(out/f'runtime-dependency-source-{phase}.json.gz','wt') as f:json.dump(dict(sorted(selected.items())),f,indent=2)
ready=json.loads(pathlib.Path('/tmp/name-getdatefields-scratch/source-ready-v2.json').read_text());actual={p:sha(root/p) for p in ready['source_ready_sha256']};assert actual==ready['source_ready_sha256']
if phase=='before':
 (out/'source-ready.json').write_text(json.dumps(ready,indent=2)+'\n')
 for p in ready['source_ready_sha256']:
  with gzip.open(out/(p.replace('/','_')+'.gz'),'wb') as f:f.write((root/p).read_bytes())
 with gzip.open(out/'runtime-cargo-metadata.json.gz','wb') as f:f.write(meta.read_bytes())
 scope={'metadata_sha256':sha(meta),'packages':sorted(local),'scope':'Actual standalone Cargo metadata graph: root all dependency kinds; non-root normal/build edges. Local package rs/c/h/tcl/txt/json and manifests/locks conservatively captured, plus exact original oracle fixtures. Not whole-workspace source claim.','count':len(selected),'excluded_concurrent_source':'rust/tcl-lsp-core/src/irule_gen.rs'}
 (out/'scope.json').write_text(json.dumps(scope,indent=2)+'\n')
else:
 with gzip.open(out/'runtime-dependency-source-before.json.gz','rt') as f:before=json.load(f)
 changed=[p for p in sorted(before.keys()|selected.keys()) if before.get(p)!=selected.get(p)]
 (out/'source-comparison.json').write_text(json.dumps({'changed_dependency_sources':changed,'actual_five_ready_sha256':actual},indent=2)+'\n')
 print('changed dependency sources',changed)
print(phase,len(selected),'local dependency packages',len(local))
