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
completed = json.loads((root/'capture07/manifest.json').read_text())
assert completed['execution']['exit_code'] == 0 and completed['rows'] == 47
assert completed['compile']['exit_code'] == 0 and completed['upstream_matches_local']
assert len(completed['provider_before_sha256']) == 247
assert 'Total    79   Passed    79  Skipped     0  Failed     0' in (root/'capture07/namespace-suite.stdout').read_text()

rows = (root/'capture03/execution.stdout').read_text().splitlines()
observed = [row for row in rows if row.startswith('OBS|')]
assert len(observed) == 11
assert observed[:9] == [row for row in (root/'capture02/execution.stdout').read_text().splitlines() if row.startswith('OBS|')]
assert observed[:9] == (root/'capture01/execution.stdout').read_text().splitlines()
assert any(row == 'UNSELECTED_SCRIPT|10|0|try-stage-probe' for row in rows)
rows04 = (root/'capture04/execution.stdout').read_text().splitlines()
rows05 = (root/'capture05/execution.stdout').read_text().splitlines()
assert [row for row in rows04 if row.startswith('OBS|')][:11] == observed
assert [row for row in rows05 if row.startswith('OBS|')][:13] == [row for row in rows04 if row.startswith('OBS|')]
assert 'UNSELECTED_SCRIPT|11|0|int' in rows05
assert 'UNREACHED_MEMBER|12|0|int' in rows05
assert 'REACHED_MEMBER|13|1|string' in rows05
rows06 = (root/'capture06/execution.stdout').read_text().splitlines()
rows07 = (root/'capture07/execution.stdout').read_text().splitlines()
assert [row for row in rows06 if row.startswith('OBS|')][:14] == [row for row in rows05 if row.startswith('OBS|')]
assert [row for row in rows07 if row.startswith('OBS|')] == [row for row in rows06 if row.startswith('OBS|')]
assert len([row for row in rows07 if row.startswith('OBS|')]) == 17
assert 'GETTER_BEFORE|14|1|message-name|try-stage-probe|1|424f4459' in rows07
assert 'GETTER_AFTER|14|1|4d53474e414d45' in rows07
assert 'GETTER_BEFORE|15|1|options-name|try-stage-probe|1|424f4459' in rows07
assert 'GETTER_AFTER|15|1|4f50544e414d45' in rows07
assert rows07.index('GETTER_AFTER|16|1|56415253') < rows07.index('GETTER_BEFORE|16|2|trap-member|try-stage-probe|1|56415253')
print('PASS: retained hashes and completed original callback capture07 (no provider execution)')
