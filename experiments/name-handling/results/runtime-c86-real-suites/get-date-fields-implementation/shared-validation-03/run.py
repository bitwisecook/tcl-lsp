#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Capture the authorised two exact native Core controls once."""
from pathlib import Path
import datetime
import gzip
import hashlib
import json
import os
import subprocess
import sys

ROOT = Path('/workspace/tcl-lsp')
OUT = Path(__file__).resolve().parent
TARGET = ROOT / 'tmp/name-target-runtime'
READY = OUT / 'source-readiness.json'
QUESTION = ROOT / 'experiments/name-handling/results/runtime-c86-real-suites/get-date-fields-question'
expected = json.loads(READY.read_text())['source_ready_sha256']
record = {'cwd': str(ROOT), 'target_dir': str(TARGET), 'started_utc': None,
          'steps': [], 'binaries': {}, 'status': 'initialising',
          'selected_tests': {'tcl-cmd-core': ['clock::c86_calendar_tests::retained_c86_numeric_calendar_rows', 'clock::c86_calendar_tests::unmeasured_native_calendar_overflow_refuses']}}

def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def save():
    (OUT / 'execution.json').write_text(json.dumps(record, indent=2) + '\n')

def freeze_check(label):
    actual = {name: sha(ROOT / name) for name in expected}
    (OUT / (label + '-source-hashes.json')).write_text(json.dumps(actual, indent=2) + '\n')
    if actual != expected:
        raise RuntimeError('Frozen five-source hashes changed: ' + label)

def run(label, argv):
    item = {'label': label, 'argv': argv, 'start_utc': now(), 'stdout': label + '.stdout.gz', 'stderr': label + '.stderr.gz'}
    record['steps'].append(item)
    save()
    stdout_path = OUT / (label + '.stdout')
    stderr_path = OUT / (label + '.stderr')
    with stdout_path.open('wb') as stdout, stderr_path.open('wb') as stderr:
        process = subprocess.run(argv, cwd=ROOT, stdout=stdout, stderr=stderr)
    item.update(exit_code=process.returncode, end_utc=now())
    stdout_bytes, stderr_bytes = stdout_path.read_bytes(), stderr_path.read_bytes()
    for path in (stdout_path, stderr_path):
        with gzip.open(str(path) + '.gz', 'wb', compresslevel=9) as stream:
            stream.write(path.read_bytes())
        path.unlink()
    save()
    print(label, 'exit', process.returncode, flush=True)
    if process.returncode:
        print(stderr_bytes.decode(errors='replace')[-12000:], flush=True)
        print(stdout_bytes.decode(errors='replace')[-12000:], flush=True)
        raise RuntimeError('First unsuccessful actual step preserved: ' + label)
    return stdout_bytes

try:
    record['started_utc'] = now()
    assert os.environ['CARGO_TARGET_DIR'] == str(TARGET)
    freeze_check('before')
    sealed = json.loads((QUESTION / 'sealed-files.json').read_text())
    question_actual = {name: sha(QUESTION / name) for name in sealed}
    assert question_actual == sealed, 'Existing original question seal changed'
    (OUT / 'question-seal-before.json').write_text(json.dumps(question_actual, indent=2) + '\n')
    with gzip.open(OUT / 'source-readiness.json.gz', 'wb') as stream:
        stream.write(READY.read_bytes())
    for name in expected:
        with gzip.open(OUT / (name.replace('/', '--') + '.gz'), 'wb') as stream:
            stream.write((ROOT / name).read_bytes())
    paths = subprocess.check_output(['git', 'ls-files', '-z'], cwd=ROOT).decode().split('\0')
    context = {name: sha(ROOT / name) for name in paths if name and (ROOT / name).is_file()
               and ((name.startswith('rust/') or name.startswith('runtime/rust/')) and Path(name).suffix in ('.rs', '.toml', '.lock', '.h', '.c')
                    or name in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.cargo/config.toml'))}
    context.update({name: sha(ROOT / name) for name in expected})
    with gzip.open(OUT / 'conservative-source-context-before.json.gz', 'wb') as stream:
        stream.write((json.dumps(context, indent=2) + '\n').encode())
    record['context_count'] = len(context)
    record['status'] = 'compiling-and-testing'
    save()
    for package, tests in record['selected_tests'].items():
        freeze_check(package + '-before-compile')
        output = run(package + '-compile', ['cargo', 'test', '--offline', '-j2', '-p', package, '--lib', '--no-run', '--message-format=json'])
        messages = [json.loads(line) for line in output.decode().splitlines() if line.startswith('{')]
        artifacts = [message for message in messages if message.get('reason') == 'compiler-artifact']
        binaries = [artifact['executable'] for artifact in artifacts if artifact.get('executable') and artifact['target']['name'] == package.replace('-', '_') and artifact['profile']['test']]
        assert len(binaries) == 1, binaries
        binary = Path(binaries[0])
        record['binaries'][package] = {'path': str(binary), 'before_sha256': sha(binary), 'artifacts': artifacts}
        save()
        listing = run(package + '-list', [str(binary), '--list'])
        names = [line.removesuffix(': test') for line in listing.decode().splitlines() if line.endswith(': test')]
        assert all(test in names for test in tests), (tests, names)
        record['binaries'][package]['observed_exact_names'] = tests
        record['binaries'][package]['listed_test_count'] = len(names)
        save()
        for index, test in enumerate(tests, 1):
            test_output = run(package + '-exact-' + str(index), [str(binary), '--exact', test, '--nocapture'])
            assert b'running 1 test' in test_output and b'1 passed; 0 failed' in test_output, test_output
            record['steps'][-1]['executed_test_count'] = 1
            save()
        record['binaries'][package]['after_sha256'] = sha(binary)
        with gzip.open(OUT / (package + '-native-test-binary.gz'), 'wb') as stream:
            stream.write(binary.read_bytes())
        assert record['binaries'][package]['before_sha256'] == record['binaries'][package]['after_sha256']
        freeze_check(package + '-after-tests')
    freeze_check('after')
    question_after = {name: sha(QUESTION / name) for name in sealed}
    assert question_after == question_actual
    (OUT / 'question-seal-after.json').write_text(json.dumps(question_after, indent=2) + '\n')
    record['status'] = 'two-core-exact-controls-passed-cache-released'
except Exception as error:
    record['status'] = 'first-failure-preserved-cache-released'
    record['error'] = str(error)
    try:
        freeze_check('after-failure')
    except Exception as hash_error:
        record['hash_error'] = str(hash_error)
    save()
    print(record['status'], record['error'], flush=True)
    sys.exit(1)
finally:
    record['completed_utc'] = now()
    record['cache_released_utc'] = now()
    save()
print(record['status'], flush=True)
