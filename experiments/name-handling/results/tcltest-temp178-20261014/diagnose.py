"""Run the unchanged original Tcltest selector against a verified current test ELF.
Temporary provider-clock and capsule-rebind instrumentation is present. Original fixture, assertion and internal budget are unchanged; no ordinary acceptance claim.
Root must grant the sole execution lease before invoking this script.
"""
from pathlib import Path
import argparse
import datetime
import gzip
import hashlib
import json
import mmap
import os
import signal
import subprocess
import time

SELECTOR = 'analyser::diagnostics::tests::tcltest_oracle_program_keeps_missing_reads_and_setup_completion'
FIXTURE = 'experiments/name-handling/tcltest-shared-frame/probe.tcl'
PARSER = argparse.ArgumentParser(description=__doc__)
PARSER.add_argument('--root', type=Path, default=Path('/workspace/tcl-lsp'))
PARSER.add_argument('--build-result', type=Path, required=True)
PARSER.add_argument('--compiled-source', type=Path, required=True)
PARSER.add_argument('--binary', type=Path, required=True)
PARSER.add_argument('--output', type=Path, required=True)
PARSER.add_argument('--timeout-seconds', type=int, choices=range(1, 181), default=180,
                    metavar='1..180', help='180 preserves the ordinary host timeout; shorter runs are incomplete phase diagnoses only')
ARGS = PARSER.parse_args()
ROOT = ARGS.root.resolve()
BINARY = ARGS.binary.resolve()
OUT = ARGS.output.resolve()
OUT.mkdir(parents=True, exist_ok=True)
if any(OUT.iterdir()):
    raise SystemExit('Output directory must be empty; do not overwrite a prior receipt')

def sha(path):
    value = hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            value.update(block)
    return value.hexdigest()

def snapshot(names):
    return {name: sha(ROOT / name) if (ROOT / name).is_file() else None for name in names}

def save(name, value):
    (OUT / name).write_text(json.dumps(value, sort_keys=True, indent=2) + '\n')

result = json.loads(ARGS.build_result.read_text())
if result.get('exit_code') != 0 or result.get('changed_sources'):
    raise SystemExit('Build must pass without captured source drift before a current diagnostic')
artifacts = result.get('test_artifacts', [])
receipt = next((item for item in artifacts if Path(item['path']).resolve() == BINARY), None)
if receipt is None:
    raise SystemExit('Binary has no test-artifact entry in the supplied build result')
with gzip.open(ARGS.compiled_source, 'rt') as source:
    compiled = json.load(source)
if not isinstance(compiled, dict) or not compiled:
    raise SystemExit('Compiled source receipt must be a nonempty path-to-SHA256 dictionary')
before = snapshot(compiled)
if before != compiled:
    save('source-mismatch.json', {key: {'compiled': compiled[key], 'current': before[key]}
                                 for key in compiled if before[key] != compiled[key]})
    raise SystemExit('Current captured source subset differs from compiled snapshot; no execution')
binary_before = sha(BINARY)
if binary_before != receipt['sha256']:
    raise SystemExit('Binary differs from the successful build artifact SHA256')
fixture = ROOT / FIXTURE
fixture_before = sha(fixture)
fixture_bytes = fixture.read_bytes()
with BINARY.open('rb') as executable, mmap.mmap(executable.fileno(), 0, access=mmap.ACCESS_READ) as mapped:
    embedded = mapped.find(fixture_bytes) >= 0
if not fixture_bytes or not embedded:
    raise SystemExit('Exact original include_str fixture bytes are absent from the current test binary')
# A list-only probe cannot execute a test; require exactly the original selector.
listed = subprocess.run([str(BINARY), '--list', '--exact', SELECTOR], cwd=ROOT,
                        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
(OUT / 'selector-list.log').write_bytes(listed.stdout)
if listed.returncode or listed.stdout.decode('utf-8', 'replace').splitlines().count(SELECTOR + ': test') != 1:
    raise SystemExit('Original selector does not name exactly one compiled test')
save('source-before.json', before)
env = os.environ.copy()
for key in tuple(env):
    if key.startswith('TCL_LSP_TRACE_'):
        env.pop(key)
env['TCL_LSP_TRACE_SOURCE_PHASES'] = '1'
command = [str(BINARY), '--exact', SELECTOR, '--nocapture', '--test-threads=1']
started = datetime.datetime.now(datetime.timezone.utc).isoformat()
clock = time.monotonic()
# Stream-compress output: a failing original assertion may print large structures.
# Reader and child terminate together; no full raw Debug output is loaded in memory.
with (OUT / 'run.log.gz').open('wb') as output:
    compressor = subprocess.Popen(['gzip', '-1', '-c'], stdin=subprocess.PIPE, stdout=output)
    try:
        process = subprocess.Popen(command, cwd=ROOT, env=env, stdout=compressor.stdin,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        compressor.stdin.close()
        try:
            code = process.wait(timeout=ARGS.timeout_seconds)
            timed_out = False
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            code = process.wait()
            timed_out = True
    finally:
        if not compressor.stdin.closed:
            compressor.stdin.close()
        gzip_code = compressor.wait()
elapsed = time.monotonic() - clock
after = snapshot(compiled)
binary_after = sha(BINARY)
fixture_after = sha(fixture)
save('source-after.json', after)
changes = [key for key in before if before[key] != after[key]]
valid = not changes and binary_before == binary_after and fixture_before == fixture_after and gzip_code == 0
save('result.json', {
    'started_utc': started, 'selector': SELECTOR, 'command': command,
    'build_result': str(ARGS.build_result.resolve()), 'build_result_sha256': sha(ARGS.build_result),
    'compiled_source': str(ARGS.compiled_source.resolve()), 'compiled_source_sha256': sha(ARGS.compiled_source),
    'captured_source_count': len(compiled), 'full_captured_subset_equal_before': True,
    'source_changes': changes, 'binary_before': binary_before, 'binary_after': binary_after,
    'fixture': FIXTURE, 'fixture_sha256': fixture_before, 'fixture_after_sha256': fixture_after,
    'exact_fixture_bytes_embedded_in_binary': embedded,
    'timeout_seconds': ARGS.timeout_seconds, 'ordinary_host_timeout_seconds': 180,
    'timed_out': timed_out, 'exit_code': code, 'elapsed_seconds': elapsed,
    'trace_flag': 'TCL_LSP_TRACE_SOURCE_PHASES', 'gzip_exit_code': gzip_code,
    'log_sha256': sha(OUT / 'run.log.gz'), 'receipt_valid': valid,
    'scope': 'TEMP176 instrumentation diagnostic of the unchanged original assertion: provider global clocks plus capsule-rebind trace source changes. A timeout is incomplete and this is not ordinary semantic acceptance. Original internal budget and fixture unchanged; captured source subset equality is not full provider conformance.'
})
if not valid:
    raise SystemExit('Source, binary, fixture or output integrity changed during diagnostic')
print(json.dumps({'output': str(OUT), 'timed_out': timed_out, 'exit_code': code,
                  'elapsed_seconds': elapsed, 'receipt_valid': valid}, sort_keys=True))
