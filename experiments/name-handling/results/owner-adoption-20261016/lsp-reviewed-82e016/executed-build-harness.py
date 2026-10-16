import pathlib,hashlib,json,subprocess,datetime,time,re,shutil
root=pathlib.Path('/workspace/tcl-lsp');out=pathlib.Path('/workspace/naming-validation/lsp-reviewed-82e016');out.mkdir(exist_ok=False)
paths=[v['path'] for v in json.load(open('/workspace/naming-validation/lsp-source-scope-successor/input-before.json'))]
def snap():return [{'path':p,'sha256':hashlib.sha256((root/p).read_bytes()).hexdigest()} for p in paths]
before=snap();(out/'input-before.json').write_text(json.dumps(before,separators=(',',':'))+'\n')
packages=['tcl-compiler','tcl-lsp-core','tcl-lsp-db','tcl-registry','tcl-lsp-server']
cmd=['cargo','test','--offline']+sum((['-p',p] for p in packages),[])+['--lib','--no-run']
binding={'source_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'rust_diff_sha256':hashlib.sha256(subprocess.check_output(['git','diff','HEAD','--','rust','runtime','Cargo.toml','Cargo.lock'],cwd=root)).hexdigest(),'ordinary':True,'instrumentation':False,'single_job':True,'test_debug':0,'packages':packages,'status':'running','input_before_count':len(before),'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'command':'CARGO_PROFILE_TEST_DEBUG=0 '+' '.join(cmd),'scope':'Complete standalone directive input, conditional no-level aliases, authentic Authored SourceMay geometry with scalar/SSA refusal, actual Jim head/argument origins and preserved LSP validation controls. No Runtime/VM/BIGIP migration.'}
(out/'source-binding.json').write_text(json.dumps(binding,indent=2)+'\n');start=time.monotonic()
with (out/'build.log').open('w') as log:
 p=subprocess.Popen(cmd,cwd=root,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 for line in p.stdout:log.write(line);log.flush();print(line,end='',flush=True)
 code=p.wait()
after=snap();drift=[a['path'] for a,b in zip(before,after) if a!=b]
binding.update(status='PASS_FIVE_LIBRARY_BUILD' if code==0 and not drift else 'FAIL',exit_code=code,build_seconds=round(time.monotonic()-start,3),input_after_count=len(after),input_drift_count=len(drift),input_drift=drift,completed_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
(out/'source-binding.json').write_text(json.dumps(binding,indent=2)+'\n')
if code==0 and not drift:
 artifacts={};dest=out/'frozen-binaries';dest.mkdir()
 for crate,binary in re.findall(r'Executable unittests src/lib.rs \(target/debug/deps/([^-)]+)-([a-f0-9]+)\)',(out/'build.log').read_text()):
  name=crate+'-'+binary;src=root/'target/debug/deps'/name;tgt=dest/name;shutil.copy2(src,tgt);artifacts[crate]={'path':str(tgt),'sha256':hashlib.sha256(tgt.read_bytes()).hexdigest(),'bytes':tgt.stat().st_size}
 (out/'artifacts.json').write_text(json.dumps(artifacts,indent=2)+'\n')
print(json.dumps(binding),flush=True)
raise SystemExit(code)
