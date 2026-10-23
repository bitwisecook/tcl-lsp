#!/usr/bin/env python3
"""Materialise only from explicit actual committed identity and actual PASS0W typing."""
from pathlib import Path
import argparse,ast,copy,datetime,hashlib,json,shutil,subprocess
BASE=Path(__file__).resolve().parent
ROOT=Path('/workspace/naming-migration-active/tcl-lsp')
OLD=Path('/tmp/ccg-post46-direct3-v6')
def sha(p):
    with Path(p).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def unique(pairs):
    v={}
    for k,x in pairs:
        if k in v:raise ValueError('duplicate JSON key '+k)
        v[k]=x
    return v
def read(p):return json.loads(Path(p).read_text(),object_pairs_hook=unique)
def write(p,v):Path(p).write_text(json.dumps(v,indent=2,sort_keys=True)+'\n')
p=argparse.ArgumentParser();p.add_argument('--identity',type=Path,required=True);p.add_argument('--typing-receipt',type=Path,required=True);p.add_argument('--typing-plan',type=Path,required=True);p.add_argument('--typing-input-manifest',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
policy=read(BASE/'builder-input-manifest.json')
for path,want in policy['inputs'].items():
    if sha(path)!=want:raise SystemExit('STOP: builder input changed '+path)
identity=read(a.identity);typing=read(a.typing_receipt)
if identity.get('schema')!='actual-integrated-product-identity-v1' or identity.get('typing') is not False or identity.get('runtime_tested') is not False:raise SystemExit('STOP: immutable actual identity schema/flags')
for key,size in [('approved_integrated_head',40),('approved_integrated_tree',40),('source_manifest_sha256',64),('candidate_manifest_sha256',64),('candidate_patch_sha256',64)]:
    x=identity.get(key)
    if not isinstance(x,str) or len(x)!=size or any(c not in '0123456789abcdef' for c in x):raise SystemExit('STOP: exact actual identity '+key)
source=Path(identity['source_manifest_path'])
if sha(source)!=identity['source_manifest_sha256']:raise SystemExit('STOP: actual map SHA')
source_map=read(source);rows=source_map['paths'];count=identity['path_count']
if not isinstance(count,int) or isinstance(count,bool) or count<1 or len(rows)!=count or source_map.get('exact_path_count')!=count or len({r['path'] for r in rows})!=count:raise SystemExit('STOP: exact unique actual map count')
head=identity['approved_integrated_head'];tree=identity['approved_integrated_tree']
if source_map.get('approved_product_head')!=head or source_map.get('approved_product_tree')!=tree:raise SystemExit('STOP: actual map HEAD/tree')
if subprocess.check_output(['git','rev-parse',head+'^{tree}'],cwd=ROOT,text=True).strip()!=tree:raise SystemExit('STOP: actual committed tree')
for row in rows:
    if sha(ROOT/row['path'])!=row['after_sha256']:raise SystemExit('STOP: actual current map source '+row['path'])
    if hashlib.sha256(subprocess.check_output(['git','show',head+':'+row['path']],cwd=ROOT)).hexdigest()!=row['after_sha256']:raise SystemExit('STOP: actual committed map source '+row['path'])
packages=['tcl-compiler','tcl-lsp-core','tcl-lsp-db','tcl-lsp-server','tcl-vm','tcl-fuzz','tcl-spectcl','bpf-tcl-ir']
if (typing.get('status')!='PASS' or typing.get('guard_valid') is not True or typing.get('warning_free_json_stderr_stdout') is not True or typing.get('source_stable') is not True or typing.get('providers_stable') is not True or typing.get('cargo_exit_code')!=0 or typing.get('plan_sha256')!=sha(a.typing_plan) or typing.get('approved_product_head')!=head or typing.get('approved_product_tree')!=tree or typing.get('selected_packages')!=packages):raise SystemExit('STOP: actual-source exact eight-package PASS0W typing receipt required')
type_inputs=read(a.typing_input_manifest)
for path,want in type_inputs['inputs'].items():
    if sha(path)!=want:raise SystemExit('STOP: typing runner input changed '+path)
out=a.out.resolve()
if out.exists():raise SystemExit('STOP: immutable materialisation output exists')
out.mkdir(parents=True)
plan=copy.deepcopy(read(OLD/'plan.json'));hook=read('/tmp/ccg-pack-hook-exact2-v1/plan.json')
plan.update({'version':'successor-focused5-actual-v1','status':'Materialised from actual identity and actual PASS0W typing; runtime UNRUN/rootGO absent','approved_integrated_head':head,'approved_integrated_tree':tree,'source_commit':source_map['source_commit'],'integrated_manifest_path':str(source.resolve()),'integrated_manifest_sha256':sha(source),'integrated_path_count':count,'actual_identity':{'path':str(a.identity.resolve()),'sha256':sha(a.identity)},'candidate_manifest_sha256':identity['candidate_manifest_sha256'],'candidate_patch_sha256':identity['candidate_patch_sha256'],'typing_receipt':{'path':str(a.typing_receipt.resolve()),'sha256':sha(a.typing_receipt)},'typecheck_plan_path':str(a.typing_plan.resolve()),'typecheck_plan_sha256':sha(a.typing_plan),'typecheck_runner_input_manifest':{'path':str(a.typing_input_manifest.resolve()),'sha256':sha(a.typing_input_manifest)},'expected_filters':5,'expected_selected':5})
plan.pop('repaired_identity',None)
plan['build_environment']['source_typecheck_plan_sha256']=sha(a.typing_plan)
plan['required_filters']+=copy.deepcopy(hook['required_filters'])
assert len(plan['required_filters'])==5 and len({r['selector'] for r in plan['required_filters']})==5
for row in plan['required_filters']:
    row['env'].update({'TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC':'first-decline-v1','TCL_LSP_TRACE_N17_FOLD':'first-gate-v1'})
plan['diagnostic_scope']='Original native1 +derived2 +livehookpositive1 +hostlessnegative1; N17G3 census existing pure observer, N17G_LIVE actual gates/host. No assertion/fixture alteration.'
plan['source_guard']='Full source/provider before and after entire build+five-row group. Root HOLD prevents writes; per-row GO/identity/typing/runner/tool+artifact pins, explicit group-pending source/provider fields until final.'
plan['root_go_evidence_requirement']='Root independently reviews actual identity+typing/launcher/zero-warning receipt then writes exact root-go.json pins; no autoGO.'
plan['limits']='Runtime UNRUN; no guessed HEAD/tree/count; original five assertions/selectors unchanged. No per-row full100k source/provider scan, group final mandatory; guards fail source/provider drift. No general parity or release closure.'
# Inventory pin deltas must be authorised actual map paths, never silently refreshed.
map_rows={r['path']:r for r in rows};deltas=[]
for row in plan['test_inventory_sources']:
    rel=row['path'];current=sha(ROOT/rel)
    if current!=row['sha256']:
        if rel not in map_rows or map_rows[rel]['after_sha256']!=current:raise SystemExit('STOP: unreviewed inventory drift '+rel)
        deltas.append({'path':rel,'old_sha256':row['sha256'],'actual_sha256':current,'authority':'explicit actual candidate map'})
        row['sha256']=current
plan['reviewed_inventory_pin_deltas']=deltas
write(out/'plan.json',plan);shutil.copyfile(source,out/'source-manifest.json')
(out/'suite.py').write_text((BASE/'suite.py.template.txt').read_text());(out/'guarded-cargo.py').write_text((BASE/'guarded-cargo.py.template.txt').read_text());shutil.copyfile(OLD/'guarded-exec.py',out/'guarded-exec.py')
assert sha(out/'guarded-exec.py')=='88125bd5267ac897dda4b5f665872c5ae0ab9984b0f42a41edeadf718873d811'
(out/'run.sh').write_text('#!/bin/bash\nset -u\nsource /tmp/tcl-lsp-naming-env.sh\nexec python3 '+str(out/'suite.py')+' "$1"\n')
launch=(OLD/'launch.py').read_text().replace("BASE=Path('/tmp/ccg-post46-direct3-v6')",'BASE=Path(__file__).resolve().parent').replace("RUNS=Path('/tmp/ccg-post46-direct3-v6-runs')","RUNS=Path(str(BASE)+'-runs')").replace("BASE/'source46-manifest.json'","BASE/'source-manifest.json'")
(out/'launch.py').write_text(launch)
for filename in ['suite.py','guarded-cargo.py','guarded-exec.py','launch.py']:ast.parse((out/filename).read_text())
subprocess.run(['bash','-n',str(out/'run.sh')],check=True)
inputs={str(p):sha(p) for p in [out/'plan.json',out/'suite.py',out/'guarded-cargo.py',out/'guarded-exec.py',out/'launch.py',out/'run.sh',out/'source-manifest.json',a.identity,a.typing_receipt,a.typing_plan,a.typing_input_manifest,source,Path('/tmp/compiler-c6-provider-manifest.py'),Path('/tmp/tcl-lsp-naming-env.sh')]}
for item in plan['legacy_runner_inputs']:inputs[item['path']]=item['sha256']
for path,want in type_inputs['inputs'].items():inputs[path]=want
write(out/'runner-input-manifest.json',{'inputs':inputs,'version':'successor-focused5-actual-v1'})
write(out/'root-go.required.json',{'plan_sha256':sha(out/'plan.json'),'source_commit':plan['source_commit'],'approved_integrated_head':head,'approved_integrated_tree':tree,'runner_input_manifest_sha256':sha(out/'runner-input-manifest.json'),'actual_identity_sha256':sha(a.identity),'source_manifest_sha256':sha(source),'typing_receipt_sha256':sha(a.typing_receipt),'note':'Schema/value guide only; root must independently review and issue root-go.json. This file is not GO.'})
write(out/'materialisation-receipt.json',{'status':'Materialised actual identity+typing template; runtimeUNRUN/rootGOabsent','identity_sha256':sha(a.identity),'typing_receipt_sha256':sha(a.typing_receipt),'path_count':count,'plan_sha256':sha(out/'plan.json'),'runner_manifest_sha256':sha(out/'runner-input-manifest.json'),'source_inventory_pin_deltas':deltas,'builder_manifest_sha256':sha(BASE/'builder-input-manifest.json')})
print(json.dumps({'out':str(out),'plan_sha256':sha(out/'plan.json'),'runner_manifest_sha256':sha(out/'runner-input-manifest.json'),'GO_created':False}))
