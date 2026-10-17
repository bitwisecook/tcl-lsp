import os,pathlib,hashlib,json,subprocess,datetime,time,re,sys
assert not [k for k,v in os.environ.items()if v and(k.startswith('TCL_LSP_TRACE')or k=='TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC')], 'Ordinary build refuses diagnostic environment'
root=pathlib.Path('/workspace/tcl-lsp');out=pathlib.Path('/workspace/naming-validation/lsp-retained-report-action-source-successor');out.mkdir(exist_ok=False)
assert os.environ.get('CARGO_PROFILE_TEST_DEBUG')=='0' and os.environ.get('CARGO_BUILD_JOBS')=='1'
assert len(sys.argv)==2,'Pass the exact approved union manifest path'
union_manifest=pathlib.Path(sys.argv[1]);union_bytes=union_manifest.read_bytes();union=json.loads(union_bytes);union_files=union['files'];assert union_files
for item in union_files:
 current=root/item['path'];expected=item.get('after_sha256',item.get('after'));assert expected and hashlib.sha256(current.read_bytes()).hexdigest()==expected,item['path']
paths=sorted(set([v['path']for v in json.load(open('/workspace/naming-validation/lsp-current-source-owners-successor-repair3/input-before.json'))]+[v['path']for v in union_files]+[p for p in subprocess.check_output(['git','ls-files','-z','--','rust','runtime','scripts','specs','Cargo.toml','Cargo.lock','.cargo/config.toml'],cwd=root,text=True).split('\0')if p]))
def sha(path):
 with pathlib.Path(path).open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()
def snap():return [{'path':p,'sha256':sha(root/p)}for p in paths]
diff=subprocess.check_output(['git','diff','HEAD','--','rust','runtime','Cargo.toml','Cargo.lock'],cwd=root);(out/'source.diff').write_bytes(diff);before=snap();(out/'input-before.json').write_text(json.dumps(before,separators=(',',':'))+'\n')
packages=['tcl-compiler','tcl-lsp-core','tcl-lsp-db','tcl-registry','tcl-lsp-server'];cmd=['cargo','test','--offline','--locked']+sum((['-p',p]for p in packages),[])+['--lib','--no-run']
binding={'approved_union_manifest':str(union_manifest),'approved_union_manifest_sha256':hashlib.sha256(union_bytes).hexdigest(),'all_union_afterimages_match_before_build':True,'explicit_union_paths_in_source_snapshot':len(union_files),'source_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'rust_diff_sha256':hashlib.sha256(diff).hexdigest(),'ordinary':True,'instrumentation':False,'single_job':True,'test_debug':0,'packages':packages,'status':'running','input_before_count':len(before),'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'command':'CARGO_PROFILE_TEST_DEBUG=0 '+' '.join(cmd),'scope':'Actual reviewed current LSP owner/adopter source batch and necessary Compiler/DB/Registry only. Actual workspace dual original/fresh provider source lease, selected definition grammar cursor, original optimiser source/full worker/full report collection+DB context, both chat optimiser callers, completed diagnostic source/session/settings/samepublisher rejoin, and actual source-owner checker routes. Retained902+extra10 preserved; next added controls bind only after actual successful artifacts. Runtime/VM/BIGIP dedicated work deferred. No traces, budget changes or stronger Native/edit/absence/Normal grants.'};(out/'source-binding.json').write_text(json.dumps(binding,indent=2)+'\n');start=time.monotonic()
with(out/'build.log').open('w')as log:
 p=subprocess.Popen(cmd,cwd=root,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 for line in p.stdout:log.write(line);log.flush();print(line,end='',flush=True)
 code=p.wait()
after=snap();drift=[a['path']for a,b in zip(before,after)if a!=b];binding.update(status='PASS_FIVE_LIBRARY_BUILD'if code==0 and not drift else'FAIL',exit_code=code,build_seconds=round(time.monotonic()-start,3),input_after_count=len(after),input_drift_count=len(drift),input_drift=drift,completed_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
if code==0 and not drift:
 artifacts={};dest=out/'frozen-binaries';dest.mkdir();found=re.findall(r'Executable unittests src/lib.rs \(target/debug/deps/([^-)]+)-([a-f0-9]+)\)',(out/'build.log').read_text());assert{a for a,b in found}=={'tcl_compiler','tcl_lsp_core','tcl_lsp_db','tcl_registry','tcl_lsp_server'}and len(found)==5,found
 for crate,binary in found:
  name=crate+'-'+binary;src=root/'target/debug/deps'/name;tgt=dest/name;assert src.is_file()and not tgt.exists();digest=sha(src);size=src.stat().st_size
  # Cargo has exited. Relocate this exact owned artifact without a duplicate.
  src.rename(tgt);assert sha(tgt)==digest and tgt.stat().st_size==size;artifacts[crate]={'path':str(tgt),'sha256':digest,'bytes':size,'original_cargo_artifact':str(src),'storage':'Exact owned ELF relocated after Cargo completion; before/after SHA256 match.'}
  forbidden=[b'TCL_LSP_TRACE_ORIGINAL_STARTUP_INDEX_STAGES',b'TCL_LSP_TRACE_READONLY_MEMBER_STAGES',b'TCL_LSP_TRACE_COMMENT_CASE232'];tail=b''
  with tgt.open('rb')as f:
   for chunk in iter(lambda:f.read(1024*1024),b''):
    data=tail+chunk;assert not any(flag in data for flag in forbidden),'instrumented artifact';tail=data[-128:]
 (out/'artifacts.json').write_text(json.dumps(artifacts,indent=2)+'\n');binding['all_five_artifact_relocations_hash_verified']=True
(out/'source-binding.json').write_text(json.dumps(binding,indent=2)+'\n');print(json.dumps(binding),flush=True);raise SystemExit(code)
