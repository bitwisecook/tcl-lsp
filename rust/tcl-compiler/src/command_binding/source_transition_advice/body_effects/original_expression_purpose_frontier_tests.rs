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

fn assert_native_expression_requires_its_parser(
    owner: &AdviceInvocationContext<'_>,
    graph: &mut AdviceGraph,
    advice: &crate::command_binding::source_transition_advice::OriginalSourceCommandTransitionAdvice,
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    input: &crate::analyser::ResolvedAnalysisInput,
) {
    let native_policy = AdviceNamingPolicy::Native(
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
            .authored_name_policy()
            .unwrap(),
    );
    let native_owner = AdviceInvocationContext {
        origin: owner.origin,
        context: owner.context,
        config: owner.config,
        dialect: owner.dialect,
        policy: &native_policy,
        query: None,
    };
    let mut native = BodyWalk {
        context: &native_owner,
        visited: 0,
        unavailable_lifecycle_effects: false,
        inventory: None,
        authored_prefixes: OriginalSourceTransitionAdviceTape::default(),
        source_body: None,
        logical_scope: None,
        member_catalogue_body: None,
        catalogue_body: None,
    };
    assert!(native.expressions(graph, advice, schema, 0).is_none());
    assert_eq!(native.visited, 0);
    assert!(advice.original_head().native_input().is_none());
    assert_eq!(advice.logical_source_input(), Some(input));
}

#[test]
fn original_logical_expression_topology_cannot_supply_missing_native_parser_purpose() {
    // naming.source.authored-command-transition-advice
    // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
    // Descriptive source/name/parser axes only, without a Native invocation.
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
    let source = "if {![regexp x y out]} {puts $out}";
    let analysis = crate::analyser::Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, "presentation");
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
    assert!(matches!(policy, AdviceNamingPolicy::Logical(_)));
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
    let owner = AdviceInvocationContext {
        origin,
        context: &context,
        config,
        dialect,
        policy: &policy,
        query: None,
    };
    let advice = owner
        .schema_advice(
            AdviceInvocation {
                native: &invocation.words,
                head,
                arguments: &arguments,
                lineage: &lineage,
            },
            &graph,
            &schema,
        )
        .unwrap();
    assert_eq!(advice.logical_source_input(), Some(&input));
    assert_eq!(
        dialect
            .expression_parse_context(context.commands().profile())
            .native_syntax,
        tcl_syntax::expr::parser::NativeExprSyntax::Unknown
    );
    let mut logical = BodyWalk {
        context: &owner,
        visited: 0,
        unavailable_lifecycle_effects: false,
        inventory: Some(OriginalSourceTransitionAdviceTape::default()),
        authored_prefixes: OriginalSourceTransitionAdviceTape::default(),
        source_body: None,
        logical_scope: None,
        member_catalogue_body: None,
        catalogue_body: None,
    };
    assert!(
        logical
            .expressions(&mut graph, &advice, &schema, 0)
            .is_some()
    );
    assert_eq!(logical.visited, 1);
    assert_native_expression_requires_its_parser(&owner, &mut graph, &advice, &schema, &input);
}
