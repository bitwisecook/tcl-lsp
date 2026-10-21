from pathlib import Path
import datetime,hashlib,json,os,subprocess,sys

from _paths import R,W,CHECKPOINT09
import tarfile
C=W/'cp09'
if not (C/'selections').is_dir():
 with tarfile.open(CHECKPOINT09) as t:t.extractall(C,members=[m for m in t.getmembers() if m.name.startswith('selections/')],filter='data')
B=W/sys.argv[2];O=W/sys.argv[1]
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
assert json.loads((B/'build-result.json').read_text())['exit']==0
O.mkdir(exist_ok=False)
env=os.environ.copy();env.update(TCL_LSP_REQUIRE_TCLSH='9.0,8.6',TCL_LSP_JIMSH=str(R/'tmp/jimtcl-oracle/jimsh'))
artifacts=json.loads((B/'artifacts.json').read_text())
for item in artifacts.values():assert sha(item['path'])==item['sha256']
for script,name,reference in [('capture-inputs.py','pre-inputs.json','build-inputs.json'),('capture-providers.py','pre-providers.json','build-providers.json')]:
 subprocess.run([sys.executable,str(Path(__file__).resolve().parent/script),str(O/name)],env=env,check=True)
 assert (O/name).read_bytes()==(B/reference).read_bytes(),reference+' drift before LSP execution'
remote=subprocess.run(['git','ls-remote','https://github.com/msteveb/jimtcl.git','HEAD','refs/heads/master'],text=True,capture_output=True,timeout=60)
(O/'jim-remote-at-execution.json').write_text(json.dumps({'observed_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'argv':remote.args,'exit':remote.returncode,'stdout':remote.stdout,'stderr':remote.stderr,'scope':'Upstream ref observation at LSP execution; provider semantics come from the exact invoked executables.'},indent=2)+'\n')
selections=[('original',C/'selections/original1024.json','b2ba07fcace4c53e099112488b514b1a76009235f45cd37581363f66a5ff2922',['tcl_lsp_db','tcl_lsp_core','tcl_lsp_server']),('additive21',C/'selections/additive314.json','8b276c87e3611ac599f0cf550eb71cf8172f3b206e15325e6b693001b9cc503d',['tcl_lsp_core','tcl_lsp_server'])]
inventory=C/'selections/additional5-inventory.json';assert sha(inventory)=='bd9f6a6a46eeb63e3eca5d7402cb118180b656fa4fbc63b21fdfe7403c20c813'
controls=json.loads(inventory.read_text())['selectors'];assert len(controls)==5 and all(row['crate']=='tcl_lsp_core' for row in controls)
pin_drift=[{'selector':row['selector'],'source_path':row['source_path'],'pinned_sha256':row['current_source_sha256'],'actual_sha256':sha(R/row['source_path'])} for row in controls if sha(R/row['source_path'])!=row['current_source_sha256']]
extra_label='additional33' if not pin_drift else 'additional33-source-changed'
prior={(target,name) for _,manifest,pin,_ in selections for target,names in json.loads(manifest.read_text())['groups'].items() for name in names};assert all((row['crate'],row['selector']) not in prior for row in controls)
new_manifest=O/'additional33-selection.json';new_manifest.write_text(json.dumps({'group':'Five additional direct LSP source controls admissions22-33; separate from unchanged retained selections','total_exact_selectors':5,'groups':{'tcl_lsp_core':[row['selector'] for row in controls]},'expected_group_counts':{'tcl_lsp_core':5},'execution_order':[{'target':row['crate'],'selector':row['selector']} for row in controls],'test_timeout_seconds':180,'listing_timeout_seconds':30,'environment':{'set':{'TCL_LSP_REQUIRE_TCLSH':'9.0,8.6'},'unset':[]},'inventory_sha256':sha(inventory)},indent=2)+'\n')
selections.append((extra_label,new_manifest,sha(new_manifest),['tcl_lsp_core']))
runner=R/'experiments/name-handling/validation/retained-selections/replay-selection.py';assert sha(runner)=='aaa269431e0a0f3024367d0a2528b4671408e11cf7a57e801caa2429f181e6c3'
summary={'status':'RUNNING_DIRECT_LSP_1085','started':datetime.datetime.now(datetime.timezone.utc).isoformat(),'git_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=R,text=True).strip(),'source_sha256':sha(O/'pre-inputs.json'),'providers_sha256':sha(O/'pre-providers.json'),'artifacts_sha256':sha(B/'artifacts.json'),'runs':[],'extra5_pin_drift':pin_drift,'extra5_note':'Pinned control sources changed; the five selectors run under a separate label and are not the pinned-source controls.' if pin_drift else None,'scope':'Direct LSP original850/additive230/extra5 subgroups on frozen current source. Not original1024/additive314 full-group acceptance, compiler verification, integration/host acceptance or VM/runtime migration.'}
def save(): (O/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
save()
for label,manifest,pin,groups in selections:
 for group in groups:
  out=O/(label+'-'+group)
  command=[sys.executable,str(runner),'--manifest',str(manifest),'--manifest-sha256',pin,'--artifacts',str(B/'artifacts.json'),'--root',str(R),'--out',str(out),'--group',group]
  with (O/(out.name+'.runner.log')).open('wb') as log: p=subprocess.run(command,env=env,stdout=log,stderr=subprocess.STDOUT)
  result=json.loads((out/'result.json').read_text());assert not result['errors'] and not result['artifact_drift'] and result['selected_vector_complete'],out
  summary['runs'].append({'selection':label,'target':group,'exit':p.returncode,'result_sha256':sha(out/'result.json'),'passed':result['passed_count'],'failed':result['failed_count'],'timeouts':sum(r['timed_out'] for r in result['runs']),'count':len(result['runs']),'partial_target':result['partial_target'],'full_group_complete':result['full_group_complete']});save()
for script,name,reference in [('capture-inputs.py','post-inputs.json','pre-inputs.json'),('capture-providers.py','post-providers.json','pre-providers.json')]:
 subprocess.run([sys.executable,str(Path(__file__).resolve().parent/script),str(O/name)],env=env,check=True)
 assert (O/name).read_bytes()==(O/reference).read_bytes(),reference+' drift during LSP execution'
for item in artifacts.values():assert sha(item['path'])==item['sha256']
summary.update(status='DIRECT_LSP_1085_EXECUTED_GUARDS_PASS',ended=datetime.datetime.now(datetime.timezone.utc).isoformat(),completed=sum(r['count'] for r in summary['runs']),passed=sum(r['passed'] for r in summary['runs']),failed=sum(r['failed'] for r in summary['runs']),timeouts=sum(r['timeouts'] for r in summary['runs']),source_provider_artifact_drift=False);save();print(json.dumps({k:v for k,v in summary.items() if k!='runs'}))
