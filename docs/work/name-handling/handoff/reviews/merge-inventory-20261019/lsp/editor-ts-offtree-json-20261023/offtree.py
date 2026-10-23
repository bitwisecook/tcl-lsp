import pathlib,json,hashlib,subprocess,datetime,time,os,shutil
base=pathlib.Path('/tmp/lsp-editor-ts-offtree-json-20261023');actual=pathlib.Path('/workspace/naming-migration-active/tcl-lsp/editors/vscode');repo=actual.parents[1];shadow=base/'vscode';base.mkdir(exist_ok=True);shadow.mkdir()
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def inputs():
 files=[p for d in ['src','node_modules'] for p in (actual/d).rglob('*') if p.is_file()]
 files.extend(p for p in actual.iterdir() if p.is_file() and (p.suffix in {'.json','.js','.cjs','.mjs','.yaml','.yml'} or p.name.startswith('.prettier')))
 return {str(p.relative_to(actual)):sha(p) for p in sorted(set(files))}
before=inputs();(base/'actual-inputs-before.json').write_text(json.dumps(before,sort_keys=True,indent=2)+'\n')
shutil.copytree(actual/'src',shadow/'src')
for p in actual.iterdir():
 if p.is_file():shutil.copyfile(p,shadow/p.name)
(shadow/'node_modules').symlink_to(actual/'node_modules',target_is_directory=True)
additions={}
for src,dst in [('ai/shared/diagnostics.json','diagnostics.json'),('ai/prompts/manifest.json','manifest.json')]:
 target=shadow/'src/chat/canonical'/dst;target.parent.mkdir(exist_ok=True);assert not target.exists();shutil.copyfile(repo/src,target);additions[str(target.relative_to(shadow))]={'origin':str(repo/src),'sha256':sha(target)}
def sourcefiles(root):return {str(p.relative_to(root)):sha(p) for p in (root/'src').rglob('*') if p.is_file()}|{p.name:sha(p) for p in root.iterdir() if p.is_file()}
canonical=sourcefiles(actual);proposed=sourcefiles(shadow);assert {k:v for k,v in proposed.items() if k not in additions}==canonical
for p in actual.rglob('*.ts'):
 if not any(x in {'node_modules','.vscode-test','out'} for x in p.relative_to(actual).parts):assert p.relative_to(actual).parts[0]=='src',str(p)
args=[str(pathlib.Path(shutil.which('node')).resolve()),str(actual/'node_modules/typescript/bin/tsc'),'--noEmit','--incremental','false','--pretty','false','--project','tsconfig.json','--listFiles'];log=base/'typescript.log';started=datetime.datetime.now(datetime.timezone.utc).isoformat();over=False
with log.open('wb') as f:
 p=subprocess.Popen(args,cwd=shadow,stdout=f,stderr=subprocess.STDOUT,start_new_session=True);t=time.monotonic()
 while p.poll() is None:
  if time.monotonic()-t>180 or log.stat().st_size>20000000:over=True;os.killpg(p.pid,9);break
  time.sleep(.1)
 code=p.wait()
listed=[];other=[]
for line in log.read_text().splitlines():
 q=pathlib.Path(line)
 if q.is_absolute() and q.is_file():listed.append({'tsc_path':line,'resolved_path':str(q.resolve()),'sha256':sha(q)})
 else:other.append(line)
(base/'tsc-actual-inputs.json').write_text(json.dumps(listed,indent=2)+'\n');after=inputs();(base/'actual-inputs-after.json').write_text(json.dumps(after,sort_keys=True,indent=2)+'\n');assert sourcefiles(shadow)==proposed
receipt={'actual_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'OFFTREE proposed two-generated-JSON layout TypeScript check only; no shipping/editor/browser/runtime admission','started_utc':started,'argv':args,'cwd':str(shadow),'exit_code':code,'budget_exceeded':over,'log_bytes':log.stat().st_size,'log_sha256':sha(log),'tsc_input_count':len(listed),'non_input_output':other,'canonical_input_count':len(before),'canonical_inputs_unchanged':before==after,'shadow_source_config_exact_except':additions,'default_discovered_typescript_outside_src':False,'shadow_inputs_unchanged':True,'node_executable_sha256':sha(pathlib.Path(args[0]))};(base/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
