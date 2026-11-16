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
fn conditional_named_instance_reports_cannot_enter_legacy_label_classification() {
    let library = Analyser::new().analyse(
        "oo::class create C {method ping {} {}}; C create obj",
        "tcl",
    );
    let cards =
        tcl_compiler::registry_invocation::source_structure::OriginalProjectClassCards::capture(
            &library,
        )
        .unwrap();
    let mut reports = NamedInstanceMap::default();
    reports.extend_conditional_source_cards(&cards);
    assert_eq!(reports.get("::obj"), Some("::C"));
    assert_eq!(
        reports.report_rows().collect::<Vec<_>>(),
        vec![("::obj", "::C")]
    );
    assert_eq!(reports.compatibility_rows().count(), 0);
    let mut merged = NamedInstanceMap::default();
    merged.merge_reports(&reports);
    assert_eq!(merged, reports);
    assert_eq!(merged.compatibility_rows().count(), 0);
    let source = "obj ping";
    let caller = Analyser::new().analyse(source, "tcl");
    let tokens = full_with_cu_and_facts(
        source,
        caller.resolved_profile().unwrap(),
        caller.resolved_registry().unwrap(),
        None,
        WorkspaceTokenFacts {
            classes: Some(library.class_hierarchy()),
            project_classes: None,
            proc_roles: None,
            named_instances: Some(&merged),
            analysis: Some(&caller),
        },
    );
    assert_ne!(
        token_kind_at(
            source,
            &tests::decode_semantic(&tokens),
            source.find("ping").unwrap()
        ),
        Some(TokenKind::Method as u32)
    );
    let providers = [cards];
    let tokens = full_with_cu_and_facts(
        source,
        caller.resolved_profile().unwrap(),
        caller.resolved_registry().unwrap(),
        None,
        WorkspaceTokenFacts {
            classes: Some(library.class_hierarchy()),
            project_classes: Some(&providers),
            proc_roles: None,
            named_instances: Some(&merged),
            analysis: Some(&caller),
        },
    );
    assert_eq!(
        token_kind_at(
            source,
            &tests::decode_semantic(&tokens),
            source.find("ping").unwrap()
        ),
        Some(TokenKind::Method as u32)
    );
}

#[test]
// naming.core.original-math-function-selection
// docs/design/analysis/name-resolution-proofs/core-original-math-function-selection.md
fn logical_math_default_library_uses_configured_source_catalogue() {
    let source = "expr {sqrt(4) + cbrt(8) + invented(0)}";
    let tokens = retained_tokens(source, "tcl");
    for (name, expected) in [("sqrt", MOD_DEFAULT_LIBRARY), ("cbrt", 0), ("invented", 0)] {
        let start = u32::try_from(source.find(name).unwrap()).unwrap();
        assert!(
            tokens.iter().any(|&(line, col, _, kind, modifiers)| {
                line == 0
                    && col == start
                    && kind == TokenKind::Function as u32
                    && modifiers == expected
            }),
            "{name}: {tokens:?}"
        );
    }
}

fn retained_tokens(source: &str, dialect: &str) -> Vec<(u32, u32, u32, u32, u32)> {
    let analysis = Analyser::new().analyse(source, dialect);
    tests::decode_semantic(&full_with_cu_and_analysis(
        source,
        analysis.resolved_profile().unwrap(),
        analysis.resolved_registry().unwrap(),
        None,
        Some(&analysis),
    ))
}

fn token_kind_at(source: &str, tokens: &[(u32, u32, u32, u32, u32)], offset: usize) -> Option<u32> {
    let position = LineIndex::new(source).position_at_utf16(u32::try_from(offset).unwrap(), source);
    tokens.iter().find_map(|&(line, column, _, kind, _)| {
        (line == position.line && column == position.character.get()).then_some(kind)
    })
}

#[test]
fn member_source_roles_keep_original_heads_names_formals_and_availability() {
    for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
        for source in [
            r"oo::class create C {self self metho\u0064 café {argument {optional 2}} {puts visible}}",
            r"oo::class create C {}; oo::define C self metho\u0064 café {argument {optional 2}} {puts visible}",
        ] {
            let tokens = retained_tokens(source, dialect);
            for (word, kind) in [
                (r"metho\u0064", TokenKind::Keyword),
                ("café", TokenKind::Method),
                ("argument", TokenKind::Parameter),
                ("puts visible", TokenKind::Function),
            ] {
                assert_eq!(
                    token_kind_at(source, &tokens, source.find(word).unwrap()),
                    Some(kind as u32),
                    "{dialect}: {source}: {tokens:?}"
                );
            }
        }
    }
    let source = "oo::class create C {self property name}";
    let tokens = retained_tokens(source, "tcl8.6");
    assert_ne!(
        token_kind_at(source, &tokens, source.find("property").unwrap()),
        Some(TokenKind::Keyword as u32)
    );
    let source = "oo::class create C {definitionnamespace -instance ::defs}";
    let tokens = retained_tokens(source, "tcl9.0");
    for (word, kind) in [
        ("-instance", TokenKind::Decorator),
        ("::defs", TokenKind::Namespace),
    ] {
        assert_eq!(
            token_kind_at(source, &tokens, source.find(word).unwrap()),
            Some(kind as u32)
        );
    }
    let source =
        r"proc oo::class args {}; oo::class create C {metho\u0064 hidden {argument} {puts inert}}";
    let tokens = retained_tokens(source, "tcl");
    assert_ne!(
        token_kind_at(source, &tokens, source.find("hidden").unwrap()),
        Some(TokenKind::Method as u32)
    );
}

