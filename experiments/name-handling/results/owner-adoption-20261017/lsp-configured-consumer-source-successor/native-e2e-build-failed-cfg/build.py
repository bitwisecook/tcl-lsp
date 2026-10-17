from pathlib import Path
import datetime,hashlib,json,os,subprocess,time
r=Path('/workspace/tcl-lsp');b=Path('/workspace/naming-validation/lsp-configured-consumer-source-successor-repair2');out=b/'native-e2e-build';out.mkdir(exist_ok=False)
assert os.environ.get('CARGO_BUILD_JOBS')=='1'and os.environ.get('CARGO_PROFILE_TEST_DEBUG')=='0'
verify=['python3','/tmp/lsp-configured-extra18-binder-template-v2/launch.py','--package-manifest','/tmp/lsp-configured-extra18-binder-template-v2/package.json','--package-sha256','a2ae0ed082e170001c4d35e5019ba1bef1626b6998ec880897b5d8fb5d038c4e','--binding-manifest',str(b/'extra18-bound-v1/manifest.json'),'--binding-sha256','b7b25db08ebe73578e2aff3711e8eff4770b6843188ff14cd26301f0b6bd60f0','--verify-only']
def check():
 p=subprocess.run(verify,cwd=r,text=True,capture_output=True);assert p.returncode==0,(p.stdout,p.stderr);j=json.loads(p.stdout);assert j['status']=='STRICT_EXTRA18_PREFLIGHT_PASS';return j
sha=lambda p:hashlib.file_digest(Path(p).open('rb'),'sha256').hexdigest()
before=check();(out/'before.json').write_text(json.dumps(before,indent=2)+'\n');cmd=['cargo','test','--offline','--locked','-p','tcl-lsp-server','--test','e2e','--no-run','--message-format=json-render-diagnostics'];receipt={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'RUNNING','argv':cmd,'ordinary':True,'instrumentation':False,'single_job':True,'scope':'Actual native LSP e2e test and server binary build only; no tests executed. BIG-IP/Runtime/VM suites deferred. Original39058 source/all9 actual ELF/provider bindings checked before/after.'};(out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');start=time.monotonic();built=[]
with(out/'build.log').open('w')as log:
 proc=subprocess.Popen(cmd,cwd=r,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 for line in proc.stdout:
  log.write(line);log.flush()
  try:
   item=json.loads(line)
   if item.get('reason')=='compiler-artifact'and item.get('executable'):built.append(item)
   if item.get('reason')=='compiler-message':print(item.get('message',{}).get('rendered',''),end='',flush=True)
  except json.JSONDecodeError:print(line,end='',flush=True)
 code=proc.wait()
receipt.update(exit_code=code,seconds=round(time.monotonic()-start,3),completed_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),actual_executable_cargo_messages=built)
try:
 after=check();(out/'after.json').write_text(json.dumps(after,indent=2)+'\n');assert before==after;receipt['all39058_source_nine_ELFs_provider_pins_prepost_match']=True
except Exception as error:receipt['postflight_error']=repr(error)
receipt['status']='PASS_NATIVE_E2E_BUILD'if code==0 and receipt.get('all39058_source_nine_ELFs_provider_pins_prepost_match')else'FAIL_NATIVE_E2E_BUILD'
if receipt['status']=='PASS_NATIVE_E2E_BUILD':
 selected=[x for x in built if x['target']['name']in ['e2e','tcl-lsp-server']];assert {x['target']['name']for x in selected}=={'e2e','tcl-lsp-server'};dest=out/'frozen-binaries';dest.mkdir();artifacts={}
 for item in selected:
  p=Path(item['executable']);n=item['target']['name'];digest=sha(p);size=p.stat().st_size;q=dest/(n+'-'+digest[:16]);os.link(p,q);assert q.read_bytes()[:4]==b'\x7fELF'and sha(q)==digest;artifacts[n]={'path':str(q),'sha256':digest,'bytes':size,'original_cargo_artifact':str(p),'storage':'Hard-linked exact actual executable after Cargo exit; original server path retained for embedded e2e harness.'}
 (out/'artifacts.json').write_text(json.dumps(artifacts,indent=2)+'\n');receipt['artifacts']=artifacts
(out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2),flush=True);raise SystemExit(0 if receipt['status']=='PASS_NATIVE_E2E_BUILD' else 1)
