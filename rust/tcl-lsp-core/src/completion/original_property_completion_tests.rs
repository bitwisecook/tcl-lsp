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
use tcl_compiler::analyser::types::MemberSide;

#[test]
fn original_property_method_advice_uses_selected_grammar_and_own_source_ledger() {
    let source = r"oo::configurable create C {property p\uD800 -kind readable p\uD801 -kind writable; method own {} {}}";
    let mut analysis = Analyser::new().analyse(source, "tcl9.1");
    analysis.all_classes.clear();
    analysis.global_scope.classes.clear();
    let record = analysis.original_class_declarations().next().unwrap();
    assert_eq!(
        record
            .metadata()
            .original_properties
            .properties(MemberSide::Instance)
            .unwrap()
            .len(),
        2
    );
    let items = original_method_candidate_items(&analysis, record, "", MemberSide::Instance);
    let names = items
        .iter()
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    // tclsh 9.0.4 and 9.1.0 (ASCII names for the escapes):
    //   oo::configurable create C {property pa -kind readable pb -kind writable; method own {} {}}
    //   lsort [info object methods [C new] -all]  ->  configure destroy own
    assert_eq!(names, ["configure", "destroy", "own"]);
    assert!(!names.contains(&"cget"));
    let accessor = items.iter().find(|item| item.label == "configure").unwrap();
    assert_eq!(
        accessor.detail.as_deref(),
        Some("method — property declaration candidate")
    );
    assert_eq!(
        original_method_candidate_items(&analysis, record, "", MemberSide::ClassObject),
        [] as [crate::completion::CompletionItem; 0]
    );
    let ordinary = Analyser::new().analyse("oo::class create C {method own {} {}}", "tcl9.1");
    let record = ordinary.original_class_declarations().next().unwrap();
    // tclsh 8.6.18 and 9.1.0: lsort [info object methods [C new] -all]  ->  destroy own
    assert_eq!(
        original_method_candidate_items(&ordinary, record, "", MemberSide::Instance)
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["destroy", "own"]
    );
}
