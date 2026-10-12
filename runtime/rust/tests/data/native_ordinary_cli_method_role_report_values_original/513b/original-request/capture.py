"""Root-only original CLI capture; this file does not compile any native program."""
from pathlib import Path
import hashlib
import json
import os
import re
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parent
PLAN = json.loads((ROOT / 'launch-plan.json').read_bytes())
OUT = Path(PLAN['output_directory'])

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

# Manifest verification precedes creating any output. Root launches this exact
# request, and a previous directory is never reused or patched.
manifest = json.loads((ROOT / 'manifest.json').read_bytes())
verify({str(ROOT / path): sha for path, sha in manifest['files'].items()})
verify(PLAN['common_inputs'])
for provider in PLAN['providers']:
    verify(provider['inputs'])
OUT.mkdir(exist_ok=False)
for path in ROOT.rglob('*'):
    if path.is_file():
        target = OUT / 'original-request' / path.relative_to(ROOT)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target)
common = dict(PLAN['common_inputs'])
for path in (OUT / 'original-request').rglob('*'):
    if path.is_file():
        common[str(path)] = digest(path)
request_sha = digest(ROOT / 'request.json')
launcher_sha = digest(ROOT / 'capture.py')
common[str(ROOT / 'capture.py')] = launcher_sha
common[str(ROOT / 'launch-plan.json')] = digest(ROOT / 'launch-plan.json')
common[str(ROOT / 'manifest.json')] = digest(ROOT / 'manifest.json')

def run(directory, label, argv, environment, pins):
    before = verify(pins)
    start = time.monotonic()
    timed_out = False
    try:
        completed = subprocess.run(argv, cwd=str(OUT), env=environment,
                                   capture_output=True,
                                   timeout=PLAN['process_timeout_seconds'])
        code, stdout, stderr = completed.returncode, completed.stdout, completed.stderr
    except subprocess.TimeoutExpired as error:
        code, stdout, stderr = None, error.stdout or b'', error.stderr or b''
        timed_out = True
    (directory / (label + '.stdout')).write_bytes(stdout)
    (directory / (label + '.stderr')).write_bytes(stderr)
    # Record reached streams even if a callback/child modified a pinned input.
    after = {path: digest(path) for path in pins}
    receipt = {'argv': argv, 'cwd': str(OUT), 'exit': code,
               'signal': -code if code is not None and code < 0 else None,
               'timeout': timed_out, 'seconds': time.monotonic() - start,
               'selected_environment': {k: environment[k] for k in
                   ['TCL_LIBRARY','TCLLIBPATH','LD_PRELOAD','LD_LIBRARY_PATH']
                   if k in environment},
               'stdout_file': label + '.stdout', 'stderr_file': label + '.stderr',
               'stdout_sha256': hashlib.sha256(stdout).hexdigest(),
               'stderr_sha256': hashlib.sha256(stderr).hexdigest(),
               'inputs_before': before, 'inputs_after': after,
               'inputs_unchanged': before == after,
               'request_sha256': request_sha, 'launcher_sha256': launcher_sha}
    write(directory / (label + '.receipt.json'), receipt)
    print(directory.name, label, 'exit', code,
          'unchanged', receipt['inputs_unchanged'], flush=True)
    if not receipt['inputs_unchanged']:
        raise RuntimeError('input changed during ' + label)
    return receipt, stdout

base_environment = os.environ.copy()
for variable in ['LD_PRELOAD','LD_LIBRARY_PATH','TCL_LIBRARY','TCLLIBPATH']:
    base_environment.pop(variable, None)
