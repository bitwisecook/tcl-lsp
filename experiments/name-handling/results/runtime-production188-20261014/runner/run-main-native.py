from pathlib import Path
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

provider_paths = set()
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
providers.append({'kind': 'Jim', 'executable': str(jim_executable), 'executable_sha256': sha(jim_executable), 'probe': 'puts [list [info patchlevel]]\n', 'latest_check': json.loads((packet / 'provider-check.json').read_text())})
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

result = {'started_utc': now(), 'providers': providers, 'builds': [], 'runs': [], 'errors': [], 'passed': False, 'scope': 'Separate shared API four exact controls' if args.api_only else 'Runtime/query/registry full117: all113 prior selectors plus four original global-address/hidden-option controls; Engine/WASM and CPP/#2286 remain separate'}
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
        previous113 = json.loads((packet / 'baseline113-plan.json').read_text())
        if not set(previous113['runtime']).issubset(runtime) or not set(previous113['query']).issubset(plan['query']) or not set(previous113['shared_registry']).issubset(plan['shared_registry']):
            raise ValueError('A previous113 selector was dropped')
        previous = json.loads((packet / 'baseline110-plan.json').read_text())
        if not set(previous['runtime']).issubset(runtime) or not set(previous['query']).issubset(plan['query']) or not set(previous['shared_registry']).issubset(plan['shared_registry']):
            raise ValueError('A previous full110 selector was dropped')
        expected = sum(map(len, groups.values()))
        if expected != 117:
            raise ValueError('Expected all113 previous plus four original global-address/hidden-option selectors, got ' + str(expected))
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
