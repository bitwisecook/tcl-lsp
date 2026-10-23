// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional Direct argv roles and List shape under genuine pre-argv worlds.

use super::{
    CommandRegistry, CommandTokens, InvocationMetadataContext, ResolvedStatementInvocation,
};

pub(super) struct DirectSourceTaintArguments {
    pub(super) parent: ResolvedStatementInvocation,
    pub(super) realm: tcl_dialect::model::InvocationRealm,
    pub(super) child: CommandTokens,
    pub(super) literal_list_head: Option<String>,
}

/// Conditional on normally producing this original argv. This is neither a
/// reached wrapper invocation nor a Native instruction/result/Normal proof.
/// Only literal/scalar reads and one closed selected List producer qualify.
pub(super) fn original_direct_source_taint_arguments(
    registry: &CommandRegistry,
    metadata: InvocationMetadataContext<'_>,
    parent: &CommandTokens,
) -> Option<DirectSourceTaintArguments> {
    use tcl_registry::world_effect::{CallbackKinds, Reentrancy};
    let input = metadata.source_analysis_input()?;
    if !metadata.matches_registry(registry)
        || input.has_logical_source_name_context()
        || input.has_hosted_source_name_context()
    {
        return None;
    }
    let binding = parent.source_binding.as_ref()?;
    let syntax = binding.original_source_argument_syntax(parent, input, registry)?;
    let config = syntax.config();
    let children = crate::word_subst::checked_argument_lifted_calls(parent, config)?;
    let mut children = children.into_iter();
    let child = children.next()?.tokens?;
    if children.next().is_some()
        || parent
            .words()
            .iter()
            .filter(|word| matches!(word, crate::ir::WordExpr::CommandSubstitution { .. }))
            .count()
            != 1
        || parent.words().iter().any(|word| {
            !matches!(word, crate::ir::WordExpr::CommandSubstitution { .. })
                && !literal_or_scalar_read(word, config)
        })
        || child
            .words()
            .iter()
            .any(|word| !literal_or_scalar_read(word, config.nested()))
    {
        return None;
    }
    let parents = binding.original_direct_argument_lookups(parent, None, input, registry)?;
    let producers =
        binding.original_direct_argument_lookups(parent, Some(&child), input, registry)?;
    if parents.len() != producers.len() {
        return None;
    }
    let mut selected = None;
    for (parent_lookup, producer_lookup) in parents.iter().zip(&producers) {
        let invocation =
            super::resolve_original_source_layout(registry, Some(metadata), parent, parent_lookup)?;
        let producer = super::resolve_original_source_layout(
            registry,
            Some(metadata),
            &child,
            producer_lookup,
        )?;
        let facts = &producer.facts;
        if facts.native_result
            != Some(tcl_registry::native_result::NativeResultContract::ListArguments { from: 0 })
            || facts.arity_accepts_frozen_arguments() != Some(true)
            || !facts.arg_roles_complete
            || facts.successful_handler
                != Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::Leaf)
            || facts.effects.callback().kinds != CallbackKinds::NONE
            || facts.effects.callback().reentrancy != Reentrancy::Never
            || !facts.effects.accesses().is_empty()
            || facts.effects.legacy().command_table_mutation
            || !facts.effects.legacy().frame_effects.is_empty()
            || !facts.effects.legacy().side_effects.is_empty()
            || !facts.state_transitions.declared()?.facts().is_empty()
            || facts.source_callback_retention()
                != tcl_registry::resolved_invocation::SourceCallbackRetention::Absent
        {
            return None;
        }
        producer.with_metadata_schema(registry, metadata, producer_lookup.realm(), |schema| {
            closed_source_list_producer(schema, producer.dialect?)
        })?;
        let current = (invocation, parent_lookup.realm(), producer);
        if selected
            .as_ref()
            .is_some_and(|previous| previous != &current)
        {
            return None;
        }
        selected = Some(current);
    }
    let (parent, realm, producer) = selected?;
    let head = producer
        .argument_literal(0)
        .filter(|head| !head.contains('\0') && registry.get(head).is_some());
    Some(DirectSourceTaintArguments {
        parent,
        realm,
        child,
        literal_list_head: head,
    })
}

fn literal_or_scalar_read(word: &crate::ir::WordExpr, config: tcl_lexer::LexerConfig) -> bool {
    match word {
        crate::ir::WordExpr::Literal { .. } | crate::ir::WordExpr::BracedLiteral { .. } => true,
        crate::ir::WordExpr::Variable { .. } => {
            word.sole_variable_substitution()
                .is_some_and(|(spelling, _)| {
                    tcl_syntax::naming::whole_scalar_variable_reference_name(spelling, config)
                        .is_some()
                })
        }
        _ => false,
    }
}

