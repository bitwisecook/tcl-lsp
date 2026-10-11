from pathlib import Path
import hashlib
import json
import os
import re
import shutil
import subprocess
import time

base = Path(__file__).resolve().parent
request_path = base / 'request.json'

def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()

def identity(path):
    path = Path(path)
    stat = path.stat()
    return {'path': str(path), 'resolved_path': str(path.resolve()),
            'sha256': sha(path), 'size': stat.st_size, 'device': stat.st_dev,
            'inode': stat.st_ino, 'mtime_ns': stat.st_mtime_ns, 'ctime_ns': stat.st_ctime_ns}

request_digest = sha(request_path)
request = json.loads(request_path.read_bytes())

def verify():
    assert sha(request_path) == request_digest, 'immutable request changed'
    for path, expected in request['required_sha256'].items():
        assert sha(path) == expected, path

verify()
# Reject output reuse before any compiler/provider process is entered.
providers_root = base / 'providers'
assert not providers_root.exists(), 'original output root already exists'
providers_root.mkdir()
records = []
env = dict(os.environ)
for inherited in ['COMPILER_PATH', 'LIBRARY_PATH', 'GCC_EXEC_PREFIX', 'LD_PRELOAD',
                  'LD_LIBRARY_PATH', 'BASH_ENV', 'ENV']:
    env.pop(inherited, None)
env.update({'PATH': '/usr/bin:/bin', 'LANG': 'C', 'LC_ALL': 'C'})
selected_env = {'PATH': env['PATH'], 'LANG': env['LANG'], 'LC_ALL': env['LC_ALL'],
                'removed_inherited_lookup_variables': ['COMPILER_PATH', 'LIBRARY_PATH',
                'GCC_EXEC_PREFIX', 'LD_PRELOAD', 'LD_LIBRARY_PATH', 'BASH_ENV', 'ENV']}

def run(provider, channel, command, directory, environment, before=None):
    verify()
    stdout_path = directory / (channel + '.stdout')
    stderr_path = directory / (channel + '.stderr')
    assert not stdout_path.exists() and not stderr_path.exists(), 'output stream already exists'
    start = time.monotonic(); timeout = False
    actual_env = dict(env); actual_env.update(environment)
    try:
        result = subprocess.run(command, env=actual_env, cwd=base, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, timeout=120)
        returncode = result.returncode; stdout = result.stdout; stderr = result.stderr
    except subprocess.TimeoutExpired as failure:
        timeout = True; returncode = None
        stdout = failure.stdout or b''; stderr = failure.stderr or b''
    stdout_path.write_bytes(stdout); stderr_path.write_bytes(stderr)
    receipt = {'provider': provider, 'channel': channel, 'status': 'closed',
               'command': command, 'cwd': str(base),
               'selected_environment': dict(selected_env, **environment),
               'request_sha256': request_digest, 'source_sha256': request['probe_sha256'],
               'returncode': returncode, 'timeout': timeout,
               'seconds': time.monotonic() - start,
               'stdout_sha256': sha(stdout_path), 'stderr_sha256': sha(stderr_path)}
    if before is not None:
        receipt['executable_before'] = before
        receipt['executable_after'] = identity(command[0])
        receipt['executable_unchanged'] = before == receipt['executable_after']
    verify()
    return receipt, stdout

def save(receipt, directory):
    path = directory / (receipt['channel'] + '.receipt.json')
    assert not path.exists(), 'receipt already exists'
    path.write_text(json.dumps(receipt, indent=2) + '\n')
    records.append({'provider': receipt['provider'], 'channel': receipt['channel'],
                    'path': str(path), 'sha256': sha(path), 'status': 'closed'})
    return path

