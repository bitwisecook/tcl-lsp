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
fn native_class_navigation_cannot_borrow_a_deleted_head_or_unknown_argument() {
    for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for (source, missing) in [
            ("oo::class create C {}\nrename C {}\nC new\n", "C new"),
            (
                "oo::class create C {}\nunknown_command C\n",
                "unknown_command C",
            ),
        ] {
            let analysis = Analyser::new().analyse(source, release);
            assert!(!analysis.allows_lexical_declaration_advice());
            let (_, line, character) = cursor(source, "oo::class create C");
            assert_eq!(
                definition(source, line, character, &analysis)
                    .iter()
                    .map(|range| range.start_line)
                    .collect::<Vec<_>>(),
                vec![0],
                "current source header {release}"
            );
            let missing_offset = u32::try_from(
                source.find(missing).unwrap()
                    + if missing == "C new" {
                        0
                    } else {
                        "unknown_command ".len()
                    },
            )
            .unwrap();
            let missing_position = LineIndex::new(source).position_at_utf16(missing_offset, source);
            let (line, character) = (missing_position.line, missing_position.character.get());
            assert!(
                definition(source, line, character, &analysis).is_empty(),
                "{release}: {source}"
            );
        }
    }
}

#[test]
fn logical_unknown_namespace_retains_body_tables_without_a_coordinate() {
    let source = "set root ROOT\nnamespace eval $unknown {\n set local OWN\n puts $local\n puts $Target::x\n}\n";
    let analysis = Analyser::new().analyse(source, "tcl");
    assert!(analysis.allows_lexical_declaration_advice());
    let (offset, line, character) = cursor(source, "puts $local");
    let advice = analysis
        .logical_variable_scope_advice(
            &tcl_lexer::SourceImage::document(source),
            analysis.body_lexer_config.unwrap(),
            offset,
        )
        .unwrap();
    assert!(advice.namespace().is_none());
    assert!(!advice.is_namespace_frame());
    assert!(
        advice
            .eligible_scopes()
            .any(|scope| scope.variables.contains_key("local"))
    );
    assert_eq!(
        definition(source, line, character, &analysis)
            .iter()
            .map(|range| range.start_line)
            .collect::<Vec<_>>(),
        vec![2]
    );
    let (_, line, character) = cursor(source, "puts $Target::x");
    assert!(
        qualified_variable_cell_at(
            source,
            analysis.resolved_profile().unwrap(),
            &analysis,
            line,
            character
        )
        .is_none()
    );
}

#[test]
fn logical_interpreter_source_body_cannot_borrow_the_parent_root() {
    for source in [
        "set x PARENT\ninterp create child\ninterp eval child {puts $x}\n",
        "set x PARENT\ninterp eval $child {puts $x}\n",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        let (offset, line, character) = cursor(source, "puts $x");
        assert!(
            analysis
                .logical_variable_scope_advice(
                    &tcl_lexer::SourceImage::document(source),
                    analysis.body_lexer_config.unwrap(),
                    offset
                )
                .is_none()
        );
        assert_eq!(
            definition(source, line, character, &analysis),
            [] as [crate::definition::LspRange; 0]
        );
    }
}

#[test]
fn logical_array_completion_keeps_the_actual_cursor_edit_extent() {
    for (source, character, expected_end) in [
        ("set arr(key) 1\nset v $arr(k more stuff\n", 12, 12),
        ("set arr(key) 1\nset v $arr(k)\n", 12, 13),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        let items = crate::completion::completions(
            source,
            1,
            character,
            &analysis,
            analysis.resolved_registry(),
            None,
            analysis.resolved_profile().unwrap(),
        );
        let candidate = items
            .iter()
            .find(|item| item.label == "$arr(key)")
            .expect("current Logical source array candidate");
        let edit = candidate.text_edit.as_ref().expect("cursor replacement");
        assert_eq!((edit.start_char, edit.end_char), (6, expected_end));
        assert_eq!(edit.new_text, "$arr(key)");
        let stale = format!("{source} ");
        assert_eq!(
            crate::completion::completions(
                &stale,
                1,
                character,
                &analysis,
                analysis.resolved_registry(),
                None,
                analysis.resolved_profile().unwrap()
            ),
            [] as [crate::completion::CompletionItem; 0]
        );
    }
}

