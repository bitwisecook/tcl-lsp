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
use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};

fn analysis(source: &str) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::projected_from_point(
        "package-authoring-logical",
        &[],
        "Explicit package authoring",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )
    .intern();
    let registry = tcl_registry::CommandRegistry::build_default();
    let context = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.6")
            .with_command_store(std::sync::Arc::new(registry)),
    );
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    Analyser::new()
        .with_resolved_input(ResolvedAnalysisInput::new(
            profile, profile, context, config,
        ))
        .analyse(source, profile.name)
}

#[test]
fn authored_package_metacharacters_remain_one_inert_complete_word() {
    let source = "puts done\n";
    let current = analysis(source);
    for package in ["a b", "a; puts injected", "$name[command]", "a\\b"] {
        let result = package_require_source(source, &current, package)
            .expect("configured ASCII package authoring");
        assert!(!result.already_required);
        let config = current.body_lexer_config.unwrap();
        let plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(&result.source),
            tcl_lexer::Span::new(0, u32::try_from(result.source.len()).unwrap()),
            config,
        )
        .unwrap();
        assert!(plan.fatal_tail.is_none());
        assert_eq!(plan.commands.len(), 2);
        assert_eq!(plan.commands[0].words.len(), 3);
        assert_eq!(
            tcl_syntax::word_rules::original_static_word_unicode_value(&plan.commands[0].words[2])
                .unwrap(),
            package.as_bytes()
        );
        assert!(result.source.ends_with(source));
    }
}

#[test]
fn authored_package_insertion_preserves_unicode_prologue_and_quoted_requirement() {
    let source = "#! interpreter 😀\npackage require {old name}\nputs café\n";
    let current = analysis(source);
    let result = package_require_source(source, &current, "new name").unwrap();
    assert_eq!(
        result.source,
        "#! interpreter 😀\npackage require {old name}\npackage require {new name}\nputs café\n"
    );
    let duplicate = package_require_source(source, &current, "old name").unwrap();
    assert!(duplicate.already_required);
    assert_eq!(duplicate.source, source);
}

#[test]
fn package_authoring_withdraws_stale_source_full_config_and_foreign_input() {
    let source = "puts done\n";
    let current = analysis(source);
    assert!(package_require_source(source, &current, "owned").is_some());
    assert!(package_require_source("puts changed\n", &current, "owned").is_none());
    let mut changed = current.clone();
    changed.body_lexer_config.as_mut().unwrap().strict_quoting =
        !changed.body_lexer_config.unwrap().strict_quoting;
    assert!(package_require_source(source, &changed, "owned").is_none());
    let mut foreign = current.clone();
    let input = foreign.resolved_input.as_ref().unwrap();
    foreign.resolved_input = Some(ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry(),
        input.lexer_config(),
    ));
    assert!(package_require_source(source, &foreign, "owned").is_none());
    assert!(package_require_source(source, &foreign, "café").is_none());
    let mut missing = current;
    missing.resolved_input = None;
    assert!(package_require_source(source, &missing, "owned").is_none());
}

#[test]
fn package_authoring_distinguishes_unicode_values_and_unknown_coverage() {
    let source = "puts done\n";
    let unicode = package_require_source(source, &analysis(source), "café").unwrap();
    assert_eq!(unicode.source, "package require café\nputs done\n");
    let foreign = "package require {café différent}\nputs done\n";
    let inserted = package_require_source(foreign, &analysis(foreign), "café").unwrap();
    assert!(!inserted.already_required);
    assert_eq!(
        inserted.source,
        "package require {café différent}\npackage require café\nputs done\n"
    );
    let duplicate =
        package_require_source(&inserted.source, &analysis(&inserted.source), "café").unwrap();
    assert!(duplicate.already_required);
    assert_eq!(duplicate.source, inserted.source);
    let dynamic = "package require $selected\n";
    assert!(package_require_source(dynamic, &analysis(dynamic), "owned").is_none());
}
