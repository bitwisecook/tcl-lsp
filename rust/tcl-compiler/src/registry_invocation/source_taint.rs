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

//! Conditional source taint metadata, independent of handler/effect receipts.

use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tcl_lexer::{LexerConfig, SourceImage};
use tcl_registry::{CommandRegistry, model::ContextRegistry};

/// Original operands selected at one SSA operation, without a read or value proof.
pub(crate) struct OriginalSourceTaintReadOperands {
    words: Vec<crate::ir::WordExpr>,
    original: Vec<tcl_lexer::NativeWord>,
    config: LexerConfig,
}

impl OriginalSourceTaintReadOperands {
    pub(crate) const fn config(&self) -> LexerConfig {
        self.config
    }

    pub(crate) fn owns_word(&self, word: &crate::ir::WordExpr) -> bool {
        self.words
            .iter()
            .zip(&self.original)
            .skip(1)
            .any(|(actual, original)| actual == word && original.span() == word.source().span)
    }
}

/// Join the current operation's own argv or an authentic lexical child to its
/// original input. Same-offset words from another analysis cannot donate reads.
pub(crate) fn original_source_taint_read_operands(
    registry: &CommandRegistry,
    context: super::InvocationMetadataContext<'_>,
    point: crate::ssa::SsaSourceView<'_>,
    tokens: &crate::ir::CommandTokens,
) -> Option<OriginalSourceTaintReadOperands> {
    let parent = point.source_tokens()?;
    let actual = if parent == tokens {
        parent.clone()
    } else {
        let mut children =
            crate::word_subst::checked_original_argument_lifted_calls(parent, registry, context)?;
        let index = children
            .iter()
            .position(|child| child.tokens.as_ref() == Some(tokens))?;
        children.swap_remove(index).tokens?
    };
    let input = context.source_analysis_input()?;
    context.matches_registry(registry).then_some(())?;
    let binding = actual.source_binding.as_ref()?;
    binding.original_argument_metadata_for_input(&actual, input, registry)?;
    let config = binding.original_argument_lexer_config(&actual)?;
    let image = binding.original_argument_source_image(&actual)?;
    let original = super::original_native_compiler_words(
        image,
        actual.words(),
        actual.argv.first()?.start(),
        config,
    )?;
    Some(OriginalSourceTaintReadOperands {
        words: actual.words().to_vec(),
        original,
        config,
    })
}

/// Read operands for one command nested in a retained conditional expression.
/// This is a distinct source phase from argv-child lifting: it requires the
/// original expression evaluation owner and exact command node in that tree.
pub(crate) fn original_expression_taint_read_operands(
    registry: &CommandRegistry,
    context: super::InvocationMetadataContext<'_>,
    parent_tokens: &crate::ir::CommandTokens,
    nested: &crate::ir::CommandTokens,
    evaluation: &crate::command_binding::SourceConditionalExpressionEvaluation,
    spelling: &str,
    span: tcl_lexer::Span,
) -> Option<OriginalSourceTaintReadOperands> {
    if !evaluation.semantic_lookup_closed()
        || !context.matches_registry(registry)
        || span.end().checked_sub(span.start())? as usize != spelling.len()
    {
        return None;
    }
    let input = context.source_analysis_input()?;
    let source = evaluation.source();
    let relative_start = span.start().checked_sub(source.base())?;
    let relative_end = span.end().checked_sub(source.base())?;
    if !expression_contains_command(evaluation.tree(), relative_start, relative_end, spelling) {
        return None;
    }
    let image = source.origin.source_image();
    if image
        .bytes()
        .get(span.start() as usize..span.end() as usize)
        != Some(spelling.as_bytes())
    {
        return None;
    }
    let parent = parent_tokens.source_binding.as_ref()?;
    let parent_site = parent.invocation_site()?;
    parent.original_argument_metadata_for_input(parent_tokens, input, registry)?;
    if !std::sync::Arc::ptr_eq(&parent_site.source, &source.origin)
        || parent.conditional_expression_evaluation(registry, parent_tokens)? != *evaluation
    {
        return None;
    }
    let binding = nested.source_binding.as_ref()?;
    let site = binding.invocation_site()?;
    if !std::sync::Arc::ptr_eq(&site.source, &source.origin)
        || site.offset < span.start()
        || site.offset >= span.end()
        || binding.invocation_realm()
            != Some(tcl_dialect::model::InvocationRealm::InterpreterRuntime)
    {
        return None;
    }
    let config = parent.original_argument_lexer_config(parent_tokens)?;
    let nested_config = config.nested();
    let original = super::original_native_compiler_words(
        image,
        nested.words(),
        nested.argv.first()?.start(),
        nested_config,
    )?;
    Some(OriginalSourceTaintReadOperands {
        words: nested.words().to_vec(),
        original,
        config: nested_config,
    })
}

