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
use std::sync::Arc;
use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};

fn analyse_with(
    source: &str,
    context: Arc<tcl_registry::model::ContextRegistry>,
) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "reporting label")
}

#[test]
fn special_variable_source_card_never_overrides_declaration_or_unavailable_root() {
    let context = tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
        .unwrap()
        .default_context_registry();
    let source = "set auto_path custom; puts $auto_path";
    let analysis = analyse_with(source, context.clone());
    let cursor = u32::try_from(source.rfind("$auto_path").unwrap() + 2).unwrap();
    let card = hover(source, 0, cursor, &analysis, None).unwrap();
    assert!(!card.value.contains("special variable"), "{}", card.value);

    for source in [
        "proc p {} {puts $auto_path}",
        "namespace eval ::foo {puts $env}",
    ] {
        let analysis = analyse_with(source, context.clone());
        let cursor = u32::try_from(source.find('$').unwrap() + 2).unwrap();
        if let Some(card) = hover(source, 0, cursor, &analysis, None) {
            assert!(
                !card.value.contains("special variable") && !card.value.contains("special array"),
                "{}",
                card.value
            );
        }
    }
    for rooted in [
        "puts $::auto_path",
        "proc p {} {puts $::auto_path}",
        "namespace eval ::foo {puts $::auto_path}",
    ] {
        let rooted_analysis = analyse_with(rooted, context.clone());
        let rooted_cursor = u32::try_from(rooted.find("auto_path").unwrap() + 1).unwrap();
        assert!(
            hover(rooted, 0, rooted_cursor, &rooted_analysis, None)
                .is_some_and(|card| card.value.contains("special variable")),
            "{rooted}"
        );
    }
    for source in [
        "set ::auto_path custom; puts $auto_path",
        "namespace eval ::foo {puts $auto_path}",
        "puts $::foo::auto_path",
    ] {
        let analysis = analyse_with(source, context.clone());
        let cursor = u32::try_from(source.rfind("auto_path").unwrap() + 1).unwrap();
        assert!(
            hover(source, 0, cursor, &analysis, None)
                .is_none_or(|card| !card.value.contains("special variable")),
            "{source}"
        );
    }

    for source in ["puts {$auto_path}", "# $auto_path", "puts \\$auto_path"] {
        let analysis = analyse_with(source, context.clone());
        let cursor = u32::try_from(source.find("$auto_path").unwrap() + 2).unwrap();
        assert!(
            hover(source, 0, cursor, &analysis, None).is_none(),
            "{source}"
        );
    }
}

#[test]
fn special_variable_card_requires_occurrence_and_excludes_array_index() {
    let context = tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
        .unwrap()
        .default_context_registry();
    let source = "puts $auto_path";
    let mut analysis = analyse_with(source, context.clone());
    let cursor = u32::try_from(source.find("auto_path").unwrap() + 1).unwrap();
    assert!(hover(source, 0, cursor, &analysis, None).is_some());
    analysis.original_variable_symbols.clear();
    assert!(hover(source, 0, cursor, &analysis, None).is_none());

    let source = "puts $tcl_platform(os)";
    let analysis = analyse_with(source, context);
    let root = u32::try_from(source.find("tcl_platform").unwrap() + 1).unwrap();
    assert!(
        hover(source, 0, root, &analysis, None)
            .is_some_and(|card| card.value.contains("special array"))
    );
    let index = u32::try_from(source.find("(os)").unwrap() + 2).unwrap();
    assert!(
        hover(source, 0, index, &analysis, None)
            .is_none_or(|card| !card.value.contains("special array"))
    );
}

