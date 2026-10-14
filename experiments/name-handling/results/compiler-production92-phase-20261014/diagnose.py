from pathlib import Path
import datetime, gzip, hashlib, json, os, subprocess, time

root = Path('/workspace/tcl-lsp')
packet = Path('/tmp/name-tcltest-92-phase-plan-20261014')
receipt = root / 'experiments/name-handling/results/compiler-production92-20261014/depth-executable.json'
retained = json.loads(receipt.read_text())
binary = Path(retained['path'])
compiled = json.loads(gzip.decompress(Path(retained['source']).read_bytes()))
selector = 'analyser::diagnostics::tests::tcltest_oracle_program_keeps_missing_reads_and_setup_completion'
fixture = 'experiments/name-handling/tcltest-shared-frame/probe.tcl'

def sha(path):
    value = hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            value.update(block)
    return value.hexdigest()

def source_snapshot():
    return {name: sha(root / name) if (root / name).is_file() else None for name in compiled}

before = source_snapshot()
binary_before = sha(binary)
if binary_before != retained['sha256']:
    raise SystemExit('Retained Compiler92 binary no longer matches its immutable receipt')
if before.get(fixture) != compiled.get(fixture):
    raise SystemExit('Original compiled test fixture is no longer current; do not infer test semantics')
(packet / 'current-source-before.json').write_text(json.dumps(before, sort_keys=True, indent=2) + '\n')
env = os.environ.copy()
for key in tuple(env):
    if key.startswith('TCL_LSP_TRACE_'):
        env.pop(key)
env['TCL_LSP_TRACE_SOURCE_PHASES'] = '1'
command = [str(binary), '--exact', selector, '--nocapture', '--test-threads=1']
started = datetime.datetime.now(datetime.timezone.utc).isoformat()
start = time.monotonic()
with (packet / 'run.log').open('wb') as output:
    process = subprocess.Popen(command, cwd=root, env=env, stdout=output, stderr=subprocess.STDOUT)
    try:
        code = process.wait(timeout=30)
        timed_out = False
    except subprocess.TimeoutExpired:
        process.kill()
        code = process.wait()
        timed_out = True
elapsed = time.monotonic() - start
after = source_snapshot()
binary_after = sha(binary)
(packet / 'current-source-after.json').write_text(json.dumps(after, sort_keys=True, indent=2) + '\n')
result = {'started_utc': started, 'selector': selector, 'command': command,
          'binary_receipt': str(receipt), 'compiled_snapshot': retained['source'],
          'binary_before': binary_before, 'binary_after': binary_after,
          'source_changes': [key for key in before if before[key] != after.get(key)],
          'fixture_sha256': before[fixture], 'compiled_fixture_sha256': compiled[fixture],
          'timeout_seconds': 30, 'timed_out': timed_out, 'exit_code': code,
          'elapsed_seconds': elapsed, 'trace_flag': 'TCL_LSP_TRACE_SOURCE_PHASES',
          'scope': 'Historical Compiler92 phase diagnosis only. Original assertion retained. No current-source or semantic acceptance from a bounded incomplete run.'}
(packet / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
if binary_after != binary_before or result['source_changes']:
    raise SystemExit('Source or binary changed during diagnostic; receipt is invalid')
print(json.dumps(result, indent=2))
