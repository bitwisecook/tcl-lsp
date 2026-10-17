"""Read-only source/artifact joins shared by library9 binder and replay wrapper."""
from pathlib import Path
import hashlib
import importlib.util
import json
import sys

ROOT = Path('/workspace/tcl-lsp')
DEST = Path('/workspace/naming-validation/lsp-retained-report-action-source-successor')
COUNTS = {'tcl_lexer': 6, 'tcl_syntax': 1, 'tcl_spectcl': 2}
GROUP_ORDER = ['tcl_lexer', 'tcl_syntax', 'tcl_spectcl']
GUARDS = ['test_timeout_seconds', 'listing_timeout_seconds', 'environment', 'runtime_guards', 'validation_note', 'host_scope']
LIB_COMMAND = ['cargo', 'test', '--offline', '--locked', '-p', 'tcl-lexer', '-p', 'tcl-syntax', '-p', 'tcl-spectcl', '--lib', '--no-run']


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def checked_files(package):
    for name, record in package['files'].items():
        require(Path(record['path']).is_file() and sha(record['path']) == record['sha256'], 'Prepared package pin changed: ' + name)


def imported(package, name):
    path = Path(package['files'][name]['path'])
    require(sha(path) == package['files'][name]['sha256'], 'Helper changed: ' + name)
    spec = importlib.util.spec_from_file_location('_library9_' + name, path)
    require(spec is not None and spec.loader is not None, 'Cannot load pinned helper')
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def context(package):
    checked_files(package)
    require(package['status'] == 'PREPARED_LIBRARY9_ONLY_SOURCE_PACKAGE_REQUIRES_ROOT_REVIEW', 'Wrong preparation purpose')
    require(package['selected'] == 9 and package['blocked_xtask_controls'] == 3 and package['prospective_full_extra_scope'] == 12, 'Wrong bounded available scope')
    guard = imported(package, 'standard_guard')
    standard = package['files']['standard930_admission']
    ctx = guard.load_standard(standard['path'], standard['sha256'])
    old_admission = read(package['files']['old_extra10_admission']['path'])
    old_manifest = read(package['files']['old_v10_manifest']['path'])
    old_path = Path(package['files']['old_extra10_plans']['path'])
    old_hash = sha(old_path)
    require(old_admission['files']['v10_additional_artifacts'] == package['files']['old_extra10_plans'], 'Wrong old extra10 selector pin')
    require(old_manifest['summary']['outputs']['additional-artifacts.json'] == old_hash, 'Old V10 summary extra vector mismatch')
    sealed = next((row for row in old_manifest['files'] if row['path'] == 'generated-frozen-standard-2/additional-artifacts.json'), None)
    require(sealed is not None and sealed['sha256'] == old_hash, 'Old V10 file seal mismatch')
    old = read(old_path)
    full = read(package['files']['prospective_full12_plans']['path'])
    plans = read(package['files']['prepared_plans']['path'])
    require(set(plans) == set(COUNTS), 'Library9 plan included a different target')
    require(set(full) == set(COUNTS) | {'xtask'} and len(full['xtask']['execution_order']) == 3, 'Full12 requirement disappeared')
    require(full['xtask']['execution_order'][:1] == old['xtask']['execution_order'], 'Original xtask selector dropped')
    inventory = read(package['files']['new_root_control_inventory']['path'])
    new_xtask = [row for row in inventory['added'] if row['target'] == 'xtask']
    require(len(new_xtask) == 2 and [row['selector'] for row in full['xtask']['execution_order'][1:]] == [row['selector'] for row in new_xtask], 'New owed xtask controls changed')
    for target, count in COUNTS.items():
        plan = plans[target]
        require(plan == full[target], 'Available subset changed the full12 plan: ' + target)
        require(plan['execution_order'] == old[target]['execution_order'], 'Original library rows changed: ' + target)
        require(all(plan[key] == old[target][key] for key in GUARDS), 'Original library guards/budgets changed: ' + target)
        require(plan['test_timeout_seconds'] == 180 and plan['listing_timeout_seconds'] == 30, 'Budget changed')
        require(len(plan['execution_order']) == plan['total_exact_selectors'] == count, 'Wrong target count')
        require(plan['groups'] == {target: [row['selector'] for row in plan['execution_order']]} and plan['expected_group_counts'] == {target: count}, 'Wrong target group')
        require(len(plan['source_evidence']) == count and [(row['target'], row['selector']) for row in plan['source_evidence']] == [(row['target'], row['selector']) for row in plan['execution_order']], 'Wrong current source vector')
        for row in plan['source_evidence']:
            require(row['current_native_linux_source_eligible'] is True, 'Current source cfg unavailable')
            for source in [row] + row['module_route']:
                require(ctx['expected'].get(source['source_path']) == source['source_sha256'], 'Current library selector source outside actual39030')
    blocked = read(package['files']['failed_xtask_build']['path'])
    require(blocked['status'] == 'FAIL' and blocked['exit_code'] == 101 and blocked['input_count'] == 39030 and blocked['input_drift_count'] == 0, 'Blocked xtask failure provenance changed')
    return ctx, plans, full['xtask']['execution_order']