providers = []
for item in PLAN['providers']:
    directory = OUT / item['provider']
    directory.mkdir()
    pins = {**common, **item['inputs']}
    environment = base_environment.copy()
    environment.update(item['selected_environment'])
    receipt = {'question_id': PLAN['question_id'], 'provider': item['provider'],
               'scope': PLAN['scope'], 'cli': item['cli'],
               'selected_environment': item['selected_environment'],
               'declared_provider_build_input_receipt': item['prior_build_input_receipt'],
               'declared_provider_build_input_receipt_sha256': item['prior_build_input_receipt_sha256'],
               'request_sha256': request_sha, 'launcher_sha256': launcher_sha,
               'value_scope_supported': item['value_scope_supported']}
    dependency, dependency_stdout = run(directory, 'dependencies',
        PLAN['dependency_argv_prefix'] + [item['cli']], environment, pins)
    receipt['dependencies'] = dependency
    libraries = {}
    unpinned = []
    for word in re.findall(rb'/[^\s()]+', dependency_stdout):
        path = Path(os.fsdecode(word))
        if not path.is_file():
            unpinned.append(str(path))
            continue
        sha = digest(path)
        libraries[str(path)] = sha
        if pins.get(str(path), pins.get(str(path.resolve()))) != sha:
            unpinned.append(str(path))
        else:
            pins[str(path)] = sha
    receipt['actual_dependency_files'] = libraries
    receipt['unpredicted_dependency_files'] = unpinned
    if dependency['exit'] != 0 or unpinned or not libraries:
        receipt['status'] = 'dependency-closure-unavailable-no-cli-probe'
    else:
        version, version_stdout = run(directory, 'version',
            [item['cli'], str(OUT / 'original-request' / item['version_source'])],
            environment, pins)
        receipt['version'] = version
        availability, availability_stdout = run(directory, 'availability',
            [item['cli'], str(OUT / 'original-request' / 'availability.tcl')],
            environment, pins)
        receipt['availability'] = availability
        availability_lines = availability_stdout.splitlines()
        vectors = [line.split(b'\t')[1:] for line in availability_lines
                   if line.startswith(b'AVAILABILITY\t')]
        complete = len(vectors) == 1 and b'AVAILABILITY_DONE\t1' in availability_lines
        receipt['availability_protocol_complete'] = complete
        receipt['availability_vector'] = [os.fsdecode(x) for x in vectors[0]] if len(vectors) == 1 else None
        version_complete = version['exit'] == 0 and version_stdout.startswith(b'VERSION ')
        if version['exit'] != 0 or availability['exit'] != 0 or not complete or not version_complete:
            receipt['status'] = 'version-or-availability-channel-incomplete'
        elif not item['value_scope_supported']:
            receipt['status'] = 'closed-availability-only'
        elif vectors[0] != [b'1',b'1',b'1']:
            receipt['status'] = 'supported-value-entry-unavailable-no-values'
        else:
            execution, stdout = run(directory, 'values',
                [item['cli'], str(OUT / 'original-request' / 'values.tcl')],
                environment, pins)
            receipt['values'] = execution
            lines = stdout.splitlines()
            raw_rows = [line.split(b'\t') for line in lines if line.startswith(b'VALUE\t')]
            tags = [os.fsdecode(row[1]) for row in raw_rows if len(row) == 4]
            receipt['observed_value_tags'] = tags
            receipt['value_protocol_complete'] = (
                len(raw_rows) == 6 and tags == PLAN['expected_value_tags']
                and b'VALUES_DONE\t6' in lines)
            # Values are retained as measured bytes. There is no expected-value
            # assertion here and no header/refcount inference from these rows.
            receipt['status'] = ('closed-values' if execution['exit'] == 0
                and receipt['value_protocol_complete'] else 'value-channel-incomplete')
    receipt['inputs_verified_before_and_after'] = verify(pins)
    write(directory / 'receipt.json', receipt)
    providers.append(receipt)
write(OUT / 'summary.json', {
    'question_id': PLAN['question_id'], 'request_sha256': request_sha,
    'launcher_sha256': launcher_sha, 'scope': PLAN['scope'],
    'process_capture_complete': len(providers) == len(PLAN['providers']),
    'all_requested_channels_successful': all(
        r['status'] in ['closed-values','closed-availability-only'] for r in providers),
    'providers': [{'provider':r['provider'],'status':r['status'],
                   'receipt_sha256':digest(OUT/r['provider']/'receipt.json')}
                  for r in providers]})
verify(common)
