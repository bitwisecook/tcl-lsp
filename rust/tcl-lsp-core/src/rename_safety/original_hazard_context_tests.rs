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

fn store() -> Arc<tcl_registry::CommandRegistry> {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let surface = registry.get("dict").unwrap().surface;
    registry.insert(tcl_registry::CommandSpec {
        name: "source_var",
        arg_roles: &[(0, tcl_registry::ArgRole::VarWrite)],
        arity: tcl_registry::Arity::exact(2),
        surface,
        ..tcl_registry::CommandSpec::DEFAULT
    });
    registry.insert(tcl_registry::CommandSpec {
        name: "source_my",
        traits: tcl_registry::Traits::TCLOO_SELF_DISPATCH,
        arity: tcl_registry::Arity::at_least(1),
        surface,
        ..tcl_registry::CommandSpec::DEFAULT
    });
    Arc::new(registry)
}

fn analysis(
    source: &str,
    environment: &str,
    store: Arc<tcl_registry::CommandRegistry>,
    config: tcl_lexer::LexerConfig,
) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = Arc::new(
        tcl_registry::model::ingress::static_context_for(environment).with_command_store(store),
    );
    Analyser::new()
        .with_resolved_input(ResolvedAnalysisInput::new(
            profile, profile, context, config,
        ))
        .analyse(source, profile.name)
}

fn var_reason(source: &str, analysis: &AnalysisResult) -> Option<String> {
    namespace_variable_rename_hazard(
        source,
        tcl_dialect::DialectProfile::plain_tcl(),
        analysis,
        "::ns::v",
        &LineIndex::new(source),
    )
    .map(|refusal| refusal.reason)
}

#[test]
fn original_variable_hazards_keep_selected_alias_origins_and_unknown_roles() {
    // naming.core.original-dynamic-name-value-purpose
    // docs/design/analysis/name-resolution-proofs/core-original-dynamic-name-value-purpose.md
    // Conditional source hazard advice; no current variable cell or rename grant.
    let store = store();
    let config =
        tcl_lexer::LexerConfig::from_grammar(tcl_dialect::DialectProfile::plain_tcl().grammar);
    for source in [
        "source_var ::ns::$n 1",
        "interp alias {} write {} source_var; write ::ns::$n 1",
    ] {
        let current = analysis(source, "tcl8.6", store.clone(), config);
        assert!(
            var_reason(source, &current)
                .unwrap()
                .contains("computed at run time")
        );
        let unavailable = analysis(source, "tcl8.4", store.clone(), config);
        assert!(
            var_reason(source, &unavailable)
                .unwrap()
                .contains("roles are unavailable")
        );
    }
    for source in [
        "source_var {::ns::$n} 1",
        "interp alias {} write {} source_var ::other::fixed; write $value",
    ] {
        assert!(
            var_reason(source, &analysis(source, "tcl8.6", store.clone(), config)).is_none(),
            "{source}"
        );
    }
    let source = "proc source_var args {}; source_var ::ns::$n 1";
    assert!(
        var_reason(source, &analysis(source, "tcl8.6", store, config))
            .unwrap()
            .contains("roles are unavailable")
    );
}

#[test]
fn supplied_variable_hazard_patterns_keep_qualified_and_open_roots_possible() {
    // naming.core.original-dynamic-name-value-purpose
    // docs/design/analysis/name-resolution-proofs/core-original-dynamic-name-value-purpose.md
    let store = store();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    for (source, cell, expected) in [
        ("source_var ::other::$n 1", "::ns::v", false),
        ("source_var ${prefix}::v 1", "::ns::v", true),
        ("source_var a($key 1", "::ns::a(TAIL", true),
        ("source_var a($key) 1", "::ns::a(TAIL", false),
        (
            "interp alias {} write {} source_var ::other::fixed; write $value",
            "::ns::v",
            false,
        ),
    ] {
        let current = analysis(source, "tcl8.6", store.clone(), config);
        assert_eq!(
            namespace_variable_rename_hazard(
                source,
                profile,
                &current,
                cell,
                &LineIndex::new(source),
            )
            .is_some(),
            expected,
            "{source}"
        );
    }
    let source = "source_var ::other::$n 1";
    let older = analysis(source, "tcl8.4", store, config);
    assert!(
        var_reason(source, &older)
            .unwrap()
            .contains("roles are unavailable")
    );
}

