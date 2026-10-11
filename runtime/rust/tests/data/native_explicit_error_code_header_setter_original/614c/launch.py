"""Fresh original producers; save every process receipt before verification.

Strict header/link/runtime admission is unchanged. Verification/refusal records
are independent of process receipts, and no output from614/614b is executable input.
"""
from pathlib import Path
import hashlib
import json
import os
import re
import shutil
import subprocess
import time
import traceback

base = Path(__file__).resolve().parent
request_path = base / 'request.json'
records, executions = [], []
request_digest = None
request = None
current_provider = 'harness'
current_channel = 'initial-input-verification'


def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def identity(path):
    path = Path(path)
    s = path.stat()
    return {'path': str(path), 'resolved_path': str(path.resolve()),
            'sha256': sha(path), 'size': s.st_size, 'device': s.st_dev,
            'inode': s.st_ino, 'mtime_ns': s.st_mtime_ns, 'ctime_ns': s.st_ctime_ns}


def durable(path, raw):
    assert not path.exists(), 'immutable output already exists: ' + str(path)
    with path.open('xb') as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    fd = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def save_json(path, value):
    durable(path, (json.dumps(value, indent=2) + '\n').encode())


def verify():
    assert sha(request_path) == request_digest, 'immutable request changed'
    for path, expected in request['required_sha256'].items():
        assert sha(path) == expected, 'immutable original input changed: ' + path


def run(provider, channel, command, directory, environment, cwd=None):
    global current_provider, current_channel
    current_provider, current_channel = provider, channel
    verify()
    stdout_path = directory / (channel + '.stdout')
    stderr_path = directory / (channel + '.stderr')
    assert not stdout_path.exists() and not stderr_path.exists(), 'output reuse'
    actual_env = dict(env)
    actual_env.update(environment)
    actual_cwd = Path(cwd) if cwd is not None else base
    assert actual_cwd.is_dir()
    start = time.monotonic()
    timeout = False
    try:
        result = subprocess.run(command, env=actual_env, cwd=actual_cwd,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                timeout=120)
        returncode, stdout, stderr = result.returncode, result.stdout, result.stderr
    except subprocess.TimeoutExpired as failure:
        timeout, returncode = True, None
        stdout, stderr = failure.stdout or b'', failure.stderr or b''
    duration = time.monotonic() - start
    durable(stdout_path, stdout)
    durable(stderr_path, stderr)
    receipt = {'provider': provider, 'channel': channel, 'status': 'closed',
               'command': command, 'cwd': str(actual_cwd),
               'selected_environment': dict(selected_env, **environment),
               'request_sha256': request_digest, 'source_sha256': request['probe_sha256'],
               'returncode': returncode, 'timeout': timeout, 'seconds': duration,
               'stdout_sha256': sha(stdout_path), 'stderr_sha256': sha(stderr_path),
               'verification_scope': 'Actual process/streams only. Header, link, runtime, own-executable and final input verification are separate records.'}
    receipt_path = directory / (channel + '.receipt.json')
    # This write precedes all after-process dependency/input verification.
    save_json(receipt_path, receipt)
    records.append({'provider': provider, 'channel': channel, 'status': 'closed',
                    'path': str(receipt_path), 'sha256': sha(receipt_path)})
    verify()
    return receipt, receipt_path, stdout


def verification_record(directory, channel, process_path, value):
    path = directory / (channel + '.verification.json')
    record = dict(value, process_receipt=str(process_path),
                  process_receipt_sha256=sha(process_path),
                  request_sha256=request_digest)
    save_json(path, record)
    return path


def refuse_record(exc):
    path = base / 'harness-refusal.json'
    value = {'status': 'harness-verification-refused', 'provider': current_provider,
             'channel': current_channel, 'exception_type': type(exc).__name__,
             'reason': str(exc), 'request_sha256': request_digest,
             'actual_process_receipts': list(records),
             'later_processes': 'not entered after this refusal',
             'native_or_rust_assertion_outcome': 'not inferred'}
    save_json(path, value)
    return {'path': str(path), 'sha256': sha(path), 'status': value['status']}


