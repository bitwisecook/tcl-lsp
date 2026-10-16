from pathlib import Path
from frozen_source_guard import validate as validate_frozen_source, live_hash, validate_fresh_jim
import argparse, datetime, gzip, hashlib, json, os, re, shlex, signal, subprocess, time

parser = argparse.ArgumentParser()
parser.add_argument('--root', default='/workspace/tcl-lsp')
parser.add_argument('--out', required=True)
parser.add_argument('--target', required=True)
parser.add_argument('--api-only', action='store_true')
args = parser.parse_args()
root = Path(args.root).resolve()
out = Path(args.out).resolve()
target = Path(args.target).resolve()
out.mkdir(parents=True, exist_ok=False)
target.mkdir(parents=True, exist_ok=True)
packet = Path(__file__).parent
plan = json.loads((packet / 'frozen-plan.json').read_text())
compat = json.loads((packet / 'jim-original-compatibility-manifest.json').read_text())
api = json.loads((packet / 'api-selector-manifest.json').read_text())
env = os.environ.copy()
env.update(CARGO_TARGET_DIR=str(target), CARGO_BUILD_JOBS='1', CARGO_PROFILE_TEST_DEBUG='0', CARGO_INCREMENTAL='0')
env.pop('TCL_LIBRARY', None)
env.pop('TCLLIBPATH', None)

