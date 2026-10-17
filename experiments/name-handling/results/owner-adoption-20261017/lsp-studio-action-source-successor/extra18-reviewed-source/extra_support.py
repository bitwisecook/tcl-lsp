"""Prospective full18 source/build joins; Root supplies actual reviewed binding."""
from pathlib import Path
import hashlib
import importlib.util
import json
import sys

ROOT = Path('/workspace/tcl-lsp')
COUNTS = {'tcl_lexer': 6, 'tcl_syntax': 3, 'tcl_spectcl': 6, 'xtask': 3}
GROUP_ORDER = ['tcl_lexer', 'tcl_syntax', 'tcl_spectcl', 'xtask']
GUARDS = ['test_timeout_seconds', 'listing_timeout_seconds', 'environment', 'runtime_guards', 'validation_note', 'host_scope']
LIB_COMMAND = ['cargo', 'test', '--offline', '--locked', '-p', 'tcl-lexer', '-p', 'tcl-syntax', '-p', 'tcl-spectcl', '--lib', '--no-run']
BIN_COMMAND = ['cargo', 'test', '--offline', '--locked', '-p', 'xtask', '--bin', 'xtask', '--no-run']


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
        require(Path(record['path']).is_file() and sha(record['path']) == record['sha256'], 'Package pin changed: ' + name)


def imported(package, name):
    record = package['files'][name]
    require(sha(record['path']) == record['sha256'], 'Helper changed: ' + name)
    spec = importlib.util.spec_from_file_location('_extra18_' + name, record['path'])
    require(spec is not None and spec.loader is not None, 'Cannot load pinned helper')
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def context(package):
    checked_files(package)
    require(package['status'] == 'PREPARED_EXTRA18_REQUIRES_ROOT_REVIEW' and package['selected'] == 18, 'Wrong full scope')
    guard = imported(package, 'standard_guard')
    record = package['files']['standard_admission']
    ctx = guard.load_standard(record['path'], record['sha256'])
    require(len(ctx['expected']) == 39052, 'Wrong full current source inventory')
    historical = read(package['files']['old_extra9_manifest']['path'])
    for name, record in historical['files'].items():
        require(package['files']['historical_extra9_' + name] == record, 'Original extra9 pin removed/replaced: ' + name)
    # Retain the original historical extra10 seal and all blocked xtask evidence.
    old_admission = read(historical['files']['old_extra10_admission']['path'])
    old_manifest = read(historical['files']['old_v10_manifest']['path'])
    old_record = historical['files']['old_extra10_plans']
    require(old_admission['files']['v10_additional_artifacts'] == old_record, 'Original extra10 selector pin changed')
    require(old_manifest['summary']['outputs']['additional-artifacts.json'] == old_record['sha256'], 'Original extra10 summary seal changed')
    sealed = next((row for row in old_manifest['files'] if row['path'] == 'generated-frozen-standard-2/additional-artifacts.json'), None)
    require(sealed is not None and sealed['sha256'] == old_record['sha256'], 'Original extra10 file seal changed')
    old10 = read(old_record['path'])
    old12 = read(package['files']['old12_plans']['path'])
    plans = read(package['files']['prepared_plans']['path'])
    inventory = read(package['files']['new_inventory']['path'])
    added = [row for row in inventory['added'] if row['target'] in {'tcl_syntax', 'tcl_spectcl'}]
    require(len(added) == 6 and set(plans) == set(COUNTS) == set(old12), 'Wrong four-target/full18 source scope')
    require(old12['xtask']['execution_order'][:1] == old10['xtask']['execution_order'], 'Original xtask row dropped')
    old_inventory = read(historical['files']['new_root_control_inventory']['path'])
    old_added = [row for row in old_inventory['added'] if row['target'] == 'xtask']
    require(len(old_added) == 2 and [row['selector'] for row in old12['xtask']['execution_order'][1:]] == [row['selector'] for row in old_added], 'Original two owed xtask controls changed')
    blocked = read(historical['files']['failed_xtask_build']['path'])
    require(blocked['status'] == 'FAIL' and blocked['exit_code'] == 101 and blocked['input_count'] == 39030 and blocked['input_drift_count'] == 0, 'Historical xtask failure evidence changed')
    for target in GROUP_ORDER:
        plan = plans[target]
        previous = old12[target]
        require(plan['execution_order'][:len(previous['execution_order'])] == previous['execution_order'], 'Original12 ordered rows changed: ' + target)
        require(all(plan[key] == previous[key] == old10[target][key] for key in GUARDS), 'Original guards/provider/host/budgets changed: ' + target)
        require(plan['test_timeout_seconds'] == 180 and plan['listing_timeout_seconds'] == 30, 'Budget changed')
        appended = [row for row in added if row['target'] == target]
        require([row['selector'] for row in plan['execution_order'][len(previous['execution_order']):]] == [row['selector'] for row in appended], 'New6 vector changed: ' + target)
        require(len(plan['execution_order']) == plan['total_exact_selectors'] == COUNTS[target], 'Wrong exact target count')
        require(plan['groups'] == {target: [row['selector'] for row in plan['execution_order']]} and plan['expected_group_counts'] == {target: COUNTS[target]}, 'Group/order mismatch')
        require(len(set(plan['groups'][target])) == COUNTS[target], 'Duplicate exact selector')
        require([(row['target'], row['selector']) for row in plan['source_evidence']] == [(row['target'], row['selector']) for row in plan['execution_order']], 'Source route vector changed')
        evidence = {row['selector']: row for row in plan['source_evidence']}
        for row in plan['source_evidence']:
            require(row['current_native_linux_source_eligible'] is True, 'Current Native Linux cfg unavailable')
            for source in [row] + row['module_route']:
                require(ctx['expected'].get(source['source_path']) == source['source_sha256'], 'Selected source outside actual full39052 inputs')
        for row in appended:
            require(evidence[row['selector']]['actual_test_function_sha256'] == row['body_sha256'], 'New actual function changed')
    return ctx, plans


