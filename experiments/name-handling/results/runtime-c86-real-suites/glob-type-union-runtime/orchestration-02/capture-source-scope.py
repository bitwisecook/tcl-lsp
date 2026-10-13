import gzip,hashlib,json,pathlib,sys
ROOT=pathlib.Path('/workspace/tcl-lsp');OUT=ROOT/'experiments/name-handling/results/runtime-c86-real-suites/glob-type-union-runtime/orchestration-02';OUT.mkdir(exist_ok=True);sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();PRIOR=OUT.parent/'validation-02'
with gzip.open(PRIOR/'runtime-dependency-source-before.json.gz','rt') as f:paths=set(json.load(f))
for directory in [ROOT/'tmp/tcl9.0.4/libtommath',ROOT/'tmp/tcl8.6.18/library',ROOT/'tmp/tcl8.6.18/tests']:
 paths.update(str(p.relative_to(ROOT)) for p in directory.rglob('*') if p.is_file() and p.suffix in ('.c','.h','.tcl','.test'))
paths.add('experiments/name-handling/results/split-native-implementation-c86/capture-01/all-tcl-var.tcl')
current={p:sha(ROOT/p) for p in sorted(paths)};scope=json.loads((PRIOR/'scope.json').read_text());assert all(current[p]==h for p,h in scope['frozen_ready_sha256'].items());phase=sys.argv[1]
with gzip.open(OUT/f'runtime-source-{phase}.json.gz','wt') as f:json.dump(current,f,indent=2)
if phase=='before':
 with gzip.open(PRIOR/'runtime-cargo-metadata.json.gz','rb') as f:metadata=f.read()
 (OUT/'runtime-cargo-metadata.json.gz').write_bytes(gzip.compress(metadata,mtime=0))
 scope['runner_scope']='Selected production run_script example; conservative local path closure reuses the just-built default-feature Runtime14 metadata, including root dev dependencies. No new Registry/workspace metadata. Source hashes add unchanged actual C86 library/tests and default libtommath headers; not every input is compiled.';scope['count']=len(current);scope['metadata_reference']=str((PRIOR/'scope.json').relative_to(ROOT));(OUT/'scope.json').write_text(json.dumps(scope,indent=2)+'\n')
 for p in ['runtime/rust/src/cmd_clock.rs','runtime/rust/src/cmd_fs.rs','runtime/rust/examples/run_script.rs']:
  with gzip.open(OUT/(p.replace('/','--')+'.gz'),'wb') as f:f.write((ROOT/p).read_bytes())
else:
 with gzip.open(OUT/'runtime-source-before.json.gz','rt') as f:before=json.load(f)
 changed=[p for p in sorted(before.keys()|current.keys()) if before.get(p)!=current.get(p)];(OUT/'source-comparison.json').write_text(json.dumps({'changed':changed,'frozen_owner_sha256':scope['frozen_ready_sha256']},indent=2)+'\n');print('source changes',changed)
print(phase,len(current),flush=True)
