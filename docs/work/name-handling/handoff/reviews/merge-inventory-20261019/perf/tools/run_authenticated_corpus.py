#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Require complete current report controls, then build/run/delete the corpus probes."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('root', type=Path)
parser.add_argument('--controls', type=Path, required=True)
parser.add_argument('--realm-controls', type=Path, required=True)
parser.add_argument('--evidence', type=Path, required=True)
args = parser.parse_args()
root = args.root.resolve()
controls = args.controls.resolve()
final = json.loads((controls / 'final.json').read_text())
assert final['source_unchanged'], 'controls source changed'
rows = {row['label']: row for row in final['results']}
assert rows['build-controls']['exit'] == 0 and rows['build-controls']['warnings'] == 0
listed = set(re.findall(r'^(.+): test$', (controls / 'controls-list.out').read_text(), re.M))
assert listed, 'no authenticated controls'
for mode in ['new', 'old']:
    assert rows[f'controls-{mode}']['exit'] == 0, f'{mode} report controls are not green'
    observed = dict(re.findall(r'^test (.+?) \.\.\. (ok|FAILED|ignored)$',
                               (controls / f'controls-{mode}.out').read_text(), re.M))
    assert set(observed) == listed and all(value == 'ok' for value in observed.values())
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
inputs = json.loads((controls / 'inputs.json').read_text())
realm_controls = args.realm_controls.resolve()
realm_inputs = json.loads((realm_controls / 'tests-compiler-build-inputs.json').read_text())
assert realm_inputs['head'] == inputs['head'], 'realm controls use another commit'
assert realm_inputs['rust_diff_sha256'] == inputs['rust_diff_sha256'], 'realm controls use another batch'
realm_name = 'object_types::logical_source_candidates::tests::full_report_candidates_require_the_actual_diagnostic_output_realm'
for mode in ['new', 'old']:
    output = (realm_controls / f'tests-compiler-lib-{mode}.out').read_text()
    actual = dict(re.findall(r'^test (.+?) \.\.\. (ok|FAILED|ignored)$', output, re.M))
    assert actual.get(realm_name) == 'ok', f'{mode} diagnostic-output realm control has not passed'
assert head == inputs['head'], 'controls use another source commit'
diff = subprocess.check_output(['git', 'diff', '--', 'rust'], cwd=root)
assert hashlib.sha256(diff).hexdigest() == inputs['rust_diff_sha256'], 'batch changed since controls'
tools = Path(__file__).resolve().parent
perf = tools.parent
evidence = args.evidence.resolve()
evidence.mkdir(parents=True, exist_ok=False)
rosters = {}
expected_hashes = json.loads((perf / 'evidence/origin-inputs.json').read_text())['inputs']
expected_hashes.update(json.loads((perf / 'evidence/corpus-roster-audit.json').read_text())['text_input_sha256'])
for name, expected in [('corpus.txt', 102), ('corpus_irules.txt', 15)]:
    paths = (perf / 'evidence' / name).read_text().splitlines()
    assert len(paths) == expected and len(set(paths)) == expected
    for relative in paths:
        (root / relative).read_text(encoding='utf-8')
        assert hashlib.sha256((root / relative).read_bytes()).hexdigest() == expected_hashes[relative], f'baseline input bytes changed: {relative}'
    rosters[name] = paths
sources = {'zz_authenticated_perfprobe': 'authenticated_perfprobe.rs.txt',
           'zz_complete_spectclprobe': 'complete_spectclprobe.rs.txt'}
examples = root / 'rust/tcl-compiler/examples'
examples.mkdir(exist_ok=True)
env = dict(os.environ, CARGO_INCREMENTAL='0', CARGO_PROFILE_DEV_DEBUG='0',
           CARGO_PROFILE_DEV_OPT_LEVEL='0', CARGO_PROFILE_TEST_DEBUG='0',
           CARGO_PROFILE_TEST_OPT_LEVEL='0', CARGO_TARGET_DIR=str(root / 'target'))
