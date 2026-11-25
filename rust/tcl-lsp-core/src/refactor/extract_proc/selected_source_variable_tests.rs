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
fn original_extraction_classifies_captured_variable_roles_and_rejects_replaced_helpers() {
    // naming.refactor.original-frame-traversal
    // docs/design/analysis/name-resolution-proofs/refactor-original-frame-traversal.md
    let source = "interp alias {} write {} set held
write 2
proc set args {}
set hidden 1";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    let walk = super::super::FrameWalk::new(source, &analysis).unwrap();
    let commands = walk.segment_document(source);
    let roles = classify_variables(source, &[&commands[1]], &walk).unwrap();
    assert_eq!(roles.written.keys().cloned().collect::<Vec<_>>(), ["held"]);
    assert!(classify_variables(source, &[&commands[3]], &walk).is_err());
}

#[test]
fn original_logical_extraction_keeps_current_read_only_edit_and_withdraws_missing_input() {
    // naming.refactor.original-frame-traversal
    // docs/design/analysis/name-resolution-proofs/refactor-original-frame-traversal.md
    let source = "set x 5
puts $x
puts done";
    let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    let registry = CommandRegistry::build_default();
    let start = u32::try_from(source.find("puts $x").unwrap()).unwrap();
    let selection = (start, start + 7);
    let action = extract_proc(source, selection, &analysis, &registry).unwrap();
    assert!(action.disabled.is_none(), "{:?}", action.disabled);
    assert!(
        action
            .try_apply(source)
            .unwrap()
            .contains("proc extracted_proc {x}")
    );
    analysis.resolved_input = None;
    assert!(extract_proc(source, selection, &analysis, &registry).is_none());
}

#[test]
fn logical_extraction_scalar_proposals_keep_selected_grammar_and_refuse_unwritable_names() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let driver = tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
        .unwrap()
        .default_context_registry();
    for (style, source_word, expected) in [
        (BracedVarStyle::FirstClose, r"a\{b", Some("${a{b}")),
        (BracedVarStyle::FirstClose, r"a\{b\}c", None),
        (BracedVarStyle::Tcl9Nesting, r"a\{b\}c", Some("${a{b}c}")),
        (BracedVarStyle::Tcl9Nesting, r"a\{b", None),
    ] {
        let selected = format!("set {source_word}");
        let source = format!("{selected}\nputs done\n");
        let config = LexerConfig {
            braced_var: style,
            ..LexerConfig::for_profile(Some(profile))
        };
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            driver.clone(),
            config,
        );
        let analysis = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(&source, "reporting label");
        assert!(analysis.allows_lexical_declaration_advice());
        let action = extract_proc(
            &source,
            (0, u32::try_from(selected.len()).unwrap()),
            &analysis,
            driver.commands(),
        )
        .unwrap();
        if let Some(reference) = expected {
            assert!(
                action.disabled.is_none(),
                "{style:?} {source_word}: {:?}",
                action.disabled
            );
            let rewritten = action.try_apply(&source).unwrap();
            assert!(
                rewritten.contains(&format!("extracted_proc {reference}")),
                "{rewritten}"
            );
            assert!(
                rewritten.contains(&selected),
                "the original selected body is retained"
            );
        } else {
            assert_eq!(action.edits, [] as [crate::refactor::RefactorEdit; 0]);
            assert!(
                action
                    .disabled
                    .as_deref()
                    .unwrap()
                    .contains("original source grammar")
            );
        }
    }
}

