from pathlib import Path
import gzip,hashlib,json,os,subprocess,time
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[3]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
env=dict(os.environ);env['CARGO_TARGET_DIR']=str(ROOT/'tmp/name-target-issues')
argvmeta=['cargo','metadata','--offline','--no-deps','--format-version','1','--manifest-path','Cargo.toml']
with (HERE/'vm-metadata-local.json').open('wb') as stdout,(HERE/'vm-metadata-local.stderr.txt').open('wb') as stderr:meta=subprocess.run(argvmeta,cwd=ROOT,env=env,stdout=stdout,stderr=stderr)
if meta.returncode!=0:raise SystemExit(meta.returncode)
argv=['cargo','test','--offline','-j2','--manifest-path','Cargo.toml','-p','tcl-vm','--lib','--message-format=json','exec::native_string_trim_tests::original_string_trim_matches_all_216_native_header_windows','--','--exact','--test-threads=1']
print('START VM exact216',flush=True);start=time.monotonic()
with (HERE/'vm-stdout.txt').open('wb') as stdout,(HERE/'vm-stderr.txt').open('wb') as stderr:r=subprocess.run(argv,cwd=ROOT,env=env,stdout=stdout,stderr=stderr)
artifacts=[]
for line in (HERE/'vm-stdout.txt').read_text().splitlines():
 if line.startswith('{'):
  try:item=json.loads(line)
  except json.JSONDecodeError:continue
  if item.get('reason')=='compiler-artifact':artifacts.append(item)
(HERE/'vm-artifacts.json').write_text(json.dumps(artifacts,indent=2)+'\n')
ids={x['package_id'] for x in artifacts};metadata=json.loads((HERE/'vm-metadata-local.json').read_text());packages=[x for x in metadata['packages'] if x['id'] in ids]
prefix=[str(Path(x['manifest_path']).parent.relative_to(ROOT))+'/' for x in packages if Path(x['manifest_path']).is_relative_to(ROOT)]
before=json.loads(gzip.decompress((HERE/'coordination-source-before.json.gz').read_bytes()));local={k:v for k,v in before.items() if any(k.startswith(p) for p in prefix)};after={k:sha(ROOT/k) for k in local}
for stage,data in [('before',local),('after',after)]:(HERE/('vm-compiled-package-source-'+stage+'.json.gz')).write_bytes(gzip.compress(json.dumps(data,sort_keys=True).encode(),mtime=0))
record={'label':'vm','argv':argv,'metadata_argv':argvmeta,'target_dir':env['CARGO_TARGET_DIR'],'exit':r.returncode,'seconds':round(time.monotonic()-start,3),'binary_sha256':{x['executable']:sha(Path(x['executable'])) for x in artifacts if x.get('executable') and x['profile']['test']},'compiled_local_packages':[{'id':x['id'],'manifest_path':x['manifest_path']} for x in packages],'local_package_source_inventory_count':len(local),'local_package_changed_sources':[k for k in local if local[k]!=after[k]],'stdout_sha256':sha(HERE/'vm-stdout.txt'),'stderr_sha256':sha(HERE/'vm-stderr.txt'),'scope':'Actual Cargo artifact IDs plus --no-deps local package metadata. Conservative local-package files, not all compiled. External deps reported in artifacts but no source-currency claim. Broad offline metadata failure retained separately.'}
validation=json.loads((HERE/'validation.json').read_text());validation['records'].append(record);validation['metadata_failure']='vm-metadata-failed01.json';(HERE/'validation.json').write_text(json.dumps(validation,indent=2)+'\n')
whole={k:sha(ROOT/k) for k in before};(HERE/'coordination-source-after-vm.json.gz').write_bytes(gzip.compress(json.dumps(whole,sort_keys=True).encode(),mtime=0));(HERE/'currency-vm.json').write_text(json.dumps({'count':len(before),'changed':[k for k in before if before[k]!=whole[k]],'scope':'Conservative coordinator inventory, not full compiled acceptance.'},indent=2)+'\n');print('END VM exit',r.returncode,'seconds',record['seconds'],'source changes',record['local_package_changed_sources'],flush=True)
