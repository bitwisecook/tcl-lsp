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

fn receiver_projection_for_fixture(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<(String, bool)> {
    let image = tcl_lexer::SourceImage::document(source);
    let plan = tcl_lexer::native_script_words_in(
        image,
        tcl_lexer::Span::new(0, u32::try_from(source.len()).ok()?),
        config,
    )
    .ok()?;
    if plan.fatal_tail.is_some() {
        return None;
    }
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    let [word] = command.words.as_slice() else {
        return None;
    };
    dispatch_receiver_of(word)
}

#[test]
fn logical_object_completion_keeps_selected_names_and_visibility_masks() {
    let source = "oo::class create C {method m {} {}}\nset o [C new]\noo::objdefine $o {method {two words} {} {}; method {$literal} {} {}; export {$literal}; unexport m}\n$o ";
    let analysis = Analyser::new().analyse(source, "tcl");
    let state = crate::definition::object_member_state_at(&analysis, source, "o", 3, 3)
        .expect("actual current receiver source state");
    let items = method_items(
        &analysis,
        analysis.resolved_registry(),
        "::C",
        crate::definition::MethodBucket::Instance,
        Some(state),
        analysis.resolved_profile().unwrap(),
    )
    .expect("selected original own members and masks");
    for (name, expected) in [("two words", "{two words}"), ("$literal", "{$literal}")] {
        let item = items
            .iter()
            .find(|item| item.label == name)
            .expect("actual per-object selected name");
        assert_eq!(item.insert_text, expected);
    }
    assert!(items.iter().all(|item| item.label != "m"));
    assert!(
        method_items(
            &analysis,
            analysis.resolved_registry(),
            "::C",
            crate::definition::MethodBucket::Instance,
            Some(&state.clone()),
            analysis.resolved_profile().unwrap(),
        )
        .is_none()
    );
    let mut changed = analysis.clone();
    changed.object_member_state.get_mut("o").unwrap()[0]
        .unexports
        .clear();
    let state = crate::definition::object_member_state_at(&changed, source, "o", 3, 3).unwrap();
    assert!(
        method_items(
            &changed,
            changed.resolved_registry(),
            "::C",
            crate::definition::MethodBucket::Instance,
            Some(state),
            changed.resolved_profile().unwrap(),
        )
        .is_none()
    );
}

#[test]
fn generated_member_completion_requires_current_property_source_and_registry() {
    let source = "oo::configurable create C {property p}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let registry = analysis.resolved_registry().unwrap();
    let items = generated_property_method_items(&analysis, Some(registry), "::C");
    let configure = items
        .iter()
        .find(|item| item.label == "configure")
        .expect("genuine current property source generator");
    assert_eq!(configure.insert_text, "configure");
    let mut changed = analysis.clone();
    changed
        .all_classes
        .get_mut("::C")
        .unwrap()
        .properties
        .clear();
    assert_eq!(
        generated_property_method_items(&changed, changed.resolved_registry(), "::C"),
        [] as [crate::completion::CompletionItem; 0]
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert_eq!(
        generated_property_method_items(&missing, Some(registry), "::C"),
        [] as [crate::completion::CompletionItem; 0]
    );
    let foreign = tcl_registry::model::ingress::static_context_for("jim");
    assert_eq!(
        generated_property_method_items(&analysis, Some(foreign.commands()), "::C"),
        [] as [crate::completion::CompletionItem; 0]
    );
}

#[test]
fn logical_member_completion_quotes_actual_selected_name_data() {
    let source =
        "oo::class create C {method {$literal} {} {}; export {$literal}; method {two words} {} {}}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let items = method_items(
        &analysis,
        analysis.resolved_registry(),
        "::C",
        crate::definition::MethodBucket::Instance,
        None,
        analysis.resolved_profile().unwrap(),
    )
    .unwrap();
    for (name, expected) in [("$literal", "{$literal}"), ("two words", "{two words}")] {
        let item = items
            .iter()
            .find(|item| item.label == name)
            .expect("actual selected method name");
        assert_eq!(item.insert_text, expected);
    }
    let mut changed = analysis.clone();
    changed
        .all_classes
        .get_mut("::C")
        .unwrap()
        .methods
        .get_mut("$literal")
        .unwrap()
        .body_span = tcl_lexer::Span::new(0, 1);
    let items = method_items(
        &changed,
        changed.resolved_registry(),
        "::C",
        crate::definition::MethodBucket::Instance,
        None,
        changed.resolved_profile().unwrap(),
    )
    .unwrap();
    assert!(
        items
            .iter()
            .all(|item| item.label != "$literal" && item.label != "two words")
    );
}

#[test]
fn completion_command_geometry_keeps_whole_literal_and_variable_heads() {
    for (source, name, variable, argument) in [
        ("${two words} method", "two words", true, 1),
        ("${$literal} method", "$literal", true, 1),
        ("{$literal} method", "$literal", false, 1),
        ("{two words} method", "two words", false, 1),
        ("set x 1; ${two words} ", "two words", true, 1),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let character = u32::try_from(source.encode_utf16().count()).unwrap();
        let command = command_context_on_line(source, 0, character, &analysis)
            .expect("actual configured original command prefix");
        assert_eq!(command.word_index, argument, "{source}");
        assert_eq!(
            dispatch_receiver_of(&command.head),
            Some((name.into(), variable))
        );
        assert!(command_context_on_line(&format!("#{source}"), 0, character, &analysis).is_none());
    }
    for source in [
        "set x 1; ",
        "${two words}(index) method",
        "[compute] method",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let character = u32::try_from(source.encode_utf16().count()).unwrap();
        assert!(
            command_context_on_line(source, 0, character, &analysis)
                .and_then(|command| dispatch_receiver_of(&command.head))
                .is_none()
        );
    }
}

#[test]
fn logical_method_completion_keeps_grouped_scalar_receiver_names() {
    for (declaration, head) in [
        ("{two words}", "${two words}"),
        ("{$literal}", "${$literal}"),
    ] {
        for selector in ["longMet", "longMethd"] {
            let source = format!(
                "oo::class create C {{method longMethod {{}} {{}}}}\nset {declaration} [C new]\n{head} {selector}"
            );
            let analysis = Analyser::new().analyse(&source, "tcl");
            let character =
                u32::try_from(source.lines().nth(2).unwrap().encode_utf16().count()).unwrap();
            let items = completions(
                &source,
                2,
                character,
                &analysis,
                analysis.resolved_registry(),
                None,
                analysis.resolved_profile().unwrap(),
            );
            assert!(
                items.iter().any(|item| item.label == "longMethod"),
                "{source}: {items:?}"
            );
        }
    }
}

#[test]
fn logical_family_method_completion_uses_selected_source_name_words() {
    for source in [
        "snit::type C {method {two words} {} {}}",
        "itcl::class C {method {two words} {} {}}",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let items = method_items(
            &analysis,
            analysis.resolved_registry(),
            "::C",
            crate::definition::MethodBucket::Instance,
            None,
            analysis.resolved_profile().unwrap(),
        )
        .unwrap();
        let item = items
            .iter()
            .find(|item| item.label == "two words")
            .expect("actual selected family method");
        assert_eq!(item.insert_text, "{two words}");
    }
}

#[test]
fn logical_method_completion_keeps_whole_scalar_heads_and_current_source() {
    // naming.consumer.original-receiver-completion-domain
    // docs/design/analysis/name-resolution-proofs/original-receiver-completion-domain.md
    let source_for = |head: &str, selector: &str| {
        format!(
            "oo::class create C {{method longMethod {{}} {{}}}}\nset {{箱}} [C new]\n{head} {selector}"
        )
    };
    for selector in ["longMet", "longMethd"] {
        let source = source_for("${箱}", selector);
        let analysis = Analyser::new().analyse(&source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        let character =
            u32::try_from(source.lines().nth(2).unwrap().encode_utf16().count()).unwrap();
        let items = completions(
            &source,
            2,
            character,
            &analysis,
            analysis.resolved_registry(),
            None,
            analysis.resolved_profile().unwrap(),
        );
        assert!(
            items.iter().any(|item| item.label == "longMethod"),
            "{items:?}"
        );
        let changed = format!("# changed\n{source}");
        assert_eq!(
            completions(
                &changed,
                3,
                character,
                &analysis,
                analysis.resolved_registry(),
                None,
                analysis.resolved_profile().unwrap()
            ),
            [] as [crate::completion::CompletionItem; 0]
        );
        for head in [
            "${箱}suffix",
            "${箱}[list suffix]",
            "$箱",
            "\\${箱}",
            "${箱(k)}",
            "$arr(k)",
        ] {
            let source = source_for(head, selector);
            let analysis = Analyser::new().analyse(&source, "tcl");
            let character =
                u32::try_from(source.lines().nth(2).unwrap().encode_utf16().count()).unwrap();
            let items = completions(
                &source,
                2,
                character,
                &analysis,
                analysis.resolved_registry(),
                None,
                analysis.resolved_profile().unwrap(),
            );
            assert!(
                items.iter().all(|item| item.label != "longMethod"),
                "{head}: {items:?}"
            );
        }
    }
}

#[test]
fn logical_array_completion_keeps_original_literal_data_and_computed_index_syntax() {
    let source = "set {arr($literal)} ONE\nset {arr([list redirected])} TWO\nset {arr(\\x41)} THREE\nset {arr(key)} KEY\nset value $arr($computed)\nputs $arr(";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let selected = tcl_registry::model::ingress::resolve_environment(environment);
        let config = tcl_lexer::LexerConfig::for_profile(Some(selected.analyser_profile()));
        let mut analysis = Analyser::new()
            .with_resolved_input(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                selected.default_context_registry(),
                config,
            ))
            .analyse(source, "display only");
        analysis
            .global_scope
            .variables
            .get_mut("arr")
            .unwrap()
            .array_indices
            .insert("UNOWNED".to_owned());
        let items = variable_completions(source, 5, 10, &analysis, "arr(", '$');
        for reference in [
            "${arr($literal)}",
            "${arr([list redirected])}",
            "${arr(\\x41)}",
            "$arr(key)",
            "$arr($computed)",
        ] {
            assert!(
                items.iter().any(|item| item.insert_text == reference),
                "{environment}: {reference}: {items:?}"
            );
        }
        assert!(items.iter().all(|item| !item.label.contains("UNOWNED")));
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert_eq!(
            variable_completions(source, 5, 10, &missing, "arr(", '$'),
            [] as [crate::completion::CompletionItem; 0]
        );
        let mut changed = analysis.clone();
        changed.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        assert_eq!(
            variable_completions(source, 5, 10, &changed, "arr(", '$'),
            [] as [crate::completion::CompletionItem; 0]
        );
        assert_eq!(
            variable_completions(&format!("#{source}"), 5, 10, &analysis, "arr(", '$'),
            [] as [crate::completion::CompletionItem; 0]
        );
    }
    let native = Analyser::new().analyse(source, "tcl9.1");
    assert_eq!(
        variable_completions(source, 5, 10, &native, "arr(", '$'),
        [] as [crate::completion::CompletionItem; 0]
    );
}

