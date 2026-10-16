#!/usr/bin/env python3
"""Generate a source-bound exact selection; never invoke Cargo or test binaries."""
import argparse
import copy
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path('/workspace/tcl-lsp')
BASE = Path('/workspace/naming-validation/lsp-source-path-diagnostics-repair/selection.json')
DESTINATION = '/workspace/naming-validation/lsp-readonly-cache-successor-repair2'
TARGETS = {name.replace('-', '_'): ROOT / 'rust' / name / 'src' / 'lib.rs' for name in
           ['tcl-compiler', 'tcl-registry', 'tcl-lsp-core', 'tcl-lsp-db', 'tcl-lsp-server']}
RAW_STRING = re.compile(r'(?:br|cr|r)(#{0,255})"')
CHARACTER = re.compile(r"'(?:[^'\\\n]|\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.))'")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def mask_rust(text):
    """Mask Rust comments/literals while preserving offsets and newlines."""
    out = list(text)
    i = 0
    while i < len(text):
        end = None
        if text.startswith('//', i):
            end = text.find('\n', i)
            if end < 0:
                end = len(text)
        elif text.startswith('/*', i):
            cursor, nesting = i + 2, 1
            while cursor < len(text) and nesting:
                if text.startswith('/*', cursor):
                    nesting += 1
                    cursor += 2
                elif text.startswith('*/', cursor):
                    nesting -= 1
                    cursor += 2
                else:
                    cursor += 1
            assert nesting == 0, 'unterminated source comment'
            end = cursor
        else:
            raw = RAW_STRING.match(text, i) if text[i] in 'rbc' else None
            if raw:
                closing = '"' + raw.group(1)
                close = text.find(closing, raw.end())
                assert close >= 0, 'unterminated raw string'
                end = close + len(closing)
            elif text[i] == '"':
                cursor = i + 1
                while cursor < len(text):
                    if text[cursor] == '\\':
                        cursor += 2
                    elif text[cursor] == '"':
                        cursor += 1
                        break
                    else:
                        cursor += 1
                end = cursor
            elif text[i] == "'":
                char = CHARACTER.match(text, i)
                if char:
                    end = char.end()
        if end is not None:
            for pos in range(i, end):
                if out[pos] != '\n':
                    out[pos] = ' '
            i = end
        else:
            i += 1
    return ''.join(out)


class SourceIndex:
    def __init__(self):
        self.cache = {}

    def load(self, path):
        if path not in self.cache:
            text = path.read_text()
            masked = mask_rust(text)
            depths, stack, pairs = [], [], {}
            for pos, char in enumerate(masked):
                depths.append(len(stack))
                if char == '{':
                    stack.append(pos)
                elif char == '}':
                    assert stack, (path, pos)
                    pairs[stack.pop()] = pos
            assert not stack, path
            self.cache[path] = text, masked, depths, pairs
        return self.cache[path]

    def resolve(self, target, selector, purpose):
        parts = selector.split('::')
        path = TARGETS[target]
        start, end, level = 0, len(path.read_text()), 0
        module_dir = path.parent
        route = []
        for part in parts[:-1]:
            text, masked, depths, pairs = self.load(path)
            matches = [m for m in re.finditer(r'\bmod\s+' + re.escape(part) + r'\s*([;{])', masked, re.M)
                       if start <= m.start() < end and depths[m.start()] == level]
            assert len(matches) == 1, (target, selector, path, part, len(matches))
            match = matches[0]
            route.append({'source_path': str(path.relative_to(ROOT)),
                          'source_sha256': digest(path),
                          'source_line': text.count('\n', 0, match.start()) + 1,
                          'module': part})
            delimiter = match.end() - 1
            if match.group(1) == '{':
                start, end, level = delimiter + 1, pairs[delimiter], level + 1
                module_dir = module_dir / part
            else:
                prefix = text[max(start, match.start() - 1500):match.start()]
                prefix = re.sub(r'pub(?:\([^)]*\))?\s*$', '', prefix)
                tail = re.search(r'((?:\s*#\[[^\]]*\]\s*)*)$', prefix).group(1)
                attr = re.search(r'#\[path\s*=\s*"([^"]+)"\s*\]', tail)
                candidates = [module_dir / attr.group(1)] if attr else [module_dir / (part + '.rs'), module_dir / part / 'mod.rs']
                candidates = [candidate for candidate in candidates if candidate.is_file()]
                assert len(candidates) == 1, (selector, part, candidates)
                path = candidates[0].resolve()
                module_dir = path.parent if path.name == 'mod.rs' else path.parent / path.stem
                start, end, level = 0, len(path.read_text()), 0
        text, masked, depths, pairs = self.load(path)
        matches = [m for m in re.finditer(r'\b(?:async\s+)?fn\s+' + re.escape(parts[-1]) + r'\s*\(', masked)
                   if start <= m.start() < end and depths[m.start()] == level]
        assert len(matches) == 1, (target, selector, path, len(matches))
        match = matches[0]
        assert re.search(r'#\[(?:test|tokio::test|test_case)[^\]]*\]', text[max(start, match.start() - 200):match.start()]), (selector, 'missing actual test attribute')
        return {'target': target, 'selector': selector, 'source_path': str(path.relative_to(ROOT)),
                'source_line': text.count('\n', 0, match.start()) + 1,
                'source_sha256': digest(path), 'purpose': purpose, 'module_route': route}


