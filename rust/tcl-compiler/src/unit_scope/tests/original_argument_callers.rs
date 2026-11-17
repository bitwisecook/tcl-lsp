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

//! Callers in original argument words and terminators: argument counts, child roles and owner refusals.

use super::*;

#[test]
fn original_argument_caller_phase_keeps_real_return_and_direct_argument_counts() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The actual catch region captures this Return as an argv phase.
    let (_, unit) = native_argument_caller_unit(
        "proc add {b c} {catch {return [expr {$b + $c}]}}; add 2 4; add 3 5",
    );
    let function = unit
        .cfg_module
        .procedures
        .get(&fixture_id(&unit.cfg_module.procedures, "::add"))
        .expect("authored fixture body");
    assert!(
        function.blocks.iter().any(|(&block, data)| {
            data.statements.iter().enumerate().any(|(index, _)| {
                function
                    .source_tokens_at(block, index)
                    .is_some_and(|tokens| {
                        tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments)
                            && tokens
                                .argv_texts
                                .first()
                                .is_some_and(|head| head == "return")
                    })
            })
        }),
        "actual return producer retains its argv boundary"
    );
    let evidence = unit.caller_scope.call_sites.get("::add").unwrap();
    assert!(!evidence.opaque_caller, "{evidence:?}");
    assert_eq!(evidence.arg_counts, BTreeSet::from([2]));
    assert_eq!(
        evidence.slots[&0].values,
        BTreeSet::from(["2".into(), "3".into()])
    );
    assert_eq!(
        evidence.slots[&1].values,
        BTreeSet::from(["4".into(), "5".into()])
    );
}

#[test]
fn original_argument_caller_child_keeps_its_own_expression_roles() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let (_, unit) = native_argument_caller_unit(
        "proc helper {x} {return $x}; proc run {} {return [expr {[helper SECOND]}]}; helper FIRST; run",
    );
    let evidence = unit.caller_scope.call_sites.get("::helper").unwrap();
    assert!(!evidence.opaque_caller, "{evidence:?}");
    assert_eq!(evidence.arg_counts, BTreeSet::from([1]));
    assert_eq!(
        evidence.slots[&0].values,
        BTreeSet::from(["FIRST".into(), "SECOND".into()])
    );
    assert_eq!(evidence.uniform_literal_at(0), None);
    // The enclosing braced argument never inherits the child's Expr role.
    let (_, literal) = native_argument_caller_unit(
        "proc helper {x} {return $x}; proc run {} {return {[helper LITERAL]}}; helper FIRST; run",
    );
    let evidence = literal.caller_scope.call_sites.get("::helper").unwrap();
    assert!(!evidence.opaque_caller, "{evidence:?}");
    assert_eq!(evidence.slots[&0].values, BTreeSet::from(["FIRST".into()]));
}

fn assert_original_argv_metadata_has_separate_purpose(
    phase: &crate::ir::CommandTokens,
    owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
    module: &IrModule,
    registry: &CommandRegistry,
) {
    let binding = phase.source_binding.as_ref().unwrap();
    assert!(
        binding
            .original_invocation_metadata_for_owner(phase, owner, registry)
            .is_err()
    );
    assert!(
        binding
            .original_argument_metadata_for_owner(phase, owner, registry)
            .unwrap()
            .is_some()
    );
    assert!(
        binding
            .original_argument_metadata_for_module(phase, module, registry)
            .is_some()
    );
}

fn assert_original_argv_child_refusals(
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    phase: &crate::ir::CommandTokens,
    owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
) {
    use crate::registry_invocation::OwnedInvocationMetadataContext as Owner;
    fn caller(
        owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
    ) -> CallerFrame<'_> {
        CallerFrame {
            metadata_owner: owner,
            scope: CallerScopeId::Extra(0),
            params: &[],
            callers_tracked: true,
        }
    }
    let mut missing_child = (*phase).clone();
    missing_child.nested_bindings_mut().clear();
    let mut changed_vector = (*phase).clone();
    changed_vector.argv_texts[1] = "changed".into();
    for refused_tokens in [&missing_child, &changed_vector] {
        let mut refused = CallSiteEvidence::default();
        record_original_lifted_call_sites(&mut refused, ctx, &caller(owner), Some(refused_tokens));
        assert!(refused.get("::add").unwrap().opaque_caller);
    }
    // A typed residual producer explicitly performs no argv evaluation.
    let residual = crate::ir::CommandTokens::marker(crate::ir::SyntheticMarker::EvaluatedWrapper);
    let mut empty = CallSiteEvidence::default();
    record_original_lifted_call_sites(
        &mut empty,
        ctx,
        &caller(&Owner::Unavailable),
        Some(&residual),
    );
    assert!(empty.is_empty());
}

