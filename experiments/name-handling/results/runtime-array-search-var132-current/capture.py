import argparse
import gzip
import hashlib
import json
import os
import pathlib
import re
import resource
import shlex
import subprocess
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parents[4]
OUT = pathlib.Path(__file__).resolve().parent
ORIGINAL = ROOT / 'experiments/name-handling/results/runtime-c86-real-suites/array-search-isolation/var-13.2'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
parser = argparse.ArgumentParser()
parser.add_argument('label')
parser.add_argument('--trace', action='store_true')
options = parser.parse_args()
destination = OUT / options.label
destination.mkdir(exist_ok=False)
runner = ROOT / 'tmp/name-target-runtime/debug/examples/run_script'
oracle = ROOT / 'tmp/tcl8.6.18/unix/tclsh'
prior = json.loads((ORIGINAL / 'record.json').read_text())
inputs = {path: sha(pathlib.Path(path)) for path in prior['original_inputs']}
assert inputs == prior['original_inputs'], inputs
wrapper = ORIGINAL / 'wrapper.tcl'
assert sha(wrapper) == prior['wrapper_sha256']
paths = [pathlib.Path(p).resolve() for p in shlex.split((runner.with_suffix('.d')).read_text().split(': ', 1)[1]) if pathlib.Path(p).is_file()]
scope = {str(p.relative_to(ROOT)): sha(p) for p in paths}
with gzip.open(destination / 'compiled-inputs-before.json.gz', 'wt') as stream:
    json.dump(scope, stream, indent=2)
env = dict(os.environb)
for key in [b'NH_CLOCK_GETENV_MISSING', b'TCL_BACKEND_CONSTRAINTS', b'TCLLIBPATH', b'TCL_TEST_VERBOSE', b'TCL_TEST_CONSTRAINTS']:
    env.pop(key, None)
env.update({b'TCL_LIBRARY': str(ROOT / 'tmp/tcl8.6.18/library').encode(), b'NH_CLOCK_GETENV_TEST': b'host-value', b'NH_CLOCK_GETENV_\xff': b'raw-name-value', b'NH_CLOCK_GETENV_RAW_VALUE': b'\xff'})
record = {'git_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(), 'original_inputs': inputs, 'wrapper_sha256': sha(wrapper), 'runner_sha256': sha(runner), 'oracle_sha256': sha(oracle), 'compiled_input_count': len(scope), 'limits': {'timeout_seconds': 60, 'kill_after_seconds': 5, 'address_space_bytes': 1024**3, 'core_bytes': 0}, 'runs': {}}
def limits():
    resource.setrlimit(resource.RLIMIT_AS, (1024**3, 1024**3))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
def save():
    (destination / 'record.json').write_text(json.dumps(record, indent=2) + '\n')
save()
for provider, arguments in [('runtime', [str(runner), '--quiet', '--tcl-version', '8.6', '--init', str(wrapper)]), ('c86', [str(oracle), str(wrapper)])]:
    run_env = dict(env)
    if provider == 'runtime' and options.trace:
        run_env[b'LD_PRELOAD'] = b'/tmp/name-runtime-realloc-trace.so'
    argv = ['timeout', '--kill-after=5', '60', *arguments]
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix='nh-var132-') as cwd, (destination / (provider + '.stdout.txt')).open('wb') as stdout, (destination / (provider + '.stderr.txt')).open('wb') as stderr:
        process = subprocess.run(argv, cwd=cwd, env=run_env, stdout=stdout, stderr=stderr, preexec_fn=limits, timeout=67)
    output = (destination / (provider + '.stdout.txt')).read_text(errors='replace')
    record['runs'][provider] = {'argv': argv, 'exit_code': process.returncode, 'seconds': round(time.monotonic()-started, 3), 'test_starts': re.findall(r'^---- (\S+) start', output, re.M), 'suite_count_lines': [line for line in output.splitlines() if re.search(r'Total\s+\d+.*Passed\s+\d+', line)], 'orchestration_return': 'NH_ORCHESTRATION_RETURN' in output, 'failure_banner_count': len(re.findall(r'^==== .* FAILED', output, re.M)), 'stdout_sha256': sha(destination / (provider + '.stdout.txt')), 'stderr_sha256': sha(destination / (provider + '.stderr.txt'))}
    save()
    print(provider, json.dumps(record['runs'][provider]), flush=True)
record['compiled_inputs_changed_during_runs'] = [p for p, h in scope.items() if sha(ROOT / p) != h]
record['original_inputs_after'] = {p: sha(pathlib.Path(p)) for p in inputs}
record['runner_sha256_after'] = sha(runner)
save()
(destination / 'sealed-files.json').write_text(json.dumps({p.name: sha(p) for p in destination.iterdir() if p.is_file() and p.name != 'sealed-files.json'}, indent=2) + '\n')
