import collections, hashlib, json
from pathlib import Path

root = Path('/workspace/naming-migration-active/tcl-lsp')
base = root / 'docs/work/name-handling/handoff/reviews/merge-inventory-20261019/ci-5201'
coord = root / 'docs/work/name-handling/handoff/reviews/merge-inventory-20261019/coordinator-20261022'
job_ids = [119285770088, 119285770029, 119285770151]

# CI-5201 text identity forms observed in the failure inventories:
#   FAIL <crate> <lib selector...>
#   FAIL <crate>::<integration-binary> <test selector...>
def parse_baseline(path):
    rows = []
    for lineno, raw in enumerate(path.read_text(errors='replace').splitlines(), 1):
        parts = raw.split()
        if not parts or parts[0] != 'FAIL' or len(parts) < 3:
            continue
        target, selector = parts[1], ' '.join(parts[2:])
        if '::' in target:
            package, binary = target.split('::', 1)
        else:
            package, binary = target, 'lib'
        rows.append({'package': package, 'binary': binary, 'selector': selector,
                     'source': path.name, 'line': lineno, 'raw': raw})
    return rows

baseline_files = sorted(p for p in base.glob('*.txt') if p.name != 'by-crate.txt')
baseline = [row for f in baseline_files for row in parse_baseline(f)]
base_keys = {(r['package'], r['binary'], r['selector']): r for r in baseline}
by_package = collections.Counter(r['package'] for r in baseline)
current = []
job_summaries = []
for jid in job_ids:
    obj = json.loads((coord / f'job-{jid}-outcomes.json').read_text())
    cnt = collections.Counter(row['outcome'] for row in obj['outcomes'])
    job_summaries.append({'job_id': jid, 'job_conclusion': obj['job_conclusion'],
                          'coverage': obj['coverage'], 'counts': dict(cnt),
                          'outcome_records': len(obj['outcomes'])})
    for row in obj['outcomes']:
        current.append({'job_id': jid, 'package': row['binary'], 'binary': row['binary'],
                        'selector': row['selector'], 'outcome': row['outcome']})

exact_matches = []
for row in current:
    key = (row['package'], row['binary'], row['selector'])
    if key in base_keys:
        exact_matches.append({'current': row, 'baseline': base_keys[key]})
current_counts = collections.Counter(row['outcome'] for row in current)
current_failure_keys = {(r['package'], r['binary'], r['selector']) for r in current if r['outcome']=='FAIL'}
current_pass_keys = {(r['package'], r['binary'], r['selector']) for r in current if r['outcome']=='PASS'}

source_hashes = []
for f in baseline_files:
    data = f.read_bytes()
    source_hashes.append({'file': f.name, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest(),
                          'failure_lines': sum(1 for line in data.splitlines() if line.startswith(b'FAIL '))})
outcome_hashes = []
for jid in job_ids:
    f = coord / f'job-{jid}-outcomes.json'
    data = f.read_bytes()
    outcome_hashes.append({'file': f.name, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
by_crate = base / 'by-crate.txt'
crate_rows = []
for line in by_crate.read_text().splitlines():
    fields=line.split()
    if len(fields)==2 and fields[0].isdigit(): crate_rows.append((fields[1],int(fields[0])))
by_crate_total = sum(n for _, n in crate_rows)
failures = [r for r in current if r['outcome']=='FAIL']
receipt = {
    'comparison': 'observed e2e partition results against recorded CI-5201 failed identities',
    'identity_rule': 'exact package + test-binary + selector; no substring or selector-only joins',
    'current_source_run': 39616304947,
    'current_jobs': job_summaries,
    'current_outcome_count': len(current),
    'current_counts': dict(current_counts),
    'current_unique_failure_identities': len(current_failure_keys),
    'baseline_reported_failure_count_in_current_state': 3634,
    'baseline_inventory_failure_lines': len(baseline),
    'baseline_by_crate_summary_total': by_crate_total,
    'baseline_by_crate_summary_sha256': hashlib.sha256(by_crate.read_bytes()).hexdigest(),
    'baseline_files': source_hashes,
    'current_outcome_files': outcome_hashes,
    'exact_matches': exact_matches,
    'exact_match_count': len(exact_matches),
    'current_failures_exactly_matching_recorded_baseline_failures': sum(1 for k in current_failure_keys if k in base_keys),
    'current_passes_exactly_matching_recorded_baseline_failures': sum(1 for k in current_pass_keys if k in base_keys),
    'unmatched_current_failures': failures,
    'coverage_limitations': [
        'The CI-5201 files are failure-only lists, not complete test inventories.',
        'There is no README in the ci-5201 directory or its merge-inventory parent; identity format was read directly from the failure files.',
        'The supplied files are shard1, shard3, shard4, shard5, and heavy-failures; shard2 is absent.',
        'The supplied failure lines total 3605, the by-crate summary totals 3607, and current-state reports 3634. The 27–29 missing/discordant rows are not recoverable from these files.',
        'All 71 current failures use package/test binary tcl-lsp-server, absent from the recorded package list; no exact match appears in available failure inventories.',
        'No exact match is not evidence that these selectors passed in CI-5201. Missing and unexecuted selectors remain unknown; no zero pass-to-fail conclusion follows.'
    ]
}
(coord / 'ci-5201-partial-comparison.json').write_text(json.dumps(receipt, indent=2) + '\n')
print(json.dumps({k: receipt[k] for k in ['current_outcome_count','current_counts','current_unique_failure_identities','baseline_reported_failure_count_in_current_state','baseline_inventory_failure_lines','baseline_by_crate_summary_total','exact_match_count','coverage_limitations']}, indent=2))
