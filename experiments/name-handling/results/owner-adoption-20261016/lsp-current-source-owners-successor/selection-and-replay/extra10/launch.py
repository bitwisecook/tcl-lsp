#!/usr/bin/env python3
"""Strict replay/list wrapper for the ten separately built selector controls.

Requires a separately admitted V11 binding for the three-library build and
xtask binary. It does not build artifacts or change the canonical runner,
selection vectors, provider setup, or listing/test budgets.
"""
from pathlib import Path
import argparse
import collections
import datetime
import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path('/workspace/tcl-lsp')
DESTINATION = Path('/workspace/naming-validation/lsp-current-source-owners-successor-repair3')
PACKAGE = Path('/workspace/naming-validation/agent-work/retained726-successor-v10')
RUNNER = ROOT / 'experiments/name-handling/validation/retained-selections/replay-selection.py'
RUNNER_SHA = 'aaa269431e0a0f3024367d0a2528b4671408e11cf7a57e801caa2429f181e6c3'
V10_SHA = '62fd55e80c572eab9690580f58225d6940c72e455ec7a3cdc5c08a850fd5cb37'
ORIGINAL726_SHA = '237d6e60703bdafb3836a3f1bf8009e578a6da3b9aca367061d0098cde379a80'
PROVIDER_SETUP_SHA = 'cf57d1f2af60b425a4f7c1cb644af1682118ff8173186d43a8f554068a483396'
JIM_SHA = 'c5b2b0c37c3bad8e97760350460310bde65e74cf9833979fe1c088e4dd2a4891'
SOURCE_ADMISSION_SHA = '9287fbbe88f7d000284fc5ca61112774839dee5b42e8d42bd3463464fbfbf4b7'
STANDARD = ['tcl_compiler', 'tcl_registry', 'tcl_lsp_core', 'tcl_lsp_db', 'tcl_lsp_server']
EXTRA_COUNTS = {'tcl_lexer': 6, 'tcl_syntax': 1, 'tcl_spectcl': 2, 'xtask': 1}
GUARDS = ['test_timeout_seconds', 'listing_timeout_seconds', 'environment', 'runtime_guards', 'validation_note', 'host_scope']


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def checked_file(record):
    path = Path(record['path']).resolve()
    require(path.is_file() and sha(path) == record['sha256'], 'Pinned file mismatch: ' + str(path))
    return path


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    require(spec is not None and spec.loader is not None, 'Cannot load pinned source binder')
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--admission', type=Path, required=True)
    parser.add_argument('--admission-sha256', required=True)
    parser.add_argument('--list-only', action='store_true', help='Canonical singleton listings only; no tests.')
    args = parser.parse_args()
    require(sys.flags.optimize == 0 and not os.environ.get('PYTHONOPTIMIZE'), 'Optimised Python disables required assertions')
    admission_path = args.admission.resolve()
    require(sha(admission_path) == args.admission_sha256, 'Final extra10 replay admission digest mismatch')
    admission = json.loads(admission_path.read_text())
    require(admission['status'] == 'FINAL_BOUND_EXTRA10_REPLAY_ADMISSION', 'Extra-target selection is not replay authority')
    require(admission['selected'] == 10 and admission['original_prefix'] == 726, 'Wrong exact extra-target scope')
    require(Path(admission['destination']).resolve() == DESTINATION, 'Wrong actual frozen destination')
    pins = admission['files']
    launcher_path = checked_file(pins['launcher'])
    require(launcher_path == Path(__file__).resolve(), 'Admission pins another extra10 launcher')
    v11_review_path = checked_file(pins['v11_binder_review'])
    require(v11_review_path.read_text().startswith('SOURCE CLEAR'), 'V11 binder lacks independent source clearance')
    v10_manifest_path = checked_file(pins['v10_manifest'])
    v10_selection_path = checked_file(pins['v10_additional_artifacts'])
    v11_manifest_path = checked_file(pins['v11_manifest'])
    v11_additional_path = checked_file(pins['v11_additional_artifacts'])
    v11_binder_path = checked_file(pins['v11_binder'])
    source_admission = checked_file(pins['source_admission'])
    historical_inputs = checked_file(pins['historical_inputs'])
    parent_inputs = checked_file(pins['parent_inputs'])
    parent_receipt = checked_file(pins['parent_receipt'])
    parent_source_diff = checked_file(pins['parent_source_diff'])
    parent_artifacts = checked_file(pins['parent_artifacts'])
    provider_setup = checked_file(pins['provider_setup'])
    original = checked_file(pins['original726'])
    scanner = checked_file(pins['parent_build_scanner'])
    require(sha(v10_manifest_path) == V10_SHA, 'Wrong V10 selector/source package')
    require(sha(original) == ORIGINAL726_SHA and sha(provider_setup) == PROVIDER_SETUP_SHA, 'Original selection/provider setup changed')
    require(sha(source_admission) == SOURCE_ADMISSION_SHA, 'Wrong actual approved source admission')
    v10 = json.loads(v10_manifest_path.read_text())
    require(v10['status'].startswith('SOURCE_BOUND_TO_ACTUAL_SUCCESSFUL_FIVE_LIBRARY_BUILD'), 'V10 lacks actual standard build binding')
    require(sha(v10_selection_path) == v10['summary']['outputs']['additional-artifacts.json'], 'V10 extra selector vector changed')
    v11 = json.loads(v11_manifest_path.read_text())
    require(v11['status'] == 'ACTUAL_EXTRA_TARGET_ARTIFACTS_BOUND_TO_SOURCE_IDENTICAL_BUILDS', 'V11 extra artifact binding is not complete')
    require(v11['parent_v10_manifest']['sha256'] == V10_SHA, 'V11 binds another V10 package')
    require(sha(v11_additional_path) == next(row['sha256'] for row in v11['files'] if row['path'] == 'additional-artifacts-v11.json'), 'V11 extra selectors or artifact binding changed')
    require(sha(v11_binder_path) == next(row['sha256'] for row in v11['files'] if row['path'] == 'bind_extra_artifacts.py'), 'V11 binding procedure changed')
    require(v11['standard_selector_vector']['sha256'] == v10['selector_result']['sha256'], 'Standard vector parent mismatch')
    require(v11['standard_source_diff_sha256'] == sha(parent_source_diff), 'Standard source diff mismatch')
    require(v11['full_input_snapshot_sha256'] == sha(parent_inputs), 'V11 full input snapshot differs from standard parent')
    source_rows = json.loads(parent_inputs.read_text())
    require(len(source_rows) == 39019 and len({row['path'] for row in source_rows}) == 39019, 'Parent full-source inventory malformed')
    source_map = {row['path']: row['sha256'] for row in source_rows}
    additional = json.loads(v10_selection_path.read_text())
    bound_additional = json.loads(v11_additional_path.read_text())
    require(set(additional) == set(EXTRA_COUNTS) == set(bound_additional), 'Extra target set changed')
    for target, count in EXTRA_COUNTS.items():
        old = additional[target]
        bound = bound_additional[target]
        require(len(old['execution_order']) == old['total_exact_selectors'] == count, 'V10 selector count changed: ' + target)
        require(bound['execution_order'] == old['execution_order'], 'V11 changed V10 selector rows: ' + target)
        require(bound['source_evidence'] == old['source_evidence'], 'V11 changed V10 source evidence: ' + target)
        for fact in old['source_evidence']:
            require(fact['current_native_linux_source_eligible'] is True, 'Ineligible extra selector source: ' + target)
            require(source_map.get(fact['source_path']) == fact['source_sha256'], 'Extra selector source hash is outside parent inventory: ' + target)
            for module in fact['module_route']:
                require(source_map.get(module['source_path']) == module['source_sha256'], 'Extra selector module hash is outside parent inventory: ' + target)
        require(bound['freeze_binding']['selector_count'] == count, 'Extra selector binding count mismatch: ' + target)
        require(bound['freeze_binding']['selectors'] == [row['selector'] for row in old['execution_order']], 'Extra selector vector binding mismatch: ' + target)
        require(v11['extra_target_selector_counts'][target] == count, 'V11 manifest count mismatch: ' + target)
    generator_path = checked_file(pins['generator'])
    source_tools_path = checked_file(pins['source_tools'])
    require(generator_path.parent == source_tools_path.parent == v10_manifest_path.parent, 'V10 binder package directory mismatch')
    sys.path.insert(0, str(generator_path.parent))
    source_tools = load_module('source_tools_extra10', source_tools_path)
    gen = load_module('_strict_extra10_source_binder', generator_path)
    require(gen.STANDARD == STANDARD and gen.GUARDS == ['test_timeout_seconds', 'listing_timeout_seconds', 'environment', 'runtime_guards', 'validation_note', 'host_scope'], 'Standard source binder guard contract changed')
    require(not source_tools.SOURCE_OVERRIDES, 'Candidate source overlays cannot bind replay')
    standard_selection_path = checked_file(pins['standard_selection'])
    standard_selection = json.loads(standard_selection_path.read_text())
    require(len(standard_selection['execution_order']) == 902 and standard_selection['execution_order'][:726] == json.loads(original.read_text())['execution_order'], 'Standard parent vector/prefix changed')
    require(all(standard_selection[key] == json.loads(original.read_text())[key] for key in GUARDS), 'Standard guards changed')
    require(sha(RUNNER) == RUNNER_SHA, 'Canonical runner changed')
    require(Path(admission['v10_directory']).resolve() == v10_manifest_path.parent, 'Wrong V10 directory')
    require(Path(admission['v11_directory']).resolve() == v11_manifest_path.parent, 'Wrong V11 directory')

    require(v11['standard_build_binding_sha256'] == sha(parent_receipt), 'V11 standard build binding changed')
    require(v11['standard_source_diff_sha256'] == sha(parent_source_diff), 'V11 standard diff provenance changed')
    require(v11['full_input_snapshot_sha256'] == sha(parent_inputs), 'V11 source snapshot provenance changed')
    for target in EXTRA_COUNTS:
        build_info = v11['extra_builds'][target]
        build_dir = Path(build_info['build_dir'])
        receipt = build_info['binding']['build_receipt']
        receipt_path = Path(receipt['path'])
        require(receipt_path == build_dir / 'source-binding.json' and sha(receipt_path) == receipt['sha256'], 'Extra build receipt changed: ' + target)
        require(receipt['input_count'] == 39019 and receipt['input_drift_count'] == 0 and receipt['all_source_rows_equal_parent_full_snapshot'] is True, 'Extra build source coverage mismatch: ' + target)
        for leaf, digest_key in [('input-before.json', 'before_index_sha256'), ('input-after.json', 'after_index_sha256'), ('artifacts.json', 'artifacts_sha256')]:
            file_path = build_dir / leaf
            require(sha(file_path) == receipt[digest_key], 'Extra build file changed: ' + target + '/' + leaf)
        require(json.loads((build_dir / 'input-before.json').read_text()) == source_rows, 'Extra build before-snapshot differs: ' + target)
        require(json.loads((build_dir / 'input-after.json').read_text()) == source_rows, 'Extra build after-snapshot differs: ' + target)

    def current_canonical_inventory():
        historical = json.loads(historical_inputs.read_text())
        union = json.loads(source_admission.read_text())['files']
        require(len(historical) == 39002 and len({row['path'] for row in historical}) == 39002, 'Historical39002 input inventory changed')
        require(len(union) == 59 and len({row['path'] for row in union}) == 59, 'Reviewed59 source union changed')
        argv = ['git', 'ls-files', '-z', '--', 'rust', 'runtime', 'scripts', 'specs', 'Cargo.toml', 'Cargo.lock', '.cargo/config.toml']
        tracked = subprocess.check_output(argv, cwd=ROOT, text=True).split('\0')
        paths = sorted({row['path'] for row in historical} | {row['path'] for row in union} | {path for path in tracked if path})
        require(len(paths) == 39019, 'Canonical actual input membership changed')
        expected = {row['path']: row['sha256'] for row in source_rows}
        require(set(paths) == set(expected), 'Canonical source membership differs from approved build')
        actual = {}
        for relative in paths:
            path = ROOT / relative
            require(path.is_file() and path.resolve().is_relative_to(ROOT.resolve()), 'Missing or escaping source input: ' + relative)
            actual[relative] = sha(path)
        require(actual == expected, 'Full canonical source input dictionary changed')
        return {'count': len(actual), 'parent_inputs_sha256': sha(parent_inputs), 'membership_and_bytes_equal': True, 'scanner_argv': argv}

    def check_binding():
        require(sha(admission_path) == args.admission_sha256, 'Extra replay admission changed')
        for record in pins.values():
            checked_file(record)
        require(sha(RUNNER) == RUNNER_SHA, 'Canonical replay runner changed')
        require(parent_receipt == DESTINATION / 'source-binding.json' and parent_artifacts == DESTINATION / 'artifacts.json' and parent_inputs == DESTINATION / 'input-before.json' and parent_source_diff == DESTINATION / 'source.diff', 'Wrong standard parent receipt paths')
        current_canonical_inventory()
        build = json.loads(parent_receipt.read_text())
        require(build['status'] == 'PASS_FIVE_LIBRARY_BUILD' and build['ordinary'] is True and build['instrumentation'] is False and build['exit_code'] == 0, 'Successful standard build binding required')
        require(build['input_before_count'] == build['input_after_count'] == 39019 and build['input_drift_count'] == 0, 'Standard build source input mismatch')
        require(build['all_five_artifact_relocations_hash_verified'] is True and build['approved_union_manifest_sha256'] == SOURCE_ADMISSION_SHA, 'Standard build/source admission mismatch')
        require(Path(build['approved_union_manifest']).resolve() == source_admission, 'Standard source admission path mismatch')
        require(build['test_debug'] == 0 and build['single_job'] is True, 'Standard ordinary build mode changed')
        for row in json.loads(source_admission.read_text())['files']:
            require(sha(ROOT / row['path']) == row['after_sha256'], 'Approved source owner drift: ' + row['path'])
        binding = gen.bind_actual_build(DESTINATION, STANDARD, standard_selection['source_evidence'], ROOT)
        require(binding == standard_selection['freeze_binding'], 'Standard selector/build binding changed')
        require(binding['full_input_count'] == 39019 and set(binding['binary_bindings']) == set(STANDARD), 'Wrong standard build scope')
        artifacts = json.loads(parent_artifacts.read_text())
        require(sha(parent_artifacts) == v10['successful_build']['artifacts_sha256'], 'Standard artifacts manifest changed')
        for target, expected in v10['successful_build']['actual_five_ELF_bindings'].items():
            require(artifacts[target] == expected and sha(expected['path']) == expected['sha256'], 'Standard ELF binding changed: ' + target)
        extra_targets = {}
        for target in EXTRA_COUNTS:
            info = bound_additional[target]['freeze_binding']
            for binary_name, item in info['artifact_bindings'].items():
                p = Path(item['path'])
                require(p.is_file() and p.stat().st_size == item['bytes'] and sha(p) == item['sha256'], 'Extra ELF binding changed: ' + binary_name)
                with p.open('rb') as stream:
                    require(stream.read(4) == b'\x7fELF', 'Extra artifact is not ELF: ' + binary_name)
            if target == 'xtask':
                require(info['target_kind'] == 'bin:xtask' and info['actual_artifact_kind'] == 'bin', 'xtask artifact kind mismatch')
                build_dir = Path(v11['extra_builds'][target]['build_dir'])
                manifest_path = build_dir / 'artifacts.json'
                rows = json.loads(manifest_path.read_text())
                require(set(rows) == {'xtask'} and rows == info['artifact_bindings'], 'Actual xtask artifact manifest differs from V11 binding')
            else:
                require(info['target_kind'] == info['actual_artifact_kind'] == 'lib', 'Library artifact kind mismatch: ' + target)
                build_dir = Path(v11['extra_builds'][target]['build_dir'])
                manifest_path = build_dir / 'artifacts.json'
                rows = json.loads(manifest_path.read_text())
                require(set(rows) == {'tcl_lexer', 'tcl_syntax', 'tcl_spectcl'} and rows == info['artifact_bindings'], 'Actual combined library artifact manifest differs from V11 binding')
            extra_targets[target] = {'manifest_path': str(manifest_path), 'manifest_sha256': sha(manifest_path), 'artifacts': rows}
        check_environment()
        return binding, extra_targets

    required_env = dict(json.loads(provider_setup.read_text())['environment'])
    required_env['CARGO_PROFILE_TEST_DEBUG'] = '0'
    def check_environment():
        require(all(os.environ.get(key) == value for key, value in required_env.items()), 'Original provider/debug environment mismatch')
        require(not [key for key, value in os.environ.items() if value and (key.startswith('TCL_LSP_TRACE') or key == 'TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC')], 'Diagnostic environment is not ordinary replay')
        for name in ['tclsh9.0', 'tclsh8.6']:
            require(shutil.which(name) == '/workspace/.toolchains/tcl/bin/' + name, 'Original Tcl provider path changed: ' + name)
            item = admission['native_provider_bindings'][name]
            require(Path(item['path']) == Path('/workspace/.toolchains/tcl/bin') / name and sha(item['path']) == item['sha256'], 'Native Tcl provider binding changed')
        require(sha(required_env['TCL_LSP_JIMSH']) == JIM_SHA, 'Original Jim binary changed')

    standard_binding, extra_artifacts = check_binding()
    preflight_inventory = current_canonical_inventory()
    out_root = DESTINATION / ('extra10-listing-results' if args.list_only else 'extra10-results')
    receipt_path = DESTINATION / (out_root.name + '-invocation.json')
    require(not out_root.exists() and not receipt_path.exists(), 'Refuse to overwrite prior extra10 replay/receipt')
    out_root.mkdir()
    results = []
    code = 0
    try:
        for target, count in EXTRA_COUNTS.items():
            rows = additional[target]['execution_order']
            manifest = {
                'group': target,
                'execution_order': rows,
                'groups': {target: [row['selector'] for row in rows]},
                'expected_group_counts': {target: count},
                'total_exact_selectors': count,
                'test_timeout_seconds': 180,
                'listing_timeout_seconds': 30,
                'environment': additional[target]['environment'],
                'runtime_guards': additional[target]['runtime_guards'],
                'validation_note': additional[target]['validation_note'],
                'host_scope': additional[target]['host_scope'],
            }
            manifest_path = out_root / (target + '-selection.json')
            manifest_path.write_text(json.dumps(manifest, indent=2) + '\n')
            manifest_sha = sha(manifest_path)
            artifact_manifest_path = Path(extra_artifacts[target]['manifest_path'])
            require(sha(artifact_manifest_path) == extra_artifacts[target]['manifest_sha256'], 'Actual extra artifact manifest changed: ' + target)
            target_out = out_root / target
            argv = [sys.executable, str(RUNNER), '--manifest', str(manifest_path), '--manifest-sha256', manifest_sha, '--artifacts', str(artifact_manifest_path), '--root', str(ROOT), '--out', str(target_out), '--group', target]
            if args.list_only:
                argv.append('--list-only')
            try:
                result_code = subprocess.run(argv, cwd=ROOT, check=False).returncode
                run_error = None
            except Exception as error:
                result_code = 1
                run_error = repr(error)
            results.append({'target': target, 'selected': count, 'argv': argv, 'wrapper_exit_code': result_code, 'error': run_error, 'manifest_sha256': manifest_sha, 'artifact_manifest_path': str(artifact_manifest_path), 'artifact_manifest_sha256': sha(artifact_manifest_path), 'target_artifact': extra_artifacts[target]['artifacts'][target]})
            if result_code != 0:
                code = 1
    finally:
        try:
            post_binding, post_extra_artifacts = check_binding()
            require(post_binding == standard_binding and post_extra_artifacts == extra_artifacts, 'Standard/extra source or ELF binding changed during replay')
            final_inventory = current_canonical_inventory()
            postflight = {
                'all39019_source_inputs_and_five_standard_plus_four_extra_ELFs_match': True,
                'canonical_inventory': final_inventory,
            }
        except Exception as error:
            postflight = {'error': repr(error)}
            code = 1
        receipt = {
            'status': 'PREFLIGHT_PASS_EXTRA10' if code == 0 and 'error' not in postflight else 'EXTRA10_REPLAY_OR_POSTFLIGHT_FAILED',
            'list_only': args.list_only,
            'selected': 10,
            'exact_target_counts': EXTRA_COUNTS,
            'all902_standard_prefix_and_guard_contract_retained': True,
            'canonical_inventory_preflight': preflight_inventory,
            'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
            'admission': str(admission_path), 'admission_sha256': args.admission_sha256,
            'v10_manifest_sha256': sha(v10_manifest_path), 'v11_manifest_sha256': sha(v11_manifest_path),
            'standard_binding': standard_binding,
            'extra_artifacts': extra_artifacts,
            'invocations': results,
            'postflight': postflight,
            'canonical_runner_sha256': RUNNER_SHA,
            'provider_environment': required_env,
            'native_provider_bindings': admission['native_provider_bindings'],
            'jim_binary_sha256': JIM_SHA,
            'scope': 'Four canonical per-target invocations for exactly 10 separately built extra selectors. Not the standard902 invocation and not ordinary acceptance.',
            'wrapper_exit_code': code,
        }
        with receipt_path.open('x') as stream:
            stream.write(json.dumps(receipt, indent=2) + '\n')
        print(json.dumps({key: value for key, value in receipt.items() if key not in ['standard_binding', 'extra_artifacts']}, indent=2), flush=True)
    return code


if __name__ == '__main__':
    raise SystemExit(main())
