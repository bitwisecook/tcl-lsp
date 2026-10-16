from pathlib import Path
import sys,json,hashlib,os,shutil,subprocess,datetime
sys.dont_write_bytecode=True
pkg=Path('/tmp/lsp-cumulative-source-owners-selection-v5');sys.path.insert(0,str(pkg));import generate_selection as gen
root=Path('/workspace/tcl-lsp');d=Path('/workspace/naming-validation/lsp-expression-wasi-hierarchy-successor');plan=d/'selection-v5-frozen/selection.json';out=d/'retained726-results';receipt_path=d/'retained726-results-invocation.json';runner=root/'experiments/name-handling/validation/retained-selections/replay-selection.py';sha=gen.sha
assert sha(pkg/'package-manifest.json')=='7c3a36f0acbaf8de9713008fc2275aafb2193baf7a54c7d4249ced01731bc300'
assert sha(pkg/'generate_selection.py')=='94c9dcd7c183c0687f6191881a3da90e08a41f84bf5ae367882e7f858036f1c5'
assert sha(runner)=='aaa269431e0a0f3024367d0a2528b4671408e11cf7a57e801caa2429f181e6c3'
assert sha(plan)=='237d6e60703bdafb3836a3f1bf8009e578a6da3b9aca367061d0098cde379a80'
m=json.loads(plan.read_text());old=json.loads(__import__('gzip').decompress((pkg/'retained584.json.gz').read_bytes()));assert m['execution_order'][:584]==old['execution_order'];assert len(m['execution_order'])==m['total_exact_selectors']==726
for k in gen.GUARDS:assert m[k]==old[k]
assert m['test_timeout_seconds']==180 and m['listing_timeout_seconds']==30
required=json.loads((pkg/'provider-corrected-setup.json').read_text())['environment'];required['CARGO_PROFILE_TEST_DEBUG']='0'
for k,v in required.items():assert os.environ.get(k)==v,('Provider/debug environment mismatch',k)
assert not[k for k,v in os.environ.items()if v and(k.startswith('TCL_LSP_TRACE')or k=='TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC')]
for n in ['tclsh9.0','tclsh8.6']:assert shutil.which(n)=='/workspace/.toolchains/tcl/bin/'+n
assert sha(required['TCL_LSP_JIMSH'])=='c5b2b0c37c3bad8e97760350460310bde65e74cf9833979fe1c088e4dd2a4891'
facts=m['pending_source_owner_successor']['source_evidence'];binding=gen.freeze(d,gen.STANDARD,facts,root);assert binding==m['pending_source_owner_successor']['freeze_binding'];assert binding['full_input_count']==39002
argv=['python3',str(runner),'--manifest',str(plan),'--manifest-sha256',sha(plan),'--artifacts',str(d/'artifacts.json'),'--root',str(root),'--out',str(out)]
receipt={'status':'PREFLIGHT_PASS_RETAINED726','utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'argv':argv,'selected':726,'all584_ordered_rows_unchanged':True,'actual_successful_binding':binding,'all39002_current_inputs_and_five_actual_ELFs_checked':True,'provider_environment':required,'jim_binary_sha256':sha(required['TCL_LSP_JIMSH']),'canonical_runner_sha256':sha(runner),'scope':'Actual726 complete selection only; no extra Lexer/Syntax/xtask/host or gate acceptance. Native source remains frozen through postflight.'};receipt_path.write_text(json.dumps(receipt,indent=2)+'\n');code=1
try:code=subprocess.run(argv,cwd=root).returncode
finally:
 receipt['canonical_exit_code']=code
 try:
  assert gen.freeze(d,gen.STANDARD,facts,root)==binding
  for k,v in required.items():assert os.environ.get(k)==v
  assert sha(required['TCL_LSP_JIMSH'])==receipt['jim_binary_sha256'];assert sha(plan)=='237d6e60703bdafb3836a3f1bf8009e578a6da3b9aca367061d0098cde379a80';assert sha(runner)==receipt['canonical_runner_sha256'];receipt['all39002_input_and_all5_ELF_postflight_match']=True
 except Exception as e:receipt['postflight_error']=repr(e);code=1
 receipt['completed_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat();receipt_path.write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps({k:v for k,v in receipt.items()if k!='actual_successful_binding'}),flush=True)
raise SystemExit(code)
