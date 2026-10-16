import json,pathlib,hashlib,subprocess,tarfile,datetime
r=pathlib.Path('/workspace/naming-validation/lsp-browser-logical-resource-successor-repair6');assert json.loads((r/'retained584-results-invocation.json').read_text()).get('all39001_input_and_all5_ELF_postflight_match');a=json.load(open(r/'artifacts.json'));out=pathlib.Path('/workspace/naming-validation/archives/lsp-browser-logical-resource-successor-repair6-frozen.tar.zst');assert not out.exists()
for v in a.values():assert hashlib.sha256(pathlib.Path(v['path']).read_bytes()).hexdigest()==v['sha256']
subprocess.run(['tar','--zstd','-cf',str(out),'-C',str(r),'frozen-binaries'],check=True)
p=subprocess.Popen(['zstd','-dc',str(out)],stdout=subprocess.PIPE)
seen=set()
with tarfile.open(fileobj=p.stdout,mode='r|') as t:
 for m in t:
  if m.isfile():
   h=hashlib.sha256(t.extractfile(m).read()).hexdigest();v=next(v for v in a.values() if pathlib.Path(v['path']).name==pathlib.Path(m.name).name);assert h==v['sha256'];seen.add(v['path'])
assert p.wait()==0 and len(seen)==len(a)
receipt={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'archive':str(out),'sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'bytes':out.stat().st_size,'members':a,'restoration':'Restore to original frozen-binaries paths and verify original SHA256 before execution. Lossless storage compaction only, no new acceptance.'}
(r/'frozen-storage-archive.json').write_text(json.dumps(receipt,indent=2)+'\n')
for v in a.values():pathlib.Path(v['path']).unlink()
(r/'frozen-binaries/ARCHIVED.md').write_text('Frozen payloads retained losslessly: '+str(out)+'\nRestore and verify original member hashes before replay.\n')
print(json.dumps({k:v for k,v in receipt.items() if k!='members'}))
