#!/usr/bin/env python3
"""Root-run evidence capture only. --check reads/validates and writes nothing.

Copies original text evidence, never executes a test/build/Git/network command,
never imports an ELF/full input index and never updates audit/count/status files.
"""
from pathlib import Path
import argparse
import collections
import datetime
import hashlib
import json
import shutil
import subprocess

ROOT = Path('/workspace/tcl-lsp')
VALIDATION = Path('/workspace/naming-validation')
DESTINATION = ROOT / 'experiments/name-handling/results/owner-adoption-20261016'
HERE = Path(__file__).resolve().parent
INPUTS = HERE / 'naming-capture-e0612-inputs.json'
RUNS = ('lsp-source-path-diagnostics-repair', 'lsp-spectcl-e0612')
PREVIOUS57 = [VALIDATION / 'lsp-reviewed-82e016-retry4/results/result.json']
PREVIOUS120 = [VALIDATION / 'lsp-libs-a49f3' / lane / 'result.json'
               for lane in ('retained65', 'delta', 'new10')]


def digest(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def read_json(path):
    return json.loads(Path(path).read_text())


def identity(run):
    return run['target'], run['selector']


def outcome(run):
    if run.get('timed_out'):
        return 'TIMEOUT'
    if run.get('test_passed'):
        return 'PASS'
    return 'FAIL'


def selected_runs(paths):
    result = {}
    origins = []
    for path in paths:
        data = read_json(path)
        assert data.get('selected_vector_complete') is True, path
        assert not data.get('listed_only'), path
        assert not data.get('errors'), path
        assert not data.get('artifact_drift'), path
        origins.append({'path': str(path), 'sha256': digest(path),
                        'runs': len(data['runs'])})
        for run in data['runs']:
            key = identity(run)
            assert key not in result, ('duplicate selected identity', key)
            result[key] = run
    return result, origins


def exact_rows(keys, previous, current):
    return [{'target': key[0], 'selector': key[1],
             'previous': outcome(previous[key]), 'current': outcome(current[key]),
             'previous_log': previous[key]['log'], 'current_log': current[key]['log']}
            for key in sorted(keys)]


def compare(paths, current, expected_count):
    previous, origins = selected_runs(paths)
    assert len(previous) == expected_count
    common = previous.keys() & current.keys()
    missing = previous.keys() - current.keys()
    gains = {key for key in common
             if outcome(previous[key]) != 'PASS' and outcome(current[key]) == 'PASS'}
    regressions = {key for key in common
                   if outcome(previous[key]) == 'PASS' and outcome(current[key]) != 'PASS'}
    changes = {key for key in common if outcome(previous[key]) != outcome(current[key])}
    return {'source_results': origins, 'previous_count': len(previous),
            'previous_outcomes': dict(collections.Counter(map(outcome, previous.values()))),
            'retained_count': len(common),
            'current_retained_outcomes': dict(collections.Counter(outcome(current[k]) for k in common)),
            'missing': [{'target': k[0], 'selector': k[1]} for k in sorted(missing)],
            'gains': exact_rows(gains, previous, current),
            'previous_PASS_regressions': exact_rows(regressions, previous, current),
            'all_outcome_changes': exact_rows(changes, previous, current)}


def preflight():
    bindings = read_json(INPUTS)
    for item in bindings['files']:
        path = Path(item['path'])
        assert path.is_file() and not path.is_symlink(), path
        assert path.stat().st_size == item['bytes'], ('size drift', path)
        assert digest(path) == item['sha256'], ('source evidence drift', path)
        with path.open('rb') as handle:
            assert handle.read(4) != b'\x7fELF', ('ELF forbidden', path)
        assert path.name != 'input-before.json', ('full input index forbidden', path)
    equivalence = read_json('/tmp/naming-e0612-source-equivalence.json')
    source = VALIDATION / RUNS[0]
    binding = read_json(source / 'source-binding.json')
    assert equivalence['byte_equal'] is True
    assert equivalence['sha256'] == digest(source / 'source.diff') == binding['rust_diff_sha256']
    assert equivalence['base'] == binding['source_head']
    assert binding['exit_code'] == 0 and binding['input_drift_count'] == 0
    assert binding['input_before_count'] == binding['input_after_count'] == 38998
    summaries = {}
    for name, expected_count in zip(RUNS, (220, 3)):
        directory = VALIDATION / name
        data = read_json(directory / 'results/result.json')
        selected, _ = selected_runs([directory / 'results/result.json'])
        assert len(selected) == expected_count == data['expected_selected_count']
        assert data['manifest_sha256'] == digest(directory / 'selection.json')
        assert data['full_group_complete'] is True
        assert data['artifacts'] == read_json(directory / 'artifacts.json')
        for run in selected.values():
            assert run['log_sha256'] == digest(directory / 'results' / run['log'])
            assert run.get('running_counts') == ['1'], ('not a singleton', identity(run))
        counts = collections.Counter(map(outcome, selected.values()))
        assert data['passed_count'] == counts['PASS']
        assert data['failed_count'] == counts['FAIL'] + counts['TIMEOUT']
        summaries[name] = {'selected': len(selected), **dict(counts),
                           'singleton_rejections': 0,
                           'failed_and_timeout_selectors': [
                               {'target': r['target'], 'selector': r['selector'],
                                'outcome': outcome(r), 'exit_code': r['exit_code'],
                                'seconds': r['seconds'], 'log': r['log']}
                               for r in selected.values() if outcome(r) != 'PASS']}
    current, _ = selected_runs([source / 'results/result.json'])
    comparisons = {'retained57': compare(PREVIOUS57, current, 57),
                   'retained120': compare(PREVIOUS120, current, 120)}
    old57, _ = selected_runs(PREVIOUS57)
    old120, _ = selected_runs(PREVIOUS120)
    overlap = old57.keys() & old120.keys()
    previous_union = old57.keys() | old120.keys()
    new = current.keys() - previous_union
    comparisons['overlap_note'] = {
        'overlapping_selectors': len(overlap), 'union_unique_selectors': len(previous_union),
        'predecessor_outcomes_differ': [
            {'target': k[0], 'selector': k[1], 'previous57': outcome(old57[k]),
             'previous120': outcome(old120[k]), 'current': outcome(current[k])}
            for k in sorted(overlap) if outcome(old57[k]) != outcome(old120[k])],
        'rule': 'Compare each genuine source variant separately. Do not merge conflicting historical outcomes.'}
    comparisons['additional_to_both'] = {
        'selected': len(new), 'outcomes': dict(collections.Counter(outcome(current[k]) for k in new)),
        'selectors': [{'target': k[0], 'selector': k[1], 'outcome': outcome(current[k]),
                       'log': current[k]['log']} for k in sorted(new)]}
    owner_log = (source / 'owner-resolution.log').read_text()
    owner_errors = [line[4:] for line in owner_log.splitlines() if line.startswith('  - ')]
    assert len(owner_errors) == 309
    return bindings, summaries, comparisons, equivalence, owner_errors


def copy_exact(source, target, receipts):
    source, target = Path(source), Path(target)
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.exists():
        assert digest(target) == digest(source), ('refuse overwrite of different durable evidence', target)
    else:
        shutil.copy2(source, target)
    assert digest(target) == digest(source)
    receipts.append({'source': str(source), 'stored': str(target.relative_to(DESTINATION)),
                     'sha256': digest(target), 'bytes': target.stat().st_size})


def save_json(path, value):
    encoded = json.dumps(value, indent=2, ensure_ascii=False) + '\n'
    path = Path(path)
    if path.exists():
        assert path.read_text() == encoded, ('refuse different generated durable evidence', path)
    else:
        path.write_text(encoded)


def capture(bindings, summaries, comparisons, equivalence, owner_errors):
    copied = []
    for name in RUNS:
        source, destination = VALIDATION / name, DESTINATION / name
        destination.mkdir(parents=True, exist_ok=True)
        results = destination / 'results'
        results.mkdir(exist_ok=True)
        listing_storage = []
        for entry in bindings['files']:
            path = Path(entry['path'])
            if source not in path.parents:
                continue
            relative = path.relative_to(source)
            target = destination / relative
            if path.suffix != '.list':
                copy_exact(path, target, copied)
                continue
            compressed = target.with_suffix(target.suffix + '.zst')
            if not compressed.exists():
                with compressed.open('xb') as output:
                    subprocess.run(['zstd', '-q', '-3', '--stdout', str(path)], stdout=output, check=True)
            restored = subprocess.check_output(['zstd', '-dc', str(compressed)])
            assert hashlib.sha256(restored).hexdigest() == entry['sha256']
            assert len(restored) == entry['bytes']
            listing_storage.append({'path': path.name, 'sha256': entry['sha256'],
                                    'bytes': len(restored), 'stored': compressed.name,
                                    'stored_sha256': digest(compressed),
                                    'stored_bytes': compressed.stat().st_size,
                                    'roundtrip_verified': True})
        save_json(results / 'listing-storage.json', {
            'lossless': True, 'members': listing_storage,
            'scope': 'Exact original singleton listing bytes only. Compression grants no new execution acceptance.'})
        save_json(destination / 'summary.json', {
            'capture_status': 'COMPLETED_ORIGINAL_BOUNDED_EXECUTION_EVIDENCE',
            'actual_source_binding': read_json(source / 'source-binding.json'),
            'result_status_retained': read_json(source / 'results/result.json')['status'],
            'execution_counts': summaries[name],
            'source_equivalence': equivalence if name == RUNS[0] else {
                'equivalent_commit': equivalence['equivalent_commit'],
                'basis': 'Separate SpecTcl source-binding.json records unchanged immediately preceding five-library input; separate actual artifact/build command.'},
            'limits': 'Frozen bounded native Linux controls only. No full repository/issue/Native/runtime/VM/Bigip acceptance; no later cache/arity/Logical-prefix repairs in these frozen binaries. Actual Windows/WASI execution remains separately required. Standard push-only protocol remains unchanged.'})
    main = DESTINATION / RUNS[0]
    for source, relative in [
        (Path('/tmp/naming-build-source-path-diagnostics-repair.py'), 'executed-build-harness.py'),
        (Path('/tmp/naming-prepare-input-replay.py'), 'executed-selection-preparation.py'),
        (ROOT / 'experiments/name-handling/validation/retained-selections/replay-selection.py', 'replay-selection-harness.py'),
        (Path('/tmp/naming-e0612-source-equivalence.json'), 'source-equivalence.json'),
        (Path(__file__).resolve(), 'capture-evidence.py'),
        (INPUTS, 'capture-input-bindings.json')]:
        copy_exact(source, main / relative, copied)
    save_json(main / 'retained-comparisons.json', comparisons)
    save_json(main / 'owner-resolution-summary.json', {
        'status': 'FAIL', 'diagnostic_count': len(owner_errors), 'diagnostics': owner_errors,
        'log': 'owner-resolution.log',
        'source_variant': 'Separate later actual xtask owner-resolution build/run after ordinary five-library freeze; exact text log retained. No gate success claimed and no paused Runtime/VM migration acceptance.'})
    save_json(main / 'additional-checks.json', {
        'checks': [
            {'logs': ['examples-check.log', 'examples-check-repair.log', 'examples-check-repair2.log'],
             'outcomes': ['FAIL E0599 commands().registry()', 'FAIL E0425 missing line_starts/byte_to_line_col', 'PASS compile-check'],
             'scope': 'Actual example compilation only; two subsequent source/import repairs. These repaired sources are separate from the frozen native220 artifact; no example functional-output execution is inferred.'},
            {'log': 'zed-query-check.log', 'outcome': 'PASS generator sync check',
             'scope': 'Actual Registry generated outputs. Tcl/Expect baseline purposes only; incidental shared iApps/iRules projections grant no paused Bigip migration acceptance.'},
            {'logs': ['browser-check.log', 'browser-fetch.log', 'browser-check-ready.log', 'browser-check-abi-repair.log'],
             'outcomes': ['FAIL offline missing console_error_panic_hook', 'dependency fetch/lock changes, not a check pass', 'FAIL E0425 libc c_int/c_long target availability', 'PASS compile-check with one Server dead_code warning'],
             'actual_command_reported_by_Root': 'source /tmp/naming-env.sh; source scripts/dev/wasm-cc-env.sh; export WASI_SDK_PATH=/workspace/.toolchains/wasi-sdk-34.0; wasm_cc_prepare; CARGO_TARGET_DIR=/workspace/tcl-lsp/rust/tcl-lsp-server-wasm/target cargo check --offline --locked --manifest-path rust/tcl-lsp-server-wasm/Cargo.toml --target wasm32-unknown-unknown',
             'command_basis': 'Root supplied exact executed command. Environment file is intentionally not imported; no automated whole-source receipt exists for this check.',
             'scope': 'Separate browser/WASM compilation after dependency readiness and ABI source repair; not the frozen native220 source variant and not actual browser or WASI runtime execution.'}],
        'source_binding_limit': 'Supplemental logs do not themselves include a full per-check input-index/source receipt. Exact original text and hashes are retained; commands/source variants are not silently relabeled as the native e0612 frozen artifact. Later cache/arity/prefix repairs require fresh ordinary validation.'})
    for name in ('five-library-cd0', 'lsp-source-parent-continuation'):
        source = VALIDATION / name / 'frozen-storage-archive.json'
        if source.exists():
            copy_exact(source, DESTINATION / name / source.name, copied)
    save_json(main / 'capture-storage-receipt.json', {
        'copied': copied, 'omitted': ['ELF/frozen-binaries', 'full input-before.json index', 'archives'],
        'no_execution': True, 'no_audit_count_updates': True,
        'listing_storage': ['results/listing-storage.json', '../lsp-spectcl-e0612/results/listing-storage.json']})
    print(json.dumps({'destinations': [str(DESTINATION / name) for name in RUNS],
                      'summaries': summaries, 'copy_receipts': len(copied)}, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='Read-only preflight and actual computed summary; no durable writes.')
    args = parser.parse_args()
    values = preflight()
    if args.check:
        _, summaries, comparisons, _, _ = values
        print(json.dumps({'preflight': 'PASS', 'summaries': summaries,
                          'retained_comparisons': comparisons}, indent=2))
    else:
        capture(*values)


if __name__ == '__main__':
    main()
