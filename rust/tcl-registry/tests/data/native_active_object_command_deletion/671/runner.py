from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import hashlib, json, os, re, shlex, subprocess, time

REPO = Path('/workspace/tcl-lsp')
DESIGN = Path('/tmp/sourceperf-active-command-retirement-design670')
ROOT = Path('/tmp/native-active-command-retirement671')
BASE = REPO / 'rust/tcl-registry/tests/data/native_command_recreation_original/request'
assert not ROOT.exists()
ROOT.mkdir()

def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

def write_json(path, value):
    with path.open('x') as stream:
        stream.write(json.dumps(value, indent=2) + '\n')

def pins(mapping):
    actual = {name: sha(Path(name)) for name in mapping}
    assert actual == mapping, [name for name in mapping if mapping[name] != actual[name]]
    return actual

assert sha(DESIGN / 'manifest.json') == '61ce40d23b44966fe428bb8a4c3242291a55eeeac089e88abbfaa0acba07227a'
design = json.loads((DESIGN / 'manifest.json').read_text())
for row in design['files']:
    assert sha(DESIGN / row['path']) == row['sha256']
plan = json.loads((BASE / 'launch-plan.json').read_text())
original_inputs = dict(plan['common_inputs'])
for provider in plan['providers']:
    for name, digest in provider['inputs'].items():
        assert name not in original_inputs or original_inputs[name] == digest
        original_inputs[name] = digest
pins(original_inputs)

# The callback experiment is byte-exact. Add only the existing public version
# observer, in its own fresh interpreter before either experiment interpreter.
probe = (DESIGN / 'probe.c').read_text()
base = (BASE / 'probe.c').read_text()
hex_result = base[base.index('static void hex_result('):base.index('static void window(')]
metadata = base[base.index('static int metadata('):base.index('int main(void)')]
for backend, helper in [
    ('Jim_SetResultString(i,s,-1); }', 'static int eval(Interp *i,const char *s) { return Jim_EvalGlobal(i,s); }'),
    ('Tcl_SetObjResult(i,Tcl_NewStringObj(s,-1)); }', 'static int eval(Interp *i,const char *s) { return Tcl_EvalEx(i,s,-1,TCL_EVAL_GLOBAL); }'),
]:
    assert probe.count(backend) == 1
    probe = probe.replace(backend, backend + '\n' + helper)
assert probe.count('int main(void) {') == 1
probe = probe.replace('int main(void) {', hex_result + metadata + 'int main(void) {\n    int metadata_status=metadata(); if(metadata_status)return metadata_status;')
(ROOT / 'probe.c').write_text(probe)
for name in ['c-version.tcl', 'jim-version.tcl']:
    (ROOT / name).write_bytes((BASE / name).read_bytes())
for row in design['files']:
    (ROOT / ('design-' + row['path'])).write_bytes((DESIGN / row['path']).read_bytes())
write_json(ROOT / 'request.json', {
    'question_id': 'naming.embedding.original-active-object-command-deletion',
    'design_manifest_sha256': sha(DESIGN / 'manifest.json'),
    'design_probe_sha256': sha(DESIGN / 'probe.c'),
    'probe_sha256': sha(ROOT / 'probe.c'),
    'original_callback_experiment': 'The two source-defined callbacks/cases are unchanged. The independently pinned base public VERSION/BUILD/ABI observer is added before them in a third fresh interpreter per process.',
    'scope': 'Public active object-command deletion timing and once-only same-name recreation, with independent stack-backed client records and one held original caller word. No client-data freeing, command refcounts, address identity, Procedure, imported-command, recursive, rename, hide, Rust or BIG-IP parity is inferred.',
    'process_environment': 'Complete deliberately minimal environment is recorded for every launched process; no ambient credentials or unrelated application settings are inherited.',
    'base_launch_plan': str(BASE / 'launch-plan.json'),
    'base_launch_plan_sha256': sha(BASE / 'launch-plan.json'),
})
request_inputs = {str(p): sha(p) for p in ROOT.iterdir() if p.is_file()}
all_initial = {**original_inputs, **request_inputs}
write_json(ROOT / 'inputs-before.json', pins(all_initial))

def capture(directory, label, argv, env, timeout=60):
    started = time.monotonic()
    result = subprocess.run(argv, cwd=directory, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout)
    stdout = directory / (label + '.stdout')
    stderr = directory / (label + '.stderr')
    stdout.write_bytes(result.stdout)
    stderr.write_bytes(result.stderr)
    record = {'argv': argv, 'cwd': str(directory), 'environment': env, 'exit': result.returncode, 'seconds': time.monotonic() - started, 'stdout': str(stdout), 'stdout_sha256': sha(stdout), 'stderr': str(stderr), 'stderr_sha256': sha(stderr)}
    write_json(directory / (label + '.json'), record)
    return result, record

env_base = {'PATH': '/usr/bin:/bin', 'LANG': 'C', 'LC_ALL': 'C', 'TZ': 'UTC'}
compiler, compiler_record = capture(ROOT, 'compiler-version', plan['compiler_version_argv'], env_base)
assert compiler.returncode == 0

