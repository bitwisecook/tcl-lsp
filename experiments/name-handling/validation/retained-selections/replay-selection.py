#!/usr/bin/env python3
"""Capture an exact retained selection against explicitly supplied fresh artifacts.

This adapter performs no build, Git, provider fetch or ordinary-source admission.
Root's independent current source/provider/compiled-input receipts remain required.
"""
import argparse
import collections
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument('--manifest', required=True)
parser.add_argument('--manifest-sha256', required=True)
parser.add_argument('--artifacts', required=True)
parser.add_argument('--root', required=True)
parser.add_argument('--out', required=True)
parser.add_argument('--group', help='Capture one explicit target; never report the full group complete.')
parser.add_argument('--list-only', action='store_true')
args = parser.parse_args()
root = Path(args.root).resolve()
out = Path(args.out).resolve()
manifest_path = Path(args.manifest)

def sha(path):
    with Path(path).open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256')
    return digest.hexdigest()

assert sha(manifest_path) == args.manifest_sha256, 'Selection manifest digest mismatch'
manifest = json.loads(manifest_path.read_text())
artifacts = json.loads(Path(args.artifacts).read_text())
groups = manifest['groups']
assert sum(map(len, groups.values())) == manifest['total_exact_selectors']
assert {key: len(value) for key, value in groups.items()} == manifest['expected_group_counts']
assert all(names and len(names) == len(set(names)) for names in groups.values())
order = manifest['execution_order']
assert collections.Counter((row['target'], row['selector']) for row in order) == collections.Counter((key, name) for key, names in groups.items() for name in names)
if args.group:
    assert args.group in groups, 'Unknown explicit subgroup'
    groups = {args.group: groups[args.group]}
    order = [row for row in order if row['target'] == args.group]
assert set(groups).issubset(artifacts)
assert manifest['test_timeout_seconds'] == 180
assert manifest['listing_timeout_seconds'] == 30
env = os.environ.copy()
assert not [key for key, value in env.items() if value and (key.startswith('TCL_LSP_TRACE') or key == 'TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC')], 'Diagnostic environment is not ordinary replay'
for key in manifest.get('environment', {}).get('unset', []):
    env.pop(key, None)
for key, value in manifest.get('environment', {}).get('set', {}).items():
    expected = value.replace('{root}', str(root))
    if key.startswith('TCL_LSP_') and key in env:
        assert env[key] == expected, 'Foreign provider environment: ' + key
    env[key] = expected
for target in groups:
    item = artifacts[target]
    assert Path(item['path']).is_file() and sha(item['path']) == item['sha256'], 'Artifact binding mismatch: ' + target
out.mkdir(parents=True, exist_ok=False)
result = {'status': 'SELECTION_CAPTURE_ONLY_NO_ORDINARY_ACCEPTANCE', 'selection': manifest['group'], 'manifest_sha256': args.manifest_sha256, 'partial_target': args.group, 'expected_selected_count': len(order), 'full_group_expected_count': manifest['total_exact_selectors'], 'artifacts': {key: artifacts[key] for key in groups}, 'environment': {key: value for key, value in env.items() if key.startswith('TCL_LSP_') or key in ['TCL_LIBRARY', 'TCLLIBPATH']}, 'runs': [], 'errors': []}

def save():
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')

def capture(command, log, timeout):
    started = time.monotonic()
    timed_out = False
    with log.open('wb') as stream:
        process = subprocess.Popen(command, cwd=root, env=env, stdin=subprocess.DEVNULL, stdout=stream, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
    return {'argv': command, 'exit_code': process.returncode, 'timed_out': timed_out, 'seconds': time.monotonic() - started, 'log': log.name, 'log_sha256': sha(log)}

try:
    for target, names in groups.items():
        log = out / (target + '.list')
        listed = capture([artifacts[target]['path'], '--list'], log, 30)
        assert listed['exit_code'] == 0 and not listed['timed_out'], 'Listing failed: ' + target
        counts = collections.Counter(line.removesuffix(': test') for line in log.read_text().splitlines() if line.endswith(': test'))
        assert counts and all(counts[name] == 1 for name in names), 'Missing or duplicate exact selector: ' + target
    if not args.list_only:
        for index, row in enumerate(order, 1):
            target, name = row['target'], row['selector']
            log = out / ('test-%03d.log' % index)
            item = capture([artifacts[target]['path'], name, '--exact', '--nocapture', '--test-threads=1'], log, 180)
            running, summaries, previous = [], [], ''
            with log.open('rt', errors='replace') as stream:
                while chunk := stream.read(65536):
                    window = previous + chunk
                    boundary = len(previous)
                    running.extend(match.group(1) for match in re.finditer(r'running (\d+) tests?', window) if match.end() > boundary)
                    summaries.extend(list(match.groups()) for match in re.finditer(r'test result: (\w+)\. (\d+) passed; (\d+) failed;', window) if match.end() > boundary)
                    previous = chunk[-256:]
            item.update(target=target, selector=name, running_counts=running, summaries=summaries)
            item['test_passed'] = item['exit_code'] == 0 and not item['timed_out'] and running == ['1'] and summaries == [['ok', '1', '0']]
            result['runs'].append(item)
            save()
except Exception as error:
    result['errors'].append(repr(error))
finally:
    result['artifact_drift'] = {key: {'expected': item['sha256'], 'actual': sha(item['path']) if Path(item['path']).is_file() else None} for key, item in result['artifacts'].items() if not Path(item['path']).is_file() or sha(item['path']) != item['sha256']}
    result['listed_only'] = args.list_only
    result['selected_vector_complete'] = not args.list_only and not result['errors'] and not result['artifact_drift'] and len(result['runs']) == len(order)
    result['full_group_complete'] = result['selected_vector_complete'] and not args.group
    result['passed_count'] = sum(row['test_passed'] for row in result['runs'])
    result['failed_count'] = len(result['runs']) - result['passed_count']
    save()
print(json.dumps({key: result[key] for key in ['selection', 'partial_target', 'full_group_complete', 'passed_count', 'failed_count', 'errors']}, indent=2))
raise SystemExit(0 if not result['errors'] and not result['artifact_drift'] and (args.list_only or result['selected_vector_complete'] and result['failed_count'] == 0) else 1)