#[test]
fn original_argument_caller_owner_refusal_stays_distinct_from_effect_only_empty() {
    fn caller(
        owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
    ) -> CallerFrame<'_> {
        CallerFrame {
            metadata_owner: owner,
            scope: CallerScopeId::Extra(0),
            params: &[],
            callers_tracked: true,
        }
    }
    use crate::registry_invocation::OwnedInvocationMetadataContext as Owner;
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The actual catch region captures this Return as an argv phase.
    let source = "proc add {b c} {catch {return [expr {$b + $c}]}}; add 2 4; add 3 5";
    let (context, unit) = native_argument_caller_unit(source);
    let module = &unit.ir_module;
    let input = module.source_metadata_input.as_ref().unwrap();
    let function = unit
        .cfg_module
        .procedures
        .get(&fixture_id(&unit.cfg_module.procedures, "::add"))
        .expect("authored fixture body");
    let phase = function
        .blocks
        .iter()
        .find_map(|(&block, data)| {
            data.statements.iter().enumerate().find_map(|(index, _)| {
                function.source_tokens_at(block, index).filter(|tokens| {
                    tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments)
                        && tokens
                            .argv_texts
                            .first()
                            .is_some_and(|head| head == "return")
                })
            })
        })
        .expect("actual return argv boundary");
    assert_original_argv_metadata_has_separate_purpose(
        phase,
        &function.metadata_context,
        module,
        context.commands(),
    );
    let identities =
        crate::realm::document_realm_bindings_for_module(module, context.commands()).unwrap();
    let known = HashSet::from(["::add".to_owned()]);
    let reach = vec!["::add".to_owned()];
    let facts = ModuleVarFacts::default();
    let previous = CallSiteEvidence::default();
    let ctx = CallSiteScanCtx {
        known: &known,
        registry: context.commands(),
        module,
        source_input: Some(input),
        identities: &identities,
        declared: None,
        var_facts: &facts,
        previous: &previous,
        procedures: &module.procedures,
        unenumerable_reach: &reach,
        unresolved_handler: None,
    };
    let mut current = CallSiteEvidence::default();
    record_original_lifted_call_sites(
        &mut current,
        &ctx,
        &caller(&function.metadata_context),
        Some(phase),
    );
    assert!(
        current.is_empty(),
        "the actual expr child calls no local procedure"
    );
    let mut changed_config = input.lexer_config();
    changed_config.leading_bom = tcl_lexer::LeadingBom::Skip;
    let changed = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        changed_config,
    );
    let foreign = supplied_unit_input(
        input,
        tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
            .unwrap()
            .default_context_registry(),
    );
    for owner in [
        Owner::Unavailable,
        Owner::Standalone,
        Owner::SuppliedSource(Box::new(changed)),
        Owner::SuppliedSource(Box::new(foreign)),
    ] {
        let mut refused = CallSiteEvidence::default();
        record_original_lifted_call_sites(&mut refused, &ctx, &caller(&owner), Some(phase));
        assert!(refused.get("::add").unwrap().opaque_caller);
        assert!(refused.get("::add").unwrap().arg_counts.contains(&0));
    }
    assert_original_argv_child_refusals(&ctx, phase, &function.metadata_context);
}

#[test]
fn original_argument_caller_metadata_joins_from_the_module_to_its_nested_owner() {
    use crate::registry_invocation::OwnedInvocationMetadataContext as Owner;
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual retained Native source and CFG argv phase; no handler entry or
    // runtime variable result follows from this metadata-only observation.
    let (context, unit) =
        native_argument_caller_unit("proc add {b c} {catch {return [expr {$b + $c}]}}; add 2 4");
    let module = &unit.ir_module;
    let input = module.source_metadata_input.as_ref().unwrap();
    let function = unit
        .cfg_module
        .procedures
        .get(&fixture_id(&unit.cfg_module.procedures, "::add"))
        .expect("authored fixture body");
    let phase = function
        .blocks
        .iter()
        .find_map(|(&block, data)| {
            data.statements.iter().enumerate().find_map(|(index, _)| {
                function.source_tokens_at(block, index).filter(|tokens| {
                    tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments)
                        && tokens
                            .argv_texts
                            .first()
                            .is_some_and(|head| head == "return")
                })
            })
        })
        .expect("actual return argv boundary");
    let identities =
        crate::realm::document_realm_bindings_for_module(module, context.commands()).unwrap();
    let known = HashSet::from(["::add".to_owned()]);
    let reach = vec!["::add".to_owned()];
    let facts = ModuleVarFacts::default();
    let previous = CallSiteEvidence::default();
    let ctx = CallSiteScanCtx {
        known: &known,
        registry: context.commands(),
        module,
        source_input: Some(input),
        identities: &identities,
        declared: None,
        var_facts: &facts,
        previous: &previous,
        procedures: &module.procedures,
        unenumerable_reach: &reach,
        unresolved_handler: None,
    };
    for owner in [
        Owner::for_source_input(Some(input)),
        Owner::for_source_input(Some(&input.for_nested_source())),
        function.metadata_context.clone(),
    ] {
        assert!(
            ctx.original_argument_metadata(phase, &owner)
                .unwrap()
                .is_some()
        );
    }
    let mut changed_config = input.lexer_config();
    changed_config.leading_bom = tcl_lexer::LeadingBom::Skip;
    let changed_file_input = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        changed_config,
    );
    assert_ne!(changed_file_input, *input);
    assert_eq!(
        changed_file_input.for_nested_source(),
        input.for_nested_source()
    );
    let foreign = supplied_unit_input(
        input,
        tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
            .unwrap()
            .default_context_registry(),
    );
    for owner in [
        Owner::SuppliedSource(Box::new(changed_file_input)),
        Owner::SuppliedSource(Box::new(foreign)),
        Owner::Unavailable,
        Owner::Standalone,
    ] {
        assert!(ctx.original_argument_metadata(phase, &owner).is_err());
        // Invocation purpose remains independently refused for an argv phase.
        assert!(ctx.original_metadata(phase, &owner).is_err());
    }
}

