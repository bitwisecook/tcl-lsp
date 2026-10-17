from pathlib import Path
import json,hashlib,subprocess,datetime,sys,collections
r=Path('/workspace/tcl-lsp');bound=Path('/workspace/naming-validation/configured-consumer994-bound-repair2-v1');dest=Path('/workspace/naming-validation/lsp-configured-consumer-source-successor-repair2');sha=lambda p:hashlib.file_digest(Path(p).open('rb'),'sha256').hexdigest();assert len(sys.argv)==2 and sha(bound/'manifest.json')==sys.argv[1];bm=json.loads((bound/'manifest.json').read_text())
for key in ['admission','launcher','plan','source_evidence','actual_successful_build','source_package','binding_preflight']:
 rec=bm[key];assert sha(rec['path'])==rec['sha256'],key
ad=bm['admission'];preflight_argv=[sys.executable,str(bound/'launch.py'),'--admission',ad['path'],'--admission-sha256',ad['sha256'],'--verify-only']
def check():
 assert sha(bound/'manifest.json')==sys.argv[1]
 p=subprocess.run(preflight_argv,cwd=r,stdout=subprocess.PIPE,stderr=subprocess.PIPE);assert p.returncode==0,p.stderr.decode(errors='replace');j=json.loads(p.stdout);assert j['status']=='STRICT994_BINDING_PREFLIGHT_ONLY_PASS';return j
before=check();plan=json.loads((bound/'selection.json').read_text());assert len(plan['execution_order'])==994
names=['source_structure::tests::original_definition_cursor_source_owners_reach_nested_spectcl_command_bodies','studio_source::retirement_publication_tests::retire_and_shutdown_cannot_acknowledge_failed_withdrawal_and_can_retry'];rows=[x for x in plan['execution_order']if x['selector']in names];assert len(rows)==2 and{v['selector']for v in rows}==set(names)
groups={}
for v in rows:groups.setdefault(v['target'],[]).append(v['selector'])
selected={**plan,'group':'Focused existing parent-transport and Studio retirement controls before mandatory full994; no retained obligation discharged','execution_order':rows,'groups':groups,'total_exact_selectors':2,'expected_group_counts':{k:len(v)for k,v in groups.items()},'scope':'Focused actual source-owner repair check only. Full994+extra18 and all existing changed integration obligations remain owed; no ordinary/verified acceptance.'}
for k in ['test_timeout_seconds','listing_timeout_seconds','environment','runtime_guards','validation_note','host_scope']:assert selected[k]==plan[k]
assert selected['test_timeout_seconds']==180 and selected['listing_timeout_seconds']==30
out=dest/'focused-owner2';out.mkdir(exist_ok=False);mp=out/'selection.json';mp.write_text(json.dumps(selected,indent=2)+chr(10));canonical=r/'experiments/name-handling/validation/retained-selections/replay-selection.py';assert sha(canonical)=='aaa269431e0a0f3024367d0a2528b4671408e11cf7a57e801caa2429f181e6c3';argv=[sys.executable,str(canonical),'--manifest',str(mp),'--manifest-sha256',sha(mp),'--artifacts',str(dest/'artifacts.json'),'--root',str(r),'--out',str(out/'results')]
receipt={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'STRICT994_PREFLIGHT_PASS_FOCUSED2_RUNNING','selected':2,'mandatory_full994_still_owed':True,'bound_manifest_sha256':sys.argv[1],'before':before,'argv':argv,'ordinary_acceptance':False};rp=out/'invocation.json';rp.write_text(json.dumps(receipt,indent=2)+chr(10));code=1
try:code=subprocess.run(argv,cwd=r).returncode
finally:
 receipt['canonical_exit_code']=code;after=check();receipt['after']=after;assert before==after;receipt.update(status='FOCUSED2_COMPLETED_FULL994_SOURCE_PROVIDER_FIVE_ELF_PREFLIGHTS_MATCH',completed_utc=datetime.datetime.now(datetime.timezone.utc).isoformat());result=json.loads((out/'results/result.json').read_text());assert len(result['runs'])==2;receipt['counts']=dict(collections.Counter('TIMEOUT'if x['timed_out']else'PASS'if x['test_passed']else'FAIL'for x in result['runs']));receipt['harness_errors']=result['errors'];rp.write_text(json.dumps(receipt,indent=2)+chr(10));print(json.dumps(receipt),flush=True)
raise SystemExit(code)