#[test]
fn logical_variable_insertions_use_exact_current_reference_grammar() {
    let source = "set {a{b}c} VALUE\nset {a\\b} SLASH\nset a_\\}_closebrace BAD\nputs $";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let selected = tcl_registry::model::ingress::resolve_environment(environment);
        let config = tcl_lexer::LexerConfig::for_profile(Some(selected.analyser_profile()));
        let analysis = Analyser::new()
            .with_resolved_input(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                selected.default_context_registry(),
                config,
            ))
            .analyse(source, "display only");
        let items = variable_completions(source, 3, 6, &analysis, "", '$');
        assert!(
            items.iter().any(|item| item.insert_text == "${a\\b}"),
            "{environment}: {items:?}"
        );
        let nested = config.braced_var == tcl_dialect::BracedVarStyle::Tcl9Nesting;
        assert_eq!(
            items.iter().any(|item| item.insert_text == "${a{b}c}"),
            nested,
            "{environment}: {items:?}"
        );
        assert!(items.iter().all(|item| !item.label.contains("closebrace")));
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert_eq!(
            variable_completions(source, 3, 6, &missing, "", '$'),
            [] as [crate::completion::CompletionItem; 0]
        );
        let mut changed = analysis.clone();
        changed.body_lexer_config.as_mut().unwrap().braced_var = if nested {
            tcl_dialect::BracedVarStyle::FirstClose
        } else {
            tcl_dialect::BracedVarStyle::Tcl9Nesting
        };
        assert_eq!(
            variable_completions(source, 3, 6, &changed, "", '$'),
            [] as [crate::completion::CompletionItem; 0]
        );
        assert_eq!(
            variable_completions(&format!("#{source}"), 3, 6, &analysis, "", '$'),
            [] as [crate::completion::CompletionItem; 0]
        );
    }
    let native = Analyser::new().analyse(source, "tcl9.1");
    assert_eq!(
        variable_completions(source, 3, 6, &native, "", '$'),
        [] as [crate::completion::CompletionItem; 0]
    );
    let source = "namespace eval N {set {a{b}c} VALUE}\nputs $";
    let selected = tcl_registry::model::ingress::resolve_environment("tcl9.1");
    let config = tcl_lexer::LexerConfig::for_profile(Some(selected.analyser_profile()));
    let analysis = Analyser::new()
        .with_resolved_input(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            selected.default_context_registry(),
            config,
        ))
        .analyse(source, "display only");
    let items = variable_completions(source, 1, 6, &analysis, "", '$');
    assert!(
        items.iter().any(|item| item.insert_text == "${::N::a{b}c}"),
        "{items:?}"
    );
}

