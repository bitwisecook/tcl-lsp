import pathlib,hashlib,json,subprocess,datetime,time,re,shutil
root=pathlib.Path('/workspace/tcl-lsp');out=pathlib.Path('/workspace/naming-validation/lsp-source-order-member-head-successor');out.mkdir(exist_ok=False)
paths=sorted(set([v['path'] for v in json.load(open('/workspace/naming-validation/lsp-reviewed-82e016-retry4/input-before.json'))]+[p for p in subprocess.check_output(['git','ls-files','-z','--','rust','runtime','scripts','specs','Cargo.toml','Cargo.lock','.cargo/config.toml'],cwd=root,text=True).split('\0') if p]))
def snap():return [{'path':p,'sha256':hashlib.sha256((root/p).read_bytes()).hexdigest()} for p in paths]
(out/'source.diff').write_bytes(subprocess.check_output(['git','diff','HEAD','--','rust','runtime','Cargo.toml','Cargo.lock'],cwd=root))
before=snap();(out/'input-before.json').write_text(json.dumps(before,separators=(',',':'))+'\n')
packages=['tcl-compiler','tcl-lsp-core','tcl-lsp-db','tcl-registry','tcl-lsp-server']
cmd=['cargo','test','--offline','--locked']+sum((['-p',p] for p in packages),[])+['--lib','--no-run']
binding={'source_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'rust_diff_sha256':hashlib.sha256(subprocess.check_output(['git','diff','HEAD','--','rust','runtime','Cargo.toml','Cargo.lock'],cwd=root)).hexdigest(),'ordinary':True,'instrumentation':False,'single_job':True,'test_debug':0,'packages':packages,'status':'running','input_before_count':len(before),'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'command':'CARGO_PROFILE_TEST_DEBUG=0 '+' '.join(cmd),'scope':'Reviewed actual conditional source-order owner/allWorkspace+Definitionadopters and genuine dual-head-issuer member correspondence successor; allNative/edit/Normalguards and oldassertions retained. Previous reviewed LSP isolated proposal/actual recovery-provider ledger/fullcontext/editor adapters and selected minifier syntax census successor; instrumentation restored on39000inputs. Includes14new actual provider/recovery/poison/endpoint/minifier controls, retains all446 for future replay. Previous source geometry: exact full-context UTF16 cursor and body/scope visibility; current-analysis original declaration outline with actual nonzero overlay Registry; genuine original formal-name minifier geometry under independent edit barriers; selected source package positive/incomplete census and shared typed syntax coverage; genuine earlier-Written operand continuity and static original anchor for Logical document links. Two exact UFCS call spellings and accurate private/public/internal contract routes preserve all guards; Registry2048-source traversal gate stays open. All original338controls and additional scoped controls retained for future replay. Runtime/VM/BIG-IP dedicated work paused; no execution/Normal/absence/edit authority from source geometry.'}
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
 assert all(not any(flag in pathlib.Path(v['path']).read_bytes() for flag in [b'TCL_LSP_TRACE_ORIGINAL_STARTUP_INDEX_STAGES',b'TCL_LSP_TRACE_READONLY_MEMBER_STAGES',b'TCL_LSP_TRACE_COMMENT_CASE232']) for v in artifacts.values()), 'instrumented artifact must not be reused'
 (out/'artifacts.json').write_text(json.dumps(artifacts,indent=2)+'\n')
print(json.dumps(binding),flush=True)
raise SystemExit(code)