fn closed_source_list_producer(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    dialect: tcl_registry::InvocationDialect,
) -> Option<()> {
    use tcl_registry::world_effect::{CallbackKinds, Reentrancy};
    let arguments = schema.authored_source_list_arguments()?;
    if arguments.start != 0
        || arguments.end != schema.words.arguments().exact_argv_len()?
        || !schema.semantics.world_effects.is_declared()
        || !schema.semantics.state_transitions.is_declared()
        || schema.semantics.frame_effect.is_some()
        || schema.semantics.body_kind != tcl_registry::BodyKind::Plain
        || schema.semantics.body_interpreter != tcl_registry::BodyInterpreter::Current
    {
        return None;
    }
    let (effects, transitions) = schema.authored_source_effects_and_state_transitions(dialect);
    (effects.callback().kinds == CallbackKinds::NONE
        && effects.callback().reentrancy == Reentrancy::Never
        && effects.accesses().is_empty()
        && !effects.legacy().command_table_mutation
        && effects.legacy().frame_effects.is_empty()
        && effects.legacy().side_effects.is_empty()
        && transitions.facts().is_empty())
    .then_some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected(source: &str) -> Option<(Option<String>, Vec<usize>)> {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = context.commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source,
            registry,
            false,
            registry.profile().unwrap(),
        );
        let ssa = &unit.top_level.ssa;
        let point = ssa
            .blocks
            .iter()
            .flat_map(|(block, data)| {
                (0..data.statements.len())
                    .map(move |index| crate::ssa::SsaSourceView::at_statement(ssa, *block, index))
            })
            .find(|point| {
                point.source_tokens().is_some_and(|tokens| {
                    tokens
                        .argv_texts
                        .first()
                        .is_some_and(|word| word == "interp")
                })
            })?;
        let tokens = point.source_tokens()?;
        let metadata = InvocationMetadataContext::for_module(registry, &unit.ir_module)?;
        let selected = original_direct_source_taint_arguments(registry, metadata, tokens)?;
        let reads = super::super::source_taint::original_source_taint_read_operands(
            registry, metadata, point, tokens,
        )?;
        let scripts = reads
            .script_arguments(
                &selected.parent.facts.canonical_command,
                &selected.parent.effective,
            )?
            .to_vec();
        Some((selected.literal_list_head, scripts))
    }

    #[test]
    fn original_direct_list_shape_keeps_undefined_path_and_tainted_head_distinct() {
        // Exact original FP-TNT-03 inputs retained: no child initialisation and
        // no invented compiler visit or normal-completion fact.
        assert_eq!(
            selected("set raw [gets stdin]\ninterp eval $child [list puts $raw]\n"),
            Some((Some("puts".to_owned()), vec![2]))
        );
        assert_eq!(
            selected("set raw [gets stdin]\ninterp eval $child [list $raw]\n"),
            Some((None, vec![2]))
        );
        assert_eq!(
            selected(
                "set child [interp create]; set raw [gets stdin]; interp eval $child [list puts $raw]"
            ),
            Some((Some("puts".to_owned()), vec![2]))
        );
        assert_eq!(
            selected(
                "set child [interp create]; set raw [gets stdin]; interp eval $child [list $raw]"
            ),
            Some((None, vec![2]))
        );
    }

    #[test]
    fn original_direct_list_shape_preserves_concat_trace_replacement_and_read_refusals() {
        let concatenated =
            selected("set raw [gets stdin]; interp eval $child [list puts $raw] $raw").unwrap();
        assert_eq!(concatenated.1, vec![2, 3]);
        for source in [
            "proc list args {return INVALID}; set raw [gets stdin]; interp eval $child [list puts $raw]",
            "proc mutate args {rename list oldList}; set raw [gets stdin]; trace add variable raw read mutate; interp eval $child [list puts $raw]",
            "set raw [gets stdin]; interp eval $child [list puts $arr($raw)]",
            "set raw [gets stdin]; interp eval $child [list puts [set raw]]",
            "set raw [gets stdin]; interp eval $child [list {*}{puts} $raw]",
            "set raw [gets stdin]; interp eval $child [list puts $raw]; unknown_writer",
        ] {
            // The last unknown operation comes after this argument and must
            // not retroactively replace the genuine pre-argv owner.
            if source.ends_with("unknown_writer") {
                assert_eq!(selected(source), Some((Some("puts".to_owned()), vec![2])));
            } else {
                assert!(selected(source).is_none(), "{source}");
            }
        }
    }
}
