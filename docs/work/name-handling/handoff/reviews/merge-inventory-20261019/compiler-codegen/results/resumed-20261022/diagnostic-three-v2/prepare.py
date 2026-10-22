from pathlib import Path
import json,hashlib,datetime,ast,shutil
out=Path('/tmp/ccg-diagnostic-three-v2'); root=Path('/workspace/naming-migration-active/tcl-lsp'); evidence=root/'docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen/results/resumed-20261022/diagnostic-three-v2'; evidence.mkdir(parents=True,exist_ok=True)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def pin(p):return {'path':str(p),'sha256':sha(p)}
def dump(p,v):Path(p).write_text(json.dumps(v,indent=2)+'\n')
old=Path('/tmp/compiler-focused-runtime-watchdog-v5.py').read_text()
new=old.replace('/tmp/compiler-focused-suite.py','/tmp/ccg-postbatch-functional-v3-suite.py').replace('/tmp/compiler-focused-guarded-cargo.py','/tmp/compiler-c6-guarded-cargo.py').replace("test['exe'].startswith('tcl_compiler-')","test['exe'].startswith(('cmd_control_e2e-', 'pack_source_e2e-'))")
assert new!=old
assert new.replace('/tmp/ccg-postbatch-functional-v3-suite.py','/tmp/compiler-focused-suite.py').replace('/tmp/compiler-c6-guarded-cargo.py','/tmp/compiler-focused-guarded-cargo.py').replace("test['exe'].startswith(('cmd_control_e2e-', 'pack_source_e2e-'))","test['exe'].startswith('tcl_compiler-')")==old
ast.parse(new); watchdog=out/'runtime-watchdog.py';watchdog.write_text(new)
plans=[]; preflights={}
for label,original in [('native1','/tmp/ccg-native-first-decline-v1/plan.json'),('n17two','/tmp/ccg-derived-two-v3-plan.json')]:
 p=json.loads(Path(original).read_text());p['status']='OFF-TREE immutable exact diagnostic plan; NOT QUEUED; root GO and watchdog peer review required'
 p['immutable_predecessor']=pin(original)
 if label=='native1':
  r=next(x for x in p['test_inventory_sources'] if x['path']=='rust/tcl-compiler/src/compile_service.rs');assert r['sha256']=='76a8b1a8c9fa7c13cc9e226d13062da57035a2f4b093a286bf40bea522ac5470';r['sha256']='41f76c717a37f9be2ccafbdda0daf1c23c44641cc190acd3e849e5faee356e66'
  p['source_pin_delta']='Only reviewed corrected76d0 rustfmt two-line opt-in if joins one line; no semantic/assertion delta.'
 p['external_runtime_watchdog']={'adapted':pin(watchdog),'reviewed_model':pin('/tmp/compiler-focused-runtime-watchdog-v5.py'),'budget_seconds':180,'scope':'Each selected diagnostic executable; compilation excluded. Exact PID/start/owned-group authority and bounded INT/TERM/KILL cleanup unchanged.','attachment':'Coordinator starts watcher with actual suite PID and that suite exact receipt directory, before any selected executable begins. Require runtime_timer_attached_before_test_start=true for all selected processes; otherwise no runtime acceptance.','command_template':'python3 /tmp/ccg-diagnostic-three-v2/runtime-watchdog.py <actual-suite-PID> <actual-suite-receipt-directory>','acceptance':'Require completed watcher receipt with no watchdog_failure, no timed_out/acceptance_invalid, observed exact selected process inventory and exited_before_deadline for every process. Suite result alone is insufficient.'}
 p['execution_command']='bash /tmp/ccg-postbatch-functional-v3-run.sh '+str(out/(label+'-plan.json'))
 p['root_prerequisites']=['Compiler remaining33 sole build slot completed; root explicitly assigns slot. No queued Cargo.','Corrected six-component76d0 source applied coherently; Float16a excluded. Exact sourcepins must match; do not refresh silently.','Deadline watcher adaptation peer reviewed; attach before test process begins. Full51v3 guard/suite/run scripts remain byte unchanged.','Before/eachjob/after actual provider/tool/runner guards and fullsource stability required;2GiB minimum,20MiB filtered logs,2jobs/debug0/incremental0,ordinary Cargo artifact provenance unchanged.']
 path=out/(label+'-plan.json');dump(path,p);plans.append(pin(path))
 preflights[label]=[{**r,'actual_now':sha(root/r['path']),'matches_now':sha(root/r['path'])==r['sha256']} for r in p['test_inventory_sources']]
dump(out/'source-preflight.json',preflights)
receipt={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'OFF-TREE preparation only; no queue/build/sourcewrite','plans':plans,'watchdog':pin(watchdog),'model':pin('/tmp/compiler-focused-runtime-watchdog-v5.py'),'exact_inverse':'Only suite path, guard path, and two actual executable stems changed; inverse byte equality and AST parse pass. Budget180 and all ownership/cleanup semantics unchanged.','full51v3':pin('/tmp/ccg-postbatch-functional-v3-plan.json'),'supersedes':'v1 unreviewed whole-command2700 wrapper is NOT proposed for execution. No45min ceiling used or substituted for180sec selected executable runtime.','pins_unchanged':'Native test a89c3a8e / bytecode20dae111; N17 helper e1ed4fb2 + support6f430e9f. Only native compile_service becomes formatted41f76c71 from corrected76d0.','limits':'Static preparation only. Native084 and N17 public041eb preserve original assertions; diagnostic facts require separate review. No full51 acceptance and no claim of all87 or general parity closure.'}
dump(out/'receipt.json',receipt)
for p in out.iterdir():
 if p.is_file():shutil.copyfile(p,evidence/p.name)
print(json.dumps(receipt,indent=2))
