"""Freeze exact26 from explicit actual identity and successful typing; never mint GO or execute Cargo."""
from pathlib import Path
import argparse,ast,datetime,hashlib,importlib.util,json
BASE=Path('/tmp/vm-notom16-runtime-materializer-v1');TEMPLATE=Path('/tmp/vm-notom16-runtime-template-v1');ROOT=Path('/workspace/naming-migration-active/tcl-lsp')
def require(value,message):
 if not value:raise RuntimeError('STOP: '+message)
def sha(p):
 with Path(p).open('rb')as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def unique(pairs):
 out={}
 for key,value in pairs:require(key not in out,'duplicate JSON key '+key);out[key]=value
 return out
def read(p):return json.loads(Path(p).read_text(),object_pairs_hook=unique)
parser=argparse.ArgumentParser(description=__doc__)
for key in ['identity','typing-plan','typing-final','typing-launcher']:
 parser.add_argument('--'+key,required=True,type=Path);parser.add_argument('--'+key+'-sha256',required=True)
parser.add_argument('--out',required=True,type=Path);args=parser.parse_args()
manifest=read(BASE/'manifest.json')
for name,pin in manifest['files'].items():require(sha(BASE/name)==pin,'materializer input changed '+name)
for name,pin in manifest['template_inputs'].items():require(sha(TEMPLATE/name)==pin,'template input changed '+name)
binding={key:{'path':str(getattr(args,arg).resolve()),'sha256':getattr(args,arg+'_sha256')} for key,arg in [('identity','identity'),('plan','typing_plan'),('final','typing_final'),('launcher','typing_launcher')]}
spec=importlib.util.spec_from_file_location('vm16_closed_typing',BASE/'typing-proof.py');require(spec is not None and spec.loader is not None,'typing helper loader absent');proof=importlib.util.module_from_spec(spec);spec.loader.exec_module(proof);actual=proof.validate_typing(binding)
# No output directory before complete successful real typing proof.
out=args.out.resolve();require(out.is_relative_to('/tmp')and not out.exists(),'fresh /tmp protocol directory required')
plan=read(TEMPLATE/'plan.json');tp=actual['type_plan'];identity=actual['identity']
require(all(tp[k]==plan[k] for k in ['cargo_path','cargo_sha256','rustc_path','rustc_sha256','path_prefix','refused_environment_patterns']),'normal toolchain/refusal policy changed; new review required')
require(set(plan['refused_environment_exact'])==(set(tp['refused_environment_exact'])-{'TCL_RUNTIME_EXPECT_NO_TOMMATH'})|{'TCL_TOMMATH_DIR'},'NoTom refusal delta must be exact')
require(tp['required_environment']['TMPDIR']=='/workspace/naming-build-tmp'and'TCL_RUNTIME_EXPECT_NO_TOMMATH'in tp['refused_environment_exact'],'workspace temp/normal NoTom refusal missing')
for row in actual['manifest']['paths']:require(sha(ROOT/row['path'])==row['after_sha256'],'actual source differs '+row['path'])
for row in plan['source_pins']:require(sha(ROOT/row['path'])==row['sha256'],'original assertion source changed '+row['path'])
plan.update(approved_product_head=identity['approved_integrated_head'],approved_product_tree=identity['approved_integrated_tree'],actual_identity=binding['identity'],typing_binding=binding,status='Explicit actual PASS typing bound; root GO absent/runtime UNRUN',actual_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
mi=Path(binding['plan']['path']).parent/'runner-input-manifest.json';plan['typecheck_plan']={'path':binding['plan']['path'],'sha256':binding['plan']['sha256'],'runner_inputs_path':str(mi),'runner_inputs_sha256':sha(mi)}
plan['source_path_count']=identity['path_count'];plan['required_environment'].update(tp['required_environment']);plan['required_environment'].pop('TCL_TOMMATH_DIR',None);plan['required_environment']['TCL_RUNTIME_EXPECT_NO_TOMMATH']='1';plan['required_environment']['CARGO_TARGET_DIR']='/workspace/naming-runtime-notommath-'+out.name+'-target';plan['lifecycle']['source122_manifest_sha256']=identity['source_manifest_sha256'];plan['lifecycle']['post122_eight_package_typecheck_plan_sha256']=binding['plan']['sha256'];plan['typing_proof_sha256']=sha(BASE/'typing-proof.py')
files={name:(TEMPLATE/name).read_text() for name in ['suite.py','launch.py','run.sh','providers.py','provider-chain.py','guarded-exec.py']}
for name in ['suite.py','launch.py','run.sh']:files[name]=files[name].replace(str(TEMPLATE),str(out))
s=files['suite.py'];needle="SOURCE122=Path('/tmp/naming-reviewed-product-actual-identity-v11/actual-source122-manifest.json')";require(s.count(needle)==1,'source binding template changed');s=s.replace(needle,'SOURCE122=Path('+repr(actual['source_manifest_path'])+')')
needle="RUNS=Path('/workspace/naming-validation-runs/vm-notom16-runtime-template-v1')";require(s.count(needle)==1,'raw output root template changed');s=s.replace(needle,'RUNS=Path('+repr('/workspace/naming-validation-runs/notom16/'+out.name)+')')
needle="    identity=plan['actual_identity']";require(s.count(needle)==1,'source_view anchor changed')
add="""    require(sha(BASE/'typing-proof.py')==plan['typing_proof_sha256'],'typing helper changed')
    verifier_spec=importlib.util.spec_from_file_location('vm16_runtime_typing',BASE/'typing-proof.py')
    require(verifier_spec is not None and verifier_spec.loader is not None,'typing verifier absent')
    verifier=importlib.util.module_from_spec(verifier_spec);verifier_spec.loader.exec_module(verifier);verifier.validate_typing(plan['typing_binding'])
    require(go.get('post122_eight_package_typecheck_evidence')==plan['typing_binding']['final'] and go.get('post122_eight_package_typecheck_launcher_evidence')==plan['typing_binding']['launcher'],'root GO must pin same real final AND launcher')
"""
s=s.replace(needle,add+needle);files['suite.py']=s;files['typing-proof.py']=(BASE/'typing-proof.py').read_text();files['plan.json']=json.dumps(plan,indent=2)+'\n'
for name in ['suite.py','launch.py','providers.py','provider-chain.py','guarded-exec.py','typing-proof.py']:ast.parse(files[name])
# All checks above are read-only. Materialization never executes generated runner.
out.mkdir()
for name,text in files.items():(out/name).write_text(text)
inputs={str(out/name):sha(out/name)for name in files}
for item in binding.values():inputs[item['path']]=item['sha256']
for p in [Path(actual['source_manifest_path']),mi,ROOT/'docs/work/name-handling/validation-tools/capture-inputs.py',ROOT/'docs/work/name-handling/validation-tools/_paths.py',Path('/tmp/naming-oracle-setup.json'),Path('/tmp/tcl-lsp-naming-env.sh')]:inputs[str(p)]=sha(p)
(out/'runner-input-manifest.json').write_text(json.dumps({'inputs':inputs,'root_go_contract':'Exact identity+source122+same successful typing final AND launcher; no GO generated'},indent=2)+'\n')
(out/'materialization.json').write_text(json.dumps({'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'materializer_manifest_sha256':sha(BASE/'manifest.json'),'binding':binding,'plan_sha256':sha(out/'plan.json'),'runner_input_manifest_sha256':sha(out/'runner-input-manifest.json'),'root_go_created':False,'cargo_or_runtime_started':False,'raw_run_root':'/workspace/naming-validation-runs/notom16/'+out.name},indent=2)+'\n');print(out)
