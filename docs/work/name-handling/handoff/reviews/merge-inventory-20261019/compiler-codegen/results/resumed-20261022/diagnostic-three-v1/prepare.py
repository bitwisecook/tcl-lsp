from pathlib import Path
import json,hashlib,datetime,ast,shutil
root=Path('/workspace/naming-migration-active/tcl-lsp'); out=Path('/tmp/ccg-diagnostic-three-v1')
evidence=root/'docs/work/name-handling/handoff/reviews/merge-inventory-20261019/compiler-codegen/results/resumed-20261022/diagnostic-three-v1'; evidence.mkdir(parents=True,exist_ok=True)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def dump(p,v):Path(p).write_text(json.dumps(v,indent=2)+'\n')
def pin(p):return {'path':str(p),'sha256':sha(p)}
now=datetime.datetime.now(datetime.timezone.utc).isoformat()
ci=Path('/tmp/coordinator-ci-failed-shard-listing-v1/proposal.patch')
assert sha(ci)=='78fca4e5499bacb3644939b6d7ce07dce65ff262b3b8a8c1c2937e1f95227ab2'
dump(evidence/'ci-listing-peer-review.json',{'utc':now,'reviewer_role':'compiler-codegen','status':'Bounded static peer clear; no CI execution or test acceptance','patch':pin(ci),'scope':'Only selected shard listing condition adds always() && !cancelled(); original changed/docs/already-green filters preserved. Test command and its condition unchanged; listing after a failed test is evidence availability, never PASS. Cancellation still suppresses listing. Existing upload step is unchanged.','provenance':'Replacement role re-read exact patch and surrounding canonical CI listing/upload steps; confirms predecessor static clear reported by coordinator.'})
shutil.copyfile(ci,evidence/'ci-listing.patch')
old=Path('/tmp/compiler-c6-guarded-cargo.py').read_text(); new=old
new=new.replace('aborted = False\n','aborted = False\ntimed_out = False\ntermination_stage = 0\ncommand_started = time.monotonic()\ncommand_deadline_seconds = 2700\n')
needle='        if now >= next_disk:\n'
assert new.count(needle)==1
new=new.replace(needle,"""        elapsed = now - command_started
        if elapsed >= command_deadline_seconds and not timed_out:
            timed_out = True
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGINT)
            write_line(b'error: diagnostic command exceeded 2700 second deadline\\n')
        if timed_out and elapsed >= command_deadline_seconds + 30 and termination_stage == 0:
            termination_stage = 1
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        if timed_out and elapsed >= command_deadline_seconds + 40 and termination_stage == 1:
            termination_stage = 2
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
"""+needle)
new=new.replace("    'disk_abort': aborted,","    'disk_abort': aborted,\n    'deadline_abort': timed_out,\n    'deadline_seconds': command_deadline_seconds,\n    'deadline_grace_seconds': [30, 10],")
new=new.replace('sys.exit(2 if aborted else','sys.exit(2 if aborted or timed_out else')
wrapper=out/'guarded-cargo.py'; wrapper.write_text(new); ast.parse(new)
suite_old=Path('/tmp/ccg-postbatch-functional-v3-suite.py').read_text()
suite=suite_old.replace('/tmp/compiler-c6-guarded-cargo.py',str(wrapper)).replace('/tmp/ccg-postbatch-functional-v3-suite.py',str(out/'suite.py')).replace('/tmp/ccg-postbatch-functional-v3-run.sh',str(out/'run.sh')).replace("Path('/tmp/ccg-postbatch-functional-v3')","Path('/tmp/ccg-diagnostic-three-v1/results')")
(out/'suite.py').write_text(suite); ast.parse(suite)
(out/'run.sh').write_text('#!/bin/bash\nset -euo pipefail\ncd /workspace/naming-migration-active/tcl-lsp\nsource /tmp/tcl-lsp-naming-env.sh\nexec python3 /tmp/ccg-diagnostic-three-v1/suite.py "$1"\n')
assert suite.replace(str(wrapper),'/tmp/compiler-c6-guarded-cargo.py').replace(str(out/'suite.py'),'/tmp/ccg-postbatch-functional-v3-suite.py').replace(str(out/'run.sh'),'/tmp/ccg-postbatch-functional-v3-run.sh').replace("Path('/tmp/ccg-diagnostic-three-v1/results')","Path('/tmp/ccg-postbatch-functional-v3')")==suite_old
plans=[]
for label,original in [('native1','/tmp/ccg-native-first-decline-v1/plan.json'),('n17two','/tmp/ccg-derived-two-v3-plan.json')]:
 p=json.loads(Path(original).read_text())
 p['status']='OFF-TREE diagnostic plan; deadline successor requires bounded peer review and root GO; NOT QUEUED'
 p['utc']=now
 p['immutable_predecessor']=pin(original)
 for r in p['runner_inputs']:
  replacement={'/tmp/compiler-c6-guarded-cargo.py':wrapper,'/tmp/ccg-postbatch-functional-v3-suite.py':out/'suite.py','/tmp/ccg-postbatch-functional-v3-run.sh':out/'run.sh'}.get(r['path'])
  if replacement:r.update(pin(replacement))
 if label=='native1':
  record=next(r for r in p['test_inventory_sources'] if r['path']=='rust/tcl-compiler/src/compile_service.rs')
  assert record['sha256']=='76a8b1a8c9fa7c13cc9e226d13062da57035a2f4b093a286bf40bea522ac5470'
  record['sha256']='41f76c717a37f9be2ccafbdda0daf1c23c44641cc190acd3e849e5faee356e66'
  p['source_pin_delta']='Only corrected76d0 rustfmt joins two lines of diagnostic opt-in if condition; exact preformat/work diff reviewed. No predicates, assertions, fixture or admission change.'
 p['execution_command']='bash '+str(out/'run.sh')+' '+str(out/(label+'-plan.json'))
 p['deadline_contract']={'per_command_seconds':2700,'own_cargo_session_only':True,'SIGINT_at':2700,'SIGTERM_at':2730,'SIGKILL_at':2740,'timeout':'Wrapper exits2 even if test subsequently returns success; incomplete summary or nonzero result cannot count as acceptance.'}
 p['root_prerequisites']=['Compiler remaining33 sole build window complete and root assigns slot; no waiting Cargo queued.','Corrected six-component76d0 applied coherently; Float16a absent. Both native observer sourcepins must match formatted candidate; N17 two original helper pins remain exact.','Unchanged full51v3 actual compiler/provider/runner pins must pass; no silent refresh. At least2GiB free, prefer2.3GiB.','Deadline-only off-tree runner successor peer-reviewed before execution. Full51 remains separate, unchanged and requires exact041eb cleanup to restore original sourcepin.']
 path=out/(label+'-plan.json');dump(path,p);plans.append(pin(path))
 checks=[]
 for r in p['test_inventory_sources']:
  actual=sha(root/r['path']) if (root/r['path']).exists() else None
  checks.append({**r,'actual_now':actual,'matches_now':actual==r['sha256']})
 dump(out/(label+'-source-preflight.json'),checks)
summary={'utc':now,'status':'Immutable off-tree preparation only; no Cargo execution/queue/source mutation','plans':plans,'parent_guard_protocol':pin('/tmp/ccg-postbatch-functional-v3-plan.json'),'corrected_component_set':pin('/tmp/coordinator-n13-candidate-2321/receipt.json'),'runner_files':[pin(wrapper),pin(out/'suite.py'),pin(out/'run.sh')],'static_checks':{'python_AST':True,'suite_exact_inverse_routing_only':True},'limits':'Deadline wrapper remains unreviewed/unexecuted; no source/provider/functional acceptance. Native formatted pin is expected candidate, not an assertion that source applied. Original assertions and full source/provider/tool/artifact guards remain unchanged.'}
dump(out/'receipt.json',summary)
for p in out.iterdir():
 if p.is_file():shutil.copyfile(p,evidence/p.name)
print(json.dumps(summary,indent=2))
