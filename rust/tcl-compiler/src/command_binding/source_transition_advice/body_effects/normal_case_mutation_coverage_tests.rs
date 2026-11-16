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

use super::super::AdviceTapeInputs;
use super::*;

fn projection(source: &str) -> (BodyEffectProjection, bool) {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    let profile = environment.unit_profile();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let analysis = crate::analyser::Analyser::new()
        .with_resolved_input(input)
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
    assert!(matches!(
        policy
            .native()
            .map(tcl_syntax::naming::NamePolicyProtocol::recipe),
        Some(tcl_syntax::naming::NativeNameProtocol::C(_))
    ));
    let mut graph = bindings
        .initial_source_advice_graph(&context, &policy)
        .unwrap();
    let invocation = &plan.commands[0];
    let written = original_words(&invocation.words, &policy).unwrap();
    let head = written[0].input.as_ref().unwrap();
    let (command, mut arguments, lineage) = graph.resolve(head).unwrap();
    arguments.extend(written.iter().skip(1).cloned());
    let words = registry_words(&arguments);
    let schema = tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
        context.commands(),
        Some(context.context()),
        tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(&command),
            &words,
        )
        .with_dialect(dialect),
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
    .resolved()
    .unwrap();
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
    assert!(owner.normal_case_source_layout(&schema).is_some());
    let advice = owner.schema_advice(selected, &graph, &schema).unwrap();
    let following = original_words(&plan.commands[1].words, &policy).unwrap();
    let following = following[0].input.as_ref().unwrap();
    let before = graph.resolve(following);
    assert!(before.is_some());
    let projected = owner.apply_immediate_body_effects(&mut graph, &advice, &schema, None);
    (projected, graph.resolve(following) == before)
}

#[test]
fn nested_normal_source_case_withdraws_a_possibly_renamed_command() {
    let (_, unchanged) = projection(
        "switch -glob $outer {a* {switch -glob $inner {b* {rename puts other} default {}}} default {}}; puts sentinel",
    );
    assert!(
        !unchanged,
        "possible nested rename must withdraw the old binding"
    );
}

#[test]
fn nested_normal_source_case_with_no_mutations_preserves_lookup() {
    let (projected, unchanged) = projection(
        "switch -glob $outer {a* {switch -glob $inner {b* {break} default {}}} default {}}; puts sentinel",
    );
    assert!(projected.effects_complete);
    assert!(unchanged);
}
