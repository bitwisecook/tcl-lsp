from pathlib import Path
import datetime,gzip,hashlib,json,os,subprocess,time
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-main-combined-api-check-20261013-0007-13');out.mkdir(exist_ok=False)
def digest(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def inventory():
 names=subprocess.check_output(['rg','--files','--hidden',*[n for n in ['rust','runtime/rust','data'] if (root/n).exists()]],cwd=root,text=True).splitlines()
 names+=['Cargo.toml','Cargo.lock','rust-toolchain.toml']
 return {n:digest(root/n) for n in names if (root/n).is_file()}
def save(n,r):
 with gzip.open(out/n,'wt') as f:json.dump(r,f,sort_keys=True)
before=inventory();save('source-before.json.gz',before)
command=['cargo','check','--offline','-j','1','-p','tcl-lsp-db','-p','tcl-lsp-core','-p','tcl-explorer','-p','tcl-diagram','-p','tcl-spec-studio','-p','tcl-vm','-p','tcl-cli','-p','tcl-bigip','-p','tcl-irules','-p','tcl-lsp-server','-p','tcl-debugger','--message-format=json']
start=datetime.datetime.now(datetime.timezone.utc).isoformat();t=time.monotonic()
with (out/'build.log').open('wb') as f:code=subprocess.run(command,cwd=root,stdout=f,stderr=subprocess.STDOUT).returncode
after=inventory();save('source-after.json.gz',after)
messages=[];artifacts=[]
for line in (out/'build.log').read_text().splitlines():
 try:row=json.loads(line)
 except json.JSONDecodeError:continue
 if row.get('reason')=='compiler-message':messages.append(row)
 if row.get('reason')=='compiler-artifact':artifacts.append(row)
(out/'artifacts.json').write_text(json.dumps(artifacts,indent=2)+'\n')
result={'command':command,'started_utc':start,'finished_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'exit_code':code,'seconds':time.monotonic()-t,'source_paths':len(before),'changed_sources':{n:{'before':before.get(n),'after':after.get(n)} for n in set(before)|set(after) if before.get(n)!=after.get(n)},'errors':[r['message']['rendered'] for r in messages if r['message']['level']=='error'],'scope':'Combined current Main consumer API typecheck; captured authored source hashes, not complete external dependency/oracle closure. No executed tests or Native/VM/Runtime conformance acceptance.','environment':{n:os.environ.get(n) for n in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS','CARGO_PROFILE_DEV_DEBUG','CARGO_INCREMENTAL']}}
(out/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result),flush=True)