def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def run(command, log, timeout, input=None, selected_env=None):
    started = time.monotonic()
    with (out / log).open('wb') as output:
        process = subprocess.Popen(command, cwd=root, env=selected_env or env, stdin=subprocess.PIPE if input is not None else subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
        timed_out = False
        try:
            process.communicate(input=input, timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            os.killpg(process.pid, signal.SIGKILL)
            process.communicate()
    return {'argv': command, 'exit_code': process.returncode, 'timed_out': timed_out, 'elapsed_seconds': time.monotonic() - started, 'log': log, 'log_sha256': sha(out / log)}

source_successors = validate_frozen_source(packet, root)
provider_paths = {p.resolve() for p in packet.rglob("*") if p.is_file()}
providers = []
for version in ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0']:
    tree = root / 'tmp' / ('tcl' + version)
    executable = tree / 'unix/tclsh'
    required = [executable, tree / 'library/init.tcl', tree / 'library/tcltest/tcltest.tcl']
    for path in required:
        if not path.is_file():
            raise ValueError('Missing original provider input: ' + str(path))
        provider_paths.add(path.resolve())
    provider_paths.update(p.resolve() for p in tree.rglob('*') if p.is_file() and (p.suffix in {'.c', '.h', '.tcl', '.test', '.tm', '.a', '.so', '.sh'} or '.so.' in p.name))
    oracle_key = {'8.4.20': 'TCL_LSP_TCLSH84', '8.5.19': 'TCL_LSP_TCLSH85', '8.6.18': 'TCL_LSP_TCLSH86', '9.0.4': 'TCL_LSP_TCLSH90', '9.1.0': 'TCL_LSP_TCLSH91'}[version]
    if oracle_key in env and Path(env[oracle_key]).resolve() != executable.resolve():
        raise ValueError('Foreign selected interpreter: ' + oracle_key)
    env[oracle_key] = str(executable)
    providers.append({'kind': 'C Tcl', 'version': version, 'executable': str(executable), 'original_library': str(tree / 'library'), 'original_files': {str(p): sha(p) for p in required}, 'probe': 'puts [list [info patchlevel] [info library]]\n'})
jim_tree = root / 'tmp/jimtcl-oracle'
jim_executable = jim_tree / 'jimsh'
if 'TCL_LSP_JIMSH' in env and Path(env['TCL_LSP_JIMSH']).resolve() != jim_executable.resolve():
    raise ValueError('Foreign selected Jim interpreter')
env.update(TCL_LSP_JIMSH=str(jim_executable), TCL_LSP_REQUIRE_ALL_TCL_ORACLES='1', TCL_LSP_REQUIRE_JIM_ORACLE='1')
provider_paths.add(jim_executable.resolve())
provider_paths.update(p.resolve() for p in jim_tree.rglob('*') if p.is_file() and (p.suffix in {'.c', '.h', '.tcl', '.test', '.tm', '.a', '.so', '.sh'} or '.so.' in p.name))
fresh_jim = validate_fresh_jim(packet, jim_tree, out)
providers.append({'kind': 'Jim', 'executable': str(jim_executable), 'executable_sha256': sha(jim_executable), 'probe': 'puts [list [info patchlevel]]\n', 'latest_check': fresh_jim})
for provider in providers:
    linked = subprocess.check_output(['ldd', provider['executable']], text=True)
    provider['linked_libraries'] = {str(Path(word).resolve()): sha(Path(word).resolve()) for line in linked.splitlines() for word in line.split() if word.startswith('/') and Path(word).is_file()}
    provider_paths.update(Path(p) for p in provider['linked_libraries'])


def snapshot():
    paths = set(provider_paths)
    for family in ['rust', 'runtime', 'data', 'scripts', 'specs', 'experiments/name-handling', '.cargo']:
        directory = root / family
        paths.update(p.resolve() for p in directory.rglob('*') if p.is_file() and 'target' not in p.parts and '.git' not in p.parts and not p.resolve().is_relative_to(target) and not p.resolve().is_relative_to(out))
    for name in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'runtime/rust/Cargo.lock']:
        path = root / name
        if path.is_file():
            paths.add(path.resolve())
    if 'TCL_TOMMATH_DIR' in env:
        math_source = Path(env['TCL_TOMMATH_DIR']).resolve()
        paths.update(p.resolve() for p in math_source.rglob('*') if p.is_file())
    registry = Path(env.get('CARGO_HOME', '/workspace/.cargo')) / 'registry/src'
    paths.update(p.resolve() for p in registry.rglob('*') if p.is_file())
    for path in tuple(paths):
        if path.suffix == '.rs':
            for relative in re.findall(r'include_(?:str|bytes)!\(\s*"([^"]+)"\s*\)', path.read_text(errors='strict')):
                included = (path.parent / relative).resolve()
                if included.is_file():
                    paths.add(included)
    return {str(path): sha(path) for path in sorted(paths)}


def compressed(name, value):
    with gzip.open(out / name, 'wt') as stream:
        json.dump(value, stream, sort_keys=True)

result = {'started_utc': now(), 'providers': providers, 'builds': [], 'runs': [], 'errors': [], 'passed': False, 'scope': 'Separate shared API four exact controls' if args.api_only else plan['scope']}
before = snapshot()
compressed('source-before.json.gz', before)
(out / 'selected-plan.json').write_text(json.dumps({'native': plan, 'additional_original_jim': compat, 'separate_api': api, 'api_only': args.api_only}, indent=2) + '\n')
(out / 'start.json').write_text(json.dumps({'started_utc': result['started_utc'], 'runner_sha256': sha(Path(__file__)), 'source_count': len(before), 'plan_sha256': sha(packet / 'frozen-plan.json'), 'compat_manifest_sha256': sha(packet / 'jim-original-compatibility-manifest.json'), 'api_manifest_sha256': sha(packet / 'api-selector-manifest.json'), 'target': str(target), 'rustc': subprocess.check_output(['rustc', '-Vv'], text=True), 'cargo': subprocess.check_output(['cargo', '-V'], text=True), 'closure': 'Rust/runtime/data/scripts/specs/experiments/Cargo, literal include files, original C5/Jim source/interpreters/libraries, linked libraries and registry sources'}, indent=2) + '\n')
inputs = {}
artifacts = {}
cargo_artifact_rows = []
try:
    for index, provider in enumerate(providers):
        selected = env.copy()
        if provider['kind'] == 'C Tcl':
            selected['TCL_LIBRARY'] = provider['original_library']
        provider['metadata'] = run([provider['executable']], f'provider-{index + 1:02}.log', 30, input=provider['probe'].encode(), selected_env=selected)
        provider['selected_environment'] = {'TCL_LIBRARY': selected.get('TCL_LIBRARY'), 'TCLLIBPATH': selected.get('TCLLIBPATH')}
        if provider['metadata']['exit_code'] or provider['metadata']['timed_out']:
            raise ValueError('Original provider metadata failed')
        if provider['kind'] == 'C Tcl' and (out / provider['metadata']['log']).read_text().strip() != provider['version'] + ' ' + provider['original_library']:
            raise ValueError('Incorrect original init/library selection: ' + provider['version'])
    if args.api_only:
        groups = {'tcl_runtime_api': api['exact']}
        builds = [('api', ['cargo', 'test', '--offline', '-p', 'tcl-runtime-api', '--lib', '--no-run', '--message-format=json', '-j1'])]
    else:
        runtime = list(dict.fromkeys(plan['runtime'] + compat['exact']))
        baseline = json.loads((packet / 'baseline107-runtime.json').read_text())
        if not set(baseline).issubset(runtime):
            raise ValueError('A previous actual runtime selector was dropped')
        groups = {'tcl_runtime': runtime, 'native_namespace_command_query': plan['query'], 'tcl_registry': plan['shared_registry']}
        predecessor167 = json.loads((packet / 'predecessor167-frozen-plan.json').read_text())
        retained_query = predecessor167['query']
        if len(retained_query) != 50 or plan['query'][:50] != retained_query or len(plan['query']) != 51 or plan['query'][50:] != ['fresh_namespace_object_lifetimes_match_original_c_releases']:
            raise ValueError('Expected exact retained50 query prefix plus one genuine fresh namespace query')
        previous113 = json.loads((packet / 'baseline113-plan.json').read_text())
        if not set(previous113['runtime']).issubset(runtime) or not set(previous113['query']).issubset(plan['query']) or not set(previous113['shared_registry']).issubset(plan['shared_registry']):
            raise ValueError('A previous113 selector was dropped')
        previous = json.loads((packet / 'baseline110-plan.json').read_text())
        if not set(previous['runtime']).issubset(runtime) or not set(previous['query']).issubset(plan['query']) or not set(previous['shared_registry']).issubset(plan['shared_registry']):
            raise ValueError('A previous full110 selector was dropped')
        previous188 = json.loads((packet / 'baseline188-selected-tests.json').read_text())
        if sum(map(len, previous188.values())) != 117 or any(not set(names).issubset(groups.get(label, [])) for label, names in previous188.items()):
            raise ValueError('A sealed Runtime188 selector was dropped or baseline count changed')
        predecessor = json.loads((packet / 'baseline126-plan.json').read_text())
        old_runtime = list(dict.fromkeys(predecessor['runtime'] + compat['exact']))
        if len(old_runtime) + len(predecessor['query']) + len(predecessor['shared_registry']) != 126 or not set(old_runtime).issubset(runtime) or not set(predecessor['query']).issubset(plan['query']) or not set(predecessor['shared_registry']).issubset(plan['shared_registry']):
            raise ValueError('A complete126 Runtime selector was removed or baseline count changed')
        descendant = json.loads((packet / 'descendant-selector-audit.json').read_text())
        if descendant['selector'] not in runtime:
            raise ValueError('Actual dying-descendant selector removed')
        source = root / descendant['path']
        if not source.is_file() or len(re.findall(r'(?m)^\s*fn ' + re.escape(descendant['function']) + r'\s*\(', source.read_text())) != 1:
            raise ValueError('Actual dying-descendant test missing or ambiguous')
        predecessor127 = json.loads((packet / 'baseline127-plan.json').read_text())
        old127_runtime = list(dict.fromkeys(predecessor127['runtime'] + compat['exact']))
        if len(old127_runtime) + len(predecessor127['query']) + len(predecessor127['shared_registry']) != 127 or not set(old127_runtime).issubset(runtime) or not set(predecessor127['query']).issubset(plan['query']) or not set(predecessor127['shared_registry']).issubset(plan['shared_registry']):
            raise ValueError('A complete127 Runtime selector was removed or baseline count changed')
        jim_deferred = json.loads((packet / 'jim-deferred-selector-audit.json').read_text())
        if jim_deferred['added_count'] != 5 or len(jim_deferred['added']) != 5:
            raise ValueError('Added RuntimeJim source selector audit changed')
        for selected in jim_deferred['added']:
            if selected['selector'] not in runtime:
                raise ValueError('Actual RuntimeJim source selector removed: ' + selected['selector'])
            source = root / selected['path']
            if not source.is_file() or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Actual RuntimeJim source test missing or ambiguous: ' + selected['selector'])
        predecessor132 = json.loads((packet / 'baseline132-plan.json').read_text())
        old132_runtime = list(dict.fromkeys(predecessor132['runtime'] + compat['exact']))
        if len(old132_runtime) + len(predecessor132['query']) + len(predecessor132['shared_registry']) != 132 or not set(old132_runtime).issubset(runtime) or not set(predecessor132['query']).issubset(plan['query']) or not set(predecessor132['shared_registry']).issubset(plan['shared_registry']):
            raise ValueError('A complete132 Runtime selector was removed or baseline count changed')
        host_refusal = json.loads((packet / 'host-refusal-selector-audit.json').read_text())
        if host_refusal['selector'] not in runtime:
            raise ValueError('Actual unmaterialized host-refusal selector removed')
        source = root / host_refusal['path']
        if not source.is_file() or len(re.findall(r'(?m)^\s*fn ' + re.escape(host_refusal['function']) + r'\s*\(', source.read_text())) != 1:
            raise ValueError('Actual unmaterialized host-refusal source test missing or ambiguous')
        predecessor133 = json.loads((packet / 'baseline133-plan.json').read_text())
        old133_runtime = list(dict.fromkeys(predecessor133['runtime'] + compat['exact']))
        if len(old133_runtime) + len(predecessor133['query']) + len(predecessor133['shared_registry']) != 133 or not set(old133_runtime).issubset(runtime) or not set(predecessor133['query']).issubset(plan['query']) or not set(predecessor133['shared_registry']).issubset(plan['shared_registry']):
            raise ValueError('A complete133 Runtime selector was removed or baseline count changed')
        adopted = json.loads((packet / 'adopted-boundary-selector-audit.json').read_text())
        if adopted['added_count'] != 2 or len(adopted['added']) != 2:
            raise ValueError('Adopted RuntimeJim boundary selector audit changed')
        for selected in adopted['added']:
            if selected['selector'] not in runtime:
                raise ValueError('Adopted RuntimeJim boundary selector removed: ' + selected['selector'])
            source = root / selected['path']
            if not source.is_file() or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Adopted RuntimeJim boundary source test missing or ambiguous: ' + selected['selector'])
        predecessor135 = json.loads((packet / 'baseline135-plan.json').read_text())
        old135_runtime = list(dict.fromkeys(predecessor135['runtime'] + compat['exact']))
        if len(old135_runtime) + len(predecessor135['query']) + len(predecessor135['shared_registry']) != 135 or not set(old135_runtime).issubset(runtime) or not set(predecessor135['query']).issubset(plan['query']) or not set(predecessor135['shared_registry']).issubset(plan['shared_registry']):
            raise ValueError('A complete135 Runtime selector was removed or baseline count changed')
        canonical = json.loads((packet / 'canonical-display-selector-audit.json').read_text())
        if canonical['added_count'] != 2 or len(canonical['added']) != 2:
            raise ValueError('Canonical/display Runtime selector audit changed')
        for selected in canonical['added']:
            if selected['selector'] not in runtime:
                raise ValueError('Canonical/display Runtime selector removed: ' + selected['selector'])
            source = root / selected['path']
            if not source.is_file() or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Canonical/display Runtime source test missing or ambiguous: ' + selected['selector'])
        predecessor137 = json.loads((packet / 'baseline137-plan.json').read_text())
        old137_runtime = list(dict.fromkeys(predecessor137['runtime'] + compat['exact']))
        if len(old137_runtime) + len(predecessor137['query']) + len(predecessor137['shared_registry']) != 137 or not set(old137_runtime).issubset(runtime) or not set(predecessor137['query']).issubset(plan['query']) or not set(predecessor137['shared_registry']).issubset(plan['shared_registry']):
            raise ValueError('A complete137 Runtime selector was removed or baseline count changed')
        supplement = json.loads((packet / 'helper-tailcall-selector-audit.json').read_text())
        if supplement['added_count'] != 8 or len(supplement['added']) != 8:
            raise ValueError('Helper/tailcall Runtime selector audit changed')
        for selected in supplement['added']:
            if selected['selector'] not in runtime:
                raise ValueError('Helper/tailcall Runtime selector removed: ' + selected['selector'])
            source = root / selected['path']
            if not source.is_file() or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Helper/tailcall Runtime source test missing or ambiguous: ' + selected['selector'])
        predecessor145 = json.loads((packet / 'baseline145-plan.json').read_text())
        old145_runtime = list(dict.fromkeys(predecessor145['runtime'] + compat['exact']))
        if len(old145_runtime) + len(predecessor145['query']) + len(predecessor145['shared_registry']) != 145 or not set(old145_runtime).issubset(runtime) or not set(predecessor145['query']).issubset(plan['query']) or not set(predecessor145['shared_registry']).issubset(plan['shared_registry']):
            raise ValueError('A complete145 Runtime selector was removed or baseline count changed')
        reset = json.loads((packet / 'reset-order-selector-audit.json').read_text())
        if reset['added_count'] != 3 or len(reset['added']) != 3:
            raise ValueError('Reset-order Runtime selector audit changed')
        for selected in reset['added']:
            if selected['selector'] not in runtime:
                raise ValueError('Reset-order Runtime selector removed: ' + selected['selector'])
        current = json.loads((packet / 'current-source-selector-audit.json').read_text())
        for selected in current['entries']:
            source = root / selected['path']
            if not source.is_file() or sha(source) != live_hash(source_successors, selected['path'], selected['source_sha256']) or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Current source selector changed or is missing: ' + selected['selector'])
        predecessor148 = json.loads((packet / 'baseline148-plan.json').read_text())
        old148_runtime = list(dict.fromkeys(predecessor148['runtime'] + compat['exact']))
        if len(old148_runtime) + len(predecessor148['query']) + len(predecessor148['shared_registry']) != 148 or not set(old148_runtime).issubset(runtime) or not set(predecessor148['query']).issubset(plan['query']) or not set(predecessor148['shared_registry']).issubset(plan['shared_registry']):
            raise ValueError('A complete148 Runtime selector was removed or baseline count changed')
        retiring = json.loads((packet / 'retiring-result-selector-audit.json').read_text())
        if retiring['added_count'] != 2 or len(retiring['added']) != 2 or any(row['selector'] not in runtime for row in retiring['added']):
            raise ValueError('Retiring-result Runtime selector audit changed or a selector was removed')
        combined = json.loads((packet / 'combined-source-afterimages.json').read_text())
        for binding in combined['bindings']:
            source = root / binding['path']
            if not source.is_file() or sha(source) != live_hash(source_successors, binding['path'], binding['after_sha256']):
                raise ValueError('Required actual combined source afterimage changed or is not adopted: ' + binding['path'])
        predecessor150 = json.loads((packet / 'baseline150-plan.json').read_text())
        old150_runtime = list(dict.fromkeys(predecessor150['runtime'] + compat['exact']))
        if len(old150_runtime) + len(predecessor150['query']) + len(predecessor150['shared_registry']) != 150 or not set(old150_runtime).issubset(runtime) or not set(predecessor150['query']).issubset(plan['query']) or not set(predecessor150['shared_registry']).issubset(plan['shared_registry']):
            raise ValueError('A complete150 Runtime selector was removed or baseline count changed')
        code = json.loads((packet / 'code-helper-selector-audit.json').read_text())
        if code['added_count'] != 2 or len(code['added']) != 2 or any(row['selector'] not in runtime for row in code['added']):
            raise ValueError('Code helper selector audit changed or a selector was removed')
        for selected in code['added']:
            source = root / selected['path']
            if not source.is_file() or sha(source) != live_hash(source_successors, selected['path'], selected['source_sha256']) or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Code helper source changed or test is missing: ' + selected['selector'])
        predecessor152 = json.loads((packet / 'predecessor152-frozen-plan.json').read_text())
        old152_runtime = list(dict.fromkeys(predecessor152['runtime'] + compat['exact']))
        if len(old152_runtime) + len(predecessor152['query']) + len(predecessor152['shared_registry']) != 152 or not set(old152_runtime).issubset(runtime) or predecessor152['query'] != retained_query or predecessor152['shared_registry'] != plan['shared_registry']:
            raise ValueError('A complete152 selector was removed or the unchanged query/Registry selection changed')
        predecessor154 = json.loads((packet / 'predecessor154-frozen-plan.json').read_text())
        old154_runtime = list(dict.fromkeys(predecessor154['runtime'] + compat['exact']))
        if len(old154_runtime) + len(predecessor154['query']) + len(predecessor154['shared_registry']) != 154 or not set(old154_runtime).issubset(runtime) or predecessor154['query'] != retained_query or predecessor154['shared_registry'] != plan['shared_registry']:
            raise ValueError('A complete154 predecessor was removed or query/Registry selection changed')
        exists = json.loads((packet / 'exists-selector-audit.json').read_text())
        expected_exists = ['cmd_jim_exists::tests::original_jim_exists_matches_command_variable_and_enum_observations', 'cmd_jim_exists::tests::original_jim_exists_retains_hot_command_and_variable_objects_and_refuses_unknown_channel_flags']
        if exists['added_count'] != 2 or [row['selector'] for row in exists['added']] != expected_exists or set(old154_runtime) - set(old152_runtime) != set(expected_exists):
            raise ValueError('Expected exactly the two genuine Exists801 additions')
        for selected in exists['added']:
            source = root / selected['path']
            if not source.is_file() or sha(source) != live_hash(source_successors, selected['path'], selected['source_sha256']) or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Exists source changed or test is missing: ' + selected['selector'])
        required = json.loads((packet / 'required-diagnostic-source-afterimages.json').read_text())
        for binding in required['bindings']:
            source = root / binding['path']
            if not source.is_file() or sha(source) != live_hash(source_successors, binding['path'], binding['after_sha256']):
                raise ValueError('Required exact current source afterimage changed or is not adopted: ' + binding['path'])
        predecessor156 = json.loads((packet / 'predecessor156-frozen-plan.json').read_text())
        old156_runtime = list(dict.fromkeys(predecessor156['runtime'] + compat['exact']))
        origin = json.loads((packet / 'origin-selector-audit.json').read_text())
        if origin['added_count'] != 2 or len(origin['added']) != 2 or set(old156_runtime) - set(old154_runtime) != {row['selector'] for row in origin['added']}:
            raise ValueError('Expected exactly2 genuine Origin helper additions')
        for selected in origin['added']:
            source = root / selected['path']
            if not source.is_file() or sha(source) != live_hash(source_successors, selected['path'], selected['source_sha256']) or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Origin source changed or its actual test is missing: ' + selected['selector'])
        if len(old156_runtime) + len(predecessor156['query']) + len(predecessor156['shared_registry']) != 156 or not set(old156_runtime).issubset(runtime) or predecessor156['query'] != retained_query or predecessor156['shared_registry'] != plan['shared_registry']:
            raise ValueError('A complete156 predecessor was removed or query/Registry selection changed')
        predecessor158 = json.loads((packet / 'predecessor158-frozen-plan.json').read_text())
        old158_runtime = list(dict.fromkeys(predecessor158['runtime'] + compat['exact']))
        inscope = json.loads((packet / 'inscope-selector-audit.json').read_text())
        if inscope['added_count'] != 2 or len(inscope['added']) != 2 or set(old158_runtime) - set(old156_runtime) != {row['selector'] for row in inscope['added']}:
            raise ValueError('Expected exactly2 genuine Inscope additions')
        for selected in inscope['added']:
            source = root / selected['path']
            if not source.is_file() or sha(source) != live_hash(source_successors, selected['path'], selected['source_sha256']) or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Inscope source changed or actual test is missing: ' + selected['selector'])
        if len(old158_runtime) + len(predecessor158['query']) + len(predecessor158['shared_registry']) != 158 or not set(old158_runtime).issubset(runtime) or predecessor158['query'] != retained_query or predecessor158['shared_registry'] != plan['shared_registry']:
            raise ValueError('A complete158 predecessor was removed or query/Registry selection changed')
        predecessor160 = json.loads((packet / 'predecessor160-frozen-plan.json').read_text())
        old160_runtime = list(dict.fromkeys(predecessor160['runtime'] + compat['exact']))
        inventory = json.loads((packet / 'inventory-selector-audit.json').read_text())
        if inventory['added_count'] != 2 or len(inventory['added']) != 2 or set(old160_runtime) - set(old158_runtime) != {row['selector'] for row in inventory['added']}:
            raise ValueError('Expected exactly2 original inventory additions')
        for selected in inventory['added']:
            source = root / selected['path']
            if not source.is_file() or sha(source) != live_hash(source_successors, selected['path'], selected['source_sha256']) or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Inventory source changed or actual test missing: ' + selected['selector'])
        if len(old160_runtime) + len(predecessor160['query']) + len(predecessor160['shared_registry']) != 160 or not set(old160_runtime).issubset(runtime) or predecessor160['query'] != retained_query or predecessor160['shared_registry'] != plan['shared_registry']:
            raise ValueError('A complete160 predecessor was removed or query/Registry selection changed')
        predecessor162 = json.loads((packet / 'predecessor162-frozen-plan.json').read_text())
        old162_runtime = list(dict.fromkeys(predecessor162['runtime'] + compat['exact']))
        info_helper = json.loads((packet / 'info-helper-selector-audit.json').read_text())
        if info_helper['added_count'] != 2 or len(info_helper['added']) != 2 or set(old162_runtime) - set(old160_runtime) != {row['selector'] for row in info_helper['added']}:
            raise ValueError('Expected exactly2 private Info helper additions')
        for selected in info_helper['added']:
            source = root / selected['path']
            if not source.is_file() or sha(source) != live_hash(source_successors, selected['path'], selected['source_sha256']) or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Private Info helper source changed or actual test missing: ' + selected['selector'])
        if len(old162_runtime) + len(predecessor162['query']) + len(predecessor162['shared_registry']) != 162 or not set(old162_runtime).issubset(runtime) or predecessor162['query'] != retained_query or predecessor162['shared_registry'] != plan['shared_registry']:
            raise ValueError('A complete162 predecessor was removed or query/Registry selection changed')
        predecessor165 = json.loads((packet / 'predecessor165-frozen-plan.json').read_text())
        old165_runtime = list(dict.fromkeys(predecessor165['runtime'] + compat['exact']))
        variable_inventory = json.loads((packet / 'variable-inventory-selector-audit.json').read_text())
        if variable_inventory['added_count'] != 3 or len(variable_inventory['added']) != 3 or set(old165_runtime) - set(old162_runtime) != {row['selector'] for row in variable_inventory['added']}:
            raise ValueError('Expected exactly3 original variable inventory additions')
        for selected in variable_inventory['added']:
            source = root / selected['path']
            if not source.is_file() or sha(source) != live_hash(source_successors, selected['path'], selected['source_sha256']) or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Variable inventory source changed or actual test missing: ' + selected['selector'])
        if len(old165_runtime) + len(predecessor165['query']) + len(predecessor165['shared_registry']) != 165 or not set(old165_runtime).issubset(runtime) or predecessor165['query'] != retained_query or predecessor165['shared_registry'] != plan['shared_registry']:
            raise ValueError('A complete165 predecessor was removed or query/Registry selection changed')
        predecessor168 = json.loads((packet / 'predecessor168-frozen-plan.json').read_text())
        old168_runtime = list(dict.fromkeys(predecessor168['runtime'] + compat['exact']))
        delete_helper = json.loads((packet / 'delete-helper-selector-audit.json').read_text())
        if delete_helper['added_count'] != 2 or len(delete_helper['added']) != 2 or set(old168_runtime) - set(old165_runtime) != {row['selector'] for row in delete_helper['added']}:
            raise ValueError('Expected exactly2 original Delete helper additions')
        for selected in delete_helper['added']:
            source = root / selected['path']
            if not source.is_file() or sha(source) != live_hash(source_successors, selected['path'], selected['source_sha256']) or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Delete helper source changed or actual test missing: ' + selected['selector'])
        old167_runtime = list(dict.fromkeys(predecessor167['runtime'] + compat['exact']))
        if old168_runtime != old167_runtime or predecessor167['shared_registry'] != plan['shared_registry'] or len(old167_runtime) + len(retained_query) + len(predecessor167['shared_registry']) != 167:
            raise ValueError('The full immutable167 Runtime/Registry/query selection changed')
        fresh_namespace = json.loads((packet / 'fresh-namespace-selector-audit.json').read_text())
        if fresh_namespace['added_count'] != 1 or len(fresh_namespace['added']) != 1 or [row['selector'] for row in fresh_namespace['added']] != plan['query'][50:]:
            raise ValueError('Expected exactly one fresh namespace integration query addition')
        for selected in fresh_namespace['added']:
            source = root / selected['path']
            if not source.is_file() or sha(source) != live_hash(source_successors, selected['path'], selected['source_sha256']) or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Fresh namespace query source changed or actual test missing: ' + selected['selector'])
        if len(old168_runtime) + len(predecessor168['query']) + len(predecessor168['shared_registry']) != 168 or not set(old168_runtime).issubset(runtime) or predecessor168['query'] != plan['query'] or predecessor168['shared_registry'] != plan['shared_registry']:
            raise ValueError('A complete168 predecessor was removed or query/Registry selection changed')
        predecessor170 = json.loads((packet / 'predecessor170-frozen-plan.json').read_text())
        old170_runtime = list(dict.fromkeys(predecessor170['runtime'] + compat['exact']))
        upvar_helper = json.loads((packet / 'upvar-helper-selector-audit.json').read_text())
        if upvar_helper['added_count'] != 2 or len(upvar_helper['added']) != 2 or set(old170_runtime) - set(old168_runtime) != {row['selector'] for row in upvar_helper['added']}:
            raise ValueError('Expected exactly2 original Upvar helper additions')
        for selected in upvar_helper['added']:
            source = root / selected['path']
            if not source.is_file() or sha(source) != live_hash(source_successors, selected['path'], selected['source_sha256']) or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Upvar helper source changed or actual test missing: ' + selected['selector'])
        if len(old170_runtime) + len(predecessor170['query']) + len(predecessor170['shared_registry']) != 170 or not set(old170_runtime).issubset(runtime) or predecessor170['query'] != plan['query'] or predecessor170['shared_registry'] != plan['shared_registry']:
            raise ValueError('A complete170 predecessor was removed or query/Registry selection changed')
        predecessor172 = json.loads((packet / 'predecessor172-frozen-plan.json').read_text())
        old172_runtime = list(dict.fromkeys(predecessor172['runtime'] + compat['exact']))
        parent_text = json.loads((packet / 'parent-text-selector-audit.json').read_text())
        if parent_text['added_count'] != 2 or len(parent_text['added']) != 2 or set(old172_runtime) - set(old170_runtime) != {row['selector'] for row in parent_text['added']}:
            raise ValueError('Expected exactly2 original Parent text additions')
        for selected in parent_text['added']:
            source = root / selected['path']
            if not source.is_file() or sha(source) != live_hash(source_successors, selected['path'], selected['source_sha256']) or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('Parent text source changed or actual test missing: ' + selected['selector'])
        if len(old172_runtime) + len(predecessor172['query']) + len(predecessor172['shared_registry']) != 172 or not set(old172_runtime).issubset(runtime) or predecessor172['query'] != plan['query'] or predecessor172['shared_registry'] != plan['shared_registry']:
            raise ValueError('A complete172 predecessor was removed or query/Registry selection changed')
        namespace_text = json.loads((packet / 'namespace-text-selector-audit.json').read_text())
        if namespace_text['added_count'] != 2 or len(namespace_text['added']) != 2 or set(runtime) - set(old172_runtime) != {row['selector'] for row in namespace_text['added']}:
            raise ValueError('Expected exactly2 original namespace text result additions')
        for selected in namespace_text['added']:
            source = root / selected['path']
            if not source.is_file() or sha(source) != live_hash(source_successors, selected['path'], selected['source_sha256']) or len(re.findall(r'(?m)^\s*fn ' + re.escape(selected['function']) + r'\s*\(', source.read_text())) != 1:
                raise ValueError('namespace text result source changed or actual test missing: ' + selected['selector'])
        expected = sum(map(len, groups.values()))
        if expected != 174 or plan['native_total'] != 174:
            raise ValueError('Expected all172 predecessor controls plus2 namespace text controls, got ' + str(expected))
        builds = [('runtime', ['cargo', 'test', '--offline', '--manifest-path', str(root / 'runtime/rust/Cargo.toml'), '--features', 'engine', '--lib', '--test', 'native_namespace_command_query', '--no-run', '--message-format=json', '-j1']), ('registry', ['cargo', 'test', '--offline', '-p', 'tcl-registry', '--lib', '--no-run', '--message-format=json', '-j1'])]
    if any(not names or len(names) != len(set(names)) for names in groups.values()):
        raise ValueError('Empty or duplicate selection')
    for label, command in builds:
        built = run(command, label + '-build.jsonl', 1800)
        result['builds'].append(built)
        if built['exit_code'] or built['timed_out']:
            raise ValueError('Build failed: ' + label)
        for line in (out / built['log']).read_text().splitlines():
            try:
                row = json.loads(line)
            except ValueError:
                continue
            if row.get('reason') == 'compiler-artifact':
                cargo_artifact_rows.append(row)
            if row.get('reason') == 'compiler-artifact' and row.get('profile', {}).get('test') and row.get('executable') and row['target']['name'] in groups:
                artifacts[row['target']['name']] = Path(row['executable'])
    if set(artifacts) != set(groups):
        raise ValueError('Missing actual compiled binary: ' + str(set(groups) - set(artifacts)))
    actual_dep_info = set()
    for row in cargo_artifact_rows:
        candidates = set()
        for reported in row['filenames']:
            artifact = Path(reported)
            if row['target']['kind'] == ['custom-build']:
                digest = artifact.parent.name.rsplit('-', 1)[-1]
                candidates.add(artifact.parent / ('build_script_build-' + digest + '.d'))
            else:
                stem = artifact.stem if artifact.suffix in {'.rlib', '.rmeta', '.so', '.a', '.dylib', '.dll', '.exe'} else artifact.name
                if stem.startswith('lib') and row['target']['kind'] != ['bin']:
                    stem = stem[3:]
                candidates.add(artifact.parent / (stem + '.d'))
        existing = {path.resolve() for path in candidates if path.is_file()}
        if not existing:
            raise ValueError('Missing dependency info for actual Cargo artifact: ' + row['target']['name'])
        actual_dep_info.update(existing)
    (out / 'actual-cargo-artifacts.json').write_text(json.dumps(cargo_artifact_rows, indent=2) + '\n')
    dep_info, unresolved, uncovered = {}, [], []
    for path in sorted(actual_dep_info):
        dep_info[str(path)] = sha(path)
        for line in path.read_text(errors='strict').replace('\\\n', '').splitlines():
            if line.startswith('#') or ': ' not in line:
                continue
            for word in shlex.split(line.split(': ', 1)[1]):
                original = Path(word)
                if not original.is_absolute():
                    original = next((candidate for candidate in [root / 'runtime/rust' / original, root / original] if candidate.is_file()), original)
                if not original.is_file():
                    unresolved.append({'dep_info': str(path), 'input': word})
                    continue
                original = original.resolve()
                inputs[str(original)] = sha(original)
                if str(original) not in before and not original.is_relative_to(target):
                    uncovered.append(str(original))
    if not dep_info or not inputs:
        raise ValueError('Empty compiled dependency closure')
    result['artifacts'] = {str(path): sha(path) for path in artifacts.values()}
    (out / 'compiled-closure.json').write_text(json.dumps({'inputs': inputs, 'dep_info': dep_info, 'unresolved': unresolved, 'uncovered': sorted(set(uncovered)), 'artifacts': result['artifacts']}, indent=2) + '\n')
    if unresolved or uncovered:
        raise ValueError('Compiled source coverage incomplete')
    for label, binary in artifacts.items():
        listed = run([str(binary), '--list'], label + '.list', 30)
        if listed['exit_code'] or listed['timed_out']:
            raise ValueError('Actual binary list failed')
        names = [line.removesuffix(': test') for line in (out / listed['log']).read_text().splitlines() if line.endswith(': test')]
        missing = [name for name in groups[label] if name not in names]
        if not names or missing:
            raise ValueError('Missing exact selectors: ' + str(missing))
    result['selected_tests'] = groups
    (out / 'exact-plan.json').write_text(json.dumps([{'binary': str(artifacts[label]), 'selector': name} for label, names in groups.items() for name in names], indent=2) + '\n')
    for label, names in groups.items():
        for name in names:
            print('Starting ' + label + ' ' + name, flush=True)
            row = run([str(artifacts[label]), name, '--exact', '--nocapture', '--test-threads=1'], f'test-{len(result["runs"]) + 1:02}.log', 180)
            content = (out / row['log']).read_text()
            row.update(selector=name, binary_kind=label, running_counts=re.findall(r'running (\d+) tests?', content), summaries=re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed;', content))
            row['passed'] = row['exit_code'] == 0 and not row['timed_out'] and row['running_counts'] == ['1'] and row['summaries'] == [('ok', '1', '0')]
            result['runs'].append(row)
            (out / 'runs-in-progress.json').write_text(json.dumps(result['runs'], indent=2) + '\n')
    result['passed'] = len(result['runs']) == sum(map(len, groups.values())) and all(row['passed'] for row in result['runs'])
except Exception as error:
    result['errors'].append(str(error))
finally:
    after = snapshot()
    compressed('source-after.json.gz', after)
    result['source_drift'] = {path: [before.get(path), after.get(path)] for path in set(before) | set(after) if before.get(path) != after.get(path)}
    result['compiled_input_drift'] = {path: [digest, sha(Path(path)) if Path(path).is_file() else None] for path, digest in inputs.items() if not Path(path).is_file() or sha(Path(path)) != digest}
    result['artifact_drift'] = {path: [digest, sha(Path(path)) if Path(path).is_file() else None] for path, digest in result.get('artifacts', {}).items() if not Path(path).is_file() or sha(Path(path)) != digest}
    result.update(attempted=len(result['runs']), passed_count=sum(row['passed'] for row in result['runs']), failed_count=sum(not row['passed'] for row in result['runs']), finished_utc=now())
    if result['source_drift'] or result['compiled_input_drift'] or result['artifact_drift']:
        result['passed'] = False
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({key: result[key] for key in ['passed', 'attempted', 'passed_count', 'failed_count', 'errors']}), flush=True)
raise SystemExit(0 if result['passed'] else 1)
