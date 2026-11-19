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
fn original_pattern_hover_keeps_actual_schema_operands_and_current_source() {
    // naming.core.original-command-source-schema
    // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
    let source = "format {%04d} 7";
    let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    analysis.command_invocations.clear();
    analysis.all_procs.clear();
    let cursor = u32::try_from(source.find("%04d").unwrap()).unwrap();
    let selected = original_pattern_format_hover(
        source,
        &analysis,
        analysis.resolved_registry().unwrap(),
        cursor,
    )
    .expect("original source format role");
    assert!(selected.value.contains("**Format string**"));
    assert!(
        original_pattern_format_hover(
            &source.replace("%04d", "%08d"),
            &analysis,
            analysis.resolved_registry().unwrap(),
            cursor
        )
        .is_none()
    );
    for source in [
        "set data {%04d}",
        "proc format args {}; format {%04d} 7",
        "rename format {}; format {%04d} 7",
        "interp alias {} custom {} format {%04d}; custom 7",
    ] {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let cursor = u32::try_from(source.find("%04d").unwrap()).unwrap();
        assert!(
            original_pattern_format_hover(
                source,
                &analysis,
                analysis.resolved_registry().unwrap(),
                cursor
            )
            .is_none(),
            "{source}"
        );
    }
}

#[test]
fn original_format_hover_keeps_selected_availability_and_generation() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "clock scan 2020 -format {%Y}";
    let profile = crate::profile_for_dialect("tcl9.1");
    let driver =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let cursor = u32::try_from(source.find("%Y").unwrap()).unwrap();
    for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
        let actual = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        let context = std::sync::Arc::new(
            actual.with_command_store(driver.commands().snapshot().shared_registry()),
        );
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let mut analysis = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        let hover = original_pattern_format_hover(source, &analysis, context.commands(), cursor);
        assert_eq!(hover.is_some(), expected, "{environment}");
        if let Some(hover) = hover {
            assert!(hover.value.contains("4-digit year"), "{}", hover.value);
        }
        assert!(
            original_pattern_format_hover(
                "clock scan 2020 -format {%m}",
                &analysis,
                context.commands(),
                cursor,
            )
            .is_none()
        );
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(
                context.with_command_store(context.commands().snapshot().shared_registry()),
            ),
            input.lexer_config(),
        ));
        assert!(
            original_pattern_format_hover(source, &analysis, context.commands(), cursor).is_none()
        );
    }
}

#[test]
fn original_pattern_hover_keeps_selected_availability_and_generation() {
    // naming.core.original-pattern-retained-context
    // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
    let source = "lsearch -regexp -stride 2 {a b} {a+}";
    let profile = crate::profile_for_dialect("tcl9.1");
    let driver =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let cursor = u32::try_from(source.find("a+").unwrap()).unwrap();
    for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
        let actual = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        let context = std::sync::Arc::new(
            actual.with_command_store(driver.commands().snapshot().shared_registry()),
        );
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let mut analysis = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "reporting-only");
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        let hover = original_pattern_format_hover(source, &analysis, context.commands(), cursor);
        assert_eq!(hover.is_some(), expected, "{environment}");
        if let Some(hover) = hover {
            assert!(hover.value.contains("Regex pattern"));
        }
        assert!(
            original_pattern_format_hover(
                &source.replace("a+", "b+"),
                &analysis,
                context.commands(),
                cursor
            )
            .is_none()
        );
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(
                context.with_command_store(context.commands().snapshot().shared_registry()),
            ),
            input.lexer_config(),
        ));
        assert!(
            original_pattern_format_hover(source, &analysis, context.commands(), cursor).is_none()
        );
    }
}

#[test]
fn original_pattern_hover_keeps_alias_written_operands_and_captured_refusal() {
    // naming.core.original-pattern-retained-context
    // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
    for (source, expected) in [
        (
            "interp alias {} search {} lsearch -regexp; search {a b} {a+}",
            true,
        ),
        (
            "interp alias {} search {} lsearch -regexp {a b} {a+}; search",
            false,
        ),
        ("lsearch $mode {a b} {a+}", false),
        ("lsearch {*}$arguments {a+}", false),
        ("proc lsearch args {}; lsearch -regexp {a b} {a+}", false),
    ] {
        let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.1");
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        let cursor = u32::try_from(source.find("a+").unwrap()).unwrap();
        let hover = original_pattern_format_hover(
            source,
            &analysis,
            analysis.resolved_registry().unwrap(),
            cursor,
        );
        assert_eq!(hover.is_some(), expected, "{source}");
        if let Some(hover) = hover {
            assert!(hover.value.contains("Regex pattern"));
        }
    }
}

