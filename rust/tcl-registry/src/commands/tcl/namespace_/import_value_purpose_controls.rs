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

#[test]
fn original_source_and_native_operands_select_control_and_pattern_ordinal_by_actual_purpose() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let project = |words: &[InvocationWord<'_>], configured| {
        let arguments = InvocationArguments::structured(words).with_profile(configured);
        let transitions = namespace_import_state_transitions(arguments);
        transitions
            .facts()
            .iter()
            .find_map(|fact| match &fact.transition {
                StateTransition::Namespace(NamespaceTransition::Import {
                    force, patterns, ..
                }) => Some((*force, patterns.len())),
                _ => None,
            })
            .unwrap()
    };
    let source = [
        InvocationWord::Literal("import"),
        InvocationWord::Literal("-force"),
        InvocationWord::Literal("::m::*"),
    ];
    assert_eq!(project(&source, Some(profile)), (Some(true), 1));
    assert_eq!(project(&source, None), (None, 2));
    let source = [
        InvocationWord::Literal("import"),
        InvocationWord::Literal("-force\0tail"),
        InvocationWord::Literal("::m::*"),
    ];
    assert_eq!(project(&source, Some(profile)), (Some(false), 2));
    let native = [
        InvocationWord::Literal("import"),
        InvocationWord::KnownBytes(b"-force\0tail"),
        InvocationWord::Literal("::m::*"),
    ];
    assert_eq!(project(&native, Some(profile)), (Some(true), 1));
}