#[test]
fn supplied_variable_hazard_patterns_do_not_exclude_native_or_missing_targets() {
    // naming.core.original-dynamic-name-value-purpose
    // docs/design/analysis/name-resolution-proofs/core-original-dynamic-name-value-purpose.md
    use tcl_compiler::analyser::Analyser;
    let source = "set ::other::$n 1";
    for dialect in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jim",
        "f5-irules",
    ] {
        let current = Analyser::new().analyse(source, dialect);
        assert!(
            namespace_variable_rename_hazard(
                source,
                current.resolved_profile().unwrap(),
                &current,
                "::ns::v",
                &LineIndex::new(source),
            )
            .is_some(),
            "{dialect}"
        );
    }
    let mut current = Analyser::new().analyse(source, "tcl");
    assert!(var_reason(source, &current).is_none());
    current.resolved_input = None;
    assert!(
        var_reason(source, &current)
            .unwrap()
            .contains("metadata are unavailable")
    );
}

#[test]
fn original_hazard_walk_preserves_grammar_and_missing_foreign_owner_refusal() {
    // naming.core.original-dynamic-name-value-purpose
    // docs/design/analysis/name-resolution-proofs/core-original-dynamic-name-value-purpose.md
    let source = "if 1 {source_var ${a{b}c} 1}";
    let store = store();
    let mut config =
        tcl_lexer::LexerConfig::from_grammar(tcl_dialect::DialectProfile::plain_tcl().grammar);
    config.braced_var = tcl_lexer::BracedVarStyle::Tcl9Nesting;
    let mut current = analysis(source, "tcl8.6", store.clone(), config);
    assert!(var_reason(source, &current).is_some());
    let mut old = config;
    old.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
    assert!(var_reason(source, &analysis(source, "tcl8.6", store, old)).is_none());
    current.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
    assert!(
        var_reason(source, &current)
            .unwrap()
            .contains("metadata are unavailable")
    );
    current.body_lexer_config = Some(config);
    current.resolved_input = Some(ResolvedAnalysisInput::new(
        tcl_dialect::DialectProfile::plain_tcl(),
        tcl_dialect::DialectProfile::plain_tcl(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        config,
    ));
    assert!(
        var_reason(source, &current)
            .unwrap()
            .contains("metadata are unavailable")
    );
    let mut count = 0;
    walk_document(
        source,
        tcl_dialect::DialectProfile::plain_tcl(),
        &current,
        &mut |_| count += 1,
    );
    assert_eq!(count, 0);
    current.resolved_input = None;
    assert!(
        var_reason(source, &current)
            .unwrap()
            .contains("metadata are unavailable")
    );
}

#[test]
fn original_self_hazards_use_selected_helpers_and_captured_selector_positions() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let store = store();
    let config =
        tcl_lexer::LexerConfig::from_grammar(tcl_dialect::DialectProfile::plain_tcl().grammar);
    for (source, expected) in [
        ("oo::class create C {method m {} {source_my $selector}}", 1),
        ("oo::class create C {method m {} {source_my m$suffix}}", 1),
        (
            "interp alias {} call_my {} source_my; oo::class create C {method m {} {call_my $selector}}",
            1,
        ),
        (
            "interp alias {} call_my {} source_my fixed; oo::class create C {method m {} {call_my $argument}}",
            0,
        ),
        (
            "proc source_my args {}; oo::class create C {method m {} {source_my $selector}}",
            0,
        ),
    ] {
        let current = analysis(source, "tcl8.6", store.clone(), config);
        let family: Vec<_> = current.all_classes.keys().cloned().collect();
        assert_eq!(family.len(), 1, "{source}");
        let target = MethodRenameTarget {
            family: &family,
            method: "m",
            is_classmethod: false,
            new_name: "renamed",
        };
        let mut spans = Vec::new();
        walk_self_dispatch(
            source,
            tcl_dialect::DialectProfile::plain_tcl(),
            &current,
            target,
            &mut |span| spans.push(span),
        );
        assert_eq!(spans.len(), expected, "{source}: {spans:?}");
        let older = analysis(source, "tcl8.4", store.clone(), config);
        let mut old_spans = Vec::new();
        walk_self_dispatch(
            source,
            tcl_dialect::DialectProfile::plain_tcl(),
            &older,
            target,
            &mut |span| old_spans.push(span),
        );
        assert!(old_spans.is_empty(), "{source}");
    }
}