def capture():
    global current_provider, current_channel
    verify()
    providers_root = base / 'providers'
    assert not providers_root.exists(), 'fresh output root already exists'
    providers_root.mkdir()
    cr, cp, _ = run('compiler', 'compiler-version', [request['compiler'], '--version'], base, {})
    assert cr['returncode'] == 0 and not cr['timeout'], 'compiler-version process failed'
    for provider in request['provider_builds']:
        name = provider['provider']
        directory = providers_root / name
        directory.mkdir(exist_ok=False)
        command = provider['command']
        executable = Path(command[-1])
        assert not executable.exists(), 'compiler output reuse'
        receipt, compile_path, _ = run(name, 'compile', command, directory, provider['environment'], provider['compile_cwd'])
        successful = receipt['returncode'] == 0 and not receipt['timeout']
        if not successful:
            verification_record(directory, 'compile', compile_path,
                                {'status': 'compile-process-failed', 'output_existed_before': False,
                                 'execute': 'not entered'})
            continue
        current_provider, current_channel = name, 'compile-header-link-verification'
        produced = identity(executable)
        link_map, header_map = directory / 'link.map', directory / 'compile.d'
        header_inputs = []
        dependency_text = header_map.read_text(errors='strict').replace('\\\n', '')
        for selected in dependency_text.split(':', 1)[1].split():
            selected_path = Path(selected)
            path = (selected_path if selected_path.is_absolute() else Path(provider['compile_cwd']) / selected_path).resolve()
            assert str(path) in request['required_sha256'], 'unpinned original header/source input: ' + str(path)
            assert sha(path) == request['required_sha256'][str(path)]
            header_inputs.append(identity(path))
        loads = []
        for line in link_map.read_text(errors='strict').splitlines():
            if not line.startswith('LOAD '):
                continue
            selected_path = Path(line[5:])
            path = selected_path if selected_path.is_absolute() else Path(provider['compile_cwd']) / selected_path
            resolved = path.resolve()
            if path.is_file():
                assert str(resolved) in request['required_sha256'], 'unpinned real link input: ' + str(resolved)
                assert sha(path) == request['required_sha256'][str(resolved)]
                loads.append(dict(identity(path), status='retained-pinned-original-link-input'))
            else:
                assert str(path).startswith('/tmp/') and path.suffix == '.o', 'unretained original link input: ' + str(path)
                loads.append({'path': str(path), 'status': 'compiler-produced-ephemeral-object-not-retained',
                              'sha256': None, 'digest_limit': 'The actual link map retains the path; no unobserved ephemeral object digest is invented.'})
        compile_verification = verification_record(directory, 'compile', compile_path,
            {'status': 'verified', 'output_existed_before': False,
             'executable_produced': produced, 'link_map': identity(link_map),
             'header_dependency_file': identity(header_map),
             'actual_header_and_source_dependencies': header_inputs,
             'actual_link_inputs': loads})
        assert identity(executable) == produced
        ldd = str(Path(shutil.which('ldd', path=env['PATH'])).resolve())
        dr, dependency_path, dependency_stdout = run(name, 'dependencies', [ldd, str(executable)], directory, {})
        current_channel = 'runtime-dependency-verification'
        assert dr['returncode'] == 0 and not dr['timeout'], 'ldd process failed'
        dependencies = []
        for line in dependency_stdout.decode('utf-8', errors='strict').splitlines():
            for found in re.findall(r'(?<!\S)(/[^\s()]+)', line):
                path = Path(found)
                resolved = path.resolve()
                assert path.is_file(), 'missing runtime dependency: ' + str(path)
                assert str(resolved) in request['required_sha256'], 'unpinned runtime dependency: ' + str(resolved)
                assert sha(path) == request['required_sha256'][str(resolved)]
                dependencies.append(identity(path))
        assert identity(executable) == produced
        dependency_verification = verification_record(directory, 'dependencies', dependency_path,
            {'status': 'verified', 'actual_dynamic_dependencies': dependencies,
             'executable_before': produced, 'executable_after': identity(executable)})
        before = identity(executable)
        assert before == produced
        er, execution_path, stdout = run(name, 'execute', [str(executable)], directory, provider['environment'])
        current_channel = 'after-execute-verification'
        after = identity(executable)
        after_dependencies = [identity(item['path']) for item in dependencies]
        execution = dict(er, executable_before=before, executable_after=after,
            executable_unchanged=before == after,
            compile_receipt_sha256=sha(compile_path),
            compile_verification_sha256=sha(compile_verification),
            dependency_receipt_sha256=sha(dependency_path),
            dependency_verification_sha256=sha(dependency_verification),
            own_successful_compile_producer=produced,
            dependencies_before=dependencies, dependencies_after=after_dependencies,
            closed_cases=sum(line.startswith(b'CLOSED_CASE|') for line in stdout.splitlines()),
            closed_marker=b'CLOSED_ORIGINAL_EXPLICIT_ERROR_CODE_HEADER_SETTER\n' in stdout,
            actual_version_rows=[line.decode('ascii') for line in stdout.splitlines() if line.startswith(b'VERSION|')])
        assert after == before and after_dependencies == dependencies, 'own image/dependency identity changed'
        verification_record(directory, 'execute', execution_path, dict(execution, status='verified'))
        executions.append(execution)
    verify()


if __name__ == '__main__':
    env = dict(os.environ)
    excluded = ['COMPILER_PATH', 'LIBRARY_PATH', 'GCC_EXEC_PREFIX', 'LD_PRELOAD',
                'LD_LIBRARY_PATH', 'CPATH', 'C_INCLUDE_PATH', 'CPLUS_INCLUDE_PATH',
                'OBJC_INCLUDE_PATH', 'BASH_ENV', 'ENV']
    for key in excluded:
        env.pop(key, None)
    env.update({'PATH': '/usr/bin:/bin', 'LANG': 'C', 'LC_ALL': 'C'})
    selected_env = {'PATH': env['PATH'], 'LANG': env['LANG'], 'LC_ALL': env['LC_ALL'],
                    'removed_inherited_lookup_variables': excluded}
    request_digest = sha(request_path)
    request = json.loads(request_path.read_bytes())
    failure = None
    try:
        capture()
    except BaseException as exc:
        failure = refuse_record(exc)
        traceback.print_exc()
    closed = failure is None and len(executions) == 5 and all(
        e['returncode'] == 0 and not e['timeout'] and e['closed_cases'] == 4
        and e['closed_marker'] and e['executable_unchanged'] for e in executions)
    closure = {'request_sha256': request_digest, 'records': records,
        'verification_status': 'verified' if failure is None else 'refused',
        'harness_refusal': failure,
        'required_inputs_unchanged': True if failure is None else None,
        'fresh_output_root': str(base / 'providers'), 'all_capture_processes_closed': closed,
        'completed_verified_provider_executions': len(executions),
        'scope': 'Actual source/producer/dependency/process closure only. Original selected setter/free/getter/public error phases remain bounded by their exact rows. Jim Tcl API execution and all Rust assertion outcomes are unmeasured.'}
    save_json(base / 'closure.json', closure)
    print('Original explicit error-code header API closure:', closed, 'records:', len(records),
          'verification:', closure['verification_status'])
    raise SystemExit(1 if failure is not None else 0)