#[test]
fn original_definition_colours_use_genuine_wrappers_and_clear_ordinary_bodies() {
    // naming.core.original-command-source-schema
    // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
    // tclsh 8.6.18, 9.0.4 and 9.1.0 run this and report `{plain café} café`
    // for the class and object methods. `self self {…}` is not a wrapper
    // (8.6: invalid command name "self"; 9.0: wrong # args), and a bare
    // `apply` of the lambda raises invalid command name "method", so the
    // wrapper is `self {…}` and the lambda's error is caught.
    let source = "oo::class create C {\nself {method café {} {puts wrapper}}\nif 1 {method café {} {puts immediate}}\nmethod plain {} {method ordinary {} {puts inert}}\nproc p {} {method ordinary {} {puts inert}}\ncatch {apply {{} {method ordinary {} {puts inert}}}}\n}";
    let tokens = retained_tokens(source, "tcl8.6");
    for (offset, _) in source.match_indices("method café") {
        assert_eq!(
            token_kind_at(source, &tokens, offset),
            Some(TokenKind::Keyword as u32),
            "{offset}: {tokens:?}"
        );
    }
    for word in ["puts wrapper", "puts immediate"] {
        assert_eq!(
            token_kind_at(source, &tokens, source.find(word).unwrap()),
            Some(TokenKind::Function as u32),
            "{word}: {tokens:?}"
        );
    }
    for (offset, _) in source.match_indices("method ordinary") {
        assert_eq!(
            token_kind_at(source, &tokens, offset),
            Some(TokenKind::Function as u32)
        );
    }
    for (offset, _) in source.match_indices("puts inert") {
        assert_ne!(
            token_kind_at(source, &tokens, offset),
            Some(TokenKind::Function as u32)
        );
    }
}

#[test]
fn original_definition_keyword_colours_preserve_whole_escaped_source_extent() {
    // naming.core.original-command-source-schema
    // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
    let source = r"oo::class create C {metho\u0064 café {} {puts visible}}";
    let tokens = retained_tokens(source, "tcl8.6");
    let start = source.find("metho").unwrap();
    let position = LineIndex::new(source).position_at_utf16(u32::try_from(start).unwrap(), source);
    let head = tokens
        .iter()
        .find(|&&(line, column, _, kind, _)| {
            line == position.line
                && column == position.character.get()
                && kind == TokenKind::Keyword as u32
        })
        .unwrap();
    assert_eq!(
        head.2, 11,
        "the raw escape stays within the one original word"
    );
    assert_eq!(
        token_kind_at(source, &tokens, source.find("puts").unwrap()),
        Some(TokenKind::Function as u32)
    );
    assert!(
        !tokens
            .iter()
            .any(|&(line, column, _, _, _)| line == position.line
                && position.character.get() < column
                && column < position.character.get() + head.2)
    );
}

#[test]
fn original_definition_colours_keep_keyword_body_and_release_applicability_separate() {
    // naming.core.original-command-source-schema
    // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
    for (source, dialect, keyword, body) in [
        // A substituted parameter list does not hide the body. The list is
        // bound first: with `params` unset tclsh 8.6.18 stops at
        // `can't read "params"` and defines nothing; bound, it prints
        // `café` for `info class methods C`. (tclsh 9.0.4 cannot read a
        // global from the definition script, so this row is 8.6 only.)
        (
            "set params {}\noo::class create C {method café $params {puts visible}}",
            "tcl8.6",
            true,
            true,
        ),
        (
            "oo::class create C {method café -private {} {puts visible}}",
            "tcl9.0",
            true,
            true,
        ),
        (
            "oo::class create C {method café -private {} {puts hidden}}",
            "tcl8.6",
            true,
            false,
        ),
        (
            r#"oo::class create C {method café {} "\u0070uts hidden"}"#,
            "tcl8.6",
            true,
            false,
        ),
        (
            "oo::class create C {property p -get {puts hidden}}",
            "tcl8.6",
            false,
            false,
        ),
        (
            "oo::class create C {property p -get {puts visible}}",
            "tcl9.1",
            true,
            true,
        ),
    ] {
        let tokens = retained_tokens(source, dialect);
        let offset = source
            .find("method")
            .or_else(|| source.find("property"))
            .unwrap();
        assert_eq!(
            token_kind_at(source, &tokens, offset) == Some(TokenKind::Keyword as u32),
            keyword,
            "{dialect}: {source}: {tokens:?}"
        );
        if let Some(offset) = source.find("puts") {
            assert_eq!(
                token_kind_at(source, &tokens, offset) == Some(TokenKind::Function as u32),
                body,
                "{dialect}: {source}: {tokens:?}"
            );
        } else {
            assert!(
                !tokens.iter().any(|&(line, column, _, kind, _)| {
                    let begin = source.find("\\u0070uts").unwrap();
                    let position = LineIndex::new(source)
                        .position_at_utf16(u32::try_from(begin).unwrap(), source);
                    line == position.line
                        && column == position.character.get()
                        && kind == TokenKind::Function as u32
                }),
                "cooked source has no original executable body extent"
            );
        }
    }
}
