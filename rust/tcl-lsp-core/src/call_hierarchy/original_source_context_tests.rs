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

fn analyse(
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

fn store() -> Arc<tcl_registry::CommandRegistry> {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let surface = Some(tcl_dialect::model::SpecSurface::TCL90_PLUS);
    registry.insert(tcl_registry::CommandSpec {
        name: "source_dispatch",
        traits: tcl_registry::Traits::TCLOO_SELF_DISPATCH,
        surface,
        ..tcl_registry::CommandSpec::DEFAULT
    });
    Arc::new(registry)
}

fn method_body(analysis: &AnalysisResult) -> tcl_lexer::Span {
    analysis.all_classes.values().next().unwrap().methods["m"].body_span
}

#[test]
fn original_method_hierarchy_uses_selected_helpers_and_same_source_frame() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "oo::class create C {method m {} {source_dispatch; if 1 {puts x}; proc later {} {hidden}; missing}}";
    let store = store();
    let current = analyse(source, "tcl9.0", store.clone());
    let calls = segment_body_calls(source, &current, method_body(&current));
    let heads = calls
        .iter()
        .map(|(head, _)| head.as_str())
        .collect::<Vec<_>>();
    assert!(
        heads.contains(&"puts") && heads.contains(&"missing"),
        "{heads:?}"
    );
    assert!(
        !heads.contains(&"source_dispatch") && !heads.contains(&"hidden"),
        "{heads:?}"
    );
    let old = analyse(source, "tcl8.6", store);
    assert!(
        segment_body_calls(source, &old, method_body(&old))
            .iter()
            .any(|(head, _)| head == "source_dispatch")
    );
}

#[test]
fn original_method_hierarchy_keeps_replaced_helpers_unresolved_and_withdraws_stale_source() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "proc source_dispatch args {}; oo::class create C {method m {} {source_dispatch}}";
    let mut current = analyse(source, "tcl9.0", store());
    let body = method_body(&current);
    assert!(
        segment_body_calls(source, &current, body)
            .iter()
            .any(|(head, _)| head == "source_dispatch")
    );
    assert_eq!(
        segment_body_calls(
            &source.replace("source_dispatch", "other_dispatch"),
            &current,
            body
        ),
        [] as [(std::string::String, tcl_lexer::Span); 0]
    );
    current.resolved_input = None;
    assert_eq!(
        segment_body_calls(source, &current, body),
        [] as [(std::string::String, tcl_lexer::Span); 0]
    );
}
