// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conditional frame layout under an exact original Logical source input.

use super::{
    CommandRegistry, CommandTokens, InvocationMetadataContext, ResolvedStatementInvocation,
};
use tcl_registry::frame_effect::{FrameArgLayout, FrameArgumentResolution};

enum LogicalFrameSourceArguments {
    Layout(ResolvedStatementInvocation),
    Source(super::source_structure::OriginalRegistryWords),
}

pub(crate) struct OriginalLogicalFrameInvocation {
    selected: LogicalFrameSourceArguments,
    layout: Option<FrameArgLayout>,
    arguments: Option<FrameArgumentResolution>,
    offset: usize,
    original: CommandTokens,
    input: crate::analyser::ResolvedAnalysisInput,
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
        match &self.selected {
            LogicalFrameSourceArguments::Layout(selected) => selected.arguments.len(),
            LogicalFrameSourceArguments::Source(words) => words.arguments().len(),
        }
    }

    pub(crate) fn literal(&self, argument: usize) -> Option<String> {
        match &self.selected {
            LogicalFrameSourceArguments::Layout(selected) => selected.argument_literal(argument),
            LogicalFrameSourceArguments::Source(words) => {
                std::str::from_utf8(words.arguments().get(argument)?.literal_bytes()?)
                    .ok()
                    .map(str::to_owned)
            }
        }
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
        let word = match &self.selected {
            LogicalFrameSourceArguments::Layout(selected) => {
                selected.effective.words.get(argument.checked_add(1)?)?
            }
            LogicalFrameSourceArguments::Source(_) => self
                .original
                .words()
                .get(self.written_argument(argument)?.checked_add(1)?)?,
        };
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

    fn written_argument(&self, argument: usize) -> Option<usize> {
        match &self.selected {
            LogicalFrameSourceArguments::Layout(selected) => {
                selected.effective.written_argument(argument)
            }
            LogicalFrameSourceArguments::Source(words) => {
                match words.origins().get(argument.checked_add(1)?)? {
                    super::InvocationWordOrigin::Written(written) => written.checked_sub(1),
                    _ => None,
                }
            }
        }
    }

    /// Literal source-role candidates in an unavailable caller namespace.
    /// Every result retains opaque read/write hazards; descriptor candidates
    /// cannot establish which command a future caller actually binds.
    pub(crate) fn unknown_caller_variable_candidates(
        &self,
        module: &crate::ir::Module,
        registry: &CommandRegistry,
        tokens: &CommandTokens,
    ) -> Option<tcl_registry::source_name_ownership::SourceVariableNameOwnership> {
        let input = module.source_metadata_input.as_ref()?;
        if input != &self.input || tokens != &self.original {
            return None;
        }
        let tcl_registry::frame_effect::FrameArgumentResolution::Valid {
            level,
            level_word_len,
        } = self.arguments()?
        else {
            return None;
        };
        if self.layout()? != FrameArgLayout::ScriptInSelectedFrame || !level.is_caller_frame() {
            return None;
        }
        let argument = self.offset().checked_add(level_word_len)?;
        if self.count() != argument.checked_add(1)? {
            return None;
        }
        if module.source_metadata_input.as_ref() != Some(&self.input) || tokens != &self.original {
            return None;
        }
        self.literal(argument)?;
        let written = self.written_argument(argument)?;
        let body = original_logical_body_geometry_for_module(module, registry, tokens, written)?;
        let context = input.borrowed_context_registry();
        let mut out = tcl_registry::source_name_ownership::SourceVariableNameOwnership {
            opaque: true,
            ..Default::default()
        };
        for command in body.original_commands(context)? {
            let original = &command.words;
            let head =
                tcl_syntax::word_rules::original_static_word_unicode_value(original.first()?)?;
            let head = std::str::from_utf8(&head).ok()?;
            let values = original
                .iter()
                .skip(1)
                .map(tcl_syntax::word_rules::original_static_word_unicode_value)
                .collect::<Vec<_>>();
            let arguments = original
                .iter()
                .skip(1)
                .zip(&values)
                .map(|(word, value)| {
                    if word.group().expand {
                        return tcl_registry::InvocationWord::Expanded;
                    }
                    value
                        .as_deref()
                        .map_or(tcl_registry::InvocationWord::Dynamic, |value| {
                            super::source_structure::source_schema_word(
                                tcl_registry::InvocationWord::KnownBytes(value),
                            )
                        })
                })
                .collect::<Vec<_>>();
            let resolution =
                tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                    registry,
                    Some(context.context()),
                    tcl_registry::InvocationWords::structured(
                        tcl_registry::InvocationWord::Literal(head),
                        &arguments,
                    )
                    .with_dialect(tcl_registry::InvocationDialect::of_profile(
                        input.unit_profile(),
                    )),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                );
            if let Some(schema) = resolution.resolved() {
                let names = schema.authored_source_name_ownership(
                    tcl_registry::source_name_ownership::SourceRolePurpose::Logical,
                    tcl_registry::source_name_ownership::SourceNameOwnershipPurpose::Bindings,
                );
                out.bindings.extend(names.bindings);
            }
        }
        Some(out)
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
        let written = self.written_argument(argument)?;
        crate::lowering::Lowerer::original_logical_body_for_module(
            module, registry, tokens, written,
        )
    }
}