#[test]
fn logical_namespace_write_coordinates_require_current_source_and_store_cards() {
    use std::sync::Arc;
    use tcl_compiler::analyser::ResolvedAnalysisInput;
    let source = "namespace eval N {variable x FIRST; set x AGAIN; puts $x}\n";
    let analysis = Analyser::new().analyse(source, "tcl");
    let offset = u32::try_from(source.find("set x AGAIN").unwrap() + "set ".len()).unwrap();
    let position = LineIndex::new(source).position_at_utf16(offset, source);
    let query = |source: &str, analysis: &AnalysisResult| {
        qualified_variable_cell_at(
            source,
            analysis.resolved_profile().unwrap(),
            analysis,
            position.line,
            position.character.get(),
        )
    };
    assert_eq!(query(source, &analysis).as_deref(), Some("::N::x"));
    let mut missing = analysis.clone();
    missing.original_variable_write_advice.clear();
    assert!(query(source, &missing).is_none());
    assert!(query(&format!("{source} "), &analysis).is_none());

    let input = analysis.resolved_input.as_ref().unwrap();
    let mut registry = analysis
        .resolved_registry()
        .unwrap()
        .project_for_profile(input.unit_profile());
    let mut changed = registry.get("puts").unwrap().clone();
    if changed
        .traits
        .contains(tcl_registry::Traits::EVALUATES_CODE)
    {
        changed.traits.remove(tcl_registry::Traits::EVALUATES_CODE);
    } else {
        changed.traits.insert(tcl_registry::Traits::EVALUATES_CODE);
    }
    registry.insert(changed);
    let context = Arc::new(
        input
            .context_registry()
            .with_command_store(Arc::new(registry)),
    );
    let foreign_input = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        context,
        input.lexer_config(),
    );
    let foreign = Analyser::new()
        .with_resolved_input(foreign_input)
        .analyse(source, "tcl");
    assert_ne!(
        foreign.original_variable_write_advice,
        [] as [tcl_compiler::signature_scan::variable_symbol::OriginalVariableWriteAdvice; 0]
    );
    let mut donated = analysis.clone();
    donated.original_variable_write_advice = foreign.original_variable_write_advice;
    assert!(query(source, &donated).is_none());
}

#[test]
fn logical_bare_procedure_writes_cannot_publish_namespace_coordinates() {
    let source = "set x ROOT\nproc p {} {set x FIRST; set x AGAIN; set N::x QUALIFIED}\n";
    let analysis = Analyser::new().analyse(source, "tcl");
    for (needle, expected) in [
        ("set x AGAIN", None),
        ("set N::x QUALIFIED", Some("::N::x")),
    ] {
        let offset = u32::try_from(source.find(needle).unwrap() + "set ".len()).unwrap();
        let position = LineIndex::new(source).position_at_utf16(offset, source);
        assert_eq!(
            qualified_variable_cell_at(
                source,
                analysis.resolved_profile().unwrap(),
                &analysis,
                position.line,
                position.character.get(),
            )
            .as_deref(),
            expected,
        );
    }
}

#[test]
fn native_conditional_publication_navigation_preserves_source_table_refusals() {
    for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for (source, needle, expected) in [
            (
                "oo::class create C {}\nproc p {} {C new}\n",
                "C new",
                Some(0),
            ),
            (
                "proc p {} {Future new}\noo::class create Future {}\n",
                "Future new",
                Some(1),
            ),
            (
                "oo::class create C {}\nrename C {}\nproc p {} {C new}\n",
                "C new",
                None,
            ),
            (
                "oo::class create C {}\nproc C args {}\nproc p {} {C new}\n",
                "C new",
                Some(1),
            ),
            (
                // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
                "set ::auto_index(unknown_mutator) {source lib.tcl}; oo::class create C {}\nproc p {} {unknown_mutator; C new}\n",
                "C new",
                None,
            ),
        ] {
            let analysis = Analyser::new().analyse(source, release);
            assert!(!analysis.allows_lexical_declaration_advice());
            let offset = u32::try_from(source.find(needle).unwrap()).unwrap();
            let position = LineIndex::new(source).position_at_utf16(offset, source);
            let selected = definition(source, position.line, position.character.get(), &analysis)
                .into_iter()
                .map(|range| range.start_line)
                .collect::<Vec<_>>();
            assert_eq!(
                selected,
                expected.into_iter().collect::<Vec<_>>(),
                "{release}: {source}"
            );
        }
    }
}

