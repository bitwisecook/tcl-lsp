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

fn analyse(source: &str, environment: &str) -> AnalysisResult {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let surface = Some(tcl_dialect::model::SpecSurface::TCL90_PLUS);
    registry.insert(tcl_registry::CommandSpec {
        name: "source_next_named",
        traits: tcl_registry::Traits::TCLOO_NEXT_CHAIN,
        arg_roles: &[(0, tcl_registry::ArgRole::Name)],
        arity: tcl_registry::Arity::at_least(1),
        surface,
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let context = Arc::new(
        tcl_registry::model::ingress::static_context_for(environment)
            .with_command_store(Arc::new(registry)),
    );
    Analyser::new()
        .with_resolved_input(ResolvedAnalysisInput::new(
            profile, profile, context, config,
        ))
        .analyse(source, profile.name)
}

fn target(source: &str, analysis: &AnalysisResult, needle: &str) -> Option<tcl_lexer::Span> {
    let offset = source.rfind(needle).unwrap();
    let index = LineIndex::new(source);
    let position = index.position_at_utf16(u32::try_from(offset).unwrap(), source);
    next_dispatch_target(
        analysis,
        source,
        &index,
        position.line,
        position.character.get(),
    )
}

#[test]
fn original_next_definition_uses_captured_target_and_selected_availability() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "oo::class create Base {method m {} {}}\ninterp alias {} jump {} source_next_named Base\noo::class create C {superclass Base; method m {} {jump}}";
    let current = analyse(source, "tcl9.0");
    let base = current
        .all_classes
        .values()
        .find(|class| class.name == "Base")
        .unwrap();
    assert_eq!(
        target(source, &current, "jump"),
        Some(base.methods["m"].name_span)
    );
    let old = analyse(source, "tcl8.6");
    assert!(target(source, &old, "jump").is_none());
    let shadow = source.replace(
        "interp alias",
        "proc source_next_named args {}; interp alias",
    );
    assert!(target(&shadow, &analyse(&shadow, "tcl9.0"), "jump").is_none());
    let unknown = source.replace("source_next_named Base", "source_next_named $unknown");
    assert!(target(&unknown, &analyse(&unknown, "tcl9.0"), "jump").is_none());
    assert!(target(&source.replace("Base", "Else"), &current, "jump").is_none());
}
