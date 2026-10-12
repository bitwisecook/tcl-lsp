#!/usr/bin/env python3
from pathlib import Path
import gzip,hashlib,json,os,re,subprocess,time
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[3]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
checks=[('runtime','runtime/rust/Cargo.toml',None,'interp::native_body_artifact::native_string_trim::tests::original_string_trim_matches_all_216_native_header_windows','tmp/name-target-runtime'),('vm','Cargo.toml','tcl-vm','exec::native_string_trim_tests::original_string_trim_matches_all_216_native_header_windows','tmp/name-target-issues')]
records=[]
for label,manifest,package,test,cache in checks:
 env=dict(os.environ);env['CARGO_TARGET_DIR']=str(ROOT/cache)
 metadata_argv=['cargo','metadata','--offline','--format-version','1','--manifest-path',manifest]
 with (HERE/(label+'-metadata.json')).open('wb') as stdout,(HERE/(label+'-metadata.stderr.txt')).open('wb') as stderr:
  m=subprocess.run(metadata_argv,cwd=ROOT,env=env,stdout=stdout,stderr=stderr)
 if m.returncode!=0:
  records.append({'label':label,'metadata_exit':m.returncode});break
 metadata=json.loads((HERE/(label+'-metadata.json')).read_text())
 argv=['cargo','test','--offline','-j2','--manifest-path',manifest,*(['-p',package] if package else []),'--lib','--message-format=json',test,'--','--exact','--test-threads=1']
 print('START',label,test,flush=True);start=time.monotonic()
 with (HERE/(label+'-stdout.txt')).open('wb') as stdout,(HERE/(label+'-stderr.txt')).open('wb') as stderr:
  run=subprocess.run(argv,cwd=ROOT,env=env,stdout=stdout,stderr=stderr)
 artifacts=[]
 for line in (HERE/(label+'-stdout.txt')).read_text().splitlines():
  if line.startswith('{'):
   try:item=json.loads(line)
   except json.JSONDecodeError:continue
   if item.get('reason')=='compiler-artifact':artifacts.append(item)
 (HERE/(label+'-artifacts.json')).write_text(json.dumps(artifacts,indent=2)+'\n')
 tested=[item for item in artifacts if item.get('executable') and item['profile']['test']]
 bins={item['executable']:sha(Path(item['executable'])) for item in tested}
 actual_ids={item['package_id'] for item in artifacts};packages=[x for x in metadata['packages'] if x['id'] in actual_ids]
 source_before=json.loads(gzip.decompress((HERE/'coordination-source-before.json.gz').read_bytes()))
 prefix=[str(Path(x['manifest_path']).parent.relative_to(ROOT))+'/' for x in packages if Path(x['manifest_path']).is_relative_to(ROOT)]
 owned={k:v for k,v in source_before.items() if any(k.startswith(p) for p in prefix)}
 after={k:sha(ROOT/k) for k in owned}
 (HERE/(label+'-compiled-package-source-before.json.gz')).write_bytes(gzip.compress(json.dumps(owned,sort_keys=True).encode(),mtime=0))
 (HERE/(label+'-compiled-package-source-after.json.gz')).write_bytes(gzip.compress(json.dumps(after,sort_keys=True).encode(),mtime=0))
 records.append({'label':label,'argv':argv,'target_dir':str(ROOT/cache),'exit':run.returncode,'seconds':round(time.monotonic()-start,3),'binary_sha256':bins,'metadata_argv':metadata_argv,'compiled_packages':[{'id':x['id'],'manifest_path':x['manifest_path']} for x in packages],'local_package_source_inventory_count':len(owned),'local_package_changed_sources':[k for k in owned if owned[k]!=after[k]],'scope':'Cargo-reported artifact packages; cached fresh artifacts included. Local package inventory is conservative, not every file compiled. Compiler package acceptance only through this VM test graph. External Cargo source dependencies are installed inputs, not coordinator-owned sources.','stdout_sha256':sha(HERE/(label+'-stdout.txt')),'stderr_sha256':sha(HERE/(label+'-stderr.txt'))})
 (HERE/'validation.json').write_text(json.dumps({'records':records,'scope':'Existing exact runtime+VM216 windows; no fixtures/profile/assertions changed; no broader checks. Prior seven-file split readiness/receipts unchanged.'},indent=2)+'\n')
 print('END',label,'exit',run.returncode,'seconds',records[-1]['seconds'],'binaries',len(bins),'source-changes',records[-1]['local_package_changed_sources'],flush=True)
 if run.returncode!=0:break
before=json.loads(gzip.decompress((HERE/'coordination-source-before.json.gz').read_bytes()));after={k:sha(ROOT/k) for k in before};(HERE/'coordination-source-after.json.gz').write_bytes(gzip.compress(json.dumps(after,sort_keys=True).encode(),mtime=0));(HERE/'currency.json').write_text(json.dumps({'count':len(before),'changed':[k for k in before if before[k]!=after[k]],'scope':'Conservative coordinator source/fixture inventory; includes uncompiled owners, not overall compiled acceptance.'},indent=2)+'\n')
