#!/usr/bin/env python3
"""Strict current-source962 replay wrapper. Root alone executes the canonical runner."""
from pathlib import Path
import argparse
import collections
import datetime
import hashlib
import json
import os
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path('/workspace/tcl-lsp')
DEST = Path('/workspace/naming-validation/lsp-studio-action-source-successor-repair3')
CANONICAL_SHA = 'aaa269431e0a0f3024367d0a2528b4671408e11cf7a57e801caa2429f181e6c3'
PROVIDER_SETUP_SHA = 'cf57d1f2af60b425a4f7c1cb644af1682118ff8173186d43a8f554068a483396'
STANDARD = {'tcl_compiler', 'tcl_lsp_core', 'tcl_lsp_db', 'tcl_lsp_server', 'tcl_registry'}
GUARDS = ['test_timeout_seconds', 'listing_timeout_seconds', 'environment', 'runtime_guards', 'validation_note', 'host_scope']
BUILD_COMMAND = 'CARGO_PROFILE_TEST_DEBUG=0 cargo test --offline --locked -p tcl-compiler -p tcl-lsp-core -p tcl-lsp-db -p tcl-registry -p tcl-lsp-server --lib --no-run'


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


STANDARD_LAUNCHER = Path('/workspace/naming-validation/studio-action962-bound-v1/launch.py')

