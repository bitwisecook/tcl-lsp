import pathlib,hashlib,json,subprocess,datetime,time,os
base=pathlib.Path('/tmp/lsp-editor-ts-offtree-json-20261023');out=base/'node26';out.mkdir();actual=pathlib.Path('/workspace/naming-migration-active/tcl-lsp/editors/vscode');shadow=base/'vscode';node=pathlib.Path('/workspace/naming-toolchains/node26/bin/node')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def inputs(root,dependencies):
 files=[p for d in (['src','node_modules'] if dependencies else ['src']) for p in (root/d).rglob('*') if p.is_file()];files.extend(p for p in root.iterdir() if p.is_file());return {str(p.relative_to(root)):sha(p) for p in sorted(set(files))}
before=inputs(actual,True);sb=inputs(shadow,False);assert all(before[k]==v for k,v in json.loads((base/'actual-inputs-before.json').read_text()).items());(out/'actual-before.json').write_text(json.dumps(before,sort_keys=True)+'\n');(out/'shadow-before.json').write_text(json.dumps(sb,sort_keys=True)+'\n');version=subprocess.check_output([str(node),'--version'],text=True).strip();assert version=='v26.12.1'
args=[str(node),str(actual/'node_modules/typescript/bin/tsc'),'--noEmit','--incremental','false','--pretty','false','--project','tsconfig.json','--listFiles'];log=out/'typescript.log';start=datetime.datetime.now(datetime.timezone.utc).isoformat();over=False
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
(out/'tsc-actual-inputs.json').write_text(json.dumps(listed,indent=2)+'\n');after=inputs(actual,True);sa=inputs(shadow,False);(out/'actual-after.json').write_text(json.dumps(after,sort_keys=True)+'\n');(out/'shadow-after.json').write_text(json.dumps(sa,sort_keys=True)+'\n');prior=json.loads((base/'tsc-actual-inputs.json').read_text());r={'actual_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'started_utc':start,'scope':'Supported Node26 OFFTREE two-JSON proposed layout TypeScript only; no shipping/host admission','node_version':version,'node_path':str(node),'node_resolved_path':str(node.resolve()),'node_sha256':sha(node),'argv':args,'exit_code':code,'budget_exceeded':over,'log_bytes':log.stat().st_size,'log_sha256':sha(log),'actual_input_count':len(before),'actual_unchanged':before==after,'shadow_unchanged':sb==sa,'tsc_input_count':len(listed),'tsc_inputs_equal_prior_node24':listed==prior,'non_input_output':other};(out/'receipt.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2))
