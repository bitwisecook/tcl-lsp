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
    for (name, roles, min) in [
        ("source_next", &[][..], 0),
        (
            "source_next_named",
            &[(0, tcl_registry::ArgRole::Name)][..],
            1,
        ),
    ] {
        registry.insert(tcl_registry::CommandSpec {
            name,
            arg_roles: roles,
            arity: tcl_registry::Arity::at_least(min),
            traits: tcl_registry::Traits::TCLOO_NEXT_CHAIN,
            surface,
            ..tcl_registry::CommandSpec::DEFAULT
        });
    }
    Arc::new(registry)
}

fn analysis(
    source: &str,
    environment: &str,
    store: Arc<tcl_registry::CommandRegistry>,
) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let context = Arc::new(
        tcl_registry::model::ingress::static_context_for(environment).with_command_store(store),
    );
    Analyser::new()
        .with_resolved_input(ResolvedAnalysisInput::new(
            profile, profile, context, config,
        ))
        .analyse(source, profile.name)
}

fn spans(source: &str, analysis: &AnalysisResult) -> Vec<(tcl_lexer::Span, Option<String>)> {
    scan_next_dispatch_sites_with_target(
        source,
        analysis,
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
    )
}

#[test]
fn original_next_dispatch_uses_selected_roles_captured_targets_and_actual_availability() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    // Conditional source topology/values only, not a next-method invocation.
    let store = store();
    for (source, target) in [
        ("source_next", None),
        ("source_next_named {::Ancestor}", Some("::Ancestor")),
        ("if 1 {source_next_named {::Ancestor}}", Some("::Ancestor")),
        (
            "interp alias {} call_next {} source_next_named ::Ancestor; call_next",
            Some("::Ancestor"),
        ),
    ] {
        let current = analysis(source, "tcl8.6", store.clone());
        let found = spans(source, &current);
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        assert_eq!(found[0].1.as_deref(), target, "{source}");
        assert!(
            spans(source, &analysis(source, "tcl8.4", store.clone())).is_empty(),
            "{source}"
        );
    }
}

#[test]
fn original_next_dispatch_withdraws_known_replacements_missing_and_foreign_source() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let store = store();
    for source in [
        "proc source_next_named args {}; source_next_named ::Ancestor",
        "rename source_next_named {}; source_next_named ::Ancestor",
        "source_next_named $unknown",
    ] {
        assert!(
            spans(source, &analysis(source, "tcl8.6", store.clone())).is_empty(),
            "{source}"
        );
    }
    let source = "source_next_named ::Ancestor";
    let mut current = analysis(source, "tcl8.6", store);
    assert_eq!(spans(source, &current).len(), 1);
    let config = current.body_lexer_config.unwrap();
    current.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
    assert_eq!(
        spans(source, &current),
        [] as [(tcl_lexer::Span, std::option::Option<std::string::String>); 0]
    );
    current.body_lexer_config = Some(config);
    current.resolved_input = Some(ResolvedAnalysisInput::new(
        tcl_dialect::DialectProfile::plain_tcl(),
        tcl_dialect::DialectProfile::plain_tcl(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        config,
    ));
    assert_eq!(
        spans(source, &current),
        [] as [(tcl_lexer::Span, std::option::Option<std::string::String>); 0]
    );
    current.resolved_input = None;
    assert_eq!(
        spans(source, &current),
        [] as [(tcl_lexer::Span, std::option::Option<std::string::String>); 0]
    );
}