fn expression_contains_command(
    node: &crate::expr_ast::ExprNode,
    start: u32,
    end: u32,
    spelling: &str,
) -> bool {
    use crate::expr_ast::ExprNode;
    match node {
        ExprNode::Command {
            text,
            start: node_start,
            end: node_end,
        } => text == spelling && *node_start == start && *node_end == end,
        ExprNode::Binary { left, right, .. } => {
            expression_contains_command(left, start, end, spelling)
                || expression_contains_command(right, start, end, spelling)
        }
        ExprNode::Unary { operand, .. } => {
            expression_contains_command(operand, start, end, spelling)
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            expression_contains_command(condition, start, end, spelling)
                || expression_contains_command(true_branch, start, end, spelling)
                || expression_contains_command(false_branch, start, end, spelling)
        }
        ExprNode::Call { args, .. } => args
            .iter()
            .any(|arg| expression_contains_command(arg, start, end, spelling)),
        ExprNode::Literal { .. }
        | ExprNode::String { .. }
        | ExprNode::CompiledWord { .. }
        | ExprNode::Var { .. }
        | ExprNode::Raw { .. } => false,
    }
}

/// Actual lowering source/context for conditional hosted advice.
/// No constructor accepts a profile label or reconstructed native name.
#[derive(Debug, Clone)]
pub struct HostedSourceTaintContext {
    image: SourceImage,
    config: LexerConfig,
    context: Arc<ContextRegistry>,
    source_structure_input: Option<crate::analyser::ResolvedAnalysisInput>,
}
impl PartialEq for HostedSourceTaintContext {
    fn eq(&self, other: &Self) -> bool {
        self.source_structure_input == other.source_structure_input
            && self.image == other.image
            && self.config == other.config
            && self.context.context() == other.context.context()
            && self.context.commands().snapshot().semantic_key()
                == other.context.commands().snapshot().semantic_key()
    }
}
impl Eq for HostedSourceTaintContext {}
impl Hash for HostedSourceTaintContext {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source_structure_input.hash(state);
        self.image.hash(state);
        self.config.hash(state);
        // Structural equality remains authoritative; equivalent version
        // spellings can deliberately share these stable coordinate hashes.
        self.context.context().environment.id.hash(state);
        for floor in self.context.context().floors.entries() {
            floor.axis.hash(state);
        }
        self.context
            .commands()
            .snapshot()
            .semantic_key()
            .hash(state);
    }
}
#[cfg(debug_assertions)]
fn trace_hosted_source(tokens: &crate::ir::CommandTokens, stage: &str) {
    if std::env::var_os("TCL_LSP_TRACE_HOSTED_TAINT").is_some() {
        let site = tokens
            .words()
            .first()
            .map(|word| word.source().span.start());
        eprintln!(
            "HOSTED_TAINT_SOURCE site={site:?} stage={stage} words={} binding={} context={}",
            tokens.words().len(),
            tokens.source_binding.is_some(),
            tokens.hosted_taint_context.is_some()
        );
    }
}

#[cfg(not(debug_assertions))]
fn trace_hosted_source(_: &crate::ir::CommandTokens, _: &str) {}

impl HostedSourceTaintContext {
    pub(crate) fn from_lowering(
        image: SourceImage,
        config: LexerConfig,
        context: Arc<ContextRegistry>,
    ) -> Option<Self> {
        let hosted =
            tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::for_environment(
                &context.context().environment.id,
            )?;
        tcl_syntax::naming::VendorSourceNamePolicy::authored(hosted)?;
        Some(Self {
            image,
            config,
            context,
            source_structure_input: None,
        })
    }
    /// Source grammar additionally retains the complete input that issued it.
    pub(crate) fn retain_source_structure_input(
        mut self,
        input: Option<&crate::analyser::ResolvedAnalysisInput>,
    ) -> Self {
        self.source_structure_input = input
            .filter(|input| {
                input.has_hosted_source_name_context()
                    && input.lexer_config() == self.config
                    && Arc::ptr_eq(&input.context_registry(), &self.context)
                    && input.vendor_source_policy().is_none_or(|policy| {
                        tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::for_environment(
                            &self.context.context().environment.id,
                        ).and_then(tcl_syntax::naming::VendorSourceNamePolicy::authored) == Some(policy)
                    })
            })
            .cloned();
        self
    }

