#!/usr/bin/env python3
"""Prepare a source-bound TMP selection; freeze only after Root's actual five-library build.

Never list or execute a test, invoke Cargo/Git, or alter Rust source. Draft mode
has no binary bindings. --freeze reads only the actual successor's retained
input/source receipts and frozen artifact manifest, verifies all five ELF hashes,
and binds them. Runtime listing/singleton/before-after ELF checks stay mandatory.
"""
import argparse
import copy
import hashlib
import importlib.util
import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path('/workspace/tcl-lsp')
BASE = Path('/workspace/naming-validation/lsp-folding-completion-links-successor/selection.json')
OLD_LISTS = BASE.parent / 'results'
DESTINATION = Path('/workspace/naming-validation/lsp-selected-owners-successor-repair1')
PREVIOUS_GENERATOR = Path('/tmp/lsp-folding-completion-links-selection/generate_selection.py')
loader = importlib.util.spec_from_file_location('retained_folding_source_index', PREVIOUS_GENERATOR)
previous = importlib.util.module_from_spec(loader)
loader.loader.exec_module(previous)
TARGETS = previous.previous.TARGETS
GUARD_KEYS = ['test_timeout_seconds', 'listing_timeout_seconds', 'environment',
              'runtime_guards', 'validation_note', 'host_scope']


def digest(path):
    value = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(8 * 1024 * 1024), b''):
            value.update(chunk)
    return value.hexdigest()


def parse_listing(path):
    rows = []
    for line in path.read_text().splitlines():
        if line.endswith(': test'):
            rows.append(line[:-6])
    assert rows, ('empty actual frozen listing', path)
    return Counter(rows)


