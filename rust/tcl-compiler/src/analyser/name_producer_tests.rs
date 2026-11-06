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

//! Naming producers retain selected original schemas and written operand origins.

use super::Analyser;

#[test]
fn original_rule_call_producer_uses_selected_option_grammar_and_complete_target_word() {
    // naming.consumer.original-rule-reference-candidates
    // docs/design/analysis/name-resolution-proofs/original-rule-reference-candidates.md
    let source = "proc helper {} {}\nwhen RULE_INIT {call -debug {helper}; call helper}";
    let result = Analyser::new().analyse(source, "f5-irules");
    let targets = result
        .command_invocations
        .iter()
        .filter(|invocation| invocation.name == "helper" && invocation.argc.is_none())
        .collect::<Vec<_>>();
    assert_eq!(targets.len(), 2, "{:?}", result.command_invocations);
    assert!(targets.iter().any(|invocation| &source
        [invocation.range.start() as usize..invocation.range.end() as usize]
        == "{helper}"));
    assert!(targets.iter().all(|invocation| !invocation.rename_safe));
    let unknown = "when RULE_INIT {call $options helper}";
    let result = Analyser::new().analyse(unknown, "f5-irules");
    assert!(
        !result
            .command_invocations
            .iter()
            .any(|invocation| invocation.name == "helper" && invocation.argc.is_none())
    );
}

#[test]
fn original_namespace_producer_preserves_alias_written_origins_and_known_shadowing() {
    // naming.source.original-point-operand-projection
    // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
    let source = "namespace eval N {}\ninterp alias {} check {} namespace exists\ncheck N";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let reference = result
        .namespace_refs
        .iter()
        .find(|reference| {
            &source[reference.span.start() as usize..reference.span.end() as usize] == "N"
                && !reference.declares
        })
        .unwrap();
    assert_eq!(
        reference.original_name_input.as_ref().unwrap().bytes(),
        b"N"
    );
    let shadowed = "proc namespace args {}\nnamespace exists Ghost";
    let result = Analyser::new().analyse(shadowed, "tcl8.6");
    assert_eq!(
        result.namespace_refs,
        [] as [crate::analyser::types::NamespaceRef; 0]
    );
}

#[test]
fn original_dynamic_variable_producer_preserves_alias_written_origins_and_known_shadowing() {
    // naming.source.authored-registry-role-projection
    // docs/design/analysis/name-resolution-proofs/authored-registry-role-projection.md
    let source = "interp alias {} assign {} set\nset name target\nassign $name 1";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let site = result
        .dynamic_variable_names
        .iter()
        .find(|site| &source[site.span.start() as usize..site.span.end() as usize] == "$name")
        .unwrap();
    assert!(site.writes);
    assert_eq!(site.resolved.as_deref(), Some("target"));
    let shadowed = "proc set args {}\nset $name 1";
    let result = Analyser::new().analyse(shadowed, "tcl8.6");
    assert_eq!(
        result.dynamic_variable_names,
        [] as [crate::analyser::types::DynamicVariableNameSite; 0]
    );
}