env.pop('ZZ_OLD_ENV', None)
artifacts = set()
created_sources = []
results = []
try:
    for name, source in sources.items():
        target = examples / (name + '.rs')
        assert not target.exists(), f'foreign temporary probe exists: {target}'
        target.write_bytes((tools / source).read_bytes())
        created_sources.append(target)
    receipt = {'head': head, 'rust_diff_sha256': hashlib.sha256(diff).hexdigest(),
               'profile': 'dev opt-level=0 debug=0 incremental=0; assertions unchanged',
               'controls_final_sha256': hashlib.sha256((controls / 'final.json').read_bytes()).hexdigest(),
               'rosters': rosters,
               'fixture_sha256': {p: hashlib.sha256((root / p).read_bytes()).hexdigest()
                                  for paths in rosters.values() for p in paths},
               'sources_sha256': {name: hashlib.sha256((tools / source).read_bytes()).hexdigest()
                                  for name, source in sources.items()}}
    (evidence / 'inputs.json').write_text(json.dumps(receipt, indent=2) + '\n')
    argv = ['cargo', 'build', '--locked', '-j2', '-p', 'tcl-compiler',
            '--message-format=json']
    for name in sources:
        argv += ['--example', name]
    with (evidence / 'build.jsonl').open('w') as out, (evidence / 'build.stderr').open('w') as err:
        code = subprocess.run(argv, cwd=root, env=env, stdout=out, stderr=err).returncode
    warnings = []
    exes = {}
    for line in (evidence / 'build.jsonl').read_text().splitlines():
        row = json.loads(line)
        if row.get('reason') == 'compiler-artifact' and row.get('executable'):
            p = Path(row['executable'])
            artifacts.add(p)
            exes[row['target']['name']] = p
        if row.get('reason') == 'compiler-message' and row['message']['level'] == 'warning':
            warnings.append(row['message'])
    assert code == 0 and not warnings, f'build exit={code}, warnings={len(warnings)}'
    assert set(sources) <= set(exes)
    for mode in ['old-only', 'mixed']:
        mode_env = env | ({'ZZ_OLD_ENV': '1'} if mode == 'old-only' else {})
        cases = [('tcl', ['files', *rosters['corpus.txt']], {}, 102),
                 ('irules', ['files', *rosters['corpus_irules.txt']], {'ZZ_DIALECT': 'f5-irules'}, 15),
                 ('calls', ['calls', '10,20,40,80'], {}, 4),
                 ('procs', ['procs', '41'], {}, 1),
                 ('upvar', ['upvar', '10,30'], {}, 2)]
        for label, arguments, extra, count in cases:
            log = evidence / f'{mode}-{label}.out'
            start = time.monotonic()
            with log.open('w') as out:
                code = subprocess.run([str(exes['zz_authenticated_perfprobe']), *arguments],
                                      cwd=root, env=mode_env | extra, stdout=out,
                                      stderr=subprocess.STDOUT).returncode
            output = log.read_text()
            valid = (code == 0 and output.count('REPORT label=') == count
                     and output.count(' pass=true') == count
                     and f'REPORT_SUMMARY inputs={count} failed=0' in output)
            results.append({'mode': mode, 'case': label, 'exit': code, 'valid': valid,
                            'expected': count, 'wall_s': time.monotonic() - start})
            (evidence / 'progress.json').write_text(json.dumps(results, indent=2) + '\n')
            print(mode, label, results[-1], flush=True)
    start = time.monotonic()
    with (evidence / 'packset-threeway.out').open('w') as out:
        code = subprocess.run([str(exes['zz_complete_spectclprobe'])], cwd=root, env=env,
                              stdout=out, stderr=subprocess.STDOUT).returncode
    results.append({'case': 'packset', 'exit': code, 'valid': code == 0,
                    'wall_s': time.monotonic() - start})
    assert all(row['valid'] for row in results), 'complete report/PackSet proof failed'
finally:
    for p in artifacts:
        p.unlink(missing_ok=True)
        for alias in p.parent.glob(p.name + '-*'):
            if alias.is_file() and os.access(alias, os.X_OK):
                alias.unlink()
    for target in created_sources:
        target.unlink(missing_ok=True)
    current_diff = subprocess.check_output(['git', 'diff', '--', 'rust'], cwd=root)
    (evidence / 'final.json').write_text(json.dumps({
        'results': results, 'rust_diff_unchanged': current_diff == diff,
        'executables_removed': all(not p.exists() for p in artifacts)}, indent=2) + '\n')
