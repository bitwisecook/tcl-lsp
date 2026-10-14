// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conditional frame layout under an exact original Logical source input.

use super::{
    CommandRegistry, CommandTokens, InvocationMetadataContext, ResolvedStatementInvocation,
};
use tcl_registry::frame_effect::{FrameArgLayout, FrameArgumentResolution};

pub(crate) struct OriginalLogicalFrameInvocation {
    selected: ResolvedStatementInvocation,
    layout: Option<FrameArgLayout>,
    arguments: Option<FrameArgumentResolution>,
    offset: usize,
}

impl OriginalLogicalFrameInvocation {
    pub(crate) fn layout(&self) -> Option<FrameArgLayout> {
        self.layout
    }

    pub(crate) fn arguments(&self) -> Option<FrameArgumentResolution> {
        self.arguments
    }

    pub(crate) fn offset(&self) -> usize {
        self.offset
    }

    pub(crate) fn count(&self) -> usize {
        self.selected.arguments.len()
    }

    pub(crate) fn literal(&self, argument: usize) -> Option<String> {
        self.selected.argument_literal(argument)
    }

    pub(crate) fn parameter(
        &self,
        argument: usize,
        parameters: &[String],
        config: tcl_lexer::LexerConfig,
    ) -> Option<String> {
        if self.literal(argument).is_some() {
            return None;
        }
        let word = self
            .selected
            .effective
            .words
            .get(argument.checked_add(1)?)?;
        let (spelling, source) = word.sole_variable_substitution()?;
        if source.provenance != crate::ir::Provenance::Source {
            return None;
        }
        let reference =
            tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), config).ok()??;
        if reference.index.is_some() {
            return None;
        }
        let name = std::str::from_utf8(reference.name).ok()?;
        parameters
            .iter()
            .any(|parameter| parameter == name)
            .then(|| name.to_owned())
    }

    /// Original direct child only. A captured or materialized script cannot
    /// borrow another command's child source or a selected execution frame.
    pub(crate) fn source_body(
        &self,
        module: &crate::ir::Module,
        registry: &CommandRegistry,
        tokens: &CommandTokens,
        argument: usize,
    ) -> Option<crate::ir::Script> {
        self.literal(argument)?;
        let written = self.selected.effective.written_argument(argument)?;
        crate::lowering::Lowerer::original_logical_body_for_module(
            module, registry, tokens, written,
        )
    }
}

/// Original readonly frame grammar and effective argv, not an entered frame
/// or a successful variable link. The actual selected source recipe remains
/// distinct from the Native handler and compilation purposes.
pub(crate) fn original_logical_frame_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<OriginalLogicalFrameInvocation> {
    let binding = tokens.source_binding.as_ref()?;
    let context = binding.original_invocation_metadata_for_input(
        tokens,
        context.source_analysis_input()?,
        registry,
    )?;
    let selected = super::original_logical_operation_invocation_with_metadata_context(
        registry, context, tokens,
    )?;
    let realm = binding.original_source_realm_for_input(
        tokens,
        context.source_analysis_input()?,
        registry,
    )?;
    let (layout, arguments, offset) =
        selected.with_metadata_schema(registry, context, realm, |schema| {
            Some((
                schema.semantics.frame_effect.map(|frame| frame.layout),
                schema.authored_logical_source_frame_argument_layout(),
                schema.semantics.argument_offset,
            ))
        })?;
    Some(OriginalLogicalFrameInvocation {
        selected,
        layout,
        arguments,
        offset,
    })
}

#[cfg(test)]
mod tests {
    fn module(source: &str) -> crate::ir::Module {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(context.commands(), config)
                .with_resolved_analysis_input(input),
            source,
        )
    }

    #[test]
    fn original_logical_body_keeps_actual_parent_and_declines_unknown_caller_scope() {
        for (level, expected) in [("#0", true), ("0", true), ("1", false), ("$level", false)] {
            let module = module(&format!(
                "interp alias {{}} g {{}} uplevel {level}; proc p {{}} {{g {{set x VALUE}}}}"
            ));
            let registry = module.resolved_registry();
            assert!(module.source_entry.native_entry.is_none());
            let procedure = module
                .original_source_procedure_bodies()
                .next()
                .map(|(_, procedure)| procedure)
                .expect("original source procedure");
            let statement = procedure.body.statements.first().expect("original call");
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(statement)
                .expect("complete original vector");
            let body = crate::lowering::Lowerer::original_logical_body_for_module(
                &module, registry, tokens, 0,
            );
            assert_eq!(body.is_some(), expected, "{level}");
            if let Some(body) = body {
                assert!(!body.statements.is_empty());
                assert!(body.namespace_context.is_some());
                assert!(
                    body.executed_source.is_none(),
                    "source syntax does not issue an entered body"
                );
            }
            let mut stale = tokens.clone();
            stale.argv_texts[1] = "set different VALUE".to_owned();
            assert!(
                crate::lowering::Lowerer::original_logical_body_for_module(
                    &module, registry, &stale, 0,
                )
                .is_none()
            );
            let mut missing = module.clone();
            missing.retained_source_bindings = None;
            assert!(
                crate::lowering::Lowerer::original_logical_body_for_module(
                    &missing, registry, tokens, 0,
                )
                .is_none()
            );
        }
    }
}
