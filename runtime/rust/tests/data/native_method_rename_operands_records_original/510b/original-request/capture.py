"""Root-only capture of the unchanged original method rename operand/table probe."""
from pathlib import Path
import hashlib
import json
import os
import shlex
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parent
PLAN = json.loads((ROOT / 'launch-plan.json').read_bytes())
OUT = Path(PLAN['output_directory'])
SHA = lambda value: hashlib.sha256(value).hexdigest()

def digest(path):
    result = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            result.update(block)
    return result.hexdigest()

def verify(pins):
    actual = {path: digest(path) for path in pins}
    changed = [path for path, expected in pins.items() if actual[path] != expected]
    if changed:
        raise RuntimeError('changed original inputs: ' + ', '.join(changed))
    return actual

def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')

# No output directory is reused and no source is patched during a capture.
verify(PLAN['common_inputs'])
OUT.mkdir(exist_ok=False)
for path in ROOT.rglob('*'):
    if path.is_file():
        target = OUT / 'original-request' / path.relative_to(ROOT)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target)
common = dict(PLAN['common_inputs'])
common[str(ROOT / 'launch-plan.json')] = digest(ROOT / 'launch-plan.json')
common[str(ROOT / 'capture.py')] = digest(ROOT / 'capture.py')
request_sha = digest(ROOT / 'request.json')
launcher_sha = digest(ROOT / 'capture.py')

def run(directory, label, argv, environment, pins, timeout=60):
    before = verify(pins)
    start = time.monotonic()
    timed_out = False
    try:
        completed = subprocess.run(argv, cwd=str(OUT), env=environment,
                                   capture_output=True, timeout=timeout)
        code, stdout, stderr = completed.returncode, completed.stdout, completed.stderr
    except subprocess.TimeoutExpired as error:
        code, stdout, stderr = None, error.stdout or b'', error.stderr or b''
        timed_out = True
    (directory / (label + '.stdout')).write_bytes(stdout)
    (directory / (label + '.stderr')).write_bytes(stderr)
    # Retain the process and its streams even if a reached input changed.
    after = {path: digest(path) for path in pins}
    unchanged = before == after
    receipt = {'command': argv, 'cwd': str(OUT), 'exit': code,
               'timeout': timed_out, 'seconds': time.monotonic() - start,
               'stdout_file': label + '.stdout', 'stderr_file': label + '.stderr',
               'stdout_sha256': SHA(stdout), 'stderr_sha256': SHA(stderr),
               'inputs_before': before, 'inputs_after': after,
               'inputs_unchanged': unchanged,
               'request_sha256': request_sha, 'launcher_sha256': launcher_sha}
    write(directory / (label + '.receipt.json'), receipt)
    print(directory.name, label, 'exit', code, 'unchanged', unchanged, flush=True)
    if not unchanged:
        raise RuntimeError('input changed during ' + label)
    return receipt, stdout

environment_base = os.environ.copy()
compiler, _ = run(OUT, 'compiler-version', PLAN['compiler_version_argv'],
                  environment_base, common)
providers = []
for item in PLAN['providers']:
    directory = OUT / item['provider']
    directory.mkdir()
    pins = {**common, **item['inputs']}
    environment = environment_base.copy()
    environment.update(item['selected_environment'])
    compiler_prefix = item['compiler_prefix']
    source = str(OUT / 'original-request' / 'probe.c')
    # Dependency discovery is a separately recorded compiler process. Actual
    # headers are then pinned before the compilation producing the executable.
    dependency_file = directory / 'probe.dependencies'
    dependencies, _ = run(directory, 'dependencies',
        compiler_prefix + ['-M', '-MF', str(dependency_file), source],
        environment, pins)
    receipt = {'question_id': PLAN['question_id'], 'provider': item['provider'],
               'scope': PLAN['scope'], 'selected_environment': item['selected_environment'],
               'prior_build_input_receipt': item['prior_build_input_receipt'],
               'prior_build_input_receipt_sha256': item['prior_build_input_receipt_sha256'],
               'compiler_version': compiler, 'dependencies': dependencies,
               'request_sha256': request_sha, 'launcher_sha256': launcher_sha}
    if dependencies['exit'] != 0:
        receipt['status'] = 'dependency-discovery-failed-no-native-execution'
        write(directory / 'receipt.json', receipt)
        providers.append(receipt)
        continue
    dependency_words = shlex.split(dependency_file.read_text().replace('\\\n', ' '))
    header_paths = dependency_words[dependency_words.index(':') + 1:] if ':' in dependency_words else dependency_words[1:]
    for path in header_paths:
        pins[path] = digest(path)
    write(directory / 'actual-header-inputs.json', {p: pins[p] for p in header_paths})
    executable = directory / 'probe.elf'
    compile_argv = compiler_prefix + [source] + item['link_inputs'] + [
        '-Wl,--trace', '-o', str(executable)]
    compilation, link_stdout = run(directory, 'compile', compile_argv, environment, pins)
    receipt['compile'] = compilation
    receipt['status'] = 'compiled' if compilation['exit'] == 0 else 'compile-failed-no-native-execution'
    if compilation['exit'] == 0:
        observed_link_inputs = {}
        unpinned_link_inputs = []
        for line in link_stdout.decode('utf-8', errors='replace').splitlines():
            path = Path(line)
            if not path.is_absolute() or not path.is_file():
                continue
            # cc's reached temporary object is derived solely in this recorded
            # compilation; every external linker file must have a before pin.
            if str(path).startswith('/tmp/cc') and path.suffix == '.o':
                observed_link_inputs[str(path)] = 'compiler-temporary-object'
            else:
                observed_link_inputs[str(path)] = digest(path)
                resolved = path.resolve()
                before_match = pins.get(str(path)) or pins.get(str(resolved))
                if before_match != observed_link_inputs[str(path)]:
                    unpinned_link_inputs.append(str(path))
        receipt['observed_link_inputs'] = observed_link_inputs
        receipt['unpredicted_external_link_inputs'] = unpinned_link_inputs
        pins[str(executable)] = digest(executable)
        version_argv = [item['cli'], str(OUT / 'original-request' / item['version_source'])]
        pins[version_argv[1]] = digest(version_argv[1])
        version, _ = run(directory, 'cli-version', version_argv, environment, pins)
        receipt['cli_version'] = version
        if unpinned_link_inputs:
            receipt['status'] = 'link-closure-not-prepinned-no-native-probe-execution'
        elif version['exit'] != 0:
            receipt['status'] = 'cli-version-failed-no-native-probe-execution'
        else:
            execution, stdout = run(directory, 'execute', [str(executable)] + PLAN['execution_argv_tail'], environment, pins)
            receipt['execution'] = execution
            receipt['observed_record_counts'] = {label: sum(line.startswith(label.encode() + b'\t') for line in stdout.splitlines()) for label in ['ROW','EVENT','CASE','AVAILABILITY','UNAVAILABLE','API','METHOD','RESULT','ERRORCODE']}
            receipt['status'] = 'closed' if execution['exit'] == 0 else 'probe-failed'
        receipt['executable_sha256'] = pins[str(executable)]
    receipt['inputs_verified_before_and_after'] = verify(pins)
    write(directory / 'receipt.json', receipt)
    providers.append(receipt)
write(OUT / 'summary.json', {'question_id': PLAN['question_id'],
    'request_sha256': request_sha, 'launcher_sha256': launcher_sha,
    'scope': PLAN['scope'], 'providers': [{
        'provider': row['provider'], 'status': row['status'],
        'receipt_sha256': digest(OUT / row['provider'] / 'receipt.json')
    } for row in providers]})
verify(common)