#[test]
fn original_extraction_uses_retained_brace_grammar_and_withdraws_changed_owners() {
    // naming.refactor.original-frame-traversal
    // docs/design/analysis/name-resolution-proofs/refactor-original-frame-traversal.md
    let source = "if {1} {puts ${a{b}c}}\nputs done\n";
    let needle = "if {1} {puts ${a{b}c}}";
    let selection = (0, u32::try_from(needle.len()).unwrap());
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let driver = tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
        .unwrap()
        .default_context_registry();
    let registry = driver.commands();
    for (style, parameters) in [
        (BracedVarStyle::FirstClose, r"a\{b"),
        (BracedVarStyle::Tcl9Nesting, "a{b}c"),
    ] {
        let config = LexerConfig {
            braced_var: style,
            ..LexerConfig::for_profile(Some(profile))
        };
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            driver.clone(),
            config,
        );
        let analysis = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "reporting label");
        assert!(analysis.allows_lexical_declaration_advice());
        assert_eq!(analysis.body_lexer_config, Some(config));
        let action = extract_proc(source, selection, &analysis, registry).unwrap();
        assert!(action.disabled.is_none(), "{:?}", action.disabled);
        let rewritten = action.try_apply(source).unwrap();
        assert!(
            rewritten.contains(&format!("proc extracted_proc {{{parameters}}} {{")),
            "{style:?}: {rewritten}"
        );
        assert!(
            rewritten.contains(needle),
            "original selected body is retained"
        );

        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(extract_proc(source, selection, &missing, registry).is_none());
        let mut stale = analysis.clone();
        stale.body_lexer_config.as_mut().unwrap().braced_var = match style {
            BracedVarStyle::FirstClose => BracedVarStyle::Tcl9Nesting,
            BracedVarStyle::Tcl9Nesting => BracedVarStyle::FirstClose,
        };
        assert!(extract_proc(source, selection, &stale, registry).is_none());
        assert!(extract_proc(&format!("#{source}"), selection, &analysis, registry).is_none());

        let foreign = tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
            .unwrap()
            .default_context_registry();
        let older_same_store =
            std::sync::Arc::new(foreign.with_command_store(registry.snapshot().shared_registry()));
        for context in [foreign, older_same_store] {
            let mut changed = analysis.clone();
            changed.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile, profile, context, config,
            ));
            assert!(extract_proc(source, selection, &changed, registry).is_none());
        }
    }
}
#[test]
fn generated_rename_uses_genuine_afterimage_header_and_utf16_with_original_input() {
    let source = "# 😀\r\nputs café\r\n";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    let insertion = u32::try_from(source.find("puts").unwrap()).unwrap();
    let refactoring = Refactoring {
        title: "fixture".to_owned(),
        edits: vec![RefactorEdit {
            start: insertion,
            end: insertion,
            new_text: "proc café {} {}\r\n".to_owned(),
        }],
        kind: ActionKind::RefactorExtract,
        data_group: None,
        disabled: None,
    };
    let command =
        extract_proc_rename_command_for_analysis(&refactoring, source, &analysis).unwrap();
    assert_eq!(
        command.string_args[0],
        refactoring.try_apply(source).unwrap()
    );
    assert_eq!(command.string_args[1], "café");
    assert_eq!(command.args, vec![1, 5, 9]);
    assert!(
        extract_proc_rename_command_for_analysis(&refactoring, &format!("{source} "), &analysis)
            .is_none()
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(extract_proc_rename_command_for_analysis(&refactoring, source, &missing).is_none());
}

#[test]
fn generated_rename_does_not_select_rendered_comment_or_nonheader_names() {
    let source = "puts ok\n";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    let mut refactoring = Refactoring {
        title: "fixture".to_owned(),
        edits: vec![RefactorEdit {
            start: 0,
            end: 0,
            new_text: "# proc decoy {} {}\nset data generated\n".to_owned(),
        }],
        kind: ActionKind::RefactorExtract,
        data_group: None,
        disabled: None,
    };
    assert!(extract_proc_rename_command_for_analysis(&refactoring, source, &analysis).is_none());
    refactoring.edits[0].new_text = "proc generated {} {}\n".to_owned();
    assert!(extract_proc_rename_command_for_analysis(&refactoring, source, &analysis).is_some());
    refactoring.disabled = Some("source action unavailable".to_owned());
    assert!(extract_proc_rename_command_for_analysis(&refactoring, source, &analysis).is_none());
    refactoring.disabled = None;
    refactoring.edits[0].start = 999;
    refactoring.edits[0].end = 999;
    assert!(extract_proc_rename_command_for_analysis(&refactoring, source, &analysis).is_none());
}
