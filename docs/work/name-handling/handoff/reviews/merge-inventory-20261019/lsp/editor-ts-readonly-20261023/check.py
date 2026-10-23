import pathlib,json,hashlib,subprocess,datetime,time,os
base=pathlib.Path('/tmp/lsp-editor-ts-readonly-20261023');root=pathlib.Path('/workspace/naming-migration-active/tcl-lsp/editors/vscode')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def inputs():
 files=[]
 for d in ['src','node_modules']:
  files.extend(p for p in (root/d).rglob('*') if p.is_file())
 files.extend(p for p in root.iterdir() if p.is_file() and (p.suffix in {'.json','.js','.cjs','.mjs','.yaml','.yml'} or p.name.startswith('.prettier')))
 return {str(p.relative_to(root)):sha(p) for p in sorted(set(files))}
before=inputs();(base/'inputs-before.json').write_text(json.dumps(before,sort_keys=True,indent=2)+'\n');results=[]
for label,args in [('typescript',['node','node_modules/typescript/bin/tsc','--noEmit','--incremental','false','--pretty','false','--project','tsconfig.json']),('prettier',['node','node_modules/prettier/bin/prettier.cjs','--check','src/nameGeneration.ts'])]:
 start=datetime.datetime.now(datetime.timezone.utc).isoformat(); log=base/(label+'.log'); exceeded=False
 with log.open('wb') as f:
  p=subprocess.Popen(args,cwd=root,stdout=f,stderr=subprocess.STDOUT,start_new_session=True);t=time.monotonic()
  while p.poll() is None:
   if time.monotonic()-t>180 or log.stat().st_size>20000000:
    exceeded=True;os.killpg(p.pid,9);break
   time.sleep(.1)
  code=p.wait()
 results.append({'label':label,'argv':args,'started_utc':start,'finished_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'exit_code':code,'budget_exceeded':exceeded,'log_bytes':log.stat().st_size,'log_sha256':sha(log)})
after=inputs();(base/'inputs-after.json').write_text(json.dumps(after,sort_keys=True,indent=2)+'\n');r={'actual_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'Non-writing local TypeScript and single-file formatting checks only; no editor/native/browser runtime or shipping claim','cwd':str(root),'input_count':len(before),'inputs_unchanged':before==after,'changed_inputs':[k for k in before.keys()|after.keys() if before.get(k)!=after.get(k)],'results':results};(base/'receipt.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2))
