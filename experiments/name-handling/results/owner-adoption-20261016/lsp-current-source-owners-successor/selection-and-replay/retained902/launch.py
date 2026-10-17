#!/usr/bin/env python3
"""Strict retained902 replay/listing wrapper; no build or budget changes.

This bound launcher is not a listing/test or ordinary acceptance receipt.
It never builds artifacts or alters the canonical replay runner or budgets.
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
RUNNER = ROOT / 'experiments/name-handling/validation/retained-selections/replay-selection.py'
RUNNER_SHA = 'aaa269431e0a0f3024367d0a2528b4671408e11cf7a57e801caa2429f181e6c3'
ORIGINAL726_SHA = '237d6e60703bdafb3836a3f1bf8009e578a6da3b9aca367061d0098cde379a80'
PROVIDER_SETUP_SHA = 'cf57d1f2af60b425a4f7c1cb644af1682118ff8173186d43a8f554068a483396'
JIM_SHA = 'c5b2b0c37c3bad8e97760350460310bde65e74cf9833979fe1c088e4dd2a4891'
SOURCE_ADMISSION_SHA = '9287fbbe88f7d000284fc5ca61112774839dee5b42e8d42bd3463464fbfbf4b7'
STANDARD = ['tcl_compiler', 'tcl_registry', 'tcl_lsp_core', 'tcl_lsp_db', 'tcl_lsp_server']
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
    parser.add_argument('--list-only', action='store_true', help='Canonical singleton listing checks only; no tests.')
    args = parser.parse_args()
    require(sys.flags.optimize == 0 and not os.environ.get('PYTHONOPTIMIZE'), 'Optimised Python disables required canonical/binder assertions')
    admission_path = args.admission.resolve()
    require(sha(admission_path) == args.admission_sha256, 'Final replay admission digest mismatch')
    admission = json.loads(admission_path.read_text())
    require(admission['status'] == 'FINAL_BOUND_902_REPLAY_ADMISSION', 'Prospective selection is not replay authority')
    require(admission['selected'] == 902 and admission['original_prefix'] == 726, 'Wrong exact replay scope')
    require(Path(admission['destination']).resolve() == DESTINATION, 'Wrong actual frozen destination')
    pins = admission['files']
    package_manifest = checked_file(pins['package_manifest'])
    generator = checked_file(pins['generator'])
    source_tools_path = checked_file(pins['source_tools'])
    plan = checked_file(pins['selection'])
    original = checked_file(pins['original726'])
    provider_setup = checked_file(pins['provider_setup'])
    source_admission = checked_file(pins['source_admission'])
    historical_inputs = checked_file(pins['historical_inputs'])
    parent_inputs = checked_file(pins['parent_inputs'])
    parent_receipt = checked_file(pins['parent_receipt'])
    parent_source_diff = checked_file(pins['parent_source_diff'])
    parent_artifacts = checked_file(pins['parent_artifacts'])
    require(sha(original) == ORIGINAL726_SHA and sha(provider_setup) == PROVIDER_SETUP_SHA, 'Original selection/provider setup changed')
    require(sha(source_admission) == SOURCE_ADMISSION_SHA, 'Wrong reviewed actual source admission')
    require(generator.parent == source_tools_path.parent == package_manifest.parent, 'Binder package directory mismatch')
    sys.path.insert(0, str(generator.parent))
    source_tools = load_module('source_tools', source_tools_path)
    gen = load_module('_strict_retained902_source_binder', generator)
    require(gen.STANDARD == STANDARD and gen.GUARDS == GUARDS, 'Standard-five or guard contract changed')
    require(not source_tools.SOURCE_OVERRIDES, 'Candidate overlays cannot bind ordinary replay')
    selection = json.loads(plan.read_text())
    old = json.loads(original.read_text())
    require(len(old['execution_order']) == old['total_exact_selectors'] == 726, 'Wrong original726 vector')
    require(len(selection['execution_order']) == selection['total_exact_selectors'] == 902, 'Wrong exact902 vector')
    require(selection['execution_order'][:726] == old['execution_order'], 'Original726 prefix/metadata changed')
    require(all(selection[key] == old[key] for key in GUARDS), 'Original guards/environment/budgets changed')
    require(selection['test_timeout_seconds'] == 180 and selection['listing_timeout_seconds'] == 30, 'Budget change')
    require(set(selection['groups']) == set(STANDARD), 'Extra/subgroup artifact entered standard replay')
    order = [(row['target'], row['selector']) for row in selection['execution_order']]
    require(len(set(order)) == 902, 'Duplicate exact selector')
    require(collections.Counter(order) == collections.Counter((key, name) for key, names in selection['groups'].items() for name in names), 'Order/group mismatch')
    require(selection['expected_group_counts'] == {key: len(names) for key, names in selection['groups'].items()}, 'Group count mismatch')
    require(sha(RUNNER) == RUNNER_SHA, 'Canonical runner changed')
    required = dict(json.loads(provider_setup.read_text())['environment'])
    required['CARGO_PROFILE_TEST_DEBUG'] = '0'

    def check_environment():
        require(all(os.environ.get(key) == value for key, value in required.items()), 'Original provider/debug environment mismatch')
        require(not [key for key, value in os.environ.items() if value and (key.startswith('TCL_LSP_TRACE') or key == 'TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC')], 'Diagnostic environment is not ordinary replay')
        for name in ['tclsh9.0', 'tclsh8.6']:
            require(shutil.which(name) == '/workspace/.toolchains/tcl/bin/' + name, 'Original Tcl provider path changed: ' + name)
            checked_file(admission['native_provider_bindings'][name])
            require(Path(admission['native_provider_bindings'][name]['path']) == Path('/workspace/.toolchains/tcl/bin') / name, 'Foreign Native provider binding')
        require(sha(required['TCL_LSP_JIMSH']) == JIM_SHA, 'Original Jim binary changed')

    def current_canonical_inventory():
        # Exact Root repair3 scanner membership; do not substitute --others or
        # a new filesystem walker for the actual build's admitted path universe.
        historical = json.loads(historical_inputs.read_text())
        require(len(historical) == 39002 and len({row['path'] for row in historical}) == 39002, 'Historical39002 input inventory changed')
        union = json.loads(source_admission.read_text())['files']
        require(len(union) == 59 and len({row['path'] for row in union}) == 59, 'Reviewed59 union inventory changed')
        inventory_argv = ['git', 'ls-files', '-z', '--', 'rust', 'runtime', 'scripts', 'specs', 'Cargo.toml', 'Cargo.lock', '.cargo/config.toml']
        tracked = subprocess.check_output(inventory_argv, cwd=ROOT, text=True).split('\0')
        paths = sorted({row['path'] for row in historical} | {row['path'] for row in union} | {path for path in tracked if path})
        require(len(paths) == 39019, 'Canonical actual input membership count changed')
        expected_rows = json.loads(parent_inputs.read_text())
        expected = {row['path']: row['sha256'] for row in expected_rows}
        require(len(expected_rows) == len(expected) == 39019, 'Parent input inventory is not unique39019')
        require(set(paths) == set(expected), 'Canonical actual input membership changed')
        actual = {}
        for relative in paths:
            path = ROOT / relative
            require(path.is_file() and path.resolve().is_relative_to(ROOT.resolve()), 'Missing or escaping canonical source input: ' + relative)
            actual[relative] = sha(path)
        require(actual == expected, 'Canonical actual full-input byte dictionary changed')
        return {'count': len(actual), 'parent_inputs_sha256': sha(parent_inputs), 'membership_and_bytes_equal': True, 'scanner_argv': inventory_argv}

    def check_binding():
        require(sha(admission_path) == args.admission_sha256, 'Final admission changed')
        for record in pins.values():
            checked_file(record)
        require(sha(RUNNER) == RUNNER_SHA, 'Canonical runner changed')
        require(parent_receipt == DESTINATION / 'source-binding.json' and parent_artifacts == DESTINATION / 'artifacts.json' and parent_inputs == DESTINATION / 'input-before.json' and parent_source_diff == DESTINATION / 'source.diff', 'Wrong exact parent receipt paths')
        current_canonical_inventory()
        build = json.loads(parent_receipt.read_text())
        require(build['status'] == 'PASS_FIVE_LIBRARY_BUILD' and build['ordinary'] is True and build['instrumentation'] is False and build['exit_code'] == 0, 'Successful ordinary five-library binding required')
        require(build['input_before_count'] == build['input_after_count'] == 39019 and build['input_drift_count'] == 0, 'Actual frozen source input count/drift mismatch')
        require(build['all_five_artifact_relocations_hash_verified'] is True and build['approved_union_manifest_sha256'] == SOURCE_ADMISSION_SHA, 'Artifact relocation/source admission mismatch')
        require(Path(build['approved_union_manifest']).resolve() == source_admission, 'Source admission path mismatch')
        require(build['test_debug'] == 0 and build['single_job'] is True, 'Original ordinary build mode changed')
        for row in json.loads(source_admission.read_text())['files']:
            require(sha(ROOT / row['path']) == row['after_sha256'], 'Approved source owner drift: ' + row['path'])
        binding = gen.bind_actual_build(DESTINATION, STANDARD, selection['source_evidence'], ROOT)
        require(binding == selection['freeze_binding'], 'Selection is not bound to these actual five libraries')
        require(binding['full_input_count'] == 39019 and set(binding['binary_bindings']) == set(STANDARD), 'Wrong full-input/five-artifact scope')
        check_environment()
        return binding

    binding = check_binding()
    stem = 'retained902-listing-results' if args.list_only else 'retained902-results'
    out = DESTINATION / stem
    receipt_path = DESTINATION / (stem + '-invocation.json')
    require(not out.exists() and not receipt_path.exists(), 'Refuse to overwrite an existing replay/receipt')
    argv = [sys.executable, str(RUNNER), '--manifest', str(plan), '--manifest-sha256', sha(plan), '--artifacts', str(DESTINATION / 'artifacts.json'), '--root', str(ROOT), '--out', str(out)]
    if args.list_only:
        argv.append('--list-only')
    receipt = {
        'status': 'PREFLIGHT_PASS_RETAINED902',
        'list_only': args.list_only,
        'canonical_inventory': current_canonical_inventory(),
        'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'argv': argv, 'selected': 902, 'all726_ordered_rows_and_guards_unchanged': True,
        'admission': str(admission_path), 'admission_sha256': args.admission_sha256,
        'actual_successful_binding': binding, 'provider_environment': required,
        'native_provider_bindings': admission['native_provider_bindings'], 'jim_binary_sha256': JIM_SHA,
        'canonical_runner_sha256': RUNNER_SHA,
        'scope': 'Exact902 standard-five selection capture only. Separate10 extra-artifact controls, host/gates/ordinary acceptance remain outside this invocation. Source/five ELFs/provider bindings stay frozen through postflight.',
    }
    with receipt_path.open('x') as stream:
        stream.write(json.dumps(receipt, indent=2) + '\n')
    code = 1
    try:
        code = subprocess.run(argv, cwd=ROOT, check=False).returncode
    finally:
        receipt['canonical_exit_code'] = code
        try:
            require(check_binding() == binding, 'Frozen source/artifact binding changed during replay')
            receipt['all39019_inputs_and_five_elf_and_provider_postflight_match'] = True
        except Exception as error:
            receipt['postflight_error'] = repr(error)
            code = 1
        receipt['completed_utc'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
        receipt['wrapper_exit_code'] = code
        receipt['status'] = 'CANONICAL_CAPTURE_COMPLETED_POSTFLIGHT_PASS' if code == 0 else 'CANONICAL_CAPTURE_OR_POSTFLIGHT_FAILED'
        receipt['ordinary_acceptance'] = False
        receipt_path.write_text(json.dumps(receipt, indent=2) + '\n')
        print(json.dumps({key: value for key, value in receipt.items() if key != 'actual_successful_binding'}), flush=True)
    return code


if __name__ == '__main__':
    raise SystemExit(main())
