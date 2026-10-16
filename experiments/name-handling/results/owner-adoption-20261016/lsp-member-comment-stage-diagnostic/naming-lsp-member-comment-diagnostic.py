import pathlib, json, hashlib, subprocess, time, datetime, shutil, re, signal, os

root = pathlib.Path('/workspace/tcl-lsp')
out = pathlib.Path('/workspace/naming-validation/lsp-member-comment-stage-diagnostic')
out.mkdir(exist_ok=False)
sha = lambda b: hashlib.sha256(b).hexdigest()
source_base = pathlib.Path('/workspace/naming-validation/lsp-source-geometry-successor-repair3')
ordinary = json.loads((source_base / 'source-binding.json').read_text())
assert ordinary['status'] == 'PASS_FIVE_LIBRARY_BUILD' and ordinary['input_drift_count'] == 0
paths = [x['path'] for x in json.loads((source_base / 'input-before.json').read_text())]
def snapshot():
    return [{'path': p, 'sha256': sha((root / p).read_bytes())} for p in paths]
before = snapshot()
assert before == json.loads((source_base / 'input-before.json').read_text())
(out / 'input-before.json').write_text(json.dumps(before, separators=(',', ':')) + '\n')
packets = [
    (pathlib.Path('/tmp/sol-lsp-readonly-member-stage-instrumentation-v1/instrumentation.patch'), '772da61164b7a7a13f5e00b49406d49ba8e585fb0b74185090b65603910767fb'),
    (pathlib.Path('/tmp/luna-comment232-trace-v2/diagnostic-v2.patch'), '13364e9888408dd2fee050c00d9930d8545e1407e3cf57ecc5a8b11b212f6959'),
]
combined = json.loads(pathlib.Path('/tmp/sol-compiler-comment232-trace-v2-review/verification.json').read_text())['combined_files']
backup = {x['path']: (root / x['path']).read_bytes() for x in combined}
for name, data in backup.items():
    p = out / 'beforeimages' / name
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_bytes(data)
binding = {'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'source_head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(), 'ordinary': False, 'purpose': 'One exact original member222 and comment232 diagnostic each; test-only stage tracing, no acceptance or assertion/fixture/budget changes.', 'patches': [{'path': str(p), 'sha256': h} for p, h in packets], 'test_timeout_seconds': 180, 'listing_timeout_seconds': 30, 'runs': [], 'status': 'running'}
def save():
    (out / 'binding.json').write_text(json.dumps(binding, indent=2) + '\n')
def capture(argv, name, env, timeout):
    start = time.monotonic()
    with (out / name).open('wb') as log:
        proc = subprocess.Popen(argv, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL, start_new_session=True)
        timed_out = False
        try:
            proc.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            os.killpg(proc.pid, signal.SIGKILL)
            proc.wait()
    return {'argv': argv, 'exit_code': proc.returncode, 'timed_out': timed_out, 'seconds': round(time.monotonic() - start, 3), 'log': name, 'log_sha256': sha((out / name).read_bytes())}
save()
try:
    for patch, digest in packets:
        assert sha(patch.read_bytes()) == digest
        subprocess.run(['git', 'apply', '--check', str(patch)], cwd=root, check=True)
        subprocess.run(['git', 'apply', str(patch)], cwd=root, check=True)
    assert all(sha((root / x['path']).read_bytes()) == x['combined_sha256'] for x in combined)
    instrumented = snapshot()
    (out / 'input-instrumented.json').write_text(json.dumps(instrumented, separators=(',', ':')) + '\n')
    (out / 'source.diff').write_bytes(subprocess.check_output(['git', 'diff', 'HEAD', '--', 'rust'], cwd=root))
    env = os.environ.copy()
    assert not [k for k, v in env.items() if k.startswith('TCL_LSP_TRACE') and v]
    binding['provider_environment'] = {k: v for k, v in env.items() if k.startswith('TCL_LSP_')}
    start = time.monotonic()
    with (out / 'build.log').open('w') as log:
        proc = subprocess.Popen(['cargo', 'test', '--offline', '--locked', '-p', 'tcl-compiler', '--lib', '--no-run'], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        for line in proc.stdout:
            log.write(line); log.flush(); print(line, end='', flush=True)
        code = proc.wait()
    binding['build'] = {'exit_code': code, 'seconds': round(time.monotonic() - start, 3)}
    assert snapshot() == instrumented, 'Instrumented build input drift'
    if code:
        raise RuntimeError('Diagnostic compiler build failed')
    names = re.findall(r'Executable unittests src/lib.rs \(target/debug/deps/(tcl_compiler-[a-f0-9]+)\)', (out / 'build.log').read_text())
    assert len(names) == 1
    binary = out / 'frozen-binaries' / names[0]
    binary.parent.mkdir()
    shutil.copy2(root / 'target/debug/deps' / names[0], binary)
    binding['artifact'] = {'path': str(binary), 'sha256': sha(binary.read_bytes()), 'bytes': binary.stat().st_size}
    selected = json.loads(pathlib.Path('/workspace/naming-validation/lsp-selected-owners-successor-repair2/selection.json').read_text())['execution_order']
    listing = capture([str(binary), '--list'], 'compiler.list', env, 30)
    assert listing['exit_code'] == 0 and not listing['timed_out']
    lines = (out / 'compiler.list').read_text().splitlines()
    for number, flag in [(222, 'TCL_LSP_TRACE_READONLY_MEMBER_STAGES'), (232, 'TCL_LSP_TRACE_COMMENT_CASE232')]:
        selector = selected[number - 1]['selector']
        assert lines.count(selector + ': test') == 1
        test_env = env | {flag: '1'}
        run = capture([str(binary), selector, '--exact', '--nocapture', '--test-threads=1'], f'test-{number}.log', test_env, 180)
        raw = (out / run['log']).read_text(errors='replace')
        run.update(selector=selector, opt_in={flag: '1'}, running_counts=re.findall(r'running (\d+) tests?', raw), summaries=re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed;', raw))
        binding['runs'].append(run)
        save()
    assert sha(binary.read_bytes()) == binding['artifact']['sha256']
    binding['status'] = 'DIAGNOSTIC_CAPTURE_COMPLETE_NO_ORDINARY_ACCEPTANCE'
except Exception as exc:
    binding['status'] = 'FAIL'
    binding['error'] = repr(exc)
finally:
    for name, data in backup.items():
        (root / name).write_bytes(data)
    after = snapshot()
    binding['restoration'] = {'whole_file_beforeimages_restored': all((root / p).read_bytes() == b for p, b in backup.items()), 'full_input_count': len(after), 'full_input_drift': [x['path'] for x, y in zip(before, after) if x != y], 'completed_utc': datetime.datetime.now(datetime.timezone.utc).isoformat()}
    save()
    print(json.dumps(binding), flush=True)
assert binding['restoration']['whole_file_beforeimages_restored'] and not binding['restoration']['full_input_drift']
raise SystemExit(0 if binding['status'] == 'DIAGNOSTIC_CAPTURE_COMPLETE_NO_ORDINARY_ACCEPTANCE' else 1)