#[test]
fn logical_variable_completion_keeps_actual_release_reference_closers() {
    let source = "set alpha 1\nputs ${a{b}c}\n";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    for (environment, expected_end) in [
        ("tcl8.4", 11),
        ("tcl8.5", 11),
        ("tcl8.6", 11),
        ("tcl9.0", 13),
        ("tcl9.1", 13),
        ("jim", 11),
    ] {
        let selected = tcl_registry::model::ingress::resolve_environment(environment);
        let config = tcl_lexer::LexerConfig::for_profile(Some(selected.analyser_profile()));
        let analysis = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                selected.default_context_registry(),
                config,
            ))
            .analyse(source, "display only");
        assert!(analysis.allows_retained_logical_declaration_advice());
        let items = variable_completions(source, 1, 8, &analysis, "a", '{');
        let item = items
            .iter()
            .find(|item| item.insert_text == "${alpha}")
            .expect("actual Logical declared candidate");
        let edit = item.text_edit.as_ref().unwrap();
        assert_eq!(edit.start_char, 5, "{environment}");
        assert_eq!(edit.end_char, expected_end, "{environment}");
        assert_eq!(edit.new_text, "${alpha}");
    }
}

#[test]
fn variable_cursor_completion_keeps_braced_dollars_spaces_and_edit_extent() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let selected = tcl_registry::model::ingress::resolve_environment("tcl9.1");
    let config = tcl_lexer::LexerConfig::for_profile(Some(selected.analyser_profile()));
    for (name, partial) in [
        ("cash$name", "cash$"),
        ("a b", "a "),
        ("scalar(open", "scalar("),
    ] {
        let source = format!("set {{{name}}} VALUE\nputs ${{{name}}}suffix");
        let analysis = Analyser::new()
            .with_resolved_input(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                selected.default_context_registry(),
                config,
            ))
            .analyse(&source, "display only");
        let cursor = 7 + u32::try_from(partial.encode_utf16().count()).unwrap();
        let items = variable_trigger_completions(&source, 1, cursor, &analysis).unwrap();
        let spelling = format!("${{{name}}}");
        let item = items
            .iter()
            .find(|item| item.insert_text == spelling)
            .expect("original declared braced-name candidate");
        let edit = item.text_edit.as_ref().unwrap();
        assert_eq!(edit.start_char, 5);
        assert_eq!(
            edit.end_char,
            8 + u32::try_from(name.encode_utf16().count()).unwrap()
        );
        assert_eq!(edit.new_text, spelling);
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert_eq!(
            variable_trigger_completions(&source, 1, cursor, &missing).unwrap(),
            [] as [crate::completion::CompletionItem; 0]
        );
        assert_eq!(
            variable_trigger_completions(&format!("#{source}"), 1, cursor, &analysis).unwrap(),
            [] as [crate::completion::CompletionItem; 0]
        );
    }
}

