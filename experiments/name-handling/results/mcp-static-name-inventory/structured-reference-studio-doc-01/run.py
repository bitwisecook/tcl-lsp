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

"""Run only after root authorisation and explicit release of the issues cache."""
from pathlib import Path
import datetime
import difflib
import gzip
import hashlib
import json
import os
import subprocess
import sys

ROOT = Path('/workspace/tcl-lsp')
OUT = Path(__file__).resolve().parent
TARGET = ROOT / 'tmp/name-target-issues'
REUSE = ROOT / 'experiments/name-handling/results/joint-source-metadata-validation/validation-01/build'
DOCUMENT = ROOT / 'docs/references/command-spec/fields.md'
TESTS = {
    'schema_coverage': 'schema_covers_every_subcommand_field',
    'reference_doc': 'fields_doc_matches_the_schema',
}
record = {'cwd': str(ROOT), 'target_dir': str(TARGET), 'steps': [],
          'binaries': {}, 'status': 'prepared-unexecuted',
          'prior_owner_images': str(REUSE / 'changed-owner-images.json.gz')}


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save():
    (OUT / 'execution.json').write_text(json.dumps(record, indent=2) + '\n')


def zipped(name, data):
    with gzip.open(OUT / name, 'wb', compresslevel=9) as stream:
        stream.write(data)


def run(label, argv, generate=False):
    environment = os.environ.copy()
    environment.pop('UPDATE_REFERENCE', None)
    if generate:
        environment['UPDATE_REFERENCE'] = '1'
    step = {'label': label, 'argv': argv, 'start_utc': now(),
            'document_generation': generate,
            'update_reference': '1' if generate else 'unset',
            'stdout': label + '.stdout.gz', 'stderr': label + '.stderr.gz'}
    record['steps'].append(step)
    save()
    print(label, 'started', flush=True)
    with (OUT / (label + '.stdout')).open('wb') as stdout, (OUT / (label + '.stderr')).open('wb') as stderr:
        result = subprocess.run(argv, cwd=ROOT, env=environment, stdout=stdout, stderr=stderr)
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


def sources(keys):
    return {name: sha(ROOT / name) for name in keys}


def source_snapshot(label, values):
    zipped(label + '-sources.json.gz', (json.dumps(values, indent=2) + '\n').encode())


if sys.argv[1:] != ['--run-authorised']:
    raise SystemExit('Prepared only: root GO and explicit cache release are required before --run-authorised.')
if (OUT / 'execution.json').exists():
    raise SystemExit('Existing attempt preserved; this evidence directory must execute once.')

