#!/usr/bin/env python3
"""Verify retained original capture artifacts without executing providers."""
import hashlib
import json
from pathlib import Path
root = Path(__file__).resolve().parent
for directory in sorted(root.glob('capture*')):
    if not directory.is_dir():
        continue
    manifest = json.loads((directory / 'manifest.json').read_text())
    def digest(path):
        return hashlib.sha256(path.read_bytes()).hexdigest()
    assert digest(directory / 'probe.c') == manifest['source_sha256']
    assert digest(directory / 'capture.py') == manifest['driver_sha256']
    for stage in ('upstream', 'local_revision', 'local_status', 'compiler', 'compile', 'execution', 'linked_libraries', 'original_namespace_suite'):
        record = manifest.get(stage)
        if record is None:
            continue
        label = {'local_revision': 'local-revision', 'local_status': 'local-status', 'linked_libraries': 'linked-libraries', 'original_namespace_suite': 'namespace-suite'}.get(stage, stage)
        for stream in ('stdout', 'stderr'):
            assert digest(directory / (label + '.' + stream)) == record[stream + '_sha256']
    if 'executable_sha256' in manifest:
        assert digest(directory / 'probe') == manifest['executable_sha256']
    assert manifest['provider_unchanged'] and manifest['source_unchanged']
completed = json.loads((root/'capture03/manifest.json').read_text())
assert completed['execution']['exit_code'] == 0 and completed['rows'] == 30
assert completed['compile']['exit_code'] == 0 and completed['upstream_matches_local']
assert len(completed['provider_before_sha256']) == 247
assert 'Total    79   Passed    79  Skipped     0  Failed     0' in (root/'capture03/namespace-suite.stdout').read_text()
print('PASS: retained hashes and completed original capture03 (no provider execution)')
