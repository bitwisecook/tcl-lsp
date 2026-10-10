from pathlib import Path
import hashlib
import json
import os
import subprocess
import time

P = Path(__file__).resolve().parent
request_bytes = (P / 'request.json').read_bytes()
request = json.loads(request_bytes)
sha = lambda data: hashlib.sha256(data).hexdigest()


def validate_inputs():
    for path, expected in request['required_sha256'].items():
        if sha(Path(path).read_bytes()) != expected:
            raise RuntimeError('Required input mismatch: ' + path)


validate_inputs()
environment = dict(os.environ)
environment.update(request['environment'])
receipts = []
for operation in request['operations']:
    relative = (Path('cases') / operation['family'] / str(operation['case'])
                if operation['kind'] == 'original-case' else Path('metadata'))
    directory = P / 'providers' / 'jim' / relative
    directory.mkdir(parents=True, exist_ok=True)
    stdout = directory / 'execute.stdout'
    stderr = directory / 'execute.stderr'
    started = time.time()
    timeout = False
    with stdout.open('wb') as output, stderr.open('wb') as errors:
        try:
            process = subprocess.run(operation['command'], env=environment, stdout=output,
                                     stderr=errors, timeout=20)
            returncode = process.returncode
        except subprocess.TimeoutExpired:
            timeout = True
            returncode = None
    stream = stdout.read_bytes()
    marker = b'CLOSED_CASE\n' if operation['kind'] == 'original-case' else b'CLOSED_METADATA\n'
    receipt = {'request_sha256': sha(request_bytes), 'operation': operation,
               'command': operation['command'], 'selected_environment': request['environment'],
               'executable_sha256': request['executable_sha256'], 'returncode': returncode,
               'signal': -returncode if returncode is not None and returncode < 0 else None,
               'timeout': timeout, 'start_unix_seconds': started, 'end_unix_seconds': time.time(),
               'stdout_file': str(stdout), 'stdout_sha256': sha(stream),
               'stderr_file': str(stderr), 'stderr_sha256': sha(stderr.read_bytes()),
               'closed_marker': marker in stream, 'status': 'closed'}
    if operation['kind'] == 'original-case':
        matches = [line.split(b' ', 2) for line in stream.splitlines() if line.startswith(b'CASE_RESULT ')]
        receipt['public_result_record_count'] = len(matches)
        if len(matches) == 1:
            try:
                code = int(matches[0][1])
                result_hex = matches[0][2].decode('ascii')
                bytes.fromhex(result_hex)
                receipt['observed_code'] = code
                receipt['observed_result_hex'] = result_hex
                receipt['old_public_code_and_result_match'] = (
                    code == operation['expected_code'] and result_hex == operation['expected_result_hex'])
            except (ValueError, UnicodeDecodeError):
                receipt['old_public_code_and_result_match'] = False
    (directory / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    receipts.append(receipt)
validate_inputs()
closure = {'request_sha256': sha(request_bytes), 'records': receipts,
           'all_processes_closed': len(receipts) == request['expected_process_count'],
           'all_succeeded': all(row['returncode'] == 0 and not row['timeout'] and row['closed_marker'] for row in receipts),
           'all_24_original_public_code_and_result_pairs_match':
               sum(row['operation']['kind'] == 'original-case' for row in receipts) == 24 and
               all(row.get('old_public_code_and_result_match', False) for row in receipts
                   if row['operation']['kind'] == 'original-case'),
           'required_inputs_remained_exact': True}
(P / 'closure.json').write_text(json.dumps(closure, indent=2) + '\n')
print('CLOSED original processes:', len(receipts), 'all succeeded:', closure['all_succeeded'],
      '24 public pairs match:', closure['all_24_original_public_code_and_result_pairs_match'])
