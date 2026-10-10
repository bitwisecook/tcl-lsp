#!/usr/bin/env python3
"""Root-only fixed original info-complete capture; never overwrite a prior run."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

REQUEST = Path(__file__).with_name('request.json')

def pin(path):
    path = Path(path)
    original = path.read_bytes()
    return {'path': str(path), 'resolved_path': str(path.resolve()),
            'bytes': len(original), 'sha256': hashlib.sha256(original).hexdigest()}

def command(argv, environment, directory, label, timeout):
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    monotonic = time.monotonic()
    try:
        result = subprocess.run(argv, env=environment, cwd=directory,
                                capture_output=True, timeout=timeout, check=False)
        code, stdout, stderr = result.returncode, result.stdout, result.stderr
        timed_out = False
    except subprocess.TimeoutExpired as error:
        code, stdout, stderr = None, error.stdout or b'', error.stderr or b''
        timed_out = True
    for suffix, payload in [('stdout', stdout), ('stderr', stderr)]:
        (directory / (label + '.' + suffix)).write_bytes(payload)
    receipt = {'argv': argv, 'cwd': str(directory), 'started_utc': started,
               'elapsed_seconds': time.monotonic() - monotonic,
               'returncode': code, 'timed_out': timed_out,
               'stdout': pin(directory / (label + '.stdout')),
               'stderr': pin(directory / (label + '.stderr'))}
    (directory / (label + '.receipt.json')).write_text(json.dumps(receipt, indent=2) + '\n')
    return receipt

def main():
    request = json.loads(REQUEST.read_text())
    before = [pin(item['path']) for item in request['input_pins']]
    if before != request['input_pins']:
        raise SystemExit('sealed input pins no longer match; no provider launched')
    if pin(__file__)['sha256'] != request['launcher_sha256']:
        raise SystemExit('launcher self pin mismatch; no provider launched')
    capture = Path(request['capture_path'])
    capture.mkdir(exist_ok=False)
    (capture / 'request.json').write_bytes(REQUEST.read_bytes())
    records = []
    for provider in request['providers']:
        directory = capture / provider['label']
        directory.mkdir()
        environment = os.environ.copy()
        environment.update(provider['environment'])
        version = command([provider['executable'], request['version_script']], environment,
                          directory, 'version', 30)
        actual_version = (directory / 'version.stdout').read_bytes().strip().decode('ascii')
        version_ok = (version['returncode'] == 0 and
                      actual_version == provider['expected_version'] and
                      (directory / 'version.stderr').stat().st_size == 0)
        run = None
        rows = []
        if version_ok:
            run = command([provider['executable'], request['probe_script']], environment,
                          directory, 'execute', 30)
            for raw in (directory / 'execute.stdout').read_bytes().splitlines():
                if raw.startswith(b'ROW\t'):
                    rows.append(raw.decode('ascii').split('\t'))
        record = {'provider': provider, 'version': version, 'version_ok': version_ok,
                  'execute': run, 'row_count': len(rows),
                  'exact_original_row_labels': [row[1] for row in rows] == request['case_labels'],
                  'all_public_statuses_recorded': all(len(row) == 4 for row in rows)}
        (directory / 'receipt.json').write_text(json.dumps(record, indent=2) + '\n')
        records.append(record)
    after = [pin(item['path']) for item in request['input_pins']]
    receipt = {'request': pin(REQUEST), 'launcher': pin(__file__),
               'input_pins_before': before, 'input_pins_after': after,
               'input_pins_unchanged': before == after, 'records': records,
               'purpose': request['purpose'],
               'complete': before == after and all(
                   r['version_ok'] and r['execute'] is not None and
                   r['execute']['returncode'] == 0 and not r['execute']['timed_out'] and
                   r['execute']['stderr']['bytes'] == 0 and
                   r['exact_original_row_labels'] and r['all_public_statuses_recorded']
                   for r in records)}
    (capture / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps({'capture_path': str(capture), 'receipt_sha256': pin(capture / 'receipt.json')['sha256'],
                      'complete': receipt['complete']}, indent=2))
    return 0 if receipt['complete'] else 1

if __name__ == '__main__':
    raise SystemExit(main())