def bind_extra(ctx, build_dir, kind):
    build_dir = Path(build_dir).resolve()
    destination = Path(ctx['admission']['destination']).resolve()
    require(kind in {'libs', 'xtask'} and build_dir == destination / ('extra-' + kind), 'Foreign extra build directory')
    paths = {name: build_dir / leaf for name, leaf in [('receipt', 'source-binding.json'), ('before', 'input-before.json'), ('after', 'input-after.json'), ('artifacts', 'artifacts.json')]}
    receipt = read(paths['receipt'])
    require(receipt['status'] == 'PASS_EXTRA_ARTIFACT_BUILD' and receipt['exit_code'] == 0, 'Actual successful extra build required')
    require(receipt['input_count'] == 39052 and receipt['input_drift_count'] == 0 and receipt['input_drift'] == [], 'Extra build source drift')
    require(receipt['ordinary'] is True and receipt['instrumentation'] is False and receipt['single_job'] is True and receipt['test_debug'] == 0, 'Not ordinary extra build')
    require(receipt['all_artifact_hashes_verified_after_relocation'] is True, 'Unverified actual artifact relocation')
    require(receipt['command'] == (LIB_COMMAND if kind == 'libs' else BIN_COMMAND), 'Wrong actual LIB/BIN build command')
    require(receipt['parent_binding_sha256'] == ctx['pins']['actual_build']['sha256'] and receipt['parent_input_index_sha256'] == ctx['pins']['actual_inputs']['sha256'], 'Extra build belongs to another actual standard build')
    parent = read(ctx['pins']['actual_inputs']['path'])
    require(read(paths['before']) == read(paths['after']) == parent, 'Extra actual before/after rows differ from actual full parent')
    artifacts = read(paths['artifacts'])
    require(set(artifacts) == ({'tcl_lexer', 'tcl_syntax', 'tcl_spectcl'} if kind == 'libs' else {'xtask'}), 'Wrong actual distinct extra target set')
    for name, artifact in artifacts.items():
        path = Path(artifact['path'])
        require(artifact['actual_target_kind'] == ('lib' if kind == 'libs' else 'bin'), 'Actual LIB/BIN substitution: ' + name)
        require(path.parent.resolve() == build_dir / 'frozen-binaries' and path.is_file() and path.stat().st_size == artifact['bytes'] and sha(path) == artifact['sha256'], 'Actual extra artifact changed: ' + name)
        with path.open('rb') as stream:
            require(stream.read(4) == b'\x7fELF', 'Actual extra artifact not ELF: ' + name)
    return {'kind': kind, 'build_dir': str(build_dir), 'files': {name: {'path': str(path), 'sha256': sha(path)} for name, path in paths.items()}, 'artifacts': artifacts}


def check_current(package, ctx, extras):
    checked_files(package)
    standard = ctx['check_current']()
    require(set(extras) == {'libs', 'xtask'}, 'All four extra actual artifacts required')
    for kind, bound in extras.items():
        require(bind_extra(ctx, bound['build_dir'], kind) == bound, 'Actual extra build/source/artifacts changed: ' + kind)
    return {'standard': standard, 'extra_builds': extras, 'all39052_inputs_standard5_plus_extra4_actual9_ELFs_and_provider_prepost_match': True, 'selected': 18}