try:
    assert os.environ.get('CARGO_TARGET_DIR') == str(TARGET), 'Explicit issues target required'
    record['started_utc'] = now()
    prior_inventory = json.loads(gzip.decompress((REUSE / 'before-sources.json.gz').read_bytes()))
    keys = sorted(set(prior_inventory) | {
        'rust/tcl-spec-studio/tests/schema_coverage.rs',
        'rust/tcl-spec-studio/tests/reference_doc.rs',
        'rust/tcl-spec-studio/src/reference.rs',
        'rust/tcl-registry/src/source_configuration_reference.rs',
    })
    initial = sources(keys)
    source_snapshot('before', initial)
    record['conservative_inventory_count'] = len(keys)
    record['conservative_inventory_scope'] = 'Current hashes over the retained joint-build source/build inventory; conservative superset, not proof every file compiled.'
    owner_images = json.loads(gzip.decompress((REUSE / 'changed-owner-images.json.gz').read_bytes()))
    studio_reuse = {
        name: {'image_sha256': hashlib.sha256(content.encode()).hexdigest(),
               'current_sha256': initial[name]}
        for name, content in owner_images.items() if name.startswith('rust/tcl-spec-studio/')
    }
    assert len(studio_reuse) == 9, studio_reuse
    assert all(value['image_sha256'] == value['current_sha256'] for value in studio_reuse.values()), 'Studio images changed since prior parity controls'
    (OUT / 'reused-studio-owner-images.json').write_text(json.dumps(studio_reuse, indent=2) + '\n')
    for name in ('rust/tcl-spec-studio/tests/schema_coverage.rs',
                 'rust/tcl-spec-studio/tests/reference_doc.rs',
                 'rust/tcl-spec-studio/src/reference.rs'):
        zipped(name.replace('/', '--') + '.gz', (ROOT / name).read_bytes())
    before_document = DOCUMENT.read_bytes()
    record['document_before_sha256'] = sha(DOCUMENT)
    zipped('fields.md.before.gz', before_document)
    record['status'] = 'building-selected-two-integration-targets'
    save()
    output = run('studio-two-target-compile', ['cargo', 'test', '--offline', '-j2',
                 '-p', 'tcl-spec-studio', '--test', 'schema_coverage', '--test', 'reference_doc',
                 '--no-run', '--message-format=json'])
    messages = [json.loads(line) for line in output.decode().splitlines() if line.startswith('{')]
    artifacts = [message for message in messages if message.get('reason') == 'compiler-artifact']
    zipped('actual-artifact-graph.json.gz', (json.dumps(artifacts, indent=2) + '\n').encode())
    record['actual_artifact_count'] = len(artifacts)
    record['actual_rebuilt_artifact_count'] = sum(not item['fresh'] for item in artifacts)
    for target, exact in TESTS.items():
        binaries = [item['executable'] for item in artifacts
                    if item.get('executable') and item['target']['name'] == target and item['profile']['test']]
        assert len(binaries) == 1, (target, binaries)
        binary = Path(binaries[0])
        record['binaries'][target] = {'path': str(binary), 'before_sha256': sha(binary)}
        listing = run(target + '-list', [str(binary), '--list'])
        names = [line.removesuffix(': test') for line in listing.decode().splitlines() if line.endswith(': test')]
        assert exact in names, (target, exact)
        record['binaries'][target].update(observed_exact_name=exact, listed_test_count=len(names))
    save()
    schema_bin = record['binaries']['schema_coverage']['path']
    reference_bin = record['binaries']['reference_doc']['path']
    schema_output = run('schema-exact', [schema_bin, '--exact', TESTS['schema_coverage'], '--nocapture'])
    assert b'running 1 test' in schema_output and b'1 passed; 0 failed' in schema_output
    record['steps'][-1]['executed_test_count'] = 1
    generated = run('reference-generate', [reference_bin, '--exact', TESTS['reference_doc'], '--nocapture'], generate=True)
    assert b'running 1 test' in generated and b'1 passed; 0 failed' in generated
    record['steps'][-1]['executed_test_count'] = 1
    after_document = DOCUMENT.read_bytes()
    record['document_after_sha256'] = sha(DOCUMENT)
    zipped('fields.md.after.gz', after_document)
    diff = ''.join(difflib.unified_diff(before_document.decode().splitlines(True),
                 after_document.decode().splitlines(True),
                 fromfile='docs/references/command-spec/fields.md (before)',
                 tofile='docs/references/command-spec/fields.md (generated)'))
    (OUT / 'generated-fields.patch').write_text(diff)
    checked = run('reference-check', [reference_bin, '--exact', TESTS['reference_doc'], '--nocapture'])
    assert b'running 1 test' in checked and b'1 passed; 0 failed' in checked
    record['steps'][-1]['executed_test_count'] = 1
    assert sha(DOCUMENT) == record['document_after_sha256'], 'Plain check changed generated output'
    for target, binary in record['binaries'].items():
        binary['after_sha256'] = sha(Path(binary['path']))
        assert binary['before_sha256'] == binary['after_sha256']
    final = sources(keys)
    source_snapshot('after', final)
    changed = {name: {'before': initial[name], 'after': final[name]} for name in keys if initial[name] != final[name]}
    record['changed_conservative_sources'] = changed
    assert not changed, 'Source/build inventory changed during selected checks'
    record['status'] = 'two-selected-verification-controls-passed-document-generated-cache-released'
except Exception as error:
    record['status'] = 'first-failure-preserved-cache-released'
    record['error'] = str(error)
    if 'keys' in globals():
        source_snapshot('after-failure', sources(keys))
    print(record['status'], record['error'], flush=True)
finally:
    record['completed_utc'] = now()
    record['cache_released_utc'] = now()
    save()
print(record['status'], flush=True)
sys.exit(0 if record['status'].startswith('two-selected') else 1)

