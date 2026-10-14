from pathlib import Path
import ast
import datetime
import gzip
import hashlib
import json
import mmap
import os
import re
import selectors
import signal
import subprocess
import time

OUT = Path('/tmp/name-historical196-gdb-repeat2-20261014')
OUT.mkdir(exist_ok=True)
ROOT = Path('/workspace/tcl-lsp')
ELF = Path('/tmp/name-historical196-gdb-20261014/compiler196.elf')
GDB = Path('/tmp/name-stack-debugger-helper-20261014/gdb-extracted/usr/bin/gdb')
LIBS = '/tmp/name-stack-debugger-helper-20261014/gdb-extracted/usr/lib/x86_64-linux-gnu'
SELECTOR = 'analyser::diagnostics::tests::tcltest_oracle_program_keeps_missing_reads_and_setup_completion'
EXPECTED = 'a7fcd9a1d03f738d9434012f81e145c458039c54923161bf97330ce9a57e371a'
PRIOR = Path('/tmp/name-tcltest-production201-20261014/result.json')

def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for c in iter(lambda: f.read(1024 * 1024), b''):
            h.update(c)
    return h.hexdigest()

def save(name, value):
    (OUT / name).write_text(json.dumps(value, indent=2) + '\n')

if (OUT / 'started.json').exists():
    raise SystemExit('Do not overwrite a prior debugger execution')
prior = json.loads(PRIOR.read_text())
assert prior['receipt_valid'] and prior['binary_before'] == EXPECTED
assert sha(ELF) == EXPECTED
fixture = ROOT / prior['fixture']
assert sha(fixture) == prior['fixture_sha256']
with ELF.open('rb') as f, mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ) as mapped:
    assert mapped.find(fixture.read_bytes()) >= 0
