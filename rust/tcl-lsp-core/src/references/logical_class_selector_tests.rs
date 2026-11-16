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
fn logical_class_selector_keeps_actual_alias_and_rename_source_lineage() {
    // naming.core.logical-call-target-owner
    // docs/design/analysis/name-resolution-proofs/logical-call-target-owner.md
    for tail in [
        "C m",
        "rename C D; D m",
        "interp alias {} invoke {} C; invoke m",
    ] {
        let source = format!("oo::class create C {{self method m {{}} {{}}}}; {tail}");
        let analysis = Analyser::new().analyse(&source, "tcl");
        let receiver = analysis
            .all_classes
            .values()
            .find(|class| class.name == "C")
            .unwrap();
        let sites = logical_class_command_method_sites(&source, &analysis, receiver, "m");
        assert_eq!(sites.len(), 1, "{tail}: {sites:?}");
        assert_eq!(
            &source[sites[0].start() as usize..sites[0].end() as usize],
            "m"
        );
    }
}

#[test]
fn captured_selectors_and_replaced_class_commands_have_no_written_selector_grant() {
    // naming.core.logical-call-target-owner
    // docs/design/analysis/name-resolution-proofs/logical-call-target-owner.md
    for tail in [
        "interp alias {} invoke {} C m; invoke",
        "proc C args {}; C m",
        "rename C {}; C m",
    ] {
        let source = format!("oo::class create C {{self method m {{}} {{}}}}; {tail}");
        let analysis = Analyser::new().analyse(&source, "tcl");
        let receiver = analysis
            .all_classes
            .values()
            .find(|class| class.name == "C")
            .unwrap();
        assert!(
            logical_class_command_method_sites(&source, &analysis, receiver, "m").is_empty(),
            "{tail}"
        );
    }
}

#[test]
fn logical_selector_preserves_full_source_and_unicode_and_refuses_native_donation() {
    // naming.core.logical-call-target-owner
    // docs/design/analysis/name-resolution-proofs/logical-call-target-owner.md
    let source = "oo::class create C {self method mβ::x {} {}}; C mβ::x";
    let mut analysis = Analyser::new().analyse(source, "tcl");
    let receiver = &analysis.all_classes["::C"];
    let expected = logical_class_command_method_sites(source, &analysis, receiver, "mβ::x");
    assert_eq!(expected.len(), 1);
    assert_eq!(
        &source[expected[0].start() as usize..expected[0].end() as usize],
        "mβ::x"
    );
    assert_eq!(
        logical_class_command_method_sites(
            &source.replace("mβ::x", "other"),
            &analysis,
            receiver,
            "mβ::x"
        ),
        [] as [tcl_lexer::Span; 0]
    );
    analysis.dialect = "copied reporting label".into();
    assert_eq!(
        logical_class_command_method_sites(
            source,
            &analysis,
            &analysis.all_classes["::C"],
            "mβ::x"
        ),
        expected
    );
    for dialect in ["tcl8.6", "f5-irules"] {
        let other = Analyser::new().analyse(source, dialect);
        assert!(
            logical_class_command_method_sites(
                source,
                &other,
                &analysis.all_classes["::C"],
                "mβ::x"
            )
            .is_empty(),
            "{dialect} cannot borrow a Logical receiver"
        );
    }
    analysis.resolved_input = None;
    assert_eq!(
        logical_class_command_method_sites(
            source,
            &analysis,
            &analysis.all_classes["::C"],
            "mβ::x"
        ),
        [] as [tcl_lexer::Span; 0]
    );
}