#[test]
fn original_special_hover_uses_actual_availability_and_withdraws_changed_owners() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let source = "puts $tcl_precision";
    let driver = tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
        .unwrap()
        .default_context_registry();
    let registry = driver.commands();
    for (provider, expected) in [("tcl8.6", true), ("tcl9.1", false)] {
        let selected = tcl_registry::model::ingress::resolve_known_environment(provider)
            .unwrap()
            .default_context_registry();
        let context = Arc::new(selected.with_command_store(registry.snapshot().shared_registry()));
        let analysis = analyse_with(source, context.clone());
        assert!(analysis.allows_lexical_declaration_advice());
        let answer = hover(source, 0, 8, &analysis, None);
        assert_eq!(answer.is_some(), expected, "{provider}: {answer:?}");
        if let Some(answer) = answer {
            assert!(
                answer.value.contains("significant digits"),
                "{}",
                answer.value
            );
            assert!(
                answer.value.contains("special variable"),
                "{}",
                answer.value
            );
        }
        assert!(hover("puts $tcl_precisioN", 0, 8, &analysis, None).is_none());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(hover(source, 0, 8, &missing, None).is_none());
        let mut grammar = analysis.clone();
        grammar.body_lexer_config.as_mut().unwrap().strict_quoting = true;
        assert!(hover(source, 0, 8, &grammar, None).is_none());
        let foreign = tcl_registry::model::ingress::resolve_known_environment("tcl8.4")
            .unwrap()
            .default_context_registry();
        let other_provider = if provider == "tcl8.6" {
            "tcl9.1"
        } else {
            "tcl8.6"
        };
        let other = tcl_registry::model::ingress::resolve_known_environment(other_provider)
            .unwrap()
            .default_context_registry();
        let changed_availability =
            Arc::new(other.with_command_store(registry.snapshot().shared_registry()));
        for changed_context in [foreign, changed_availability] {
            let mut changed = analysis.clone();
            let input = analysis.resolved_input.as_ref().unwrap();
            changed.resolved_input = Some(ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                changed_context,
                input.lexer_config(),
            ));
            assert!(hover(source, 0, 8, &changed, None).is_none());
        }
        let mut unavailable = analysis.clone();
        unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
            environment: provider.to_owned(),
            overlay: 0x385,
        });
        assert!(hover(source, 0, 8, &unavailable, None).is_none());
    }
}

#[test]
fn original_special_hover_keeps_authored_registry_shadow_rows_and_array_key_availability() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let driver = tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
        .unwrap()
        .default_context_registry();
    let mut registry = CommandRegistry::build_default();
    let shipped = *registry.special_var("auto_path").unwrap();
    let authored = Box::leak(Box::new(tcl_registry::SpecialVarSpec {
        summary: "Authored source search-path documentation",
        ..shipped
    }));
    // This is the same shared insertion owner used by pack special_var rows.
    // It declares readonly documentation, not an installed interpreter cell.
    registry.insert_special_var(authored);
    let registry = Arc::new(registry);
    let context = Arc::new(driver.with_command_store(registry.clone()));
    let source = "puts $auto_path";
    let analysis = analyse_with(source, context);
    let answer = hover(source, 0, 8, &analysis, None).expect("authored variable card");
    assert!(answer.value.contains(authored.summary), "{}", answer.value);
    assert!(!answer.value.contains(shipped.summary), "{}", answer.value);
    let mut withdrawn = analysis.clone();
    let input = analysis.resolved_input.as_ref().unwrap();
    withdrawn.resolved_input = Some(ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        driver.clone(),
        input.lexer_config(),
    ));
    assert!(hover(source, 0, 8, &withdrawn, Some(&registry)).is_none());

    let source = "puts $tcl_platform(os)";
    for (provider, has_pointer_size) in [("tcl8.4", false), ("tcl9.1", true)] {
        let selected = tcl_registry::model::ingress::resolve_known_environment(provider)
            .unwrap()
            .default_context_registry();
        let context = Arc::new(selected.with_command_store(registry.clone()));
        let analysis = analyse_with(source, context);
        let answer = hover(source, 0, 8, &analysis, None).expect("platform source card");
        assert!(answer.value.contains("special array"), "{}", answer.value);
        assert_eq!(
            answer.value.contains("`pointerSize`"),
            has_pointer_size,
            "{provider}: {}",
            answer.value
        );
    }
}