compiler_receipt, _ = run('compiler', 'compiler-version', [request['compiler'], '--version'], base, {})
save(compiler_receipt, base)
executions = []
for provider in request['provider_builds']:
    name = provider['provider']; directory = providers_root / name
    directory.mkdir(exist_ok=False)
    command = provider['command']; executable = Path(command[-1])
    assert not executable.exists(), 'compiler output already exists'
    compile_receipt, _ = run(name, 'compile', command, directory, provider['environment'])
    compile_receipt['output_existed_before'] = False
    successful = compile_receipt['returncode'] == 0 and not compile_receipt['timeout']
    if successful:
        compile_receipt['executable_produced'] = identity(executable)
        link_map = directory / 'link.map'
        compile_receipt['link_map'] = identity(link_map)
        header_map = directory / 'compile.d'
        compile_receipt['header_dependency_file'] = identity(header_map)
        header_inputs = []
        dependency_text = header_map.read_text(errors='strict').replace('\\\n', '')
        for selected in dependency_text.split(':', 1)[1].split():
            path = Path(selected).resolve()
            assert str(path) in request['required_sha256'], 'unpinned original header/source input: ' + str(path)
            assert sha(path) == request['required_sha256'][str(path)]
            header_inputs.append(identity(path))
        compile_receipt['actual_header_and_source_dependencies'] = header_inputs
        loads = []
        for line in link_map.read_text(errors='strict').splitlines():
            if not line.startswith('LOAD '):
                continue
            path = Path(line[5:]); resolved = path.resolve()
            if path.is_file():
                assert str(resolved) in request['required_sha256'], 'unpinned real link input: ' + str(resolved)
                assert sha(path) == request['required_sha256'][str(resolved)]
                loads.append(dict(identity(path), status='retained-pinned-original-link-input'))
            else:
                # The compiler's ephemeral source object is not an original SDK input.
                assert str(path).startswith('/tmp/') and path.suffix == '.o', str(path)
                loads.append({'path': str(path), 'status': 'compiler-produced-ephemeral-object-not-retained'})
        compile_receipt['actual_link_inputs'] = loads
    compile_path = save(compile_receipt, directory)
    if not successful:
        records.append({'provider': name, 'channel': 'execute', 'status': 'not-executed-compile-failed'})
        continue
    produced = compile_receipt['executable_produced']
    assert identity(executable) == produced
    ldd = str(Path(shutil.which('ldd', path=env['PATH'])).resolve())
    dependency_receipt, dependency_stdout = run(name, 'dependencies', [ldd, str(executable)], directory, {})
    dependencies = []
    for line in dependency_stdout.decode('utf-8', errors='strict').splitlines():
        for found in re.findall(r'(?<!\S)(/[^\s()]+)', line):
            path = Path(found); resolved = path.resolve()
            assert path.is_file(), str(path)
            assert str(resolved) in request['required_sha256'], 'unpinned runtime dependency: ' + str(resolved)
            assert sha(path) == request['required_sha256'][str(resolved)]
            dependencies.append(identity(path))
    assert dependency_receipt['returncode'] == 0 and not dependency_receipt['timeout']
    dependency_receipt['actual_dynamic_dependencies'] = dependencies
    dependency_receipt['executable_before'] = produced
    dependency_receipt['executable_after'] = identity(executable)
    assert dependency_receipt['executable_after'] == produced
    dependency_path = save(dependency_receipt, directory)
    for original_case in range(8):
        case_directory = directory / 'cases' / str(original_case)
        case_directory.mkdir(parents=True, exist_ok=False)
        before = identity(executable); assert before == produced
        receipt, stdout = run(name, 'execute', [str(executable), str(original_case)],
                              case_directory, provider['environment'], before)
        closed_cases = [int(line.split(b'|')[1]) for line in stdout.splitlines()
                        if line.startswith(b'CLOSED_CASE|')]
        row_cases = sorted({int(line.split(b'|')[1]) for line in stdout.splitlines()
                            if line.startswith(b'ROW|')})
        receipt.update({'original_case': original_case,
                        'termination_signal': -receipt['returncode']
                            if receipt['returncode'] is not None and receipt['returncode'] < 0 else None,
                        'compile_receipt_sha256': sha(compile_path),
                        'dependency_receipt_sha256': sha(dependency_path),
                        'own_successful_compile_producer': produced,
                        'dependencies_before': dependencies,
                        'dependencies_after': [identity(item['path']) for item in dependencies],
                        'closed_cases': closed_cases,
                        'closed_marker': b'CLOSED_ORIGINAL_CONSTANT_CAPTURED_CELL_ORDER\n' in stdout,
                        'actual_ROW_case_indices': row_cases,
                        'actual_version_rows': [line.decode('ascii') for line in stdout.splitlines()
                                                if b'|version|' in line]})
        assert receipt['dependencies_after'] == dependencies
        assert all(case == original_case for case in row_cases), 'unexpected case output'
        save(receipt, case_directory); executions.append(receipt)

verify()
case_process_capture_complete = len(executions) == 48 and all(
    item['returncode'] is not None and not item['timeout'] and item['executable_unchanged']
    for item in executions)
all_successful = case_process_capture_complete and all(
    item['returncode'] == 0 and item['closed_cases'] == [item['original_case']]
    and item['closed_marker'] for item in executions)
fatal = [{'provider': item['provider'], 'original_case': item['original_case'],
          'returncode': item['returncode'], 'termination_signal': item['termination_signal']}
         for item in executions if item['termination_signal'] is not None]
closure = {'request_sha256': request_digest, 'records': records,
           'required_inputs_unchanged': True, 'fresh_output_root': str(providers_root),
           'required_case_process_count': 48, 'actual_case_process_count': len(executions),
           'case_process_capture_complete': case_process_capture_complete,
           'all_capture_processes_closed': all_successful,
           'all_case_processes_exit_successfully_with_markers': all_successful,
           'native_fatal_case_processes': fatal,
           'scope': 'Each original case has a separate fresh process. Full unbuffered streams and '
                    'actual fatal/success/timeout statuses remain independent; no defined language '
                    'semantics or missing public state is inferred beyond an abnormal termination.'}
(base / 'closure.json').write_text(json.dumps(closure, indent=2) + '\n')
print('Original isolated constant case process capture complete:', case_process_capture_complete,
      'all successful:', all_successful, 'fatal case processes:', fatal, 'records:', len(records))
