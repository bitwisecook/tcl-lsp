//! Original source increment layout for conditional numeric diagnostics.

use super::{CommandRegistry, CommandTokens, EffectiveInvocationWord, InvocationWordOrigin};

#[derive(PartialEq, Eq)]
pub(crate) struct DeclarationIncrementAdvice {
    pub(crate) operand: crate::ir::WordExpr,
    pub(crate) amount: Option<String>,
}

/// Original candidates must select the same increment and written variable
/// operand. This preserves unknown runtime dispatch and conversion callbacks.
pub(crate) fn declaration_increment_advice(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<DeclarationIncrementAdvice> {
    let binding = tokens.source_binding.as_ref()?;
    let source = binding.declaration_source_operand_layout_advice(registry, tokens)?;
    source.with_conditional_registry_facts(registry, tokens, |facts, effective, values| {
        if facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Incr,
            )
            || !facts.arg_roles_complete
            || facts.argument_offset != 0
            || !(2..=3).contains(&values.len())
        {
            return None;
        }
        let InvocationWordOrigin::Written(index) = *effective.origins.get(1)? else {
            return None;
        };
        let operand = tokens.words().get(index)?.clone();
        binding.declaration_source_variable_operand_advice(registry, tokens, &operand)?;
        let amount = if values.len() == 2 {
            None
        } else {
            let EffectiveInvocationWord::Literal(value) = values.get(2)? else {
                return None;
            };
            Some(value.clone())
        };
        Some(DeclarationIncrementAdvice { operand, amount })
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn increment_layout_retains_original_operands_and_rejects_scope_or_worker_changes() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        for (source, body, expected) in [
            ("proc p {} {set x 4; incr x}", "incr x", true),
            ("proc p {} {set x 4; incr x 2}", "incr x 2", true),
            ("proc p {} {incr x $amount}", "incr x $amount", false),
            ("proc p {} {global x; incr x}", "incr x", false),
            ("proc p {x} {incr x}", "incr x", false),
            ("proc incr args {}; proc p {} {incr x}", "incr x", false),
        ] {
            let bindings =
                crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
            let segment = crate::segmenter::segment_commands_with_offset_and_config(
                body,
                u32::try_from(source.rfind(body).unwrap()).unwrap(),
                config,
            )
            .remove(0);
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceImage::document(source).source_map(),
                config,
                &segment,
            );
            bindings.stamp_original_tokens(&mut tokens);
            let advice = super::declaration_increment_advice(registry, &tokens);
            assert_eq!(advice.is_some(), expected, "{source}");
            if let Some(advice) = advice {
                assert_eq!(advice.operand, tokens.words()[1]);
                assert_eq!(
                    advice.amount.as_deref(),
                    (tokens.words().len() == 3).then_some("2")
                );
            }
        }
    }
}
