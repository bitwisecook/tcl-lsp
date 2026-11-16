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

#[test]
fn original_procedure_hints_share_expanded_children_and_formal_binding_topology() {
    // naming.source.original-procedure-argument-topology
    // docs/design/analysis/name-resolution-proofs/original-procedure-argument-topology.md
    for source in [
        "proc target {captured written} {}; target {*}{FIXED VALUE}",
        "proc target {captured written} {}; interp alias {} alias {} target FIXED; alias {*}{VALUE}",
    ] {
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        analysis.command_invocations.clear();
        let hints = inlay_hints(
            source,
            analysis.resolved_profile().unwrap(),
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 100,
                end_character: 0,
            },
            Some(&analysis),
            None,
            false,
            true,
        );
        let hint = hints
            .iter()
            .find(|hint| hint.label == "written:")
            .expect("actual child hint");
        let expected = LineIndex::new(source).position_at_utf16(
            u32::try_from(source.rfind("VALUE").unwrap()).unwrap(),
            source,
        );
        assert_eq!(hint.position_line, expected.line);
        assert_eq!(hint.position_character, expected.character.get());
        if source.contains("interp alias") {
            assert!(!hints.iter().any(|hint| hint.label == "captured:"));
        }
    }
    let source = "proc target {first {second DEFAULT}} {}; target ONLY";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let hints = inlay_hints(
        source,
        analysis.resolved_profile().unwrap(),
        LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 100,
            end_character: 0,
        },
        Some(&analysis),
        None,
        false,
        true,
    );
    let written = LineIndex::new(source).position_at_utf16(
        u32::try_from(source.rfind("ONLY").unwrap()).unwrap(),
        source,
    );
    assert_eq!(
        hints
            .iter()
            .filter(|hint| hint.position_line == written.line
                && hint.position_character == written.character.get())
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>(),
        ["first:"]
    );
    assert!(!hints.iter().any(|hint| hint.label == "second:"));
    assert_eq!(
        inlay_hints(
            &format!("#{source}"),
            analysis.resolved_profile().unwrap(),
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 100,
                end_character: 0,
            },
            Some(&analysis),
            None,
            false,
            true
        ),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
}