#[test]
fn logical_recorded_reads_use_the_same_root_current_and_unknown_source_tables() {
    let analysis = Analyser::new().analyse(SCOPED, "tcl");
    for (needle, expected) in [
        ("uplevel #0 {puts $x", Some(0)),
        ("uplevel 0 {puts $x", Some(4)),
        ("uplevel 1 {puts $x", None),
    ] {
        let (_, line, character) = cursor(SCOPED, needle);
        let spans = crate::references::references(
            SCOPED,
            analysis.resolved_profile().unwrap(),
            line,
            character,
            &analysis,
            true,
        );
        let lines = spans.iter().map(|span| span.start_line).collect::<Vec<_>>();
        if let Some(declaration) = expected {
            assert!(lines.contains(&declaration), "{needle}: {lines:?}");
            assert!(
                lines.contains(&line),
                "original selected read {needle}: {lines:?}"
            );
        } else {
            assert!(lines.is_empty(), "unknown caller {needle}: {lines:?}");
        }
        assert!(
            !lines.contains(&2),
            "namespace cannot replace frame {needle}: {lines:?}"
        );
    }
}

const SCOPED: &str = "set x ROOT\nnamespace eval N {\n variable x NAMESPACE\n proc p {} {\n  set x PROCEDURE\n  uplevel #0 {puts $x}\n  uplevel 0 {puts $x}\n  uplevel 1 {puts $x}\n }\n}\n";

fn cursor(source: &str, needle: &str) -> (u32, u32, u32) {
    let offset = u32::try_from(source.find(needle).unwrap() + needle.len() - 1).unwrap();
    let position = LineIndex::new(source).position_at_utf16(offset, source);
    (offset, position.line, position.character.get())
}

#[test]
fn logical_scope_selector_preserves_root_current_and_unknown_caller_source_tables() {
    let analysis = Analyser::new().analyse(SCOPED, "tcl");
    assert!(analysis.allows_lexical_declaration_advice());
    for (needle, expected_line) in [
        ("uplevel #0 {puts $x", Some(0)),
        ("uplevel 0 {puts $x", Some(4)),
        ("uplevel 1 {puts $x", None),
    ] {
        let (offset, line, character) = cursor(SCOPED, needle);
        let scope = analysis
            .logical_variable_scope_advice(
                &tcl_lexer::SourceImage::document(SCOPED),
                analysis.body_lexer_config.unwrap(),
                offset,
            )
            .unwrap();
        let selected = definition(SCOPED, line, character, &analysis);
        match expected_line {
            Some(expected) => assert_eq!(
                selected
                    .iter()
                    .map(|range| range.start_line)
                    .collect::<Vec<_>>(),
                vec![expected],
                "{needle}"
            ),
            None => assert!(selected.is_empty(), "{needle}: {selected:?}"),
        }
        let references = crate::references::references(
            SCOPED,
            analysis.resolved_profile().unwrap(),
            line,
            character,
            &analysis,
            true,
        );
        assert!(
            !references.iter().any(|range| range.start_line == 2),
            "outer namespace cannot substitute for the selected frame: {references:?}"
        );
        if expected_line.is_none() {
            assert!(
                !references
                    .iter()
                    .any(|range| matches!(range.start_line, 0 | 4))
            );
        }
        assert!(scope.eligible_scopes().all(|candidate| !matches!(
            candidate.kind,
            tcl_compiler::analyser::ScopeKind::Namespace
        )));
    }
}

#[test]
fn logical_method_root_and_nested_declared_local_frames_stay_distinct() {
    for (source, needle, expected_line) in [
        (
            "set x ROOT\noo::class create C {\n method m {} {\n  set x METHOD\n  uplevel #0 {puts $x}\n  uplevel 0 {puts $x}\n }\n}\n",
            "uplevel #0 {puts $x",
            0,
        ),
        (
            "set x ROOT\noo::class create C {\n method m {} {\n  set x METHOD\n  uplevel #0 {puts $x}\n  uplevel 0 {puts $x}\n }\n}\n",
            "uplevel 0 {puts $x",
            3,
        ),
        (
            "set x ROOT\nproc outer {} {uplevel #0 {\n proc inner {} {\n  set x INNER\n  puts $x\n }\n}}\n",
            "puts $x",
            3,
        ),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        let (offset, line, character) = cursor(source, needle);
        assert!(
            analysis
                .logical_variable_scope_advice(
                    &tcl_lexer::SourceImage::document(source),
                    analysis.body_lexer_config.unwrap(),
                    offset
                )
                .is_some()
        );
        let selected = definition(source, line, character, &analysis);
        assert_eq!(
            selected
                .iter()
                .map(|range| range.start_line)
                .collect::<Vec<_>>(),
            vec![expected_line],
            "{source}"
        );
    }
}

