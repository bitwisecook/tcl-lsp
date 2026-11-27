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

use tcl_compiler::analyser::{Analyser, AnalysisResult};

fn analyse(source: &str, per_item: bool) -> AnalysisResult {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    let mut analyser = Analyser::new();
    if per_item {
        analyser.analyse_per_item(source, "jim")
    } else {
        analyser.analyse(source, "jim")
    }
}

fn warns_at(result: &AnalysisResult, offset: u32) -> bool {
    result
        .diagnostics
        .iter()
        .any(|d| d.code.as_str() == "W123" && d.span.start() == offset)
}

#[test]
fn original_single_parent_definition_prefix_issues_first_entry_absence() {
    let source = "# tcl-dialect: jim\nclass Point {x 0 y 0}\nclass Point3 Point {z 0}\nPoint method norm {} { return [expr {abs($x) + abs($y)}] }\nPoint3 method who {} { not_a_command_anywhere $self $x $z }\n";
    let offset = u32::try_from(source.find("not_a_command_anywhere").unwrap()).unwrap();
    for per_item in [false, true] {
        let result = analyse(source, per_item);
        assert!(
            warns_at(&result, offset),
            "per_item={per_item}: {:?}",
            result.diagnostics
        );
        let bindings = result
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        assert_eq!(
            bindings.diagnostic_slot_presence_at(offset),
            tcl_compiler::command_binding::SourceCommandSlotPresence::unavailable()
        );
        let invocation = bindings
            .future_body_inventories()
            .iter()
            .find_map(|f| f.invocation_at(offset))
            .unwrap();
        assert!(invocation.proved_execution_target().is_none());
    }
}

#[test]
fn prior_body_effects_substitutions_and_generated_heads_remain_unknown() {
    for body in [
        "set q value; missing_first_boundary $self",
        "if {[exists -command {::<reference.<C______>.00000000000000000000>}]} {proc missing_first_boundary {} {}}; missing_first_boundary",
        "missing_first_boundary [ref value tag finalizer]",
        "missing_first_boundary $a(k)",
        "missing_first_boundary $foreign",
        "{::<reference.<C______>.00000000000000000000>}",
    ] {
        let source = format!("class C {{x 0}}; C method m {{}} {{{body}}}");
        let result = analyse(&source, false);
        assert!(
            result.diagnostics.iter().all(|d| d.code.as_str() != "W123"),
            "{source}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn unknown_history_overrides_and_conflicting_links_do_not_issue_absence() {
    for source in [
        "ref value tag finalizer; class C {}; C method m {} {missing_entry}",
        "class C {}; C method m {} {missing_entry}; ref value tag finalizer",
        "opaque; class C {}; C method m {} {missing_entry}",
        "class A {}; class B {}; class C A B {}; C method m {} {missing_entry}",
        "class C {}; C method constructor {} {proc missing_entry {} {}}; C method m {} {missing_entry}",
        "class C {self 0}; C method m {} {missing_entry}",
        "class C {x 0}; C method m {x} {missing_entry}",
        "class C {}; C method m {} {missing_entry}; proc lsort args {}",
    ] {
        let result = analyse(source, false);
        let offset = u32::try_from(source.rfind("missing_entry").unwrap()).unwrap();
        assert!(
            !warns_at(&result, offset),
            "{source}: {:?}",
            result.diagnostics
        );
    }
}
