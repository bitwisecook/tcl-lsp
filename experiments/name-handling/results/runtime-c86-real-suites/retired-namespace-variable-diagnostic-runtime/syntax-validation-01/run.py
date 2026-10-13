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

"""Capture two selected Syntax controls after root's serial resource grant."""
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
TARGET = ROOT / 'tmp/name-target-issues'
EXPECTED_SYNTAX = '814ce903919809bdf6fa627b186d26b8c3bc3f8205c8c2b898c6f20647bc8fcb'
TESTS = [
    'naming::native::tests::retired_namespace_write_sites_match_retained_original_c_rows',
    'naming::native::tests::indexed_retired_lookup_rejects_missing_or_incompatible_purpose',
]
BASE = OUT.parent
SEALS = [
    BASE.parent / 'retired-namespace-variable-diagnostic-question',
    BASE / 'route-diagnostic-01',
]
record = {'cwd': str(ROOT), 'target_dir': str(TARGET), 'steps': [],
          'expected_names_not_yet_observed': TESTS, 'observed_exact_names': [],
          'status': 'prepared-unexecuted'}


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def zipped(name, data):
    with gzip.open(OUT / name, 'wb', compresslevel=9) as stream:
        stream.write(data)


def save():
    (OUT / 'execution.json').write_text(json.dumps(record, indent=2) + '\n')


def run(label, argv):
    step = {'label': label, 'argv': argv, 'start_utc': now(),
            'stdout': label + '.stdout.gz', 'stderr': label + '.stderr.gz'}
    record['steps'].append(step)
    save()
    print(label, 'started', flush=True)
    with (OUT / (label + '.stdout')).open('wb') as stdout, (OUT / (label + '.stderr')).open('wb') as stderr:
        result = subprocess.run(argv, cwd=ROOT, stdout=stdout, stderr=stderr)
    step.update(exit_code=result.returncode, end_utc=now())
    output = (OUT / (label + '.stdout')).read_bytes()
    for stream in ('stdout', 'stderr'):
        path = OUT / (label + '.' + stream)
        zipped(path.name + '.gz', path.read_bytes())
        path.unlink()
    save()
    print(label, 'exit', result.returncode, flush=True)
    if result.returncode:
        raise RuntimeError('First unsuccessful step preserved: ' + label)
    return output


def source_hashes(keys):
    return {name: sha(ROOT / name) for name in keys}


def seal_check(label):
    records = []
    for folder in SEALS:
        expected = json.loads((folder / 'sealed-files.json').read_text())
        actual = {name: sha(folder / name) for name in expected}
        assert actual == expected, str(folder) + ' existing seal changed'
        records.append({'path': str(folder.relative_to(ROOT)), 'count': len(expected),
                        'seal_sha256': sha(folder / 'sealed-files.json'), 'changed': []})
    assert [item['count'] for item in records] == [46, 20]
    (OUT / (label + '-reused-seals.json')).write_text(json.dumps(records, indent=2) + '\n')


if sys.argv[1:] != ['--run-authorised']:
    raise SystemExit('Prepared only: serial resource release and root GO are required.')
if (OUT / 'execution.json').exists():
    raise SystemExit('Existing attempt preserved; this evidence directory must execute once.')

try:
    assert os.environ.get('CARGO_TARGET_DIR') == str(TARGET), 'Explicit issues target required'
    assert sha(ROOT / 'rust/tcl-syntax/src/naming/native.rs') == EXPECTED_SYNTAX
    record['started_utc'] = now()
    seal_check('before')
    closure = json.loads((OUT / 'manifest-closure.json').read_text())
    keys = {'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.cargo/config.toml'}
    for folder in closure['package_dirs'].values():
        for path in (ROOT / folder).rglob('*'):
            if path.is_file() and not any(part in ('target', '.git') for part in path.parts):
                keys.add(str(path.relative_to(ROOT)))
    initial = source_hashes(sorted(keys))
    zipped('before-source-context.json.gz', (json.dumps(initial, indent=2) + '\n').encode())
    record['conservative_context_count'] = len(initial)
    record['conservative_local_package_count'] = len(closure['package_dirs'])
    original = BASE / 'applied-caller-context-repair/rust--tcl-syntax--src--naming--native.rs.gz'
    assert hashlib.sha256(gzip.decompress(original.read_bytes())).hexdigest() == EXPECTED_SYNTAX
    record['reused_syntax_source_image'] = str(original.relative_to(ROOT))
    record['syntax_before_sha256'] = EXPECTED_SYNTAX
    record['status'] = 'building-selected-syntax-library-test-target'
    save()
    output = run('syntax-compile', ['cargo', 'test', '--offline', '-j2', '-p',
                 'tcl-syntax', '--lib', '--no-run', '--message-format=json'])
    messages = [json.loads(line) for line in output.decode().splitlines() if line.startswith('{')]
    artifacts = [message for message in messages if message.get('reason') == 'compiler-artifact']
    zipped('actual-artifact-graph.json.gz', (json.dumps(artifacts, indent=2) + '\n').encode())
    record['actual_emitted_artifact_count'] = len(artifacts)
    record['actual_rebuilt_artifact_count'] = sum(not artifact['fresh'] for artifact in artifacts)
    record['actual_emitted_local_packages'] = sorted(set(
        artifact['manifest_path'].removesuffix('/Cargo.toml').removeprefix(str(ROOT) + '/')
        for artifact in artifacts if artifact['package_id'].startswith('path+')))
    binaries = [artifact['executable'] for artifact in artifacts if artifact.get('executable')
                and artifact['target']['name'] == 'tcl_syntax' and artifact['profile']['test']]
    assert len(binaries) == 1, binaries
    binary = Path(binaries[0])
    record['binary_path'] = str(binary)
    record['binary_before_sha256'] = sha(binary)
    listing = run('syntax-list', [str(binary), '--list'])
    names = [line.removesuffix(': test') for line in listing.decode().splitlines() if line.endswith(': test')]
    assert all(test in names for test in TESTS), TESTS
    record['observed_exact_names'] = TESTS
    record['native_listed_test_count'] = len(names)
    save()
    for index, test in enumerate(TESTS, 1):
        result = run('syntax-exact-' + str(index), [str(binary), '--exact', test, '--nocapture'])
        assert b'running 1 test' in result and b'1 passed; 0 failed' in result
        record['steps'][-1]['executed_test_count'] = 1
        save()
    record['binary_after_sha256'] = sha(binary)
    assert record['binary_before_sha256'] == record['binary_after_sha256']
    record['syntax_after_sha256'] = sha(ROOT / 'rust/tcl-syntax/src/naming/native.rs')
    assert record['syntax_after_sha256'] == EXPECTED_SYNTAX
    final = source_hashes(sorted(keys))
    zipped('after-source-context.json.gz', (json.dumps(final, indent=2) + '\n').encode())
    record['changed_conservative_sources'] = [name for name in keys if initial[name] != final[name]]
    assert not record['changed_conservative_sources'], 'Seven-package source context changed'
    seal_check('after')
    record['status'] = 'two-selected-syntax-controls-passed-cache-released'
except Exception as error:
    record['status'] = 'first-failure-preserved-cache-released'
    record['error'] = str(error)
    if 'keys' in globals():
        zipped('after-failure-source-context.json.gz', (json.dumps(source_hashes(sorted(keys)), indent=2) + '\n').encode())
    print(record['status'], record['error'], flush=True)
finally:
    record['completed_utc'] = now()
    record['cache_released_utc'] = now()
    save()
print(record['status'], flush=True)
sys.exit(0 if record['status'].startswith('two-selected') else 1)