#[test]
fn original_native_procedure_argv_roles_do_not_invent_registry_body_callers() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual held Vm entry and source call inventory, not a successful native call.
    let (_, unit) = native_argument_caller_unit(
        "proc selected {value} {}; selected {[inert CHILD]}; selected {[inert CHILD]}",
    );
    let evidence = unit.caller_scope.call_sites.get("::selected").unwrap();
    assert!(!evidence.opaque_caller, "{evidence:?}");
    assert_eq!(evidence.arg_counts, BTreeSet::from([1]));
    assert_eq!(evidence.uniform_literal_at(0), Some("[inert CHILD]"));
    let (_, unavailable) =
        native_argument_caller_unit("proc selected {value} {}; unknown; selected FIXED");
    assert!(
        unavailable
            .caller_scope
            .call_sites
            .get("::selected")
            .unwrap()
            .opaque_caller
    );
}

#[test]
fn original_native_procedure_argv_roles_keep_authentic_alias_prefix_counts() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let (_, unit) = native_argument_caller_unit(
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; forwarded FIXED; selected PREFIX FIXED",
    );
    let evidence = unit.caller_scope.call_sites.get("::selected").unwrap();
    assert!(!evidence.opaque_caller, "{evidence:?}");
    assert_eq!(evidence.arg_counts, BTreeSet::from([2]));
    assert_eq!(evidence.uniform_literal_at(0), Some("PREFIX"));
    assert_eq!(evidence.uniform_literal_at(1), Some("FIXED"));
}

#[test]
fn original_direct_return_keeps_terminator_argv_without_captured_phase_marker() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // A procedure exit retains its original argv as a terminator. Captured
    // completions use the distinct EvaluatedArguments statement purpose.
    let (_, unit) =
        native_argument_caller_unit("proc add {b c} {return [expr {$b + $c}]}; add 2 4; add 3 5");
    let function = unit
        .cfg_module
        .procedures
        .get(&fixture_id(&unit.cfg_module.procedures, "::add"))
        .expect("authored fixture body");
    let returns = function
        .blocks
        .iter()
        .filter_map(|(&block, data)| {
            matches!(data.terminator, Some(crate::cfg::Terminator::Return { .. }))
                .then(|| function.source_tokens_at(block, usize::MAX))
                .flatten()
                .filter(|tokens| {
                    tokens
                        .argv_texts
                        .first()
                        .is_some_and(|head| head == "return")
                })
        })
        .collect::<Vec<_>>();
    assert_eq!(returns.len(), 1);
    assert!(returns[0].synthetic.is_none());
    assert_eq!(returns[0].argv_texts, ["return", "[expr {$b + $c}]"]);
    assert!(
        function
            .blocks
            .values()
            .all(|data| data.statements.iter().all(|statement| {
                statement.tokens().is_none_or(|tokens| {
                    tokens.synthetic != Some(crate::ir::SyntheticMarker::EvaluatedArguments)
                        || tokens
                            .argv_texts
                            .first()
                            .is_none_or(|head| head != "return")
                })
            }))
    );
    let evidence = unit.caller_scope.call_sites.get("::add").unwrap();
    assert!(!evidence.opaque_caller, "{evidence:?}");
    assert_eq!(evidence.arg_counts, BTreeSet::from([2]));
    assert_eq!(
        evidence.slots[&0].values,
        BTreeSet::from(["2".into(), "3".into()])
    );
    assert_eq!(
        evidence.slots[&1].values,
        BTreeSet::from(["4".into(), "5".into()])
    );
}