    fn original_catalogue_metadata(
        &self,
        registry: &CommandRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<super::OriginalConditionalVendorRegistryMetadata> {
        trace_hosted_source(tokens, "issuer-enter");
        if tokens.synthetic.is_some()
            || self.context.commands().snapshot().semantic_key()
                != registry.snapshot().semantic_key()
        {
            trace_hosted_source(tokens, "synthetic-or-registry-mismatch");
            return None;
        }
        if let Some(binding) = tokens.source_binding.as_ref()
            && binding.original_lexer_config_for_tokens(tokens) != Some(self.config)
        {
            trace_hosted_source(tokens, "binding-vector-config-mismatch");
            return None;
        }
        let offset = tokens.words().first()?.source().span.start();
        let original =
            super::original_native_compiler_words(&self.image, tokens.words(), offset, self.config)
                .or_else(|| {
                    trace_hosted_source(tokens, "original-vector-unavailable");
                    None
                })?;
        let hosted =
            tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::for_environment(
                &self.context.context().environment.id,
            )?;
        let policy = tcl_syntax::naming::VendorSourceNamePolicy::authored(hosted)?;
        let head = crate::signature_scan::vendor_name::VendorSourceNameInput::from_original_word(
            original.first()?,
            policy,
        );
        let metadata = super::original_conditional_vendor_registry_metadata(
            &self.context,
            tokens,
            &head,
            &original,
        )
        .or_else(|| {
            trace_hosted_source(tokens, "guarded-catalogue-unavailable");
            None
        })?;
        if !metadata.matches_source(&self.image, self.config)
            || !metadata.matches_registry(registry)
        {
            trace_hosted_source(tokens, "metadata-owner-mismatch");
            return None;
        }
        Some(metadata)
    }

    fn invocation(
        &self,
        registry: &CommandRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<HostedSourceTaintInvocation> {
        let metadata = self.original_catalogue_metadata(registry, tokens)?;
        let original = metadata.shape().original_words();
        let shape = metadata.shape().clone();
        if original.iter().any(|word| word.group().expand) {
            return None;
        }
        let effective =
            super::compose_effective_words(tokens, shape.command(), &[]).or_else(|| {
                trace_hosted_source(tokens, "effective-vector-unavailable");
                None
            })?;
        trace_hosted_source(tokens, "issuer-selected");
        Some(HostedSourceTaintInvocation {
            metadata,
            effective,
        })
    }
}

/// Possible authored source/sink at unchanged words in the actual host context.
/// Unknown runtime targets, effects, mutations, barriers and Normal stay intact.
pub(crate) struct HostedSourceTaintInvocation {
    pub(crate) metadata: super::OriginalConditionalVendorRegistryMetadata,
    pub(crate) effective: super::EffectiveCommandWords,
}
pub(crate) fn original_hosted_source_registry_metadata(
    registry: &CommandRegistry,
    context: super::InvocationMetadataContext<'_>,
    tokens: &crate::ir::CommandTokens,
) -> Option<super::OriginalConditionalVendorRegistryMetadata> {
    let input = context.source_analysis_input()?;
    input.has_hosted_source_name_context().then_some(())?;
    let owner = tokens.hosted_taint_context.as_ref()?;
    if owner.source_structure_input.as_ref() != Some(input)
        || owner.context.context() != input.borrowed_context_registry().context()
        || owner.config != input.lexer_config()
        || !context.matches_registry(registry)
    {
        return None;
    }
    owner.original_catalogue_metadata(registry, tokens)
}

pub(crate) fn hosted_source_taint_invocation(
    registry: &CommandRegistry,
    tokens: &crate::ir::CommandTokens,
) -> Option<HostedSourceTaintInvocation> {
    let context = tokens.hosted_taint_context.as_ref().or_else(|| {
        trace_hosted_source(tokens, "lowering-context-unavailable");
        None
    })?;
    context.invocation(registry, tokens)
}

/// Conditional Native source warning metadata under a current issued body.
/// Lookup uncertainty and unsupported alternatives remain independent.
pub(crate) struct NativeSourceTaintInvocation {
    pub(crate) words: super::source_structure::OriginalRegistryWords,
    pub(crate) effective: super::EffectiveCommandWords,
}

pub(crate) fn native_source_taint_invocation(
    registry: &CommandRegistry,
    context: super::InvocationMetadataContext<'_>,
    module: &crate::ir::Module,
    body: crate::ir::ProcedureId,
    tokens: &crate::ir::CommandTokens,
) -> Option<NativeSourceTaintInvocation> {
    let input = module.source_metadata_input.as_ref()?;
    let supplied = context.source_analysis_input()?;
    if input.has_logical_source_name_context()
        || input.has_hosted_source_name_context()
        || (supplied != input && supplied != &input.for_nested_source())
        || !context.matches_registry(registry)
        || !module
            .retained_source_bindings
            .as_ref()?
            .matches_module(module, registry)
    {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    if !binding.original_native_source_owner_for_body(tokens, module, body, registry) {
        return None;
    }
    let words = binding.original_native_source_words(tokens, input, registry)?;
    if words.origins().len() != tokens.words().len()
        || words
            .origins()
            .iter()
            .enumerate()
            .any(|(ordinal, origin)| *origin != super::InvocationWordOrigin::Written(ordinal))
        || tokens
            .words()
            .iter()
            .any(|word| matches!(word, crate::ir::WordExpr::Expand { .. }))
    {
        return None;
    }
    words.with_source_schema(input.borrowed_context_registry(), |schema| {
        (schema.facts().arity_accepts_frozen_arguments() == Some(true)
            && schema.facts().canonical_command == words.command())
        .then_some(())
    })??;
    let effective = super::compose_original_effective_words(tokens, words.command(), &[])?;
    Some(NativeSourceTaintInvocation { words, effective })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::CommandTokens;

    #[test]
    fn original_taint_operands_require_the_current_operation_and_original_child() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = context.commands().profile().unwrap();
        let build = || {
            crate::compilation_unit::CompilationUnit::build_with_context_registry(
                "set payload 1; puts [list $payload]",
                crate::compilation_unit::UnitBuildOptions {
                    registry: context.commands(),
                    defer_top_level: false,
                    config: LexerConfig::for_file_grammar(profile.grammar),
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: None,
                },
                None,
                Arc::clone(&context),
            )
        };
        let unit = build();
        let registry = context.commands();
        let metadata =
            super::super::InvocationMetadataContext::for_module(registry, &unit.ir_module).unwrap();
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
                    tokens.argv_texts.first().is_some_and(|head| head == "puts")
                })
            })
            .expect("actual source operation");
        let tokens = point.source_tokens().unwrap();
        let owned = original_source_taint_read_operands(registry, metadata, point, tokens)
            .expect("original operation operands");
        assert!(owned.owns_word(&tokens.words()[1]));
        let children =
            crate::word_subst::checked_original_argument_lifted_calls(tokens, registry, metadata)
                .expect("complete original argv children");
        let child = children
            .iter()
            .find_map(|child| child.tokens.as_ref())
            .expect("original list child");
        let owned = original_source_taint_read_operands(registry, metadata, point, child)
            .expect("original child operands");
        assert!(owned.owns_word(&child.words()[1]));
        assert!(!owned.owns_word(&tokens.words()[1]));
        let mut changed = child.clone();
        changed.argv_texts[1] = "$other".into();
        assert!(original_source_taint_read_operands(registry, metadata, point, &changed).is_none());
        let foreign = build();
        let foreign_ssa = &foreign.top_level.ssa;
        let foreign_child = foreign_ssa
            .blocks
            .iter()
            .flat_map(|(block, data)| {
                (0..data.statements.len()).filter_map(move |index| {
                    crate::ssa::SsaSourceView::at_statement(foreign_ssa, *block, index)
                        .source_tokens()
                })
            })
            .find(|candidate| candidate.argv_texts == tokens.argv_texts)
            .expect("equal text in an independent analysis");
        assert!(
            original_source_taint_read_operands(registry, metadata, point, foreign_child).is_none()
        );
    }

    fn native_fixture(source: &str) -> (crate::ir::Module, crate::analyser::ResolvedAnalysisInput) {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = context.commands().profile().unwrap();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        // Unentered native declaration bodies need explicit unknown history.
        let module = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(context.commands(), config)
                .with_resolved_analysis_input(input.clone())
                .with_explicit_unknown_entry(),
            source,
        );
        (module, input)
    }

    #[test]
    fn native_source_taint_keeps_the_actual_body_and_missing_execution_separate() {
        let (module, input) = native_fixture("proc run {value} {exec $value}");
        let registry = input.borrowed_context_registry().commands();
        let (&body, _) = module
            .original_native_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let tokens = module.body_units[&body].body.statements[0]
            .tokens()
            .unwrap();
        let metadata =
            super::super::InvocationMetadataContext::for_module(registry, &module).unwrap();
        let selected = native_source_taint_invocation(registry, metadata, &module, body, tokens)
            .expect("genuine unentered Native source warning candidate");
        assert_eq!(selected.words.command(), "exec");
        assert_eq!(selected.effective.written_argument(0), Some(0));
        assert!(!selected.words.operands_preserve_source_lookup());
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .proved_handler_target()
                .is_none()
        );
        let mut changed = module.clone();
        changed.source = SourceImage::document("proc run {value} {exec $value} ");
        assert!(
            native_source_taint_invocation(registry, metadata, &changed, body, tokens).is_none()
        );
        let (foreign, _) = native_fixture("proc run {value} {exec $value}");
        let (&foreign_body, _) = foreign
            .original_native_declaration_body_units
            .iter()
            .next()
            .unwrap();
        assert!(
            native_source_taint_invocation(registry, metadata, &module, foreign_body, tokens)
                .is_none()
        );
        let mut missing = module.clone();
        missing.source_metadata_input = None;
        assert!(
            native_source_taint_invocation(registry, metadata, &missing, body, tokens).is_none()
        );
        let mut missing = tokens.clone();
        missing.source_binding = None;
        assert!(
            native_source_taint_invocation(registry, metadata, &module, body, &missing).is_none()
        );
        let foreign_registry =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        assert!(
            native_source_taint_invocation(
                foreign_registry.commands(),
                metadata,
                &module,
                body,
                tokens
            )
            .is_none()
        );
    }

    #[test]
    fn native_source_taint_does_not_guess_dynamic_heads_or_expanded_argv() {
        for source in [
            "proc run {callee value} {$callee $value}",
            "proc run {value} {exec {*}$value}",
            "proc run {value} {exec {*}{one two} $value}",
        ] {
            let (module, input) = native_fixture(source);
            let registry = input.borrowed_context_registry().commands();
            let (&body, _) = module
                .original_native_declaration_body_units
                .iter()
                .next()
                .unwrap();
            let tokens = module.body_units[&body].body.statements[0]
                .tokens()
                .unwrap();
            let metadata =
                super::super::InvocationMetadataContext::for_module(registry, &module).unwrap();
            assert!(
                native_source_taint_invocation(registry, metadata, &module, body, tokens).is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn hosted_taint_lowering_keeps_the_invocations_whole_original_image() {
        // Implementation contract: naming.consumer.hosted-source-taint-descriptor
        // docs/design/analysis/name-resolution-proofs/hosted-source-taint-descriptor.md
        let source = "when HTTP_REQUEST {set x [HTTP::query]; HTTP::respond 200 content $x}";
        let context = tcl_registry::model::ingress::resolve_environment("f5-irules")
            .default_context_registry();
        let registry = context.commands();
        let config = LexerConfig::for_profile(registry.profile());
        let options = crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(tcl_dialect::DialectProfile::irules()),
            external_call_sites: None,
            declared_commands: None,
        };
        let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
            source,
            options,
            None,
            Arc::clone(&context),
        );
        let function = unit.function("::when::HTTP_REQUEST").unwrap();
        let sink = function
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .filter_map(|statement| statement.statement.tokens())
            .find(|tokens| {
                tokens.words().first().is_some_and(|head| {
                    head.source().span.start() as usize == source.find("HTTP::respond").unwrap()
                })
            })
            .expect("the genuine lowered sink retains its complete original vector");
        let retained = sink.hosted_taint_context.as_ref().unwrap();
        assert_eq!(retained.image, SourceImage::document(source));
        let selected = hosted_source_taint_invocation(registry, sink)
            .expect("nested hosted source metadata keeps the actual original owner");
        assert_eq!(selected.metadata.shape().command(), "HTTP::respond");
        assert!(!selected.metadata.obligations().is_empty());
        assert!(
            super::super::resolved_tokens_invocation(registry, None, sink).is_none(),
            "source advice supplies no native handler"
        );
        let mut foreign = sink.clone();
        let mut changed = retained.as_ref().clone();
        let mut padded = source.as_bytes().to_vec();
        padded[..source.find("set x").unwrap()].fill(b' ');
        changed.image = SourceImage::from_bytes(padded.as_slice(), retained.image.channel());
        foreign.hosted_taint_context = Some(Arc::new(changed));
        assert!(
            hosted_source_taint_invocation(registry, &foreign).is_none(),
            "equal command offsets cannot substitute a padded body for its whole source"
        );
    }

    #[test]
    fn hosted_source_taint_metadata_requires_actual_words_context_and_registry() {
        // Implementation contract: naming.consumer.hosted-source-taint-descriptor
        // docs/design/analysis/name-resolution-proofs/hosted-source-taint-descriptor.md
        let source = "HTTP::respond 200 content $x";
        let context = tcl_registry::model::ingress::resolve_environment("f5-irules")
            .default_context_registry();
        let registry = context.commands();
        let config = LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let image = SourceImage::document(source);
        let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let mut tokens = CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(&image),
            config,
            &segments[0],
        );
        assert!(hosted_source_taint_invocation(registry, &tokens).is_none());
        tokens.hosted_taint_context =
            HostedSourceTaintContext::from_lowering(image.clone(), config, Arc::clone(&context))
                .map(Arc::new);
        assert!(
            hosted_source_taint_invocation(registry, &tokens).is_none(),
            "context alone supplies no current source binding"
        );
        let input = crate::analyser::ResolvedAnalysisInput::new(
            tcl_dialect::DialectProfile::irules(),
            tcl_dialect::DialectProfile::irules(),
            Arc::clone(&context),
            config,
        );
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "report-only");
        analysis
            .retained_command_realm()
            .unwrap()
            .stamp_original_tokens(&mut tokens);
        let selected = hosted_source_taint_invocation(registry, &tokens).unwrap();
        assert!(!selected.metadata.obligations().is_empty());
        assert_eq!(selected.metadata.shape().command(), "HTTP::respond");
        assert_eq!(selected.effective.origins.len(), 4);
        assert!(
            super::super::resolved_tokens_invocation(registry, None, &tokens).is_none(),
            "source metadata issues no native invocation"
        );
        let old = tokens.clone();
        tokens.word_exprs[0] = tokens.word_exprs[1].clone();
        assert!(hosted_source_taint_invocation(registry, &tokens).is_none());
        tokens = old;
        let changed_image = SourceImage::document("HTTP::respond 200 content $y");
        tokens.hosted_taint_context =
            HostedSourceTaintContext::from_lowering(changed_image, config, Arc::clone(&context))
                .map(Arc::new);
        assert!(hosted_source_taint_invocation(registry, &tokens).is_none());
        let mut changed_config = config;
        changed_config.strict_quoting = !config.strict_quoting;
        tokens.hosted_taint_context = HostedSourceTaintContext::from_lowering(
            image.clone(),
            changed_config,
            Arc::clone(&context),
        )
        .map(Arc::new);
        assert!(hosted_source_taint_invocation(registry, &tokens).is_none());
        let environment = tcl_registry::model::ingress::resolve_environment("f5-irules");
        let keyed = tcl_registry::model::context::KeyedVersions {
            bigip: Some(tcl_dialect::model::Version::parse("21.1.0").unwrap()),
            ..Default::default()
        };
        let keyed_context = environment
            .context_registry(&keyed, 0)
            .expect("authored keyed hosted context is available");
        let original_context =
            HostedSourceTaintContext::from_lowering(image.clone(), config, Arc::clone(&context))
                .unwrap();
        let keyed_context =
            HostedSourceTaintContext::from_lowering(image.clone(), config, keyed_context).unwrap();
        assert_ne!(original_context, keyed_context);
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        assert!(
            HostedSourceTaintContext::from_lowering(image, config, Arc::clone(&foreign)).is_none()
        );
    }
}
