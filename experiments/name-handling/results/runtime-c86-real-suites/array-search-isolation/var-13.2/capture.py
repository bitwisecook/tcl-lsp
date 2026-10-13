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

ROOT = pathlib.Path(__file__).resolve().parents[6]
OUT = pathlib.Path(__file__).resolve().parent
PRIOR = ROOT / 'experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-runtime/orchestration-04'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
with gzip.open(PRIOR / 'production-source-after.json.gz', 'rt') as f:
    old = json.load(f)
changes = [p for p, h in old.items() if sha(ROOT / p) != h]
allowed = ['rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-input-storage.json']
assert changes == allowed, changes
old_record = json.loads((PRIOR / 'record.json').read_text())
runner = pathlib.Path(old_record['build']['binary'])
assert sha(runner) == old_record['build']['binary_sha256']
original = json.loads((PRIOR / 'orchestration-inputs.json').read_text())
assert all(sha(pathlib.Path(p)) == h for p, h in original.items())
with gzip.open(PRIOR / 'runner-rustc-depinfo.d.gz', 'rt') as f:
    depinfo = f.read()
paths = [pathlib.Path(p).resolve() for p in shlex.split(depinfo.split(': ', 1)[1])]
assert all(str(ROOT / p) not in depinfo for p in allowed)
tracked = []
for p in paths:
    rel = str(p.relative_to(ROOT))
    if subprocess.run(['git', 'cat-file', '-e', '05499cc75:' + rel], cwd=ROOT, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
        tracked.append(rel)
assert not subprocess.check_output(['git', 'diff', '--name-only', '05499cc75', 'HEAD', '--', *tracked], cwd=ROOT)
compiled_current = {str(p.relative_to(ROOT)): sha(p) for p in paths if p.is_file()}
with gzip.open(OUT / 'compiled-inputs-before.json.gz', 'wt') as f:
    json.dump(compiled_current, f, indent=2)
env = dict(os.environb)
for key in [b'NH_CLOCK_GETENV_MISSING', b'TCL_BACKEND_CONSTRAINTS', b'TCLLIBPATH', b'TCL_TEST_VERBOSE', b'TCL_TEST_CONSTRAINTS']:
    env.pop(key, None)
env.update({b'TCL_LIBRARY': str(ROOT / 'tmp/tcl8.6.18/library').encode(), b'NH_CLOCK_GETENV_TEST': b'host-value', b'NH_CLOCK_GETENV_\xff': b'raw-name-value', b'NH_CLOCK_GETENV_RAW_VALUE': b'\xff'})
wrapper = OUT / 'wrapper.tcl'
oracle = ROOT / 'tmp/tcl8.6.18/unix/tclsh'
record = {'git_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(), 'question': 'Fresh original Tcltest var-13.2 in Runtime and C8.6', 'original_inputs': original, 'wrapper_sha256': sha(wrapper), 'runner_sha256': sha(runner), 'oracle_sha256': sha(oracle), 'conservative_scope': {'input_count': len(old), 'changed': changes, 'changed_fixture_current_sha256': sha(ROOT / allowed[0]), 'fixture_in_runner_depinfo': False}, 'compiled_scope': {'depinfo_input_count': len(paths), 'tracked_input_count': len(tracked), 'tracked_delta_from_build_commit': [], 'outside_tracked_inputs': [str(p.relative_to(ROOT)) for p in paths if str(p.relative_to(ROOT)) not in tracked]}, 'limits': {'timeout_seconds': 60, 'kill_after_seconds': 5, 'address_space_bytes': 1024**3, 'core_bytes': 0}, 'environment': 'Same five removed selectors and named synthetic NH_CLOCK controls as orchestration04; inherited host environment not recorded; no TZ override', 'runs': {}}
def limits():
    resource.setrlimit(resource.RLIMIT_AS, (1024**3, 1024**3))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
def save():
    (OUT / 'record.json').write_text(json.dumps(record, indent=2) + '\n')
save()
for provider, args in [('runtime', [str(runner), '--quiet', '--tcl-version', '8.6', '--init', str(wrapper)]), ('c86', [str(oracle), str(wrapper)])]:
    argv = ['timeout', '--kill-after=5', '60', *args]
    start = time.monotonic()
    with tempfile.TemporaryDirectory(prefix='nh-var132-') as cwd, (OUT / (provider + '.stdout.txt')).open('wb') as stdout, (OUT / (provider + '.stderr.txt')).open('wb') as stderr:
        process = subprocess.run(argv, cwd=cwd, env=env, stdout=stdout, stderr=stderr, preexec_fn=limits, timeout=67)
    stdout_path = OUT / (provider + '.stdout.txt')
    stderr_path = OUT / (provider + '.stderr.txt')
    text = stdout_path.read_text(errors='replace')
    counts = [line for line in text.splitlines() if re.search(r'Total\s+\d+.*Passed\s+\d+', line)]
    record['runs'][provider] = {'argv': argv, 'exit_code': process.returncode, 'seconds': round(time.monotonic()-start, 3), 'stdout_sha256': sha(stdout_path), 'stderr_sha256': sha(stderr_path), 'test_starts': re.findall(r'^---- (\S+) start', text, re.M), 'test_pass_markers': re.findall(r'^\+\+\+\+ (\S+) PASSED', text, re.M), 'suite_count_lines': counts, 'orchestration_return': 'NH_ORCHESTRATION_RETURN' in text, 'failure_banner_count': len(re.findall(r'^==== .* FAILED', text, re.M))}
    save()
    print(provider, json.dumps(record['runs'][provider]), flush=True)
after = {str(p.relative_to(ROOT)): sha(p) for p in paths if p.is_file()}
record['compiled_inputs_changed_during_runs'] = [p for p, h in compiled_current.items() if after.get(p) != h]
record['runner_sha256_after'] = sha(runner)
record['original_inputs_after'] = {p: sha(pathlib.Path(p)) for p in original}
save()
seal = {p.name: sha(p) for p in OUT.iterdir() if p.is_file() and p.name != 'sealed-files.json'}
(OUT / 'sealed-files.json').write_text(json.dumps(seal, indent=2) + '\n')