/// Genuine whole written Logical body under the unchanged full Module owner.
/// Its lexical geometry grants no child lookup namespace or entered frame.
pub(crate) fn original_logical_body_geometry_for_module(
    module: &crate::ir::Module,
    registry: &CommandRegistry,
    parent: &CommandTokens,
    argument: usize,
) -> Option<super::OriginalSourceScriptBody> {
    let input = module.source_metadata_input.as_ref()?;
    if !input.has_logical_source_name_context() {
        return None;
    }
    let words = super::source_structure::original_logical_registry_words_for_module(
        module, registry, parent,
    )?;
    let original = parent.words().get(argument.checked_add(1)?)?;
    let mut bodies = words
        .source_script_bodies(input.borrowed_context_registry())
        .into_iter()
        .filter(|body| {
            body.original_container().span()
                == tcl_lexer::word_span_at(module.source.bytes(), original.source().span)
                && body.matches_source(&module.source, input.lexer_config())
                && body.matches_context(input.borrowed_context_registry())
        });
    let body = bodies.next()?;
    bodies.next().is_none().then_some(body)
}

/// Frame grammar under this exact current Module's readonly source owner.
pub(crate) fn original_logical_frame_invocation_for_module(
    module: &crate::ir::Module,
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<OriginalLogicalFrameInvocation> {
    let words = super::source_structure::original_logical_registry_words_for_module(
        module, registry, tokens,
    )?;
    logical_source_frame_invocation(words, module.source_metadata_input.as_ref()?, tokens)
}

fn logical_source_frame_invocation(
    words: super::source_structure::OriginalRegistryWords,
    input: &crate::analyser::ResolvedAnalysisInput,
    tokens: &CommandTokens,
) -> Option<OriginalLogicalFrameInvocation> {
    let (layout, arguments, offset) =
        words.with_source_schema(input.borrowed_context_registry(), |schema| {
            (
                schema.semantics.frame_effect.map(|frame| frame.layout),
                schema.authored_logical_source_frame_argument_layout(),
                schema.semantics.argument_offset,
            )
        })?;
    Some(OriginalLogicalFrameInvocation {
        selected: LogicalFrameSourceArguments::Source(words),
        layout,
        arguments,
        offset,
        original: tokens.clone(),
        input: input.clone(),
    })
}

/// Original readonly frame grammar and effective argv, not an entered frame
/// or a successful variable link. The actual selected source recipe remains
/// distinct from the Native handler and compilation purposes.
pub(crate) fn original_logical_frame_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<OriginalLogicalFrameInvocation> {
    if context.matches_registry(registry)
        && let Some(input) = context.source_analysis_input()
        && let Some(words) = tokens
            .source_binding
            .as_ref()?
            .original_logical_source_words(tokens, input, registry)
    {
        return logical_source_frame_invocation(words, input, tokens);
    }
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
        selected: LogicalFrameSourceArguments::Layout(selected),
        layout,
        arguments,
        offset,
        original: tokens.clone(),
        input: context.source_analysis_input()?.clone(),
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
    fn protected_source_frame_arguments_keep_actual_values_without_invocation_entry() {
        let module = module("catch {upvar 1 {} {$literal}}");
        let registry = module.resolved_registry();
        let parent = module
            .top_level
            .retained_source_tokens_for_statement(&module.top_level.statements[0])
            .unwrap();
        let children = crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(&module, registry, parent).unwrap();
        let child = &children[0];
        let selected =
            super::original_logical_frame_invocation_for_module(&module, registry, child).unwrap();
        assert_eq!(selected.literal(0).as_deref(), Some("1"));
        assert_eq!(selected.literal(1).as_deref(), Some(""));
        assert_eq!(selected.literal(2).as_deref(), Some("$literal"));
        assert_eq!(selected.count(), 3);
        assert!(selected.layout().is_some());
        assert!(selected.arguments().is_some());
        assert!(
            child
                .source_binding
                .as_ref()
                .unwrap()
                .original_invocation_metadata_for_module(child, &module, registry)
                .is_none()
        );
        let mut stale = child.clone();
        stale.argv_texts[2] = "other".to_owned();
        assert!(
            super::original_logical_frame_invocation_for_module(&module, registry, &stale)
                .is_none()
        );
        let mut missing = module.clone();
        missing.source_metadata_input = None;
        assert!(
            super::original_logical_frame_invocation_for_module(&missing, registry, child)
                .is_none()
        );
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
    #[test]
    fn unknown_caller_candidates_keep_literal_names_and_all_frame_hazards() {
        for (written, expected) in [
            ("{}", ""),
            ("{$counter}", "$counter"),
            ("ordinary", "ordinary"),
        ] {
            let module = module(&format!(
                "proc p {{}} {{uplevel 1 {{set {written} VALUE}}}}; p"
            ));
            let registry = module.resolved_registry();
            assert!(module.source_entry.native_entry.is_none());
            let procedure = module.original_source_procedure_bodies().next().unwrap().1;
            let statement = procedure.body.statements.first().unwrap();
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(statement)
                .unwrap();
            let metadata = tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_invocation_metadata_for_module(tokens, &module, registry)
                .unwrap();
            let selected = super::original_logical_frame_invocation_with_metadata_context(
                registry, metadata, tokens,
            )
            .unwrap();
            let candidates = selected
                .unknown_caller_variable_candidates(&module, registry, tokens)
                .unwrap();
            assert_eq!(candidates.bindings, [expected]);
            assert!(candidates.opaque);
            assert!(
                selected.source_body(&module, registry, tokens, 1).is_none(),
                "MAY names cannot issue a caller namespace/body-entry receipt"
            );
            let mut stale = tokens.clone();
            stale.argv_texts[2] = "set different VALUE".to_owned();
            assert!(
                selected
                    .unknown_caller_variable_candidates(&module, registry, &stale)
                    .is_none()
            );
            let mut missing = module.clone();
            missing.source_metadata_input = None;
            assert!(
                selected
                    .unknown_caller_variable_candidates(&missing, registry, tokens)
                    .is_none()
            );
        }
    }
}
