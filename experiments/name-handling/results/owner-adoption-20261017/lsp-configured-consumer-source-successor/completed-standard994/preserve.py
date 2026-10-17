from pathlib import Path
import collections, hashlib, json, shutil, subprocess, tarfile

r = Path('/workspace/tcl-lsp')
b = Path('/workspace/naming-validation/lsp-configured-consumer-source-successor-repair2')
capsule = r / 'experiments/name-handling/results/owner-adoption-20261017/lsp-configured-consumer-source-successor'
out = capsule / 'completed-standard994'
read = lambda p: json.loads(Path(p).read_text())
sha = lambda p: hashlib.file_digest(Path(p).open('rb'), 'sha256').hexdigest()
inv = read(b / 'retained994-results-invocation.json')
result = read(b / 'retained994-results/result.json')
runs = result['runs']
assert len(runs) == 994 and inv.get('completed_utc')
assert inv['preflight'] == inv['postflight'] and not inv.get('postflight_error')
assert not result['errors'] and not result['artifact_drift'] and result['selected_vector_complete']
old = read('/workspace/naming-validation/lsp-studio-action-source-successor-repair3/retained962-results/result.json')['runs']
assert len(old) == 962
assert [(v['target'], v['selector']) for v in runs[:962]] == [(v['target'], v['selector']) for v in old]
status = lambda v: 'TIMEOUT' if v['timed_out'] else 'PASS' if v['test_passed'] else 'FAIL'
changes = [{'target': before['target'], 'selector': before['selector'], 'before': status(before), 'after': status(after)} for before, after in zip(old, runs[:962]) if status(before) != status(after)]
summary = {
    'selected': 994, 'counts': dict(collections.Counter(map(status, runs))),
    'retained962_counts': dict(collections.Counter(map(status, runs[:962]))),
    'new32_counts': dict(collections.Counter(map(status, runs[962:]))),
    'retained962_status_changes': changes, 'prior_PASS_regressions': [v for v in changes if v['before'] == 'PASS'],
    'harness_errors': result['errors'], 'artifact_drift': result['artifact_drift'],
    'actual_wrapper_exit_code': inv['wrapper_exit_code'], 'actual_invocation_status': inv['status'], 'completed_utc': inv['completed_utc'],
    'all39058_inputs_five_ELFs_providers_final_pre_postflight_match': True,
    'continuous_source_freeze': True,
    'historical962_continuous_source_freeze': False,
    'historical962_exception': 'Previous962 transient two-file edit/restoration retained in its immutable capsule; current repair2 run is a separate source freeze.',
    'new_owner_control': next(v for v in runs if 'source_owners_reach_nested_spectcl' in v['selector']),
    'focused2': 'Actual repaired nested parent transport and Studio retirement retry both passed beforefull994; final fullselection outcomes recorded here.',
    'ordinary_acceptance': False,
    'limits': 'Actual outcomes against pinned current ELFs and final source/provider checks, all current source/provider/artifact postflights equal. Historical962 exception retained separately. Global/host/integration/required gates not accepted; no verified completion.'
}
out.mkdir(exist_ok=False)
(out / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
shutil.copy2(b / 'retained994-results-invocation.json', out / 'invocation.json')
files = sorted(p for p in (b / 'retained994-results').rglob('*') if p.is_file())
expected = {str(p.relative_to(b / 'retained994-results')): sha(p) for p in files}
assert sum(p.name.startswith('test-') and p.suffix == '.log' for p in files) == 994
archive = out / 'raw-results-and-logs.tar.zst'
with archive.open('wb') as stream:
    proc = subprocess.Popen(['zstd', '-q', '-9', '-c'], stdin=subprocess.PIPE, stdout=stream)
    with tarfile.open(fileobj=proc.stdin, mode='w|') as tar:
        for p in files: tar.add(p, arcname=str(p.relative_to(b / 'retained994-results')), recursive=False)
    proc.stdin.close(); assert proc.wait() == 0
proc = subprocess.Popen(['zstd', '-q', '-dc', str(archive)], stdout=subprocess.PIPE)
seen = set()
with tarfile.open(fileobj=proc.stdout, mode='r|') as tar:
    for member in tar:
        assert member.isfile() and member.name in expected and member.name not in seen
        assert hashlib.file_digest(tar.extractfile(member), 'sha256').hexdigest() == expected[member.name]
        seen.add(member.name)
assert proc.wait() == 0 and seen == set(expected)
(out / 'raw-archive.json').write_text(json.dumps({'sha256': sha(archive), 'bytes': archive.stat().st_size, 'members': expected}, indent=2) + '\n')
shutil.copy2(__file__, out / 'preserve.py')
print(json.dumps({k: summary[k] for k in ['selected', 'counts', 'retained962_counts', 'new32_counts', 'retained962_status_changes', 'actual_wrapper_exit_code', 'completed_utc', 'continuous_source_freeze']}, indent=2))