#[test]
fn logical_root_namespace_eval_keeps_the_same_root_declaration_table() {
    let source = "set x ROOT\nnamespace eval :: {puts $x}\n";
    let analysis = Analyser::new().analyse(source, "tcl");
    assert!(analysis.allows_lexical_declaration_advice());
    let (offset, line, character) = cursor(source, "puts $x");
    let advice = analysis
        .logical_variable_scope_advice(
            &tcl_lexer::SourceImage::document(source),
            analysis.body_lexer_config.unwrap(),
            offset,
        )
        .unwrap();
    assert!(!advice.root_names_require_qualification());
    assert_eq!(
        definition(source, line, character, &analysis)
            .iter()
            .map(|range| range.start_line)
            .collect::<Vec<_>>(),
        vec![0]
    );
}

#[test]
fn logical_qualified_workspace_coordinate_uses_the_same_source_scope_owner() {
    let source = "namespace eval Target {variable x ROOT_TARGET}\nnamespace eval N {\n namespace eval Target {variable x N_TARGET}\n proc p {} {\n  uplevel #0 {puts $Target::x}\n  uplevel 0 {puts $Target::x}\n  uplevel 1 {puts $Target::x}\n  uplevel 1 {puts $::Target::x}\n }\n}\n";
    let analysis = Analyser::new().analyse(source, "tcl");
    assert!(analysis.allows_lexical_declaration_advice());
    for (needle, expected) in [
        ("uplevel #0 {puts $Target::x", Some("::Target::x")),
        ("uplevel 0 {puts $Target::x", Some("::N::Target::x")),
        ("uplevel 1 {puts $Target::x", None),
        ("uplevel 1 {puts $::Target::x", Some("::Target::x")),
    ] {
        let (_, line, character) = cursor(source, needle);
        assert_eq!(
            qualified_variable_cell_at(
                source,
                analysis.resolved_profile().unwrap(),
                &analysis,
                line,
                character
            )
            .as_deref(),
            expected,
            "{needle}"
        );
        assert!(
            qualified_variable_cell_at(
                &format!("{source} "),
                analysis.resolved_profile().unwrap(),
                &analysis,
                line,
                character
            )
            .is_none()
        );
    }
}

#[test]
fn logical_scope_selector_current_image_and_domain_do_not_grant_edits() {
    let analysis = Analyser::new().analyse(SCOPED, "tcl");
    let image = tcl_lexer::SourceImage::document(SCOPED);
    let config = analysis.body_lexer_config.unwrap();
    let (offset, line, character) = cursor(SCOPED, "uplevel #0 {puts $x");
    assert!(
        analysis
            .logical_variable_scope_advice(&image, config, offset)
            .is_some()
    );
    assert!(
        analysis
            .logical_variable_scope_advice(
                &tcl_lexer::SourceImage::document(&format!("{SCOPED} ")),
                config,
                offset
            )
            .is_none()
    );
    assert!(
        analysis
            .logical_variable_scope_advice(
                &tcl_lexer::SourceImage::native(SCOPED.as_bytes()),
                config,
                offset
            )
            .is_none()
    );
    assert!(
        analysis
            .logical_variable_scope_advice(
                &image,
                tcl_lexer::LexerConfig {
                    strict_quoting: !config.strict_quoting,
                    ..config
                },
                offset
            )
            .is_none()
    );
    assert!(
        analysis
            .logical_variable_scope_advice(&image, config, u32::try_from(SCOPED.len() + 1).unwrap())
            .is_none()
    );
    for release in ["tcl8.6", "tcl9.0", "jim", "f5-iapps"] {
        let original = Analyser::new().analyse(SCOPED, release);
        assert!(
            original
                .logical_variable_scope_advice(&image, original.body_lexer_config.unwrap(), offset)
                .is_none()
        );
    }
    let mut detached = analysis.clone();
    detached.resolved_input = None;
    assert!(
        detached
            .logical_variable_scope_advice(&image, config, offset)
            .is_none()
    );
    let stale = format!("{SCOPED} ");
    assert_eq!(
        crate::rename::rename(
            &stale,
            analysis.resolved_profile().unwrap(),
            line,
            character,
            "other",
            &analysis,
            None
        ),
        [] as [crate::rename::TextEdit; 0]
    );
}

