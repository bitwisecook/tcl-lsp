#!/usr/bin/env python3
"""Capture the repository-pinned original C84 execution-environment error-name objects."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--repo', type=Path, required=True)
parser.add_argument('--sdk-dir', type=Path, required=True)
parser.add_argument('--out', type=Path, required=True)
args = parser.parse_args()
repo, sdk, out = args.repo.resolve(), args.sdk_dir.resolve(), args.out.resolve()
out.mkdir(parents=True, exist_ok=False)
source = Path(__file__).resolve().with_name('probe.c')
for item in [source, Path(__file__).resolve()]:
    (out/item.name).write_bytes(item.read_bytes())
def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def now(): return datetime.datetime.now(datetime.timezone.utc).isoformat()
env = os.environ.copy()
env['TCL_LIBRARY'] = str(sdk/'library')
def command(argv, label, timeout=30, cwd=None):
    started = now()
    try:
        result = subprocess.run(argv, cwd=cwd, env=env, capture_output=True, timeout=timeout)
        stdout, stderr, code, timed_out = result.stdout, result.stderr, result.returncode, False
    except subprocess.TimeoutExpired as error:
        stdout, stderr, code, timed_out = error.stdout or b'', error.stderr or b'', None, True
    (out/(label+'.stdout')).write_bytes(stdout)
    (out/(label+'.stderr')).write_bytes(stderr)
    return {'argv': argv, 'cwd': str(cwd) if cwd else None, 'started_utc': started, 'finished_utc': now(), 'exit_code': code, 'timed_out': timed_out, 'stdout_sha256': sha(out/(label+'.stdout')), 'stderr_sha256': sha(out/(label+'.stderr'))}
def provider():
    paths = [p for p in sdk.rglob('*') if p.is_file() and (p.suffix in {'.c', '.h', '.tcl', '.test', '.enc', '.a', '.sh'} or p.name in {'tclsh', 'Makefile', 'configure', 'config.log', 'config.status'})]
    return {p.relative_to(sdk).as_posix(): sha(p) for p in sorted(paths)}
anchor = repo/'experiments/name-handling/results/original-suite-integrity-c84-20261013'
for name in ['provenance.json', 'upstream-integrity.json', 'upstream-inputs.json']:
    (out/('retained-'+name)).write_bytes((anchor/name).read_bytes())
manifest_path = repo/'rust/tcl-dialect/data/reference-toolchains.tsv'
(out/'reference-toolchains.tsv').write_bytes(manifest_path.read_bytes())
rows = [line.split('\t') for line in manifest_path.read_text().splitlines() if line and not line.startswith('#')]
assert ['8.4', '8.4.20', 'core-8-4-20'] in rows
integrity = json.loads((anchor/'upstream-integrity.json').read_text())
assert integrity['source_tag'] == 'core-8-4-20'
original_inputs = json.loads((anchor/'upstream-inputs.json').read_text())
original_mismatch = [p for p, expected in original_inputs.items() if not (sdk/p).is_file() or sha(sdk/p) != expected]
manifest = {'started_utc': now(), 'scope': 'Original pinned Tcl8.4.20 actual ExecEnv error-name object construction, reuse, independent interpreter ownership and observed DeleteInterp reference release; explicit observer refs separated; no VM/Runtime execution.', 'sdk_dir': str(sdk), 'source_sha256': sha(source), 'driver_sha256': sha(Path(__file__)), 'reference_manifest_sha256': sha(manifest_path), 'reference_source_tag': integrity['source_tag'], 'upstream_commit': integrity['commit'], 'original_library_test_mismatches': original_mismatch, 'environment': {'TCL_LIBRARY': env['TCL_LIBRARY']}, 'provider_before_sha256': provider()}
manifest['compiler'] = command(['cc', '--version'], 'compiler')
upstream = {}
for name in ['generic/tcl.h', 'generic/tclInt.h', 'generic/tclResult.c', 'generic/tclVar.c', 'generic/tclBasic.c', 'generic/tclExecute.c']:
    label = 'upstream-'+Path(name).name
    record = command(['curl', '--fail', '--silent', '--show-error', '--location', 'https://raw.githubusercontent.com/tcltk/tcl/'+integrity['commit']+'/'+name], label)
    record['sdk_sha256'] = sha(sdk/name)
    record['matches_sdk'] = record['exit_code'] == 0 and record['stdout_sha256'] == record['sdk_sha256']
    upstream[name] = record
manifest['upstream_sources'] = upstream
manifest['setup_ready'] = not original_mismatch and all(item['matches_sdk'] for item in upstream.values())
if manifest['setup_ready']:
    config = (sdk/'unix/tclConfig.sh').read_text()
    defines = shlex.split(next(line.partition('=')[2].strip().strip("'") for line in config.splitlines() if line.startswith('TCL_DEFS=')))
    manifest['compile'] = command(['cc', '-std=c99', '-I'+str(sdk/'generic'), '-I'+str(sdk/'unix'), *defines, str(source), str(sdk/'unix/libtcl8.4.a'), '-ldl', '-lm', '-o', str(out/'probe')], 'compile')
    if manifest['compile']['exit_code'] == 0:
        manifest['executable_sha256'] = sha(out/'probe')
        manifest['execution'] = command([str(out/'probe')], 'execution', timeout=10)
        manifest['linked_libraries'] = command(['ldd', str(out/'probe')], 'linked-libraries')
        manifest['linked_library_sha256'] = {str(p.resolve()): sha(p.resolve()) for word in (out/'linked-libraries.stdout').read_text().split() if word.startswith('/') and (p := Path(word)).is_file()}
        manifest['original_result_suite'] = command([str(sdk/'unix/tclsh'), str(sdk/'tests/result.test')], 'result-suite', timeout=30, cwd=sdk/'tests')
        manifest['suite_pairing'] = {'binary': str(sdk/'unix/tclsh'), 'library': str(sdk/'library'), 'init': str(sdk/'library/init.tcl'), 'tcltest': str(sdk/'library/tcltest/tcltest.tcl'), 'suite': str(sdk/'tests/result.test'), 'harness': 'Original exact-version library, tcltest, suite and required support files; original native-test availability constraints unchanged.'}
manifest['provider_after_sha256'] = provider()
manifest['provider_unchanged'] = manifest['provider_before_sha256'] == manifest['provider_after_sha256']
manifest['source_unchanged'] = manifest['source_sha256'] == sha(source)
manifest['finished_utc'] = now()
(out/'manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
print(json.dumps({key: manifest.get(key) for key in ['setup_ready', 'original_library_test_mismatches', 'compile', 'execution', 'provider_unchanged', 'source_unchanged']}))