#[test]
fn pattern_hover_entry_keeps_explicit_logical_source_context_and_nested_words() {
    // naming.core.original-pattern-retained-context
    // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
    let mut profile = crate::profile_for_dialect("f5-irules").clone();
    profile.name = "explicit-logical-pattern-source";
    let profile = profile.intern();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context.clone(),
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    for (source, text, heading) in [
        ("string match -nocase {a*} $value", "a*", "Glob pattern"),
        ("proc p {} {regexp {a+} $value}", "a+", "Regex pattern"),
        ("if {1} {format {%04d} 7}", "%04d", "Format string"),
        (
            "interp alias {} search {} lsearch -regexp; search {a b} {a+}",
            "a+",
            "Regex pattern",
        ),
    ] {
        let mut analysis = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "reporting-only");
        assert!(analysis.allows_lexical_declaration_advice());
        assert!(!analysis.has_original_vendor_source_names());
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        let cursor = u32::try_from(source.find(text).unwrap()).unwrap();
        let position = tcl_lexer::LineIndex::new(source).position_at_utf16(cursor, source);
        let selected = hover(
            source,
            position.line,
            position.character.get(),
            &analysis,
            Some(&CommandRegistry::build_default()),
        )
        .expect("retained Logical embedded language");
        assert!(
            selected.value.contains(heading),
            "{source}: {}",
            selected.value
        );
        assert!(
            registry_pattern_format_hover(
                &source.replace(text, "different"),
                position.line,
                position.character.get(),
                &analysis,
                context.commands()
            )
            .is_none()
        );
        analysis.resolved_input = None;
        assert!(
            registry_pattern_format_hover(
                source,
                position.line,
                position.character.get(),
                &analysis,
                context.commands()
            )
            .is_none()
        );
    }
}

#[test]
fn pattern_hover_entry_keeps_logical_shadow_and_captured_anchor_refusals() {
    // naming.core.original-pattern-retained-context
    // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
    let mut profile = crate::profile_for_dialect("f5-irules").clone();
    profile.name = "explicit-logical-pattern-refusals";
    let profile = profile.intern();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context.clone(),
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    for source in [
        "proc lsearch args {}; lsearch -regexp {a b} {a+}",
        "interp alias {} search {} lsearch -regexp {a b} {a+}; search",
        "lsearch $mode {a b} {a+}",
        "lsearch {*}$arguments {a+}",
    ] {
        let analysis = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "reporting-only");
        let cursor = u32::try_from(source.find("a+").unwrap()).unwrap();
        let position = tcl_lexer::LineIndex::new(source).position_at_utf16(cursor, source);
        assert!(
            registry_pattern_format_hover(
                source,
                position.line,
                position.character.get(),
                &analysis,
                context.commands()
            )
            .is_none(),
            "{source}"
        );
    }
}

#[test]
fn original_pattern_hover_preserves_full_lexer_configuration() {
    // naming.core.original-command-source-schema
    // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
    let source = "\u{feff}format {%04d} 7";
    let context = tcl_registry::model::ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    );
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let cursor = u32::try_from(source.find("%04d").unwrap()).unwrap();
    for (leading_bom, expected) in [
        (tcl_lexer::LeadingBom::Skip, true),
        (tcl_lexer::LeadingBom::Content, false),
    ] {
        let config = tcl_lexer::LexerConfig {
            leading_bom,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let analysis = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "display-only");
        assert_eq!(
            original_pattern_format_hover(source, &analysis, context.commands(), cursor).is_some(),
            expected
        );
    }
}

#[test]
fn regex_hover_uses_the_selected_engine_and_command_options() {
    fn card(source: &str, dialect: &str, needle: &str) -> Option<String> {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, dialect);
        let column = u32::try_from(source.find(needle)?).ok()?;
        registry_pattern_format_hover(source, 0, column, &analysis, analysis.resolved_registry()?)
            .map(|hover| hover.value)
    }
    let literal = "regexp {***=a.*} literal";
    let ordinary = card(literal, "tcl8.6", "a.*").unwrap();
    assert!(ordinary.contains("Pattern options"));
    assert!(!ordinary.contains("Repetition"));
    assert!(card(literal, "jim", "a.*").is_none());
    assert!(card("regexp {(?>a)} a", "tcl9.1", "?>").is_none());
    let source = "regexp -expanded {a # ()+} a";
    let expanded = card(source, "tcl9.1", "()").unwrap();
    assert!(!expanded.contains("Group open"), "{expanded}");
    let compact = card("regexp {a # ()+} a", "tcl9.1", "()").unwrap();
    assert!(compact.contains("Group open"), "{compact}");
}
