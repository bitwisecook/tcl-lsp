#!/usr/bin/env python3
"""Read-only excerpts for the static triage seams; run from the repository root."""
from pathlib import Path
import re
import subprocess

print(subprocess.check_output(['date', '-u'], text=True).strip())
print('revision', subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip())
rows = [
    ('N01 identity maps', 'rust/tcl-compiler/src/analyser/types.rs', r'pub all_(procs|classes):', 3),
    ('N08 nested context', 'rust/tcl-compiler/src/inline_uplevel.rs', r'let selected = bindings.analyse_script_at_site', 22),
    ('N10 direct class path', 'rust/tcl-lsp-core/src/definition.rs', r'^pub\(crate\) fn resolve_class_target_at', 105),
    ('N11 test-only library helper', 'rust/tcl-lsp-server/src/lib.rs', r'^    async fn merge_library_files\(', 90),
    ('N11 production library admission', 'rust/tcl-lsp-server/src/lib.rs', r'^    async fn ensure_logical_library_candidates\(', 162),
    ('N11 production require view', 'rust/tcl-lsp-server/src/lib.rs', r'^    async fn required_library_files\(', 75),
    ('N13 propagation bound', 'rust/tcl-lsp-core/src/workspace_index.rs', r'const ROUND_CAP:', 85),
    ('N13 rehoming bound', 'rust/tcl-lsp-server/src/lib.rs', r'for _round in 0\.\.4', 100),
    ('N13 source entry', 'rust/tcl-compiler/src/analyser/state.rs', r'pub fn analyse_with_source_namespace\(', 65),
    ('N14 arity ownership', 'rust/tcl-lsp-db/src/lib.rs', r'fn original_(command_signatures|lookup_command_arities)', 65),
    ('N17 live-fold gate', 'rust/tcl-compiler/src/const_subst.rs', r'fn fold_retained_at_depth\(', 65),
    ('N17 default effects', 'rust/tcl-registry/src/resolved_invocation.rs', r'let mut footprint = if self.semantics.world_effects.is_declared\(\)', 25),
    ('extract guard', 'rust/tcl-lsp-core/src/refactor/extract_variable.rs', r'if !analysis.allows_lexical_declaration_advice\(\)', 18),
    ('inline guard', 'rust/tcl-lsp-core/src/refactor/inline_proc.rs', r'^fn plan_inline\(', 100),
    ('N15 refusal', 'rust/tcl-lsp-core/src/minify/original.rs', r'MissingAlphaRenameContract', 12),
    ('profile seam', 'rust/tcl-compiler/src/analyser/types.rs', r'pub fn ns_var_global_fallback\(', 20),
    ('registry seam', 'rust/tcl-lsp-core/src/workspace_index.rs', r'^    fn registry(_for)?\(', 20),
    ('class reanalysis seam', 'rust/tcl-lsp-server/src/lib.rs', r'^    async fn analyse_with_workspace_classes_from_analysis\(', 100),
    ('method barrier seam', 'rust/tcl-compiler/src/optimiser/method_barrier.rs', r'^fn hierarchy_components\(', 100),
    ('linked editing seam', 'rust/tcl-lsp-core/src/original_command_rename/linked_authoring.rs', r'^impl.*OriginalLinkedSourceSpelling', 110),
    ('4.1 workspace existence', 'rust/tcl-lsp-core/src/workspace_index.rs', r'pub fn workspace_command_exists_for_call\(', 65),
    ('4.1 source order paths', 'rust/tcl-lsp-core/src/source_graph.rs', r'^pub struct RunOrder', 70),
]
for label, filename, needle, count in rows:
    print('\n###', label, filename)
    lines = Path(filename).read_text().splitlines()
    matches = [i for i, line in enumerate(lines) if re.search(needle, line)]
    if not matches:
        print('NO MATCH:', needle)
    for start in matches:
        for i in range(max(0, start - 2), min(len(lines), start + count)):
            print(f'{i + 1}: {lines[i]}')
