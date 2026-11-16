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

use super::super::{AdviceTapeInputs, SourceCommandTransitionObligation};
use super::*;
use tcl_registry::{
    ArgRole, Arity, CommandRegistry, CommandSpec, InvocationArguments, ScriptTiming,
};

fn reference_only(_arguments: InvocationArguments<'_>) -> Vec<(u8, ScriptTiming)> {
    vec![(0, ScriptTiming::ReferenceOnly)]
}

fn projection(
    source: &str,
    timing: Option<tcl_registry::ScriptTimingResolver>,
) -> (BodyEffectProjection, bool) {
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "selected-script",
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Body)],
        script_timing_resolver: timing,
        ..CommandSpec::DEFAULT
    });
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = Arc::new(
        tcl_registry::model::context_for_profile(profile).with_command_store(Arc::new(registry)),
    );
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context.clone(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    assert!(input.has_logical_source_name_context());
    let analysis = crate::analyser::Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, profile.name);
    let bindings = analysis
        .retained_command_realm()
        .unwrap()
        .source_bindings_ref();
    let AdviceTapeInputs {
        origin,
        config,
        policy,
        dialect,
        plan,
    } = bindings.prepare_source_advice_tape(&context).unwrap();
    let mut graph = bindings
        .initial_source_advice_graph(&context, &policy)
        .unwrap();
    let invocation = &plan.commands[0];
    let written = original_words(&invocation.words, &policy).unwrap();
    let head = written[0].input.as_ref().unwrap();
    let (command, mut arguments, lineage) = graph.resolve(head).unwrap();
    arguments.extend(written.iter().skip(1).cloned());
    let words = registry_words(&arguments);
    let resolution =
        tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            tcl_registry::InvocationWords::structured(
                tcl_registry::InvocationWord::Literal(&command),
                &words,
            )
            .with_dialect(dialect),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
    let schema = resolution.resolved().unwrap();
    let selected = AdviceInvocation {
        native: &invocation.words,
        head,
        arguments: &arguments,
        lineage: &lineage,
    };
    let owner = AdviceInvocationContext {
        origin,
        context: &context,
        config,
        dialect,
        policy: &policy,
        query: None,
    };
    let advice = owner.schema_advice(selected, &graph, &schema).unwrap();
    assert_eq!(advice.logical_source_input(), Some(&input));
    assert!(
        advice
            .obligations()
            .contains(&SourceCommandTransitionObligation::ExplicitLogicalSourceApplicability)
    );
    let original = original_words(&plan.commands[1].words, &policy).unwrap();
    let following = original[0].input.as_ref().unwrap();
    let before = graph.resolve(following);
    assert!(before.is_some(), "real selected following Registry command");
    let projection = owner.apply_immediate_body_effects(&mut graph, &advice, &schema, None);
    (projection, graph.resolve(following) == before)
}

#[test]
fn original_reference_only_coverage_is_complete_empty_and_keeps_following_source_lookup() {
    // naming.source.original-script-region-purpose
    // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
    // Conditional source mutation coverage only; no execution or handler grant.
    let (projection, unchanged) = projection(
        "selected-script {rename set {}; rename puts {}}; set x 1",
        Some(reference_only),
    );
    assert!(projection.effects_complete);
    assert!(unchanged);
    assert!(projection.inventory.advice.is_empty());
    assert!(projection.inventory.represented.is_empty());
    assert!(projection.inventory.registry_barriers.is_empty());
}

#[test]
fn original_unavailable_evaluated_body_coverage_cannot_claim_complete_empty() {
    // naming.source.original-script-region-purpose
    // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
    // A dynamic or recooked selected body remains unavailable mutation coverage.
    for source in [
        "selected-script $body; set x 1",
        "selected-script \"rename set {};\\nrename puts {}\"; set x 1",
    ] {
        let (projection, _unchanged) = projection(source, None);
        assert!(!projection.effects_complete, "{source}");
        assert!(projection.inventory.advice.is_empty());
        assert!(projection.inventory.represented.is_empty());
        assert!(projection.inventory.registry_barriers.is_empty());
    }
}
