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

//! Golden-driven tests for `resolve_object_ref_args` (the iRules command→
//! `(arg_index, kinds)` table + the `class`/`persist` custom resolvers).
//! Self-contained.

use tcl_irules::resolve_object_ref_args;

#[test]
fn original_source_values_and_standalone_templates_agree_without_a_second_unwrap() {
    crate::support::install_surfaces();
    let source = r#"when HTTP_REQUEST {
        pool {{none}}
        pool {"none"}
        pool {'none'}
        pool {[none]}
        pool {(none)}
        pool {none,}
        pool { none }
        pool {none}
        pool "none"
        pool none
    }"#;
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let context = tcl_irules::OriginalIrulesSourceContext::capture(source, registry)
        .expect("actual original hosted source owns its words and selected schema");
    let commands = context
        .commands()
        .iter()
        .filter(|command| command.words().command() == "pool")
        .collect::<Vec<_>>();
    assert_eq!(commands.len(), 10);
    for command in commands {
        let words = command.words();
        let value = words.arguments()[0]
            .literal_bytes()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .expect("actual complete original literal operand value");
        let original = words
            .with_source_schema(context.context_registry(), |schema| {
                tcl_irules::original_source_object_operands(schema, Some("ltm"))
            })
            .expect("actual retained original schema");
        let standalone = resolve_object_ref_args("pool", &[value], Some("ltm"));
        assert_eq!(original.len(), standalone.len(), "{value:?}");
        if let Some(original) = original.first() {
            assert_eq!(original.argument(), standalone[0].0);
            assert_eq!(original.kinds(), standalone[0].1.as_slice());
        }
    }
    let references =
        tcl_irules::extract_original_irules_source_object_references(source, &context, Some("ltm"));
    let names = references
        .iter()
        .map(|reference| reference.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "{none}", "\"none\"", "'none'", "[none]", "(none)", "none,", " none "
        ]
    );
    assert_eq!(
        tcl_irules::extract_original_irules_source_object_references(
            &format!("{source}\n# changed"),
            &context,
            Some("ltm"),
        ),
        [] as [tcl_irules::IrulesObjectReference; 0]
    );
}