def bind_library(ctx, build_dir):
    build_dir = Path(build_dir).resolve()
    require(build_dir == DEST / 'extra-libs', 'Wrong actual current library build')
    paths = {name: build_dir / leaf for name, leaf in [('receipt', 'source-binding.json'), ('before', 'input-before.json'), ('after', 'input-after.json'), ('artifacts', 'artifacts.json')]}
    receipt = read(paths['receipt'])
    require(receipt['status'] == 'PASS_EXTRA_ARTIFACT_BUILD' and receipt['exit_code'] == 0, 'No successful actual extra library build')
    require(receipt['input_count'] == 39030 and receipt['input_drift_count'] == 0 and receipt['input_drift'] == [], 'Extra source drift')
    require(receipt['ordinary'] is True and receipt['instrumentation'] is False and receipt['single_job'] is True and receipt['test_debug'] == 0, 'Not ordinary extra build')
    require(receipt['all_artifact_hashes_verified_after_relocation'] is True, 'Extra relocations unverified')
    require(receipt['command'] == LIB_COMMAND, 'Wrong actual library command/kind')
    require(receipt['parent_binding_sha256'] == ctx['pins']['actual_build']['sha256'] and receipt['parent_input_index_sha256'] == ctx['pins']['actual_inputs']['sha256'], 'Extra build belongs to another standard source')
    parent_rows = read(ctx['pins']['actual_inputs']['path'])
    require(read(paths['before']) == read(paths['after']) == parent_rows, 'Actual before/after extra sources differ from full parent')
    artifacts = read(paths['artifacts'])
    require(set(artifacts) == set(COUNTS), 'Need actual combined three-library manifest')
    for name, artifact in artifacts.items():
        path = Path(artifact['path'])
        require(artifact['actual_target_kind'] == 'lib', 'BIN/library substitution: ' + name)
        require(path.parent.resolve() == build_dir / 'frozen-binaries' and path.is_file(), 'Foreign actual library artifact')
        require(path.stat().st_size == artifact['bytes'] and sha(path) == artifact['sha256'], 'Extra artifact changed: ' + name)
        with path.open('rb') as stream:
            require(stream.read(4) == b'\x7fELF', 'Extra artifact is not ELF')
    return {'status': 'BOUND_ACTUAL_THREE_LIBRARY_BUILD_ONLY', 'build_dir': str(build_dir), 'files': {name: {'path': str(path), 'sha256': sha(path)} for name, path in paths.items()}, 'artifacts': artifacts, 'actual_target_kind': 'lib', 'command': receipt['command'], 'seconds': receipt['seconds'], 'parent_binding_sha256': receipt['parent_binding_sha256'], 'parent_input_index_sha256': receipt['parent_input_index_sha256'], 'all39030_before_after_rows_equal_actual_parent': True}


def check_current(package, ctx, library):
    checked_files(package)
    standard = ctx['check_current']()
    require(bind_library(ctx, library['build_dir']) == library, 'Actual extra build/three artifacts changed')
    return {'standard': standard, 'library_build_receipt_sha256': library['files']['receipt']['sha256'], 'actual_combined_library_artifacts_sha256': library['files']['artifacts']['sha256'], 'all39030_source_inputs_five_standard_plus_three_extra_ELFs_and_provider_pins_match': True, 'selected_extra_controls': 9, 'xtask_controls_still_owed': 3}