#[test]
fn native_source_alias_candidates_cannot_borrow_logical_scope_or_edit_selection() {
    let source =
        "set shared 1\nproc first {argument} {global shared; upvar #0 shared link; puts $}\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let image = tcl_lexer::SourceImage::document(source);
    let config = analysis.body_lexer_config.unwrap();
    assert!(
        analysis
            .original_variable_alias_advice_in_source(&image, config)
            .unwrap()
            .iter()
            .any(|alias| alias.local_name().as_bytes() == b"link")
    );
    analysis.original_variable_symbols.clear();
    let offset = u32::try_from(source.find("puts $").unwrap() + "puts $".len()).unwrap();
    let position = LineIndex::new(source).position_at_utf16(offset, source);
    assert!(
        analysis
            .logical_variable_scope_advice(&image, config, offset)
            .is_none()
    );
    let candidates = crate::completion::completions(
        source,
        position.line,
        position.character.get(),
        &analysis,
        analysis.resolved_registry(),
        None,
        analysis.resolved_profile().unwrap(),
    );
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.insert_text == "$link"
                && candidate.detail.as_deref()
                    == Some("variable — source alias declaration candidate"))
    );
    let alias = u32::try_from(source.find("link;").unwrap()).unwrap();
    let position = LineIndex::new(source).position_at_utf16(alias, source);
    assert_eq!(
        crate::rename::rename(
            source,
            analysis.resolved_profile().unwrap(),
            position.line,
            position.character.get(),
            "other",
            &analysis,
            analysis.resolved_registry()
        ),
        [] as [crate::rename::TextEdit; 0]
    );
}
#[test]
fn original_autoload_requests_keep_original_heads_current_input_and_caller_scope() {
    let source = "namespace eval N {requested arg}\n";
    let offset = u32::try_from(source.find("requested").unwrap()).unwrap();
    let position = LineIndex::new(source).position_at_utf16(offset, source);
    let mut resolver = crate::package_resolver::PackageResolver::default();
    resolver.add_original_tcl_index(
            "# Tcl autoload index file, version 2.0\nset auto_index(::N::requested) [list source [file join $dir nested.tcl]]\nset auto_index(requested) [list source [file join $dir global.tcl]]\n",
            std::path::Path::new("/library"), &|_| true,
        );
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let analysis = Analyser::new().analyse(source, dialect);
        let query = |source: &str, analysis: &AnalysisResult| {
            original_autoload_request_at(source, analysis, position.line, position.character.get())
        };
        let request = query(source, &analysis).expect("retained original C caller");
        assert_eq!(
            request.files(&resolver),
            [std::path::PathBuf::from("/library/nested.tcl")],
            "{dialect}"
        );
        let mut presentation = analysis.clone();
        for invocation in &mut presentation.command_invocations {
            invocation.name = "changed label".into();
            invocation.resolution_candidates = vec!["::other::requested".into()];
        }
        assert_eq!(query(source, &presentation), Some(request.clone()));
        assert!(query(&format!("{source} "), &analysis).is_none());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(query(source, &missing).is_none());
        let mut changed = analysis.clone();
        changed.body_lexer_config.as_mut().unwrap().strict_quoting =
            !analysis.body_lexer_config.unwrap().strict_quoting;
        assert!(query(source, &changed).is_none());
        let arg_position = LineIndex::new(source)
            .position_at_utf16(u32::try_from(source.find("arg").unwrap()).unwrap(), source);
        assert!(
            original_autoload_request_at(
                source,
                &analysis,
                arg_position.line,
                arg_position.character.get()
            )
            .is_none()
        );
        assert!(
            command_head_and_namespace_at(
                source,
                &analysis,
                position.line,
                position.character.get()
            )
            .is_none()
        );
    }
}

#[test]
fn logical_autoload_words_require_current_retained_logical_source() {
    let source = "namespace eval N {requested arg}\n";
    let analysis = Analyser::new().analyse(source, "tcl");
    let position = LineIndex::new(source).position_at_utf16(
        u32::try_from(source.find("requested").unwrap()).unwrap(),
        source,
    );
    assert_eq!(
        command_head_and_namespace_at(source, &analysis, position.line, position.character.get()),
        Some(("requested".into(), "::N".into()))
    );
    assert!(
        command_head_and_namespace_at(
            &format!("{source} "),
            &analysis,
            position.line,
            position.character.get()
        )
        .is_none()
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(
        command_head_and_namespace_at(source, &missing, position.line, position.character.get())
            .is_none()
    );
}
