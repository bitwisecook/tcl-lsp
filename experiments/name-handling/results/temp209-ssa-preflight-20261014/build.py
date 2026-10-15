from pathlib import Path
import datetime,gzip,hashlib,json,os,subprocess,time
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-temp209-preflight-20261014');out.mkdir(exist_ok=True);assert not any(out.iterdir())
def now():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def snap():
 names=subprocess.check_output(['rg','--files','--hidden',*[n for n in ['rust','runtime/rust','data','.cargo'] if (root/n).exists()]],cwd=root,text=True).splitlines()+['Cargo.toml','Cargo.lock','rust-toolchain.toml']
 return {n:sha(root/n) for n in sorted(set(names)) if (root/n).is_file()}
def save(n,d):
 with gzip.open(out/n,'wt') as f:json.dump(d,f,sort_keys=True)
a=snap();save('source-before.json.gz',a)
cmd=['cargo','test','--offline','--no-run','--message-format=json','-j1','-p','tcl-compiler','-p','tcl-lsp-core','-p','tcl-lsp-db','-p','xtask','-p','tcl-syntax','-p','bpf-tcl-ir','-p','tcl-vm','--lib','--bin','xtask']
start={'utc':now(),'argv':cmd,'target':os.environ['CARGO_TARGET_DIR'],'scope':'TEMP209 seven affected-consumer test artifacts with bounded SSA equation instrumentation after typed callback correction, full semantic bundle comparison transport, literal member handle and typed Jim namespace migrations. No tests executed by build. Original #2286 selector separately retains exact fixture and 180-second host budget. Temporary instrumentation is not production acceptance or full Native/provider closure.'};(out/'start.json').write_text(json.dumps(start,indent=2)+'\n');t=time.monotonic()
with (out/'build.jsonl').open('wb') as f:code=subprocess.run(cmd,cwd=root,stdout=f,stderr=subprocess.STDOUT).returncode
b=snap();save('source-after.json.gz',b);errors=[];artifacts=[]
for line in (out/'build.jsonl').read_text().splitlines():
 try:r=json.loads(line)
 except ValueError:continue
 if r.get('reason')=='compiler-message' and r['message']['level']=='error':errors.append(r['message'])
 if r.get('reason')=='compiler-artifact' and r.get('profile',{}).get('test') and r.get('executable'):artifacts.append({'path':r['executable'],'sha256':sha(Path(r['executable']))})
result={'started':start,'finished_utc':now(),'exit_code':code,'seconds':time.monotonic()-t,'executed_tests':0,'changed_sources':{n:{'before':a.get(n),'after':b.get(n)} for n in a.keys()|b.keys() if a.get(n)!=b.get(n)},'errors':errors,'test_artifacts':artifacts};(out/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'exit_code':code,'errors':[(x.get('code'),x['message']) for x in errors],'test_artifacts':len(artifacts),'changed_sources':len(result['changed_sources'])}),flush=True)