#[test]
fn variable_cursor_completion_rejects_non_utf16_boundaries() {
    let source = "set alpha 1\nputs ${😀alpha}";
    let analysis = Analyser::new().analyse(source, "tcl9.1");
    assert_eq!(
        variable_trigger_completions(source, 1, 8, &analysis).unwrap(),
        [] as [crate::completion::CompletionItem; 0]
    );
    assert_eq!(
        variable_trigger_completions(source, 1, 1000, &analysis).unwrap(),
        [] as [crate::completion::CompletionItem; 0]
    );
}

#[test]
fn configured_scalar_completion_projection_keeps_array_and_brace_boundaries() {
    // naming.consumer.original-receiver-completion-domain
    // docs/design/analysis/name-resolution-proofs/original-receiver-completion-domain.md
    let first = tcl_lexer::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::FirstClose,
        ..tcl_lexer::LexerConfig::default()
    };
    let mut nested = first;
    nested.braced_var = tcl_dialect::BracedVarStyle::Tcl9Nesting;
    assert_eq!(strip_instance_var("${箱}", first), Some("箱".to_owned()));
    assert_eq!(
        strip_instance_var("${a{b}c}", nested),
        Some("a{b}c".to_owned())
    );
    assert!(strip_instance_var("${a{b}c}", first).is_none());
    assert_eq!(strip_instance_var("${a{b}", first), Some("a{b".to_owned()));
    assert!(strip_instance_var("${a{b}", nested).is_none());
    for config in [first, nested] {
        for head in [
            "${x}suffix",
            "$x-suffix",
            "${x}[list suffix]",
            "${x",
            "$arr(k)",
            "${arr(k)}",
            "$arr($k)",
            "$arr([list k])",
            "\\${x}",
            "$箱",
        ] {
            assert!(strip_instance_var(head, config).is_none(), "{head}");
        }
        assert_eq!(
            receiver_projection_for_fixture("${箱}", config),
            Some(("箱".to_owned(), true))
        );
        assert!(receiver_projection_for_fixture("${箱}suffix", config).is_none());
    }
}
