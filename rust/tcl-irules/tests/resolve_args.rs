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
fn resolve_args_match_golden() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/resolve_args.golden.tsv"
    );
    let golden = std::fs::read_to_string(path).expect("read golden");

    let mut checked = 0usize;
    for line in golden.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        let command = f[0];
        let args: Vec<&str> = if f[1].is_empty() {
            Vec::new()
        } else {
            f[1].split(',').collect()
        };
        let rule_module = if f[2] == "<none>" { None } else { Some(f[2]) };
        let want = f[3];

        let res = resolve_object_ref_args(command, &args, rule_module);
        let got = if res.is_empty() {
            "-".to_owned()
        } else {
            res.iter()
                .map(|(i, kinds)| format!("{i}:{}", kinds.join("|")))
                .collect::<Vec<_>>()
                .join(";")
        };
        assert_eq!(got, want, "resolve({command:?}, {args:?}, {rule_module:?})");
        checked += 1;
    }
    assert!(checked > 100, "golden unexpectedly small: {checked}");
}

#[test]
fn standalone_template_operands_keep_literal_name_punctuation() {
    for value in ["none", "NoNe", "member", "members"] {
        assert!(
            resolve_object_ref_args("pool", &[value], None).is_empty(),
            "{value:?}"
        );
    }
    for value in [
        "{none}",
        "\"none\"",
        "'none'",
        "[none]",
        "(none)",
        "none,",
        " none ",
        "{members}",
        "(members)",
        "none\0literal",
    ] {
        let references = resolve_object_ref_args("pool", &[value], None);
        assert_eq!(references.len(), 1, "{value:?}");
        assert_eq!(references[0].0, 0, "{value:?}");
        assert!(references[0].1.contains(&"ltm_pool"), "{value:?}");
    }
}

#[test]
fn original_source_values_and_standalone_templates_agree_without_a_second_unwrap() {
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
    assert!(
        tcl_irules::extract_original_irules_source_object_references(
            &format!("{source}\n# changed"),
            &context,
            Some("ltm"),
        )
        .is_empty()
    );
}