def generate(freeze=False):
    original = json.loads(BASE.read_text())
    assert original['total_exact_selectors'] == len(original['execution_order']) == 310
    original_order = copy.deepcopy(original['execution_order'])
    seen = {(row['target'], row['selector']) for row in original_order}
    assert len(seen) == 310
    assert Counter(row['target'] for row in original_order) == original['expected_group_counts']
    selection = copy.deepcopy(original)
    index = previous.previous.SourceIndex()
    additions, requests, bindings, exclusions = [], [], [], []
    old_listings = {target: parse_listing(OLD_LISTS / f'{target}.list') for target in TARGETS}
    for path in [PREVIOUS_GENERATOR, previous.PREVIOUS_GENERATOR, BASE.parent / 'artifacts.json']:
        bindings.append({'path': str(path), 'sha256': digest(path),
                         'role': 'Retained source-index dependency or predecessor-listing provenance; not a current ELF binding'})

    def bind(path):
        path = Path(path)
        bindings.append({'path': str(path), 'sha256': digest(path)})
        return json.loads(path.read_text()) if path.suffix == '.json' else path.read_text()

    def add(target, selector, purpose, category, old=False):
        assert target in TARGETS
        if old:
            assert old_listings[target][selector] == 1, ('old control must exist once in actual frozen listing', target, selector)
        row = {'target': target, 'selector': selector, 'purpose': purpose}
        requests.append(dict(row, category=category, already_retained=(target, selector) in seen,
                             prior_frozen_listing_singleton=old))
        if (target, selector) not in seen:
            seen.add((target, selector))
            additions.append(row)
        # Every requested control, including already-retained ones, must resolve
        # through its actual current source module route and test declaration.
        index.resolve(target, selector, purpose)

    member = bind('/tmp/sol-lsp-readonly-member-source-reborrow-v1/manifest.json')
    add('tcl_compiler', member['new_control'],
        'Original readonly member correspondence retains both actual issuers and the stronger query result', 'new-member-correspondence')
    cclass = bind('/tmp/compiler-c-class-world-premise-v1/manifest.json')
    add('tcl_compiler', cclass['new_exact_selector'],
        'C class source query preserves an independently completed world premise', 'new-c-class-supplement')
    registry = bind('/tmp/registry-instance-method-provider-v3/manifest.json')
    for selector in registry['new_controls']:
        target = 'tcl_registry' if selector.startswith('registry::') else 'tcl_lsp_core'
        add(target, selector, 'Exact inherited provider descriptor/package floors and complete actual-context/roster admission', 'new-registry-v3')
    bind('/tmp/registry-instance-method-context-v2/manifest.json')
    context_modules = [
        ('tcl_registry', 'rust/tcl-registry/src/registry.rs', 'registry', ['inherited_instance_context_tests']),
        ('tcl_compiler', 'rust/tcl-compiler/src/command_binding/source_transition_advice/registered_instance.rs',
         'command_binding::source_transition_advice::registered_instance', ['inherited_registered_provider_tests']),
    ]
    context_count = 0
    for target, path, prefix, modules in context_modules:
        tests, omitted = previous.module_tests(index, path, prefix, modules)
        assert not omitted
        for row in tests:
            add(target, row['selector'], 'Actual full ContextRegistry provider ownership through schema/form/options/prefix and diagnostic arity; typed unavailable cannot become unknown method', 'new-context-v2')
            context_count += 1
    assert context_count == 4
    bind('/tmp/lsp-pack-notice-range-v2/manifest.json')
    for suffix in ['pack_notice_range_uses_retained_source_utf16_and_whole_file_unavailability',
                   'pack_notice_range_projects_original_byte_anchor_across_lone_cr']:
        add('tcl_lsp_server', 'original_document_metadata_tests::' + suffix,
            'Genuine loader subject byte anchor and shared UTF16 whole-line geometry; missing/mismatched source remains whole-file anchor', 'new-range-v2')
    package = bind('/tmp/lsp-package-registration-transport-v1/manifest.json')
    for row in package['new_selectors']:
        assert row['expected_count'] == 1 and row['budget_seconds'] == 180
        add('tcl_lsp_core', row['selector'], 'Already-selected standalone registration keeps whole original source/vector/store/configuration and genuine nested survey ownership', 'new-package-registration')
    for row in package['retained_selectors']:
        target = row['target'].split()[0].replace('-', '_')
        add(target, row['selector'], 'Preserved original package-registration and startup/factory publication guards', 'preserved-package-manifest', old=True)
    forward = bind('/tmp/lsp-forward-path-projection-v1/manifest.json')
    assert forward['sha256'] == '9a4527343f803ae1ab403d29d6bfdc5e000fcda58adc71a926336a5f03406cf7'
    clear = bind('/tmp/lsp-forward-path-projection-v1/independent-review-CLEAR.txt')
    assert 'CLEAR' in clear
    add('tcl_lsp_core', 'file_uri::vscode_forward_projection_tests::product_alias_is_explicit_and_does_not_replace_lossless_path_spelling',
        'Explicit product URI alias retains independent lossless original host path', 'new-forward-path')
    add('tcl_lsp_server', 'tests::constructed_product_alias_retains_the_original_counted_path_for_admission',
        'Actual original counted path and provider admission survive product alias projection; stronger virtual rebinding refuses', 'new-forward-path')
    comment = bind('/tmp/luna-comment-conditional-source/manifest-v2.json')
    assert comment['patch_sha256'] == 'e6c39377d5e591ec93777a8b5c020aba923ca5dd8f19af81676ec3307270636f'
    patch = Path('/tmp/luna-comment-conditional-source/conditional-source-comment-walk-v2.patch')
    bindings.append({'path': str(patch), 'sha256': digest(patch)})
    assert not re.search(r'^\+\s*#\[(?:test|tokio::test)', patch.read_text(), re.M)
    assert ('tcl_compiler', comment['preserved_regression_selector']) in seen
    add('tcl_compiler', comment['preserved_regression_selector'],
        'Preserved original232 comment control traverses genuine conditional case-body candidates for both known and unknown applicability', 'preserved-comment-v2', old=True)

    package_controls = [
        'package_resolver::tests::package_registration_names_and_versions_use_original_values_and_selected_roles',
        'package_resolver::tests::parse_pkg_index_standard_form',
        'package_resolver::tests::parse_pkg_index_quoted_and_dir_slash_forms',
        'package_resolver::tests::parse_pkg_index_survives_deeply_nested_source_wrapper',
        'package_resolver::tests::parse_pkg_index_rejects_bad_version_and_falls_back_to_dir_listing',
        'package_resolver::reachability::tests::configured_body_values_and_command_children_keep_distinct_original_grammar',
        'package_resolver::reachability::tests::a_declaration_inside_an_apply_lambda_and_a_foreach_is_conditional',
        'package_resolver::reachability::tests::a_dynamic_lambda_is_not_descended',
        'package_resolver::reachability::tests::a_constant_false_branch_never_registers_1017',
        'package_resolver::reachability::tests::the_body_descent_leaves_top_level_declarations_alone',
        'package_resolver::original_names::tests::original_package_keys_keep_opaque_names_and_nested_source_owners',
        'package_resolver::original_names::tests::original_package_advice_keeps_unowned_reports_and_metadata_distinct',
    ]
    for selector in package_controls:
        add('tcl_lsp_core', selector,
            'Original standalone package registration/name/version/body/configuration/control-flow positive and refusal guards; original-analysis ownership remains separate',
            'preserved-package-registration-scope', old=True)
    # Whole URI modules were already retained in310 except this exact original
    # authority refusal. Enumerate the actual frozen listing and add omissions;
    # no name substring sweep into unrelated provider/runtime/F5 targets.
    uri_prefixes = {'tcl_lsp_core': ['file_uri::'], 'tcl_lsp_server': ['uri_norm::']}
    for target, prefixes in uri_prefixes.items():
        listing = OLD_LISTS / f'{target}.list'
        bindings.append({'path': str(listing), 'sha256': digest(listing)})
        for selector in old_listings[target]:
            if any(selector.startswith(prefix) for prefix in prefixes):
                add(target, selector, 'Original URI/path roundtrip, counted data, escaping, authority and product repair/refusal guards', 'preserved-uri-scope', old=True)
    for target in ['tcl_lsp_core', 'tcl_lsp_server', 'tcl_compiler']:
        listing = OLD_LISTS / f'{target}.list'
        if not any(row['path'] == str(listing) for row in bindings):
            bindings.append({'path': str(listing), 'sha256': digest(listing)})

    selection['execution_order'].extend(additions)
    assert selection['execution_order'][:310] == original_order
    groups = {}
    for row in selection['execution_order']:
        groups.setdefault(row['target'], []).append(row['selector'])
    selection['groups'] = groups
    selection['expected_group_counts'] = {key: len(value) for key, value in groups.items()}
    selection['total_exact_selectors'] = len(seen)
    selection['group'] = 'Selected-owner successor: all310 retained controls plus reviewed member/class/provider/context/range/package/path controls and scoped original regressions'
    selection['successor_destination'] = str(DESTINATION)
    selection['selected_owners_predecessor_binding'] = {'path': str(BASE), 'sha256': digest(BASE), 'exact_controls': 310,
        'all310_ordered_entries_verbatim': True, 'all_old_guard_environment_timeout_metadata_verbatim': True}
    selection['selected_owners_reviewed_source_artifact_bindings'] = bindings
    selection['selected_owners_requested_control_reconciliation'] = requests
    evidence = [index.resolve(row['target'], row['selector'], row['purpose']) for row in selection['execution_order']]
    selection['selected_owners_current_source_evidence'] = evidence
    sources = {}
    for row in evidence:
        for source in [row] + row['module_route']:
            name, value = source['source_path'], source['source_sha256']
            assert name not in sources or sources[name] == value
            sources[name] = value
    selection['selected_owners_current_source_files'] = [{'path': name, 'sha256': value} for name, value in sorted(sources.items())]
    selection['selected_owners_freeze_binding'] = {'status': 'AWAITING_ROOT_COMPLETED_BUILD', 'destination': str(DESTINATION),
        'binary_bindings': None, 'no_guessed_or_predecessor_elf': True}
    if freeze:
        artifacts = DESTINATION / 'artifacts.json'
        inputs = DESTINATION / 'input-before.json'
        source_binding = DESTINATION / 'source-binding.json'
        actual = json.loads(artifacts.read_text())
        assert set(actual) == set(TARGETS), ('require exactly actual five library artifacts', list(actual))
        build = json.loads(source_binding.read_text())
        assert build['ordinary'] is True and build['instrumentation'] is False
        assert build['exit_code'] == 0 and build['input_drift_count'] == 0
        assert len(build['packages']) == 5 and {name.replace('-', '_') for name in build['packages']} == set(TARGETS)
        frozen = {row['path']: row['sha256'] for row in json.loads(inputs.read_text())}
        assert all(frozen.get(name) == value for name, value in sources.items()), [name for name, value in sources.items() if frozen.get(name) != value]
        for target, row in actual.items():
            binary = Path(row['path'])
            assert binary.resolve().is_relative_to((DESTINATION / 'frozen-binaries').resolve())
            assert binary.stat().st_size == row['bytes'] and digest(binary) == row['sha256'], ('actual frozen artifact digest mismatch', target)
        selection['selected_owners_freeze_binding'] = {'status': 'BOUND_ACTUAL_ROOT_FIVE_LIBRARY_BUILD',
            'destination': str(DESTINATION), 'binary_bindings': actual,
            'artifacts_manifest': {'path': str(artifacts), 'sha256': digest(artifacts)},
            'input_manifest': {'path': str(inputs), 'sha256': digest(inputs)},
            'source_binding': {'path': str(source_binding), 'sha256': digest(source_binding)},
            'all_current_source_files_and_module_routes_match_frozen_input': True,
            'no_guessed_or_predecessor_elf': True}
    for key in GUARD_KEYS:
        assert selection[key] == original[key]
    assert len(selection['execution_order']) == len(seen) == sum(selection['expected_group_counts'].values())
    assert set(selection['groups']) == set(TARGETS)
    new_categories = Counter(row['category'] for row in requests if not row['already_retained'])
    assert {key: new_categories[key] for key in [
        'new-member-correspondence', 'new-c-class-supplement', 'new-registry-v3',
        'new-context-v2', 'new-range-v2', 'new-package-registration', 'new-forward-path'
    ]} == {
        'new-member-correspondence': 1, 'new-c-class-supplement': 1, 'new-registry-v3': 3,
        'new-context-v2': 4, 'new-range-v2': 2, 'new-package-registration': 2, 'new-forward-path': 2,
    }
    assert sum(new_categories.values()) == len(additions) == 28 and len(seen) == 338
    assert not any(target in selection['groups'] for target in ['tcl_vm', 'tcl_runtime', 'f5_query'])
    summary = {'retained': 310, 'new_deduplicated': len(additions), 'total': len(seen),
        'by_target': selection['expected_group_counts'],
        'requested_categories': dict(Counter(row['category'] for row in requests)),
        'actual_new_categories': dict(Counter(row['category'] for row in requests if not row['already_retained'])),
        'current_source_files_and_module_routes': len(sources),
        'all310_prior_entries_and_guard_timeout_environment_metadata_unchanged': True,
        'freeze_status': selection['selected_owners_freeze_binding']['status'],
        'comment_v2_new_tests': 0, 'comment_original232_retained': True,
        'old_control_source': 'Actual prior frozen listings, each exact singleton; no test listing executed by this generator.',
        'scope': 'Only five retained library targets; no Runtime/VM/F5 dedicated targets or controls newly selected.',
        'execution': 'Python selection/source checks only; no test/listing/Cargo/Git/native execution or main-source writes.'}
    return selection, summary


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('/tmp/lsp-selected-owners-selection/selection-draft.json'))
    parser.add_argument('--freeze', action='store_true', help='Bind only Root completed actual successor artifact/source receipts.')
    args = parser.parse_args()
    selection, summary = generate(args.freeze)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(selection, indent=2) + '\n')
    summary.update(selection_path=str(args.output), selection_sha256=digest(args.output),
                   generator_sha256=digest(Path(__file__)),
                   source_index_generator={'path': str(PREVIOUS_GENERATOR), 'sha256': digest(PREVIOUS_GENERATOR)})
    args.output.with_name('selection-generation-summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary, indent=2))