def generate():
    original = json.loads(BASE.read_text())
    assert original['total_exact_selectors'] == len(original['execution_order']) == 220
    assert len({(r['target'], r['selector']) for r in original['execution_order']}) == 220
    assert Counter(r['target'] for r in original['execution_order']) == original['expected_group_counts']
    original_order = copy.deepcopy(original['execution_order'])
    selection = copy.deepcopy(original)
    index = SourceIndex()
    seen = {(r['target'], r['selector']) for r in original_order}
    additions, requested, evidence, bindings = [], [], [], []

    def bind(path):
        path = Path(path)
        bindings.append({'path': str(path), 'sha256': digest(path)})

    def add(target, selector, purpose, category):
        assert target in TARGETS
        row = {'target': target, 'selector': selector, 'purpose': purpose}
        requested.append(dict(row, category=category, already_retained=(target, selector) in seen))
        if (target, selector) not in seen:
            seen.add((target, selector))
            additions.append(row)
        evidence.append(index.resolve(target, selector, purpose))

    cclass = Path('/tmp/sol-lsp-member-c-source-producer-v2-contract-20261016/manifest.json')
    bind(cclass)
    add('tcl_compiler', json.loads(cclass.read_text())['new_annex']['selector'],
        'Actual retained C class body/source namespace; conditional scope and no lexical-edit/completed-world grant', 'compiler-cclass')
    readonly = Path('/tmp/sol-lsp-readonly-member-reference-v3-20261016/manifest.json')
    bind(readonly)
    rm = json.loads(readonly.read_text())
    for row in rm['new_compiler_annex']:
        add('tcl_compiler', row['selector'], 'Actual selected member/body/schema correspondence and source-only own/inherited selector debts/refusals', 'compiler-member')
    for row in rm['new_core_annex']:
        add('tcl_lsp_core', row['selector'], 'Readonly source spans stay outside strong edit inventories; partial/unavailable counts retain honest display', 'core-readonly')
    bind('/tmp/compiler-logical-source-prefix-words-v1/manifest.json')
    add('tcl_compiler', 'command_binding::logical_operation::tests::logical_source_prefix_words_keep_actual_alias_lineage_without_name_key_policy',
        'Actual Logical captured source words and alias lineage with strict authored-key refusal retained', 'compiler-prefix')
    bind('/tmp/sol-project-arity-debt-v3/manifest.json')
    add('tcl_compiler', 'registry_invocation::source_structure::project_procedure_roles::arity_prefix_tests::project_roles_keep_original_prefix_arity_applicability_separate_from_mutations',
        'Actual earlier selected source-arity debt/current input; no deferred-body or mutation promotion', 'compiler-arity')
    bind('/tmp/sol-lsp-semantic-comment-owner-20261016/manifest.json')
    for name in ['retained_noqa_expression_commands_keep_original_comment_regions',
                 'comment_facts_are_exact_slices_and_never_promote_pre_comment_noqa',
                 'script_comment_facts_reaches_expect_clause_flags_and_lambda_bodies',
                 'script_comment_facts_reaches_definition_member_bodies',
                 'script_comment_facts_reaches_nested_command_substitution_case_arms',
                 'script_comment_facts_uses_proven_alias_identity_for_body_roles',
                 'original_comment_projection_keeps_whole_image_geometry_and_declines_unmapped_text',
                 'cooked_script_comment_markers_have_no_original_physical_line']:
        add('tcl_compiler', 'analyser::utils::tests::' + name,
            'Preserved genuine comment/body owner and physical-line/noqa guards after command-position reporting extension', 'compiler-comment-preserved')
    semantic = Path('/tmp/sol-lsp-semantic-comment-owner-20261016/baseline-annex.json')
    bind(semantic)
    for row in json.loads(semantic.read_text())['new_annexes']:
        add('tcl_lsp_core', row['selector'], 'Actual original semantic comment phase, inline comments and full-input/body refusal controls', 'core-semantic')
    bind('/tmp/compiler-logical-procedure-inlay-v1/manifest.json')
    for name in ['logical_procedure_parameter_hints_use_original_allocation_and_effective_topology',
                 'logical_procedure_parameter_hints_keep_unknown_formals_and_source_currency_terminal']:
        add('tcl_lsp_core', 'inlay_hints::retained_hint_context_tests::' + name,
            'Actual original Logical procedure allocation/formal topology and terminal unknown/currentness', 'core-inlay')
    bind('/tmp/lsp-rename-overlap-plan/manifest.json')
    add('tcl_lsp_core', 'rename::tests::rename_edit_plan_refuses_conflicts_before_utf16_publication',
        'Actual edit-plan overlap/duplicate/conflict refusal before UTF16 publication', 'core-rename')
    bind('/tmp/sol-lsp-index-loader-source-form-v2-20261016/manifest.json')
    add('tcl_lsp_core', 'package_resolver::original_names::index_loader::selected_source_form_tests::index_filename_uses_selected_source_form_before_computed_path_value',
        'Existing selected SourceFile form before computed filename value; actual arity/grammar/path refusals', 'core-index')
    wire = Path('/tmp/sol-lsp-readonly-server-wire-v2-20261016/baseline-annex.json')
    bind(wire)
    for row in json.loads(wire.read_text())['new_annexes']:
        add('tcl_lsp_server', row['selector'], 'Readonly original identity, provider publication/full-input joint currency and partial/unavailable wire reporting', 'server-readonly')
    cache = Path('/tmp/lsp-diagnostic-cache-currency-v3/manifest.json')
    bind(cache)
    for selector in json.loads(cache.read_text())['new_tests']:
        add('tcl_lsp_server', selector, 'Legacy cache retains genuine completed report source/config/version/provider owner before unchanged/empty claims', 'server-cache')
    bind('/tmp/compiler-spectcl-navigation-completeness-v1/manifest.json')
    add('tcl_compiler', 'parsing::spectcl_source::tests::pack_source_navigation_keeps_complete_prefix_without_loader_recovery',
        'Navigation requires genuine complete syntax prefix while preserving loader recovery and original malformed assertions', 'compiler-spectcl-malformed')
    titles = Path('/tmp/sol-lsp-readonly-member-reference-v3-20261016/title-reconciliation-details.json')
    bind(titles)
    title_rows = json.loads(titles.read_text())['cases']
    assert len(title_rows) == 13
    for row in title_rows:
        add('tcl_lsp_core', row['selector'],
            'Preserve original NativeLens exact-title assertion; any partial-label product-contract mismatch remains an ordinary visible failure', 'preserved-native-lens-title')

    selection['execution_order'].extend(additions)
    assert selection['execution_order'][:220] == original_order
    groups = {}
    for row in selection['execution_order']:
        groups.setdefault(row['target'], []).append(row['selector'])
    selection['groups'] = groups
    selection['expected_group_counts'] = {target: len(rows) for target, rows in groups.items()}
    selection['total_exact_selectors'] = len(seen)
    selection['group'] = 'Readonly/cache successor: all220 retained exact controls plus source-bound integrated controls and13 unchanged NativeLens title assertions'
    selection['successor_destination'] = DESTINATION
    selection['retained_predecessor'] = {'path': str(BASE), 'sha256': digest(BASE), 'exact_controls': 220,
                                        'execution_order_prefix_unchanged': True, 'timeout_environment_and_old_guard_metadata_preserved': True}
    selection['reviewed_source_artifact_bindings'] = bindings
    selection['requested_control_reconciliation'] = requested
    selection['successor_addition_source_evidence'] = evidence
    # Refresh a separate evidence set for every retained control; old provenance remains untouched.
    selection['retained_current_source_evidence'] = [index.resolve(row['target'], row['selector'], row['purpose']) for row in original_order]
    selection['runtime_guards'] = {'exact_listing_before_execution': True, 'expected_singleton_tests_per_selector': 1,
                                   'missing_or_multiple_match_is_failure': True, 'never_skip_missing_selector': True,
                                   'list_timeout_seconds': original['listing_timeout_seconds'],
                                   'test_timeout_seconds': original['test_timeout_seconds'],
                                   'preserve_exit_status_stdout_stderr_timeout': True,
                                   'bind_actual_frozen_elf_before_and_after': True}
    selection['host_scope'] = original['host_scope'] + ' Successor adds no real WASI or native Windows-target control; retained platform-spelling pure controls stay present. Paused Runtime/VM/BIG-IP targets remain excluded.'
    selection['selection_only_no_execution'] = True
    assert len(selection['execution_order']) == sum(selection['expected_group_counts'].values()) == len(seen)
    assert all(target in TARGETS for target, _ in seen)
    return selection, {'retained': 220, 'new_deduplicated': len(additions), 'total': len(seen),
                        'by_target': selection['expected_group_counts'],
                        'requested_categories': dict(Counter(row['category'] for row in requested)),
                        'actual_new_categories': dict(Counter(row['category'] for row in requested if not row['already_retained'])),
                        'core_readonly_reconciliation': 'Reviewed V3 adds2 Core and2 Compiler tests; all4 included under actual targets.',
                        'native_lens_titles': {'requested': 13, 'previously_retained': sum(row['category'] == 'preserved-native-lens-title' and row['already_retained'] for row in requested)},
                        'execution': 'Selection generation only. No Cargo, Git, tests, binary listing, native oracle or main writes.'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('/tmp/lsp-readonly-cache-successor-selection/selection.json'))
    args = parser.parse_args()
    selection, summary = generate()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(selection, indent=2) + '\n')
    summary.update(selection_path=str(args.output), selection_sha256=digest(args.output), generator_sha256=digest(Path(__file__)))
    args.output.with_name('selection-generation-summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary, indent=2))