def run_provider(provider):
    directory = ROOT / provider['provider']
    directory.mkdir()
    env = {**env_base, **provider['selected_environment']}
    selected = {**all_initial, **provider['inputs']}
    pins(selected)
    prefix = provider['compiler_prefix']
    dependency, dependency_record = capture(directory, 'headers', prefix + ['-M', str(ROOT / 'probe.c')], env)
    assert dependency.returncode == 0, (provider['provider'], 'dependency status')
    body = dependency.stdout.decode().replace('\\\n', ' ').split(':', 1)[1]
    reached_headers = {}
    for name in shlex.split(body):
        path = Path(name)
        if not path.is_absolute():
            path = directory / path
        path = path.resolve()
        reached_headers[str(path)] = sha(path)
    selected.update(reached_headers)
    write_json(directory / 'inputs-before.json', pins(selected))
    image = directory / 'probe.elf'
    compile_argv = prefix + [str(ROOT / 'probe.c')] + provider['link_inputs'] + ['-Wl,--trace', '-o', str(image)]
    compiled, compile_record = capture(directory, 'compile', compile_argv, env)
    assert compiled.returncode == 0, (provider['provider'], 'compile status')
    reached_link_files = {}
    for raw in compiled.stdout.decode().splitlines():
        name = raw.split('(', 1)[0].strip()
        path = Path(name)
        if path.is_absolute() and path.is_file():
            actual = sha(path)
            known = selected.get(name, selected.get(str(path.resolve())))
            assert known == actual, (provider['provider'], 'unpredicted link input', name)
            reached_link_files[name] = actual
    assert reached_link_files, 'no actual linker files observed'
    assert image.is_file() and not image.is_symlink()
    digest = sha(image)
    pins(selected)
    cli, cli_record = capture(directory, 'cli-version', [provider['cli'], str(ROOT / provider['version_source'])], env)
    assert cli.returncode == 0
    observed, observed_record = capture(directory, 'probe', [str(image)], env)
    assert observed.returncode == 0
    assert sha(image) == digest
    after = pins(selected)
    write_json(directory / 'inputs-after.json', after)
    # These checks establish only stream completeness, never expected behavior.
    lines = observed.stdout.decode('ascii').splitlines()
    assert len([s for s in lines if s.startswith('VERSION\t')]) == 1
    assert len([s for s in lines if s.startswith('BUILD\t')]) == 1
    assert len([s for s in lines if s.startswith('ABI\t')]) == 1
    cases = {}
    for mode in [0, 1]:
        rows = [s for s in lines if s.startswith(('EVENT\t', 'RESULT\t')) and f'case={mode}\t' in s]
        assert rows and any('label=INTERPRETER_DESTROYED\t' in s for s in rows)
        sequences = [int(re.search(r'\bsequence=(\d+)', s).group(1)) for s in rows]
        assert sequences == list(range(1, len(rows) + 1))
        assert len([s for s in rows if 'stage=first-public-invocation-returned\t' in s]) == 1
        assert len([s for s in rows if 'stage=second-public-invocation-returned\t' in s]) == 1
        cases[str(mode)] = rows
    receipt = {'provider': provider['provider'], 'whole_executable_sha256': digest, 'image': str(image), 'image_bytes': image.stat().st_size, 'source': str(ROOT / 'probe.c'), 'source_sha256': sha(ROOT / 'probe.c'), 'headers': dependency_record, 'compile': compile_record, 'actual_link_files': reached_link_files, 'cli_version': cli_record, 'observation': observed_record, 'complete_case_streams': cases, 'all_inputs_unchanged': True, 'scope': 'Two independent fresh experiment interpreters plus one independent metadata interpreter. Callback experiment only; all original streams retained. CLI and linked library versions are distinct actual observations. Prior SDK configure/build receipts remain provenance only, without invented process evidence.'}
    write_json(directory / 'receipt.json', receipt)
    return {'provider': provider['provider'], 'receipt': str(directory / 'receipt.json'), 'receipt_sha256': sha(directory / 'receipt.json'), 'exit': observed.returncode, 'case_blocks': len(cases), 'image_sha256': digest, 'version_rows': [s for s in lines if s.startswith(('VERSION\t', 'BUILD\t', 'ABI\t'))]}

with ThreadPoolExecutor(max_workers=3) as pool:
    results = list(pool.map(run_provider, plan['providers']))
write_json(ROOT / 'inputs-after.json', pins(all_initial))
summary = {'providers': results, 'processes': 6, 'case_blocks': 12, 'complete_native_inputs_unchanged': True, 'request_sha256': sha(ROOT / 'request.json'), 'probe_sha256': sha(ROOT / 'probe.c'), 'compiler_version_receipt_sha256': sha(ROOT / 'compiler-version.json'), 'scope': 'Fresh public callback observations only. No Rust assertion, general active command lifetime or client-data free/reference-count parity is claimed.'}
write_json(ROOT / 'summary.json', summary)
print(json.dumps(summary), flush=True)
