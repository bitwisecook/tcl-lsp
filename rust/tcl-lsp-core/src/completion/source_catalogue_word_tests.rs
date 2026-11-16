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

fn value(item: &CompletionItem, config: tcl_lexer::LexerConfig) -> Vec<u8> {
    let image = tcl_lexer::SourceImage::document(&item.insert_text);
    let plan = tcl_lexer::native_script_words_in(
        image,
        tcl_lexer::Span::new(0, u32::try_from(item.insert_text.len()).unwrap()),
        config,
    )
    .unwrap();
    assert!(plan.fatal_tail.is_none());
    let [command] = plan.commands.as_slice() else {
        panic!("one command")
    };
    let [word] = command.words.as_slice() else {
        panic!("one source word")
    };
    assert_eq!(
        word.span(),
        tcl_lexer::Span::new(0, u32::try_from(item.insert_text.len()).unwrap())
    );
    tcl_syntax::word_rules::original_static_word_unicode_value(word).unwrap()
}

#[test]
fn source_catalogue_insertions_encode_provider_data_before_packaging_labels() {
    use tcl_registry::scoped::{ScopedCommand, ScopedCommandEnv};
    static OPS: &[tcl_registry::SubCommand] = &[tcl_registry::SubCommand {
        name: r"op\x41",
        ..tcl_registry::SubCommand::DEFAULT
    }];
    static COMMANDS: &[ScopedCommand] = &[ScopedCommand {
        name: "café $[literal]",
        arity: tcl_registry::arity::Arity::any(),
        subcommands: OPS,
        allow_unknown_subcommands: false,
        detail: "",
        hover: None,
    }];
    static ENV: ScopedCommandEnv = ScopedCommandEnv {
        name: "source catalogue",
        commands: COMMANDS,
        include_sibling_definitions: false,
        allow_unknown_commands: false,
    };
    static NESTED: tcl_registry::SubSubCommand = tcl_registry::SubSubCommand {
        name: r"nested\u0041",
        ..tcl_registry::SubSubCommand::DEFAULT
    };
    static VALUE: tcl_registry::ArgValue = tcl_registry::ArgValue {
        value: r"value\uD800",
        ..tcl_registry::ArgValue::DEFAULT
    };
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    for (items, expected) in [
        (
            scoped_command_completions(&ENV, "", config),
            COMMANDS[0].name,
        ),
        (scoped_op_completions(&COMMANDS[0], "", config), OPS[0].name),
        (
            arg_value_completions_from_subcommands(vec![&OPS[0]], "", config),
            OPS[0].name,
        ),
        (
            sub_subcommand_completions(vec![&NESTED], "", config),
            NESTED.name,
        ),
        (
            arg_value_completions_from(vec![&VALUE], "", config),
            VALUE.value,
        ),
    ] {
        let [item] = items.as_slice() else {
            panic!("one provider candidate")
        };
        assert_eq!(item.label, expected);
        assert_eq!(value(item, config), expected.as_bytes());
        assert_ne!(item.insert_text, expected);
    }
}

#[test]
fn actual_builtin_catalogue_names_keep_literal_unicode_and_escape_data() {
    for name in [r"take\x41", "café name"] {
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name,
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let source = if name.starts_with("take") {
            "ta"
        } else {
            "caf"
        };
        let analysis =
            crate::refactor::test_logical_analysis(source, "tcl", std::sync::Arc::new(registry));
        let items = completions(
            source,
            0,
            u32::try_from(source.len()).unwrap(),
            &analysis,
            None,
            None,
            analysis.resolved_profile().unwrap(),
        );
        let item = items.iter().find(|item| item.label == name).unwrap();
        assert_eq!(
            value(item, analysis.body_lexer_config.unwrap()),
            name.as_bytes()
        );
        assert_ne!(item.insert_text, name);
        assert!(source_method_name_word(&analysis, "a\0b").is_none());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert_eq!(
            completions(
                source,
                0,
                u32::try_from(source.len()).unwrap(),
                &missing,
                None,
                None,
                analysis.resolved_profile().unwrap()
            ),
            [] as [crate::completion::CompletionItem; 0]
        );
    }
}

#[test]
fn event_candidates_use_the_actual_configured_placement() {
    use tcl_compiler::analyser::Analyser;
    let old = Analyser::new()
        .with_bigip_version(Some("9.4.0".to_owned()))
        .analyse("when ", "f5-irules");
    let current = Analyser::new()
        .with_bigip_version(Some("16.1.0".to_owned()))
        .analyse("when ", "f5-irules");
    let old_items = event_name_completions("", &old);
    assert!(old_items.iter().any(|item| item.label == "HTTP_REQUEST"));
    assert!(
        old_items
            .iter()
            .any(|item| item.label == "XML_BEGIN_DOCUMENT")
    );
    let current_items = event_name_completions("", &current);
    assert!(
        current_items
            .iter()
            .any(|item| item.label == "HTTP_REQUEST")
    );
    assert!(
        !current_items
            .iter()
            .any(|item| item.label == "XML_BEGIN_DOCUMENT")
    );
    let mut missing = current.clone();
    missing.resolved_input = None;
    assert_eq!(
        event_name_completions("", &missing),
        [] as [crate::completion::CompletionItem; 0]
    );
    let mut changed = current.clone();
    changed.body_lexer_config = Some(tcl_lexer::LexerConfig::for_dialect("jim"));
    assert_eq!(
        event_name_completions("", &changed),
        [] as [crate::completion::CompletionItem; 0]
    );
}
