// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn graph() -> AdviceGraph {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context.clone(),
        LexerConfig::for_file_grammar(profile.grammar),
    );
    AdviceGraph::new(
        &context,
        &AdviceNamingPolicy::Logical(input),
        InvocationDialect::of_profile(profile),
    )
    .unwrap()
}

fn original_words() -> Vec<NativeWord> {
    let config = LexerConfig::for_file_grammar(tcl_dialect::DialectProfile::plain_tcl().grammar);
    tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document("opaque"),
        tcl_lexer::Span::new(0, 6),
        config,
    )
    .unwrap()
    .commands
    .remove(0)
    .words
}

#[test]
fn cloned_advice_cells_share_storage_and_mutation_preserves_old_snapshot() {
    let mut current = graph();
    let old = current.clone();
    assert!(current.cells.shares_storage(&old.cells));
    let key = AdviceNamingPolicy::logical_slot(b"::retained_cell_storage_control").unwrap();
    assert!(!old.cells.contains_key(&key));
    current.cells.insert(key.clone(), AdviceCell::Shadowed);
    assert!(!current.cells.shares_storage(&old.cells));
    assert!(!old.cells.contains_key(&key));
    assert!(matches!(
        current.cells.get(&key),
        Some(AdviceCell::Shadowed)
    ));
    assert_eq!(current.cells.len(), old.cells.len() + 1);
}

#[test]
fn widening_with_no_cell_removals_keeps_shared_storage() {
    let current = graph();
    let mut branch = current.clone();
    assert!(branch.cells.shares_storage(&current.cells));
    branch.widen(&original_words());
    assert!(branch.cells.shares_storage(&current.cells));
    assert_eq!(&*branch.cells, &*current.cells);
    assert_eq!(
        current.uncertain_operations,
        [] as [std::sync::Arc<[tcl_lexer::NativeWord]>; 0]
    );
    assert_eq!(branch.uncertain_operations.len(), 1);
    assert_eq!(branch.uncertain_operations[0][0].bytes(), b"opaque");
}

#[test]
fn widening_detaches_removed_lineage_but_retains_imported_lineage() {
    let mut current = graph();
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let source = "rename list moved; moved value";
    let config = LexerConfig::for_file_grammar(tcl_dialect::DialectProfile::plain_tcl().grammar);
    let bindings = super::super::SourceCommandBindings::analyse(source, config, context.commands());
    let tape = bindings
        .original_source_transition_advice_tape(&context)
        .unwrap();
    let receipt = tape
        .advice
        .values()
        .find_map(|advice| advice.lineage().first().cloned())
        .unwrap();
    let removed = AdviceNamingPolicy::logical_slot(b"::removed_storage_control").unwrap();
    let imported = AdviceNamingPolicy::logical_slot(b"::imported_storage_control").unwrap();
    let origin = AdviceNamingPolicy::logical_slot(b"::origin_storage_control").unwrap();
    for key in [&removed, &imported] {
        current.cells.insert(
            key.clone(),
            AdviceCell::Descriptor("list".into(), vec![Arc::clone(&receipt)]),
        );
    }
    current.namespace_imports.insert(imported.clone(), origin);
    let old = current.clone();
    current.widen(&original_words());
    assert!(!current.cells.shares_storage(&old.cells));
    assert!(!current.cells.contains_key(&removed));
    assert!(old.cells.contains_key(&removed));
    assert_eq!(current.cells.get(&imported), old.cells.get(&imported));
    assert_eq!(current.cells.len() + 1, old.cells.len());
    assert_eq!(
        old.uncertain_operations,
        [] as [std::sync::Arc<[tcl_lexer::NativeWord]>; 0]
    );
}