def load_standard(admission_path, admission_sha256):
    args = argparse.Namespace(admission=Path(admission_path), admission_sha256=admission_sha256)
    require(sys.flags.optimize == 0 and not os.environ.get('PYTHONOPTIMIZE'), 'Optimised Python disables canonical guards')
    require(sha(args.admission) == args.admission_sha256, 'Admission changed')
    admission = json.loads(args.admission.read_text())
    require(admission['status'] == 'PREPARED_CURRENT_SOURCE962_REPLAY_REQUIRES_ROOT_REVIEW', 'Wrong admission purpose')
    require(admission['selected'] == 962 and admission['full_input_count'] == 39052 and admission['source_admission_paths'] == 128, 'Wrong exact current scope')
    require(Path(admission['root']).resolve() == ROOT and Path(admission['destination']).resolve() == DEST, 'Foreign root/destination')
    pins = admission['files']

    def checked(name):
        record = pins[name]
        path = Path(record['path'])
        require(path.is_file() and sha(path) == record['sha256'], 'Pinned input changed: ' + name)
        return path

    for name in pins:
        checked(name)
    require(checked('launcher').resolve() == STANDARD_LAUNCHER.resolve(), 'Wrong actual launcher')
    old_admission = json.loads(checked('old902_admission').read_text())
    # These old package/source bindings are historical provenance. They are
    # never relabelled as the current39052 source or its new frozen artifacts.
    for name, old_record in old_admission['files'].items():
        require(pins['historical902_' + name] == old_record, 'Original902 pin dropped/replaced: ' + name)
    require(old_admission['selected'] == 902 and old_admission['original_prefix'] == 726, 'Wrong historical admission')
    require(sha(checked('historical902_provider_setup')) == PROVIDER_SETUP_SHA, 'Original provider setup changed')
    provider_setup = json.loads(checked('historical902_provider_setup').read_text())
    required_environment = dict(provider_setup['environment'])
    required_environment['CARGO_PROFILE_TEST_DEBUG'] = '0'
    require(admission['provider_environment'] == required_environment, 'Copied environment does not equal original cf57 setup')
    for name, record in old_admission['native_provider_bindings'].items():
        require(admission['native_provider_bindings'][name] == record, 'Original provider binding replaced: ' + name)

    plan = json.loads(checked('plan').read_text())
    old = json.loads(checked('old902').read_text())
    original726 = json.loads(checked('historical902_original726').read_text())
    order = plan['execution_order']
    require(len(original726['execution_order']) == original726['total_exact_selectors'] == 726, 'Wrong original726')
    require(len(old['execution_order']) == old['total_exact_selectors'] == 902, 'Wrong original902')
    require(old['execution_order'][:726] == original726['execution_order'], 'Historical726 prefix changed')
    require(len(order) == plan['total_exact_selectors'] == 962 and order[:902] == old['execution_order'], 'Original902 vector changed')
    require(all(plan[key] == old[key] == original726[key] for key in GUARDS), 'Original runtime/provider/host/budget guards changed')
    require(plan['test_timeout_seconds'] == 180 and plan['listing_timeout_seconds'] == 30, 'Budgets changed')
    require(set(plan['groups']) == STANDARD, 'Wrong standard-five groups')
    require(plan['expected_group_counts'] == {key: len(value) for key, value in plan['groups'].items()}, 'Expected group counts changed')
    require(len({(row['target'], row['selector']) for row in order}) == 962, 'Duplicate exact selector')
    require(collections.Counter((row['target'], row['selector']) for row in order) == collections.Counter((key, name) for key, names in plan['groups'].items() for name in names), 'Order/group mismatch')
    inventory = json.loads(checked('source_control_inventory').read_text())
    added = [row for row in inventory['added'] if row['target'] != 'xtask']
    require(len(added) == 28 and [(row['target'], row['selector']) for row in order[902:930]] == [(row['target'], row['selector']) for row in added], 'Actual new28 vector changed')
    old930_admission = json.loads(checked('old930_admission').read_text())
    require(old930_admission['selected'] == 930 and old930_admission['full_input_count'] == 39030, 'Wrong actual historical930')
    for name, old_record in old930_admission['files'].items():
        require(pins['historical930_' + name] == old_record, 'Original930 pin dropped/replaced: ' + name)
    old930 = json.loads(checked('old930').read_text())
    require(len(old930['execution_order']) == old930['total_exact_selectors'] == 930, 'Wrong original930')
    require(order[:930] == old930['execution_order'], 'Original930 ordered prefix changed')
    require(all(plan[key] == old930[key] for key in GUARDS), 'Original930 runtime/provider/host/budget guards changed')
    successor_inventory = json.loads(checked('successor_source_control_inventory').read_text())
    successor_added = [row for row in successor_inventory['added'] if row['target'] in STANDARD and row['artifact_kind'] == 'library']
    require(len(successor_added) == 32 and [(row['target'], row['selector']) for row in order[930:]] == [(row['target'], row['selector']) for row in successor_added], 'Actual new32 source-control vector changed')
    parent = plan['actual_parent']
    require(Path(parent['path']).resolve() == DEST, 'Wrong selected build destination')
    for field, pin in [('build_receipt_sha256', 'actual_build'), ('artifacts_sha256', 'actual_artifacts'), ('full39052_input_index_sha256', 'actual_inputs'), ('source_diff_sha256', 'actual_source_diff'), ('source_admission_sha256', 'source_admission')]:
        require(parent[field] == pins[pin]['sha256'], 'Plan/build pin mismatch: ' + field)
    for pin, name in [('actual_build', 'source-binding.json'), ('actual_inputs', 'input-before.json'), ('actual_artifacts', 'artifacts.json'), ('actual_source_diff', 'source.diff')]:
        require(checked(pin).resolve() == DEST / name, 'Foreign actual build receipt: ' + pin)

    inputs = json.loads(checked('actual_inputs').read_text())
    expected = {row['path']: row['sha256'] for row in inputs}
    require(len(inputs) == len(expected) == 39052, 'Input inventory not unique39052')
    source_admission = json.loads(checked('source_admission').read_text())
    require(len(source_admission['files']) == len({row['path'] for row in source_admission['files']}) == 128, 'Wrong admitted128')
    artifacts = json.loads(checked('actual_artifacts').read_text())
    require(set(artifacts) == STANDARD, 'Wrong actual five artifacts')
    build = json.loads(checked('actual_build').read_text())
    require(build['status'] == 'PASS_FIVE_LIBRARY_BUILD' and build['exit_code'] == 0 and build['ordinary'] is True and build['instrumentation'] is False, 'Actual ordinary successful build required')
    require(build['single_job'] is True and build['test_debug'] == 0 and build['command'] == BUILD_COMMAND, 'Actual build command/mode changed')
    require(len(build['packages']) == 5 and {name.replace('-', '_') for name in build['packages']} == STANDARD, 'Actual build package set mismatch')
    require(build['input_before_count'] == build['input_after_count'] == 39052 and build['input_drift_count'] == 0 and build['input_drift'] == [], 'Actual build source drift')
    require(build['all_five_artifact_relocations_hash_verified'] is True, 'Actual artifact relocation unverified')
    require(Path(build['approved_union_manifest']).resolve() == checked('source_admission').resolve() and build['approved_union_manifest_sha256'] == pins['source_admission']['sha256'], 'Actual build/admission mismatch')
    require(build['rust_diff_sha256'] == pins['actual_source_diff']['sha256'], 'Source.diff does not belong to actual build')

    evidence = json.loads(checked('current_source_evidence').read_text())
    require(evidence['selected'] == 962 and evidence['parent_full_input_sha256'] == pins['actual_inputs']['sha256'] and evidence['source_tools_sha256'] == pins['historical902_source_tools']['sha256'], 'Current source-route evidence belongs to another input/parser')
    require([(row['target'], row['selector']) for row in evidence['rows']] == [(row['target'], row['selector']) for row in order], 'Current source-route evidence vector changed')
    by_selector = {}
    for row in evidence['rows']:
        require(row['current_native_linux_source_eligible'] is True, 'Unproved current cfg route')
        for source in [row] + row['module_route']:
            require(expected.get(source['source_path']) == source['source_sha256'], 'Current selected source absent from actual39052 build')
        by_selector[row['target'], row['selector']] = row
    for row in added:
        selected = by_selector[row['target'], row['selector']]
        require(selected['source_path'] == row['source'] and selected['actual_test_function_sha256'] == row['body_sha256'], 'New control function differs from actual Root inventory')

    for row in successor_added:
        selected = by_selector[row['target'], row['selector']]
        require(selected['source_path'] == row['source'] and selected['actual_test_function_sha256'] == row['body_sha256'], 'New successor control differs from actual Root inventory')

    def check_current():
        require(sha(args.admission) == args.admission_sha256, 'Admission changed during execution')
        for name in pins:
            checked(name)
        require(sha(checked('canonical_runner')) == CANONICAL_SHA, 'Canonical runner changed')
        require(not [key for key, value in os.environ.items() if value and (key.startswith('TCL_LSP_TRACE') or key == 'TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC')], 'Diagnostic environment is not ordinary replay')
        for key, value in required_environment.items():
            require(os.environ.get(key) == value, 'Original provider/debug environment changed: ' + key)
        for key, value in admission['optional_provider_environment'].items():
            require(os.environ.get(key) == value, 'Provider source/binary override environment changed: ' + key)
        for name, record in admission['native_provider_bindings'].items():
            require(shutil.which(name) == record['path'] and sha(record['path']) == record['sha256'], 'Actual provider changed: ' + name)
        for record in admission['additional_provider_files']:
            require(Path(record['path']).is_file() and sha(record['path']) == record['sha256'], 'Actual provider delegate/support changed: ' + record['path'])
        require(sha(required_environment['TCL_LSP_JIMSH']) == admission['jim_binary_sha256'], 'Actual Jim provider changed')
        historical = json.loads(checked('historical39019').read_text())
        require(len(historical) == len({row['path'] for row in historical}) == 39019, 'Wrong historical39019 membership')
        argv = ['git', 'ls-files', '-z', '--', 'rust', 'runtime', 'scripts', 'specs', 'Cargo.toml', 'Cargo.lock', '.cargo/config.toml']
        tracked = subprocess.check_output(argv, cwd=ROOT, text=True).split('\0')
        paths = {row['path'] for row in historical} | {row['path'] for row in source_admission['files']} | {path for path in tracked if path}
        require(paths == set(expected), 'Actual source scanner membership changed')
        actual = {}
        for relative in sorted(paths):
            path = ROOT / relative
            require(path.is_file() and path.resolve().is_relative_to(ROOT), 'Missing/escaping actual source: ' + relative)
            actual[relative] = sha(path)
        require(actual == expected, 'Actual39052 source bytes changed')
        for row in source_admission['files']:
            require(actual[row['path']] == row['after_sha256'], 'Admitted source image changed')
        for name, record in artifacts.items():
            path = Path(record['path'])
            require(path.parent.resolve() == DEST / 'frozen-binaries' and path.is_file() and path.stat().st_size == record['bytes'] and sha(path) == record['sha256'], 'Actual frozen artifact changed: ' + name)
            with path.open('rb') as stream:
                require(stream.read(4) == b'\x7fELF', 'Actual artifact is not ELF: ' + name)
        return {'count': len(actual), 'source_inputs_sha256': pins['actual_inputs']['sha256'], 'five_artifacts_sha256': pins['actual_artifacts']['sha256'], 'all_source_membership_bytes_five_ELFs_original_provider_and_support_pins_match': True, 'current_source_route_evidence_sha256': pins['current_source_evidence']['sha256']}

    return {'admission': admission, 'pins': pins, 'plan': plan, 'expected': expected, 'artifacts': artifacts, 'check_current': check_current}
