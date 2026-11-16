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
fn original_source_instance_methods_reach_prefix_and_fuzzy_consumers_without_reports() {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    for selector in ["longMet", "longMethd"] {
        let source = format!(
            "oo::class create C {{method longMethod {{}} {{}}}}; set o [C new]; $o {selector}"
        );
        let mut analysis = Analyser::new().analyse(&source, "tcl8.6");
        analysis.all_classes.clear();
        analysis.global_scope.classes.clear();
        analysis.instance_classes.clear();
        analysis.created_instance_commands.clear();
        let items = completions(
            &source,
            0,
            u32::try_from(source.len()).unwrap(),
            &analysis,
            analysis.resolved_registry(),
            None,
            analysis.resolved_profile().unwrap(),
        );
        let method = items
            .iter()
            .find(|item| item.label == "longMethod")
            .expect("source method candidate");
        assert!(
            method
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("source declaration candidate"))
        );
        if selector == "longMet" {
            let edit = method
                .text_edit
                .as_ref()
                .expect("independent complete selector edit");
            assert_eq!(
                edit.start_char,
                u32::try_from(source.rfind(selector).unwrap()).unwrap()
            );
            assert_eq!(edit.end_char, u32::try_from(source.len()).unwrap());
            assert_eq!(edit.new_text, "longMethod");
            assert!(method.filter_text.is_none());
        } else {
            assert_eq!(method.filter_text.as_deref(), Some(selector));
            assert!(
                method
                    .sort_text
                    .as_deref()
                    .is_some_and(|sort| sort.starts_with("F00_"))
            );
        }
    }
    let source = "oo::class create C {method longMethod args {}}; C create object; interp alias {} fixed {} object longMethod; fixed argument";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let items = completions(
        source,
        0,
        u32::try_from(source.len()).unwrap(),
        &analysis,
        analysis.resolved_registry(),
        None,
        analysis.resolved_profile().unwrap(),
    );
    assert!(items.iter().all(|item| item.label != "longMethod"));
}
