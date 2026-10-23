"""Materialize an immutable direct4 only from explicit actual successful typing evidence; never mint GO or run it."""
from pathlib import Path
import argparse, json, hashlib, ast, shutil, importlib.util, datetime
BASE = Path('/tmp/lsp-startup-upf-latebound-materializer-v3')
TEMPLATE = Path('/tmp/lsp-startup-upf-source122-template-v1-direct4')
ROOT = Path('/workspace/naming-migration-active/tcl-lsp')

def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()

def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise RuntimeError('STOP: duplicate JSON key: ' + key)
        result[key] = value
    return result

def read(p):
    return json.loads(Path(p).read_text(), object_pairs_hook=unique_object)

def require(value, message):
    if not value:
        raise RuntimeError('STOP: ' + message)
parser = argparse.ArgumentParser(description=__doc__)
for key in ['identity', 'typing-plan', 'typing-final', 'typing-launcher']:
    parser.add_argument('--' + key, required=True, type=Path)
    parser.add_argument('--' + key + '-sha256', required=True)
parser.add_argument('--out', required=True, type=Path)
a = parser.parse_args()
manifest = read(BASE / 'manifest.json')
for name, pin in manifest['files'].items():
    require(sha(BASE / name) == pin, 'materializer changed: ' + name)
for name, pin in manifest['template_inputs'].items():
    require(sha(TEMPLATE / name) == pin, 'template changed: ' + name)
binding = {key: {'path': str(getattr(a, arg).resolve()), 'sha256': getattr(a, arg + '_sha256')} for key, arg in [('identity', 'identity'), ('plan', 'typing_plan'), ('final', 'typing_final'), ('launcher', 'typing_launcher')]}
spec = importlib.util.spec_from_file_location('exact_typing_proof', BASE / 'typing-proof.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
actual = module.validate_typing(binding)
out = a.out.resolve()
require(out.is_relative_to('/tmp') and (not out.exists()), 'fresh /tmp output required')
tp = actual['type_plan']
identity = actual['identity']
plan = read(TEMPLATE / 'plan.json')
require(all((tp[key] == plan[key] for key in ['cargo_path', 'cargo_sha256', 'rustc_path', 'rustc_sha256', 'path_prefix', 'refused_environment_exact', 'refused_environment_patterns'])), 'normal toolchain policy changed; new review required')
require(tp['required_environment']['TMPDIR'] == '/workspace/naming-build-tmp' and 'TCL_RUNTIME_EXPECT_NO_TOMMATH' in tp['refused_environment_exact'], 'normal workspace-temp/NoTommath refusal missing')
for row in actual['manifest']['paths']:
    require(sha(ROOT / row['path']) == row['after_sha256'], 'current actual source drift')
plan['approved_product_head'] = identity['approved_integrated_head']
plan['approved_product_tree'] = identity['approved_integrated_tree']
plan['actual_identity'] = binding['identity']
plan['typecheck_plan'] = {'path': binding['plan']['path'], 'sha256': binding['plan']['sha256'], 'runner_inputs_path': str(Path(binding['plan']['path']).parent / 'runner-input-manifest.json'), 'runner_inputs_sha256': sha(Path(binding['plan']['path']).parent / 'runner-input-manifest.json')}
plan['typing_binding'] = binding
plan['required_environment'].update(tp['required_environment'])
plan['lifecycle']['source122_manifest_sha256'] = identity['source_manifest_sha256']
plan['lifecycle']['post122_eight_package_typecheck_plan_sha256'] = binding['plan']['sha256']
plan['status'] = 'Actual successful typing explicitly bound; root GO absent, runtime UNRUN'
plan['actual_utc'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
for row in plan['source_pins']:
    row['sha256'] = sha(ROOT / row['path'])
files = {name: (TEMPLATE / name).read_text() for name in ['suite.py', 'launch.py', 'run.sh', 'providers.py', 'guarded-exec.py']}
for name in ['suite.py', 'launch.py', 'run.sh']:
    files[name] = files[name].replace(str(TEMPLATE), str(out))
s = files['suite.py']
s = s.replace("SOURCE122=Path('/tmp/name-handling-eight-package-check-v10-source122/source122-manifest.json')", 'SOURCE122=Path(' + repr(actual['source_manifest_path']) + ')')
needle = "    identity=plan['actual_identity']"
add = "    require(sha(BASE/'typing-proof.py')==plan['typing_proof_sha256'],'typing proof verifier changed')\n    verifier_spec=importlib.util.spec_from_file_location('direct4_typing_proof',BASE/'typing-proof.py')\n    verifier=importlib.util.module_from_spec(verifier_spec); verifier_spec.loader.exec_module(verifier)\n    verifier.validate_typing(plan['typing_binding'])\n    require(go.get('post122_eight_package_typecheck_evidence')==plan['typing_binding']['final'] and go.get('post122_eight_package_typecheck_launcher_evidence')==plan['typing_binding']['launcher'],'root GO must bind actual successful typing final AND launcher')\n"
s = s.replace(needle, add + needle)
files['suite.py'] = s
files['typing-proof.py'] = (BASE / 'typing-proof.py').read_text()
plan['typing_proof_sha256'] = sha(BASE / 'typing-proof.py')
files['plan.json'] = json.dumps(plan, indent=2) + '\n'
for name in ['suite.py', 'launch.py', 'providers.py', 'guarded-exec.py', 'typing-proof.py']:
    ast.parse(files[name])
out.mkdir()
for name, text in files.items():
    (out / name).write_text(text)
inputs = {str(out / name): sha(out / name) for name in files}
for item in binding.values():
    inputs[item['path']] = item['sha256']
for p in [Path(actual['source_manifest_path']), Path(plan['typecheck_plan']['runner_inputs_path']), ROOT / 'docs/work/name-handling/validation-tools/capture-inputs.py', ROOT / 'docs/work/name-handling/validation-tools/_paths.py', Path(plan['original_native15_plan']['path'])]:
    inputs[str(p)] = sha(p)
(out / 'runner-input-manifest.json').write_text(json.dumps({'inputs': inputs, 'root_go_contract': 'Exact actual identity+source122+successful type final AND launcher evidence; no GO generated'}, indent=2) + '\n')
(out / 'materialization.json').write_text(json.dumps({'actual_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'materializer_manifest_sha256': sha(BASE / 'manifest.json'), 'binding': binding, 'plan_sha256': sha(out / 'plan.json'), 'runner_input_manifest_sha256': sha(out / 'runner-input-manifest.json'), 'go_created': False, 'cargo_or_runtime_started': False}, indent=2) + '\n')
print(str(out))
