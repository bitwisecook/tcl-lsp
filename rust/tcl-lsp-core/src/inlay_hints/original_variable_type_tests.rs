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
use tcl_compiler::analyser::Analyser;

fn hints(source: &str, analysis: &AnalysisResult) -> Vec<InlayHint> {
    inlay_hints(
        source,
        crate::profile_for_dialect("tcl8.6"),
        LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 100,
            end_character: 0,
        },
        Some(analysis),
        Some(crate::registry_for_dialect("tcl8.6")),
        true,
        false,
    )
}

#[test]
// Implementation contract: naming.variable.original-ssa-definition-operand
// docs/design/analysis/name-resolution-proofs/original-ssa-definition-operand.md
fn original_type_hints_match_exact_ssa_cells_without_reporting_variable_maps() {
    let source = "set v\\uD800 42\nset v\\uD801 TEXT\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    analysis.global_scope.variables.clear();
    analysis.all_procs.clear();
    let hints = hints(source, &analysis);
    assert!(
        hints
            .iter()
            .any(|hint| hint.position_line == 0 && hint.label == ": int"),
        "{hints:?}"
    );
    assert!(
        hints
            .iter()
            .any(|hint| hint.position_line == 1 && hint.label == ": str"),
        "{hints:?}"
    );
    assert!(hints.iter().all(
        |hint| (hint.position_line, hint.label.as_str()) != (0, ": str")
            && (hint.position_line, hint.label.as_str()) != (1, ": int")
    ));
    assert_eq!(
        super::original_variable_type_tests::hints(
            &format!("{source}# changed elsewhere"),
            &analysis
        ),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert_eq!(
        super::original_variable_type_tests::hints(source, &missing),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
    analysis.original_variable_symbols.clear();
    assert_eq!(
        super::original_variable_type_tests::hints(source, &analysis),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
}

#[test]
fn original_type_hints_keep_local_frames_and_alias_targets_separate() {
    fn clear(scope: &mut Scope) {
        scope.variables.clear();
        for child in &mut scope.children {
            clear(child);
        }
    }

    let source = "set shared 7\nproc first {} {set value 42; global shared; set shared 8}\nproc second {} {set value TEXT}\nfirst; second\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    clear(&mut analysis.global_scope);
    analysis.all_procs.clear();
    let hints = hints(source, &analysis);
    assert!(
        hints
            .iter()
            .any(|hint| hint.position_line == 1 && hint.label == ": int"),
        "{hints:?}"
    );
    assert!(
        hints
            .iter()
            .any(|hint| hint.position_line == 2 && hint.label == ": str"),
        "{hints:?}"
    );
    assert!(
        hints
            .iter()
            .all(|hint| hint.position_line != 2 || hint.label != ": int")
    );
}