listed = subprocess.run([str(ELF), '--list', '--exact', SELECTOR], cwd=ROOT,
                        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
(OUT / 'selector-list.log').write_bytes(listed.stdout)
assert listed.returncode == 0 and listed.stdout.decode().splitlines().count(SELECTOR + ': test') == 1
providers = []
for version in ('8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0'):
    base = ROOT / 'tmp' / ('tcl' + version)
    for rel in ('unix/tclsh', 'library/init.tcl', 'library/tcltest/tcltest.tcl'):
        p = base / rel
        providers.append({'path': str(p), 'sha256': sha(p) if p.is_file() else None})
env = os.environ.copy()
removed = sorted(key for key in env if key.startswith('TCL_LSP_TRACE_'))
for key in removed:
    env.pop(key)
ordinary_ld = env.get('LD_LIBRARY_PATH')
recorded_env = {key: env.get(key) for key in ('LANG', 'LC_ALL', 'RUST_BACKTRACE',
    'RUST_TEST_THREADS', 'RUST_MIN_STACK', 'TCL_LIBRARY', 'TCLLIBPATH', 'JIMLIB', 'TCL_LSP_REQUIRE_TCLSH', 'TCL_LSP_REQUIRE_JIM', 'TCLSH', 'TCLSH84', 'TCLSH85', 'TCLSH86', 'TCLSH90', 'TCLSH91')}
env['LD_LIBRARY_PATH'] = LIBS
command = [str(GDB), '--nx', '--nh', '--quiet', '--interpreter=mi3',
           '--data-directory=/tmp/name-stack-debugger-helper-20261014/gdb-extracted/usr/share/gdb',
           '-iex', 'set auto-load off', '-iex', 'set debuginfod enabled off',
           '--args', str(ELF), '--exact', SELECTOR, '--nocapture', '--test-threads=1']
save('started.json', {'started_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'historical_build_receipt': '/tmp/name-consumer196-preflight-20261014/result.json',
    'historical_build_receipt_sha256': sha('/tmp/name-consumer196-preflight-20261014/result.json'),
    'compiled_source': prior['compiled_source'], 'compiled_source_sha256': prior['compiled_source_sha256'],
    'ordinary201_receipt_sha256': sha(PRIOR), 'historical_elf': str(ELF), 'elf_sha256': EXPECTED,
    'original_fixture_sha256': prior['fixture_sha256'], 'provider_files_before': providers,
    'command': command, 'debugger_sha256': sha(GDB), 'recorded_nonsecret_environment': recorded_env,
    'removed_trace_flag_names': removed, 'trace_flag': None, 'inferior_host_budget_seconds': 180,
    'requested_samples_seconds': [55, 100, 160], 'maximum_frames_per_thread': 20,
    'limitations': 'Historical196 executable only: current Main has changed. Root203 Cargo build runs concurrently, so no current-source acceptance or comparable performance claim. Pauses count within the unchanged 180-second inferior wall budget. GDB-only LD_LIBRARY_PATH is restored/unset for the inferior. No arguments, locals, source Debug values or unlimited stack dump.'})
print(json.dumps({'started': True, 'output': str(OUT), 'historical_elf_sha256': EXPECTED}), flush=True)
process = subprocess.Popen(command, cwd=ROOT, env=env, stdin=subprocess.PIPE,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT, bufsize=0,
                           start_new_session=True)
selector = selectors.DefaultSelector()
selector.register(process.stdout, selectors.EVENT_READ)
raw = gzip.open(OUT / 'gdb-mi.log.gz', 'wb', compresslevel=1)
buffer = bytearray()
skipping = False
next_token = 0
results = {}
stopped = False
exited = False
inferior_pid = None
inferior_start_ticks = None
inferior_started = None
inferior_exit_code = None
samples = []
control_errors = []
console_capture = None
stack_lines = []
test_summary_lines = []


def cpu():
    if inferior_pid is None:
        return None
    rows = []
    try:
        for p in sorted(Path(f'/proc/{inferior_pid}/task').glob('*/stat')):
            data = p.read_text()
            fields = data[data.rfind(')') + 2:].split()
            rows.append({'tid': int(p.parent.name), 'state': fields[0], 'user_ticks': int(fields[11]),
                         'system_ticks': int(fields[12]), 'start_ticks': int(fields[19])})
    except OSError:
        pass
    return {'clock_ticks_per_second': os.sysconf('SC_CLK_TCK'), 'threads': rows}


def accept(line):
    global stopped, exited, inferior_pid, inferior_start_ticks, inferior_exit_code
    text = line.decode('utf-8', 'replace').strip()
    if len(text) < 4096 and ('test result:' in text or SELECTOR in text or re.fullmatch(r'running \d+ test(?:s)?', text)):
        test_summary_lines.append(text[:4096])
    match = re.match(r'(\d+)\^(\w+)(.*)', text)
    if match:
        results[int(match[1])] = {'class': match[2], 'payload': match[3][:4096]}
    if text.startswith('=thread-group-started,'):
        match = re.search(r'pid="(\d+)"', text)
        if match:
            inferior_pid = int(match[1])
            try:
                data = Path(f'/proc/{inferior_pid}/stat').read_text()
                inferior_start_ticks = data[data.rfind(')') + 2:].split()[19]
            except OSError:
                pass
    if text.startswith('*running'):
        stopped = False
    if text.startswith('*stopped'):
        stopped = True
        if 'reason="exited' in text:
            exited = True
    if text.startswith('=thread-group-exited,'):
        exited = True
        match = re.search(r'exit-code="([^"]+)"', text)
        if match:
            inferior_exit_code = match[1]
    if console_capture is not None and text.startswith('~"'):
        try:
            decoded = ast.literal_eval(text[1:])
        except (SyntaxError, ValueError):
            return
        for value in decoded.splitlines():
            if re.match(r'^(Thread \d|#\d|\[Current thread)', value):
                stack_lines.append(value[:2048])


def pump(timeout=0.2):
    global skipping, buffer
    for key, _ in selector.select(timeout):
        chunk = os.read(key.fd, 65536)
        if not chunk:
            selector.unregister(key.fileobj)
            return
        raw.write(chunk)
        for part in chunk.splitlines(keepends=True):
            if not skipping:
                buffer.extend(part)
                if len(buffer) > 32768:
                    skipping = True
                    buffer.clear()
            if part.endswith(b'\n'):
                if not skipping:
                    accept(bytes(buffer))
                buffer.clear()
                skipping = False


def send(command):
    global next_token
    next_token += 1
    process.stdin.write((str(next_token) + command + '\n').encode())
    process.stdin.flush()
    return next_token


def wait_result(token, seconds=15):
    until = time.monotonic() + seconds
    while token not in results:
        if process.poll() is not None or time.monotonic() > until:
            raise RuntimeError('debugger command response unavailable')
        pump()
    answer = results.pop(token)
    if answer['class'] == 'error':
        raise RuntimeError('debugger command failed: ' + answer['payload'])
    return answer


def execute(command, seconds=15):
    return wait_result(send(command), seconds)


def kill_owned_inferior():
    if inferior_pid is None or exited:
        return
    try:
        data = Path(f'/proc/{inferior_pid}/stat').read_text()
        if data[data.rfind(')') + 2:].split()[19] == inferior_start_ticks:
            os.kill(inferior_pid, signal.SIGKILL)
    except ProcessLookupError:
        pass

try:
    for command in ('-gdb-set mi-async on', '-gdb-set pagination off', '-gdb-set confirm off',
                    '-gdb-set startup-with-shell off', '-gdb-set disable-randomization off',
                    '-gdb-set print frame-arguments none', '-gdb-set print entry-values no',
                    '-gdb-set print thread-events off'):
        execute(command)
    if ordinary_ld is None:
        execute('-interpreter-exec console ' + json.dumps('unset environment LD_LIBRARY_PATH'))
    else:
        execute('-gdb-set environment ' + json.dumps('LD_LIBRARY_PATH=' + ordinary_ld))
    inferior_started = time.monotonic()
    execute('-exec-run', seconds=20)
    print(json.dumps({'inferior_started': True, 'pid': inferior_pid}), flush=True)
    for target in (55, 100, 160):
        while not exited and time.monotonic() - inferior_started < target:
            pump(0.2)
        if exited:
            break
        sample_start = time.monotonic()
        before = cpu()
        execute('-exec-interrupt --all')
        until = time.monotonic() + 10
        while not stopped and not exited and time.monotonic() < until:
            pump()
        if not stopped or exited:
            control_errors.append('Inferior did not reach a sample stop')
            break
        stack_lines.clear()
        console_capture = target
        execute('-interpreter-exec console ' + json.dumps('thread apply all bt 20'), seconds=20)
        console_capture = None
        sample = {'requested_seconds': target, 'inferior_wall_seconds': sample_start - inferior_started,
                  'cpu_before_interrupt': before, 'cpu_when_stopped': cpu(), 'stack_lines': list(stack_lines)}
        samples.append(sample)
        save(f'stack-{target}.json', sample)
        print(json.dumps({'sample': target, 'wall_seconds': sample['inferior_wall_seconds'],
                          'stack_lines': len(stack_lines), 'path': str(OUT / f'stack-{target}.json')}), flush=True)
        execute('-exec-continue --all')
        stopped = False
    while not exited and time.monotonic() - inferior_started < 180:
        pump(0.2)
    timed_out = not exited
    final_cpu = cpu()
    if timed_out:
        kill_owned_inferior()
        until = time.monotonic() + 10
        while not exited and time.monotonic() < until:
            pump()
    elapsed = time.monotonic() - inferior_started
except Exception as error:
    control_errors.append(str(error))
    timed_out = False
    final_cpu = cpu()
    elapsed = time.monotonic() - inferior_started if inferior_started else None
    kill_owned_inferior()
finally:
    try:
        if process.poll() is None:
            send('-gdb-exit')
            until = time.monotonic() + 5
            while process.poll() is None and time.monotonic() < until:
                pump()
    except (BrokenPipeError, OSError):
        pass
    if process.poll() is None:
        process.kill()
    process.wait()
    while selector.get_map():
        pump(0)
    raw.close()
    selector.close()
providers_after = [{'path': row['path'], 'sha256': sha(row['path']) if Path(row['path']).is_file() else None}
                   for row in providers]
valid = (sha(ELF) == EXPECTED and sha(fixture) == prior['fixture_sha256']
         and providers == providers_after and not control_errors)
result = {'historical196_diagnostic': True, 'receipt_valid': valid, 'timed_out': timed_out,
    'inferior_elapsed_seconds': elapsed, 'inferior_exit_code': inferior_exit_code,
    'debugger_exit_code': process.returncode, 'control_errors': control_errors,
    'samples': [{key: value for key, value in row.items() if key != 'stack_lines'} for row in samples],
    'actual_test_summary_lines': test_summary_lines, 'provider_profile_loop': ['tcl8.4', 'tcl8.5', 'tcl8.6', 'tcl9.0', 'tcl9.1'], 'provider_loop_has_skip_branch': False, 'external_oracle_invoked_by_selector': False,
    'final_cpu': final_cpu, 'elf_before_sha256': EXPECTED, 'elf_after_sha256': sha(ELF),
    'fixture_before_sha256': prior['fixture_sha256'], 'fixture_after_sha256': sha(fixture),
    'provider_files_after': providers_after, 'started_receipt_sha256': sha(OUT / 'started.json'),
    'raw_mi_sha256': sha(OUT / 'gdb-mi.log.gz'), 'finished_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'scope': 'Historical196 original assertion under bounded debugger observation, with concurrent Root203 build. No current-source, ordinary-run performance or semantic acceptance. Exact original fixture/assertion/internal budget retained; debugger pauses counted inside180s host wall budget.'}
save('result.json', result)
print(json.dumps({'output': str(OUT), 'valid': valid, 'timed_out': timed_out,
                  'seconds': elapsed, 'samples': len(samples), 'errors': control_errors}), flush=True)
