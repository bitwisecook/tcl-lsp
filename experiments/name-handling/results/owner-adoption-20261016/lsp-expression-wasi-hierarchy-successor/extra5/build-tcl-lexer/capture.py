from pathlib import Path
import json,subprocess,hashlib,datetime,time,shutil,os,re
root=Path('/workspace/tcl-lsp');parent=Path('/workspace/naming-validation/lsp-expression-wasi-hierarchy-successor');inputs=json.loads((parent/'input-before.json').read_text());paths=[x['path']for x in inputs]
def sha(p):
 with Path(p).open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()
def snap():return[{'path':p,'sha256':sha(root/p)}for p in paths]
assert os.environ['CARGO_PROFILE_TEST_DEBUG']=='0'and os.environ['CARGO_BUILD_JOBS']=='1';assert snap()==inputs
for pkg,kind,entry in [('tcl-lexer','lib','src/lib.rs'),('tcl-syntax','lib','src/lib.rs'),('xtask','bin:xtask','src/main.rs')]:
 out=parent/('extra-'+pkg);out.mkdir();before=snap();assert before==inputs;(out/'input-before.json').write_text(json.dumps(before,separators=(',',':'))+'\n');diff=subprocess.check_output(['git','diff','HEAD','--','rust','runtime','Cargo.toml','Cargo.lock'],cwd=root);(out/'source.diff').write_bytes(diff);argv=['cargo','test','--offline','--locked','-p',pkg]+(['--lib']if kind=='lib'else['--bin','xtask'])+['--no-run'];b={'ordinary':True,'instrumentation':False,'single_job':True,'test_debug':0,'packages':[pkg],'target_kind':kind,'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'rust_diff_sha256':hashlib.sha256(diff).hexdigest(),'input_before_count':len(before),'status':'running','command':argv,'scope':'Distinct actual unit-test artifact only, no production-dependency substitute or execution acceptance'};(out/'source-binding.json').write_text(json.dumps(b,indent=2)+'\n');start=time.monotonic()
 with(out/'build.log').open('w')as log:
  p=subprocess.Popen(argv,cwd=root,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  for line in p.stdout:log.write(line);log.flush();print(line,end='',flush=True)
  code=p.wait()
 after=snap();drift=[a['path']for a,z in zip(before,after)if a!=z];b.update(exit_code=code,build_seconds=round(time.monotonic()-start,3),input_after_count=len(after),input_drift_count=len(drift),input_drift=drift,status='PASS_DISTINCT_TEST_ARTIFACT_BUILD'if code==0 and not drift else'FAIL',completed_utc=datetime.datetime.now(datetime.timezone.utc).isoformat());(out/'source-binding.json').write_text(json.dumps(b,indent=2)+'\n');print(json.dumps(b),flush=True)
 if code or drift:raise SystemExit(code or 1)
 frozen=out/'frozen-binaries';frozen.mkdir();text=(out/'build.log').read_text();found=re.findall(r'Executable unittests '+re.escape(entry)+r' \(target/debug/deps/([^-]+)-([a-f0-9]+)\)',text);assert len(found)==1,found;crate,h=found[0];src=root/'target/debug/deps'/(crate+'-'+h);dest=frozen/src.name;shutil.copy2(src,dest);a={crate:{'path':str(dest),'bytes':dest.stat().st_size,'sha256':sha(dest)}};assert not any(flag in dest.read_bytes()for flag in[b'TCL_LSP_TRACE_ORIGINAL_STARTUP_INDEX_STAGES',b'TCL_LSP_TRACE_READONLY_MEMBER_STAGES',b'TCL_LSP_TRACE_COMMENT_CASE232']);(out/'artifacts.json').write_text(json.dumps(a,indent=2)+'\n')
