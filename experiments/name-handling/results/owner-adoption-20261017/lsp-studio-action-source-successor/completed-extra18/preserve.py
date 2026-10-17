from pathlib import Path
import collections, datetime, hashlib, json, shutil, subprocess, tarfile

r = Path('/workspace/tcl-lsp')
b = Path('/workspace/naming-validation/lsp-studio-action-source-successor-repair3')
out = r / 'experiments/name-handling/results/owner-adoption-20261017/lsp-studio-action-source-successor'
d = out / 'completed-extra18'
d.mkdir(exist_ok=False)
sha = lambda p: hashlib.file_digest(Path(p).open('rb'), 'sha256').hexdigest()
read = lambda p: json.loads(Path(p).read_text())
inv = read(b / 'extra18-results-invocation.json')
assert inv['preflight'] == inv['postflight'] and inv['attempted_groups'] == 4
runs, errors = [], []
for p in (b / 'extra18-results').glob('*/result.json'):
    j = read(p)
    assert not j['artifact_drift'] and j['selected_vector_complete']
    runs.extend(j['runs']); errors.extend(j['errors'])
assert len(runs) == 18 and not errors
outcome = lambda v: 'TIMEOUT' if v['timed_out'] else 'PASS' if v['test_passed'] else 'FAIL'
previous = {}
for p in Path('/workspace/naming-validation/lsp-retained-report-action-source-successor/extra-library9-results').glob('*/result.json'):
    for v in read(p)['runs']:
        previous[v['target'], v['selector']] = v
assert len(previous) == 9
retained = [v for v in runs if (v['target'], v['selector']) in previous]
assert len(retained) == 9
changes = [{'target': v['target'], 'selector': v['selector'], 'before': outcome(previous[v['target'], v['selector']]), 'after': outcome(v)} for v in retained if outcome(v) != outcome(previous[v['target'], v['selector']])]
summary = {
    'selected': 18, 'counts': dict(collections.Counter(map(outcome, runs))),
    'by_target': {target: dict(collections.Counter(outcome(v) for v in runs if v['target'] == target)) for target in ['tcl_lexer', 'tcl_syntax', 'tcl_spectcl', 'xtask']},
    'harness_errors': errors, 'actual_wrapper_exit_code': inv['wrapper_exit_code'],
    'completed_utc': inv['completed_utc'], 'all39052_inputs_nine_ELFs_providers_postflight_match': True,
    'retained_extra9_status_changes': changes,
    'retained_extra9_prior_PASS_regressions': [v for v in changes if v['before'] == 'PASS'],
    'old_extra9_regression_remains_unresolved': 'cache::tests::original_loader_notice_sites_survive_cold_and_disk_warm_cache_without_serialised_geometry',
    'all_three_previously_owed_xtask_controls': 'PASS',
    'ordinary_acceptance': False,
    'limits': 'Two SpecTcl failures retain original assertions and Native/source guards. Standard962 is separately running; repository and host gates remain unaccepted.'
}
(d / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
shutil.copy2(b / 'extra18-results-invocation.json', d / 'invocation.json')
files = sorted(p for p in (b / 'extra18-results').rglob('*') if p.is_file())
expected = {str(p.relative_to(b / 'extra18-results')): sha(p) for p in files}
assert sum(p.name.startswith('test-') and p.suffix == '.log' for p in files) == 18
archive = d / 'raw-results-and-logs.tar.zst'
with archive.open('wb') as stream:
    proc = subprocess.Popen(['zstd', '-q', '-9', '-c'], stdin=subprocess.PIPE, stdout=stream)
    with tarfile.open(fileobj=proc.stdin, mode='w|') as tar:
        for p in files:
            tar.add(p, arcname=str(p.relative_to(b / 'extra18-results')), recursive=False)
    proc.stdin.close(); assert proc.wait() == 0
proc = subprocess.Popen(['zstd', '-q', '-dc', str(archive)], stdout=subprocess.PIPE)
seen = set()
with tarfile.open(fileobj=proc.stdout, mode='r|') as tar:
    for m in tar:
        assert m.isfile() and m.name in expected and m.name not in seen
        assert hashlib.file_digest(tar.extractfile(m), 'sha256').hexdigest() == expected[m.name]
        seen.add(m.name)
assert proc.wait() == 0 and seen == set(expected)
(d / 'raw-archive.json').write_text(json.dumps({'sha256': sha(archive), 'bytes': archive.stat().st_size, 'members': expected}, indent=2) + '\n')

def copy(p, dest):
    dest = out / dest
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(p, dest)
    assert sha(p) == sha(dest)

for name in ['source-binding.json', 'build.log', 'artifacts.json']:
    copy(b / 'extra-xtask' / name, 'native-extra-xtask-PASS/' + name)
for p in (b / 'extra18-bound-v1').iterdir():
    assert p.suffix == '.json'
    copy(p, 'extra18-bound/' + p.name)
package = Path('/workspace/naming-validation/studio-action-extra18-source-v2')
for p in package.iterdir():
    if p.is_file():
        copy(p, 'extra18-reviewed-source/' + p.name)
copy('/workspace/naming-validation/root-reviews/extra18-source-v2/independent-source-review-CLEAR.txt', 'extra18-reviewed-source/root-independent-source-CLEAR.txt')
copy('/tmp/naming-build-studio-action-extra-v1.py', 'native-extra-build.py')
copy(__file__, 'completed-extra18/preserve.py')
print(json.dumps(summary, indent=2))
