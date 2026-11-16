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

#[test]
fn original_source_presentation_uses_selected_roles_and_effective_ordinals() {
    // naming.core.original-command-source-schema
    // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
    use crate::{ArgPresentation, InvocationWord};
    let context = crate::model::ingress::static_context_for("tcl8.6");
    let arguments = [
        InvocationWord::Literal("set i 0"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("incr i"),
        InvocationWord::Literal("puts $i"),
    ];
    let resolution = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        context.commands(),
        Some(context.context()),
        crate::InvocationWords::structured(InvocationWord::Literal("for"), &arguments),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    );
    let selected = resolution.resolved().unwrap();
    assert_eq!(
        selected.authored_source_argument_presentation(0),
        Some(ArgPresentation::InlineScript)
    );
    assert_eq!(
        selected.authored_source_argument_presentation(2),
        Some(ArgPresentation::InlineScript)
    );
    assert_eq!(
        selected.authored_source_argument_presentation(3),
        Some(ArgPresentation::BlockScript)
    );
    assert_eq!(selected.authored_source_argument_presentation(1), None);
    assert_eq!(selected.authored_source_argument_presentation(4), None);
    let expanded = [InvocationWord::Expanded];
    let uncertain = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        context.commands(),
        Some(context.context()),
        crate::InvocationWords::structured(InvocationWord::Literal("for"), &expanded),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
    .resolved()
    .unwrap();
    assert_eq!(uncertain.authored_source_argument_presentation(0), None);
}
#[test]
fn selected_option_keyword_table_retains_prefix_positions_and_values() {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let context = crate::model::ingress::static_context_for("tcl8.6");
    let arguments = [
        crate::InvocationWord::Literal("equal"),
        crate::InvocationWord::Literal("-length"),
        crate::InvocationWord::Literal("-nocase"),
        crate::InvocationWord::Literal("A"),
        crate::InvocationWord::Literal("b"),
    ];
    let resolution = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
        context.commands(),
        Some(context.context()),
        crate::InvocationWords::structured(crate::InvocationWord::Literal("string"), &arguments)
            .with_profile(context.commands().profile()),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    );
    let selected = resolution.resolved().unwrap();
    let options = selected.semantics.options;
    let table = options.keyword_table();
    assert_eq!(table.resolve("-n").unique(), Some("-nocase"));
    let occurrences = options
        .prefix_occurrences(
            selected
                .words
                .arguments()
                .slice_from(selected.semantics.argument_offset),
        )
        .unwrap();
    assert_eq!(occurrences.len(), 1);
    assert_eq!(occurrences[0].option.unwrap().name, "-length");
    assert_eq!(occurrences[0].values, 1..2);
}
