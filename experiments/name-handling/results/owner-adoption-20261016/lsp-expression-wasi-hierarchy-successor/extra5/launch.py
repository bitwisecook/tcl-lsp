from pathlib import Path
import sys,json,hashlib,os,subprocess,datetime,shutil
sys.dont_write_bytecode=True;sys.path.insert(0,'/tmp/lsp-cumulative-source-owners-selection-v5');import generate_selection as gen
root=Path('/workspace/tcl-lsp');parent=Path('/workspace/naming-validation/lsp-expression-wasi-hierarchy-successor');plan=parent/'selection-v5-all-targets-frozen';runner=root/'experiments/name-handling/validation/retained-selections/replay-selection.py';assert gen.sha(runner)=='aaa269431e0a0f3024367d0a2528b4671408e11cf7a57e801caa2429f181e6c3';required=json.loads(Path('/tmp/lsp-cumulative-source-owners-selection-v5/provider-corrected-setup.json').read_text())['environment'];required['CARGO_PROFILE_TEST_DEBUG']='0'
def envcheck():
 for k,v in required.items():assert os.environ.get(k)==v
 assert not[k for k,v in os.environ.items()if v and(k.startswith('TCL_LSP_TRACE')or k=='TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC')]
 for n in ['tclsh9.0','tclsh8.6']:assert shutil.which(n)=='/workspace/.toolchains/tcl/bin/'+n
 assert gen.sha(required['TCL_LSP_JIMSH'])=='c5b2b0c37c3bad8e97760350460310bde65e74cf9833979fe1c088e4dd2a4891'
def check():
 envcheck();bindings={}
 for target,pkg,expected in [('tcl_lexer','tcl-lexer',3),('tcl_syntax','tcl-syntax',1),('xtask','xtask',1)]:
  p=plan/f'extra-{target}-selection.json';m=json.loads(p.read_text());assert m['total_exact_selectors']==expected and set(m['groups'])=={target}and m['test_timeout_seconds']==180 and m['listing_timeout_seconds']==30
  b=gen.freeze(parent/f'extra-{pkg}',[target],m['source_evidence'],root);assert all(m['freeze_binding'][k]==v for k,v in b.items());bindings[target]={'manifest':str(p),'sha256':gen.sha(p),'binding':b,'expected':expected,'artifact_manifest':str(parent/f'extra-{pkg}/artifacts.json')}
 return bindings
bindings=check();receipt={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'PREFLIGHT_PASS_DISTINCT_EXTRA5','bindings':bindings,'required_environment':required,'canonical_runner_sha256':gen.sha(runner),'scope':'Three actual distinct test artifacts; exactly five controls, outside ordinary726. No host/global gate acceptance.','runs':[]};path=parent/'extra5-results-invocation.json';path.write_text(json.dumps(receipt,indent=2)+'\n');code=0
try:
 for target,v in bindings.items():
  out=parent/f'extra5-{target}-results';argv=['python3',str(runner),'--manifest',v['manifest'],'--manifest-sha256',v['sha256'],'--artifacts',v['artifact_manifest'],'--root',str(root),'--out',str(out)];rc=subprocess.run(argv,cwd=root).returncode;receipt['runs'].append({'target':target,'argv':argv,'exit_code':rc});code=code or rc;path.write_text(json.dumps(receipt,indent=2)+'\n')
finally:
 receipt['canonical_exit_code']=code
 try:assert check()==bindings;receipt['all39002_sources_and_all3_extra_ELF_postflight_match']=True
 except Exception as e:receipt['postflight_error']=repr(e);code=1
 receipt['completed_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat();path.write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps({k:v for k,v in receipt.items()if k!='bindings'}),flush=True)
raise SystemExit(code)
