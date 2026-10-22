// SPDX-License-Identifier: AGPL-3.0-or-later
//! OFF-TREE public observations for the unchanged original pack_source fixtures.
//! Call once after build_for_dialect and before clear_host in proved_under.
//! No evaluator invocation, fallback, new source analysis, or authority grant.

pub(super) fn report(
    dialect: &str,
    call: &str,
    registry: &tcl_registry::CommandRegistry,
    unit: &tcl_compiler::compilation_unit::CompilationUnit,
) {
    use tcl_compiler::ir::Statement;
    use tcl_compiler::registry_invocation::{
        InvocationMetadataContext, normal_transfer_invocation_with_metadata_context,
        resolved_tokens_invocation_with_metadata_context,
    };
    let input = unit.ir_module.source_metadata_input.as_ref();
    let metadata =
        input.and_then(|input| InvocationMetadataContext::for_analysis_input(registry, input));
    let original_count = unit
        .ir_module
        .procedures
        .values()
        .filter(|body| body.qualified_name == "::p")
        .count();
    eprintln!(
        "N17G2 dialect={dialect} call={call:?} original-p-count={original_count} retained-input={} exact-metadata={} host={}",
        input.is_some(),
        metadata.is_some(),
        tcl_registry::pack_hooks::has_host()
    );
    for (id, body) in &unit.ir_module.procedures {
        if body.qualified_name != "::p" {
            continue;
        }
        let function = unit.procedures.get(id);
        eprintln!(
            "N17G2 owner-id={} analysis={} body-id-rejoin={}",
            unit.ir_module.procedure_ids.owns(*id),
            function.is_some(),
            function.is_some_and(|function| function.body_id == Some(*id))
        );
        if let Some(function) = function {
            if let Some(symbol) = function.ssa.var_symbol("known") {
                let mut versions: Vec<_> = function
                    .sccp
                    .values
                    .iter()
                    .filter(|((actual, _), _)| *actual == symbol)
                    .map(|((_, version), value)| (*version, value))
                    .collect();
                versions.sort_by_key(|(version, _)| *version);
                eprintln!(
                    "N17G2 known-all-versions={versions:?} sccp-route-count={}",
                    function.sccp.explanations.len()
                );
            } else {
                eprintln!("N17G2 known-symbol=absent");
            }
        }
        for statement in &body.body.statements {
            let Statement::AssignValue { name, tokens, .. } = statement else {
                continue;
            };
            if name != "known" {
                continue;
            }
            let Some(tokens) = tokens.as_ref() else {
                eprintln!("N17G2 first-public-decline=parent-retained-tokens");
                continue;
            };
            // Missing actual metadata stops production before any catalogue query.
            let strict = metadata.and_then(|metadata| {
                resolved_tokens_invocation_with_metadata_context(registry, Some(metadata), tokens)
            });
            let normal = metadata.and_then(|metadata| {
                normal_transfer_invocation_with_metadata_context(registry, Some(metadata), tokens)
            });
            let stored = tokens.source_binding.as_ref().and_then(|binding| {
                normal
                    .as_ref()?
                    .stored_value_operand(&binding.variable_context, registry)
            });
            let selected_site = stored
                .and_then(|(_, word)| word.sole_command_substitution())
                .map(|(_, site)| site);
            let first = if metadata.is_none() {
                "exact-source-metadata"
            } else if normal.is_none() {
                "parent-normal-transfer"
            } else if stored.is_none() {
                "parent-unobserved-bounded-store"
            } else if selected_site.is_none() {
                "stored-original-sole-substitution"
            } else {
                "child-complete-inventory-and-retained-fold-private-unobserved"
            };
            eprintln!(
                "N17G2 first-public-decline={first} strict={} normal={} stored={} selected-site={:?} synthetic={:?}",
                strict.is_some(),
                normal.is_some(),
                stored.is_some(),
                selected_site.map(|site| site.span),
                tokens.synthetic
            );
            if let Some(binding) = tokens.source_binding.as_ref() {
                let places = normal
                    .as_ref()
                    .map(|normal| normal.mutation_places(&binding.variable_context, registry));
                let bounded: Option<Vec<_>> = places.as_ref().map(|places| {
                    places
                        .iter()
                        .map(|place| {
                            (
                                place.observed,
                                place.dynamic,
                                place.kind == tcl_compiler::place::PlaceKind::Unknown,
                            )
                        })
                        .collect()
                });
                eprintln!(
                    "N17G2 parent admission={:?} structured={} execution={} handler={} args-normal={} invocation-normal={} known-normal-result={} execution-unknown={} execution-maybe-absent={} binding-phase={:?} stored-argument={:?} places-observed-dynamic-unknown={bounded:?}",
                    binding.native_compilation_admission_selection(),
                    binding.original_structured_compilation(tokens).is_some(),
                    binding.proved_execution_target().is_some(),
                    binding.proved_handler_target().is_some(),
                    binding.original_arguments_complete_normally(tokens),
                    binding.original_invocation_completes_normally(tokens),
                    binding.original_normal_result(tokens).is_some(),
                    binding.execution_is_unknown(),
                    binding.execution_may_be_absent(),
                    normal
                        .as_ref()
                        .map(|normal| normal.variable_binding_phase()),
                    normal
                        .as_ref()
                        .and_then(|normal| normal.stored_value_argument())
                );
            }
            let Some(input) = input else {
                continue;
            };
            // Written word is only a candidate when parent-store proof declined.
            let written_candidate = tokens
                .words()
                .get(2)
                .and_then(|word| word.sole_command_substitution())
                .map(|(_, site)| site);
            let calls =
                tcl_compiler::word_subst::lifted_calls(Some(tokens), input.lexer_config().nested());
            eprintln!(
                "N17G2 public-child-candidates={} completeness=UNOBSERVABLE written-candidate={:?}",
                calls.len(),
                written_candidate.map(|site| site.span)
            );
            for child in calls {
                let matches_written = written_candidate.is_some_and(|site| site.span == child.span);
                let Some(child_tokens) = child.tokens.as_ref() else {
                    continue;
                };
                let Some(binding) = child_tokens.source_binding.as_ref() else {
                    continue;
                };
                let child_normal = metadata.and_then(|metadata| {
                    normal_transfer_invocation_with_metadata_context(
                        registry,
                        Some(metadata),
                        child_tokens,
                    )
                });
                eprintln!(
                    "N17G2 child candidate-span={:?} matches-written-span={} admission={:?} structured={} execution-registry={} handler-registry={} normal={} args-normal={} invocation-normal={} known-normal-result={} execution-unknown={} realm={:?}",
                    child.span,
                    matches_written,
                    binding.native_compilation_admission_selection(),
                    binding
                        .original_structured_compilation(child_tokens)
                        .is_some(),
                    binding
                        .proved_execution_target()
                        .is_some_and(|target| target.registry_backed),
                    binding
                        .proved_handler_target()
                        .is_some_and(|target| target.registry_backed),
                    child_normal.is_some(),
                    binding.original_arguments_complete_normally(child_tokens),
                    binding.original_invocation_completes_normally(child_tokens),
                    binding.original_normal_result(child_tokens).is_some(),
                    binding.execution_is_unknown(),
                    binding.invocation_realm()
                );
            }
        }
    }
}
