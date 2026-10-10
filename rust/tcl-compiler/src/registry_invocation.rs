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

//! Shared compiler adapter from semantic source words to registry facts.
//!
//! The registry must only receive a Tcl word's evaluated value when the
//! semantic IR proves it literal. This module centralises that conversion for
//! every common compiler pass: executable IR, memory SSA, and later shared
//! analyses all resolve exactly the same structured [`WordExpr`] facts under
//! an explicitly supplied [`SemanticContext`] — the resolved environment the
//! document is assisted under (redesign §11.2 D1).

mod original_procedure_call;
pub(crate) use original_procedure_call::{
    original_authored_procedure_calls_for_module, original_logical_procedure_calls_for_module,
    original_procedure_formal_count_shape, original_procedure_parameters,
    original_procedure_scalar_bindings,
};
mod metadata_context;
pub use metadata_context::{
    InvocationMetadataContext, InvocationMetadataInput, OwnedInvocationMetadataContext,
};
pub(crate) use metadata_context::{
    retained_module_metadata_context, retained_source_metadata_context,
};
mod symbol_advice;
pub(crate) use symbol_advice::{
    OriginalSymbolDeclarationAdvice, original_symbol_declaration_advice,
    vendor_symbol_declaration_advice,
};
mod conditional_vendor_metadata;
pub(crate) use conditional_vendor_metadata::original_vendor_occurrence_registry_metadata;
pub use conditional_vendor_metadata::{
    OriginalConditionalVendorRegistryMetadata, original_conditional_vendor_registry_metadata,
};
mod conditional_metadata;
pub use conditional_metadata::{
    OriginalConditionalRegistryMetadata, original_conditional_registry_metadata,
};
/// Original Logical formal binding identity and bounded alpha source coverage.
pub mod logical_formals;
mod source_availability;
pub use source_availability::{
    CommandSourceAvailabilityDomain, OriginalSourceCommandAvailability,
    original_source_command_availability,
};
mod source_publication;
pub use source_publication::{
    OriginalSourceCommandPublication, OriginalSourceHandleClassAdvice,
    OriginalSourceHandleConstruction,
};
mod source_scoped_body;
/// Sealed readonly original argv and Registry source-role structure.
pub mod source_structure;
pub use source_scoped_body::{
    OriginalDeclaredSourceScriptBody, OriginalSourceDefinitionMemberReference,
    OriginalSourceDefinitionMemberRegion, OriginalSourceDefinitionMemberScriptBody,
    OriginalSourceScopedBody, OriginalSourceScriptBody, OriginalSourceScriptPurpose,
};
pub(crate) use source_scoped_body::{
    OriginalSourceScriptBodyOrigin, declared_source_script_bodies_for,
    original_source_scoped_bodies,
};
mod source_taint;
pub use source_taint::HostedSourceTaintContext;
pub(crate) use source_taint::hosted_source_taint_invocation;
mod vendor_advice;
mod vendor_source_barriers;
pub use vendor_advice::{VendorRegistryInvocationShape, vendor_registry_invocation_shape};
pub use vendor_source_barriers::VendorSourceCatalogueBarriers;

mod variable_write_advice;
pub(crate) use variable_write_advice::{
    original_variable_write_advice, vendor_variable_write_advice,
};

mod declaration_assistance;
pub use declaration_assistance::{
    DeclarationArgument, DeclarationVariableAliasPurpose, OriginalDeclarationAssistance,
};
pub(crate) use declaration_assistance::{
    OriginalSourceVariableAliasOperands, original_declaration_assistance,
};

mod declaration_flow;
pub(crate) use declaration_flow::{
    DeclarationInvocationFlow, DeclarationLifecycleInvocation, declaration_dictionary_scope_advice,
    declaration_invocation_flow, declaration_lifecycle_invocation, declared_argument_words,
};

mod interval_advice;
pub(crate) use interval_advice::declaration_increment_advice;

mod native_compilation_source;
pub use native_compilation_source::original_native_compiler_words;
pub(crate) use native_compilation_source::{
    OriginalNativeCompilerInvocation, OriginalNativeCompilerPreparation,
    original_native_compilation,
};

mod store_advice;
pub(crate) use store_advice::{
    conditional_declared_overwrite_advice, conditional_overwritten_local_store_advice,
    conditional_unread_local_store_advice, overwritten_local_store_advice,
};

use tcl_dialect::EscapeSyntax;
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{
    CommandRegistry, InvocationFacts, InvocationResolutionUnresolved, InvocationWord,
    InvocationWordKind, InvocationWords,
};

use crate::ir::{CommandTokens, Statement, WordExpr, WordPart};
use crate::segmenter::SegmentedCommand;
use tcl_syntax::word_rules::WordValueRules;

/// Origin of a word in the post-binding argument vector. Inserted alias
/// arguments have values but have no editable range in the invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationWordOrigin {
    /// The canonical implementation head, derived from dispatch proof.
    ResolvedHead,
    /// An argument retained by an interpreter alias.
    BindingPrefix(usize),
    /// A word in the original invocation, including its head at index zero.
    Written(usize),
    /// One native list element of an original expanded word; no editable range.
    ExpandedElement {
        /// Original word index, including the head.
        written: usize,
        /// Element index in the retained native list.
        element: usize,
    },
}

/// One shared projection of proved dispatch and alias argument composition.
/// Document procedures retain identity here; registry selection separately
/// requires registry-backed implementation provenance.
/// The original token snapshot remains the runtime evaluation sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveCommandWords {
    /// Effective command head followed by effective arguments.
    pub words: Vec<WordExpr>,
    /// One origin for every effective word.
    pub origins: Vec<InvocationWordOrigin>,
    /// Frozen alias values, including unknown values with known argv cardinality.
    pub binding_prefix: Vec<EffectiveInvocationWord>,
}

/// Scope a structured projection of original contiguous written argument words.
/// Missing fragments, recovery, or changed cardinality leave no layout proof.
/// This lexical view supplies no handler identity or frozen evaluated value.
pub(crate) fn with_source_argument_words<T>(
    source: &str,
    arg_tokens: &[tcl_lexer::Token],
    config: tcl_lexer::LexerConfig,
    dialect: tcl_registry::InvocationDialect,
    apply: impl FnOnce(tcl_registry::InvocationArguments<'_>) -> T,
) -> Option<T> {
    let first = tcl_lexer::word_span_at(source, arg_tokens.first()?.span);
    let last = tcl_lexer::word_span_at(source, arg_tokens.last()?.span);
    let text = source.get(first.start() as usize..last.end() as usize)?;
    let mut segments =
        crate::segmenter::segment_commands_with_offset_and_config(text, first.start(), config)
            .into_iter();
    let segment = segments.next()?;
    if segments.next().is_some() || segment.argv.len() != arg_tokens.len() || segment.is_partial {
        return None;
    }
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        &segment,
    );
    let values: Vec<_> = tokens
        .word_exprs
        .iter()
        .map(|word| {
            crate::registry_invocation::effective_invocation_word(
                word,
                config.escapes,
                dialect.word_values,
            )
        })
        .collect();
    let words: Vec<_> = values
        .iter()
        .zip(&tokens.word_exprs)
        .map(|(effective, source)| {
            crate::registry_invocation::invocation_word_with_source(
                source,
                effective,
                config.escapes,
            )
        })
        .collect();
    Some(apply(
        tcl_registry::InvocationArguments::structured(&words).with_dialect(dialect),
    ))
}

/// Registry facts and arguments selected from one retained source-state proof.
/// Consumers use this projection together so role indices cannot drift from
/// alias composition or the original evaluation sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStatementInvocation {
    /// Interpreter policies retained independently of catalogue availability.
    pub dialect: Option<tcl_registry::InvocationDialect>,
    /// Live implementation's registry facts.
    pub facts: Box<InvocationFacts>,
    /// Effective post-head presentation slots. Written words retain their
    /// authored spellings; captured values without retained bytes remain None.
    /// Semantic consumers use frozen argument values and shapes instead.
    pub arguments: Vec<Option<String>>,
    /// Effective values and their written or inserted origins.
    pub effective: EffectiveCommandWords,
    /// Effective argument values frozen by the source owner at evaluation time.
    /// This never replaces the original words or their editable provenance.
    pub evaluated_arguments: Vec<Option<String>>,
    /// Frozen argv shape, including bounded array roots with unknown indices.
    pub evaluated_words: Vec<EffectiveInvocationWord>,
}

/// Structured semantics of one exact source invocation for analysis.
///
/// Conditional declarations describe their own frame only. This projection
/// grants no native compilation admission, executed activation, successful
/// store, caller identity or outward effect.
pub(crate) struct LogicalStructuredInvocation {
    invocation: ResolvedStatementInvocation,
}

impl LogicalStructuredInvocation {
    pub(crate) fn canonical_command(&self) -> &str {
        &self.invocation.facts.canonical_command
    }

    /// Borrow selected source descriptors under the genuine supplied generation.
    /// Original direct-word correspondence remains part of this issuer.
    pub(crate) fn with_metadata_schema<T>(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        realm: tcl_dialect::model::InvocationRealm,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        self.invocation
            .with_metadata_schema(registry, context, realm, apply)
    }

    pub(crate) fn lowering_hook(&self) -> Option<tcl_registry::hooks::LoweringHookId> {
        self.invocation.facts.lowering_hook
    }

    pub(crate) fn written_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        self.invocation.written_argument_roles()
    }

    pub(crate) fn traits(&self) -> tcl_registry::Traits {
        self.invocation.facts.traits
    }

    pub(crate) fn body_scope<'a>(
        &self,
        registry: &CommandRegistry,
        context: impl Into<InvocationMetadataContext<'a>>,
    ) -> Option<&'static tcl_registry::scoped::ScopedCommandEnv> {
        let context = context.into();
        if !context.matches_registry(registry) {
            return None;
        }
        context
            .context()
            .resolve_spec(registry, self.canonical_command())?
            .body_scope
    }

    /// Abstract routing inside this original conditional body. This cannot
    /// certify an actual completion, handler effect or native instruction.
    pub(crate) fn conditional_completion_route(
        &self,
        registry: &CommandRegistry,
    ) -> tcl_registry::completion_route::InvocationCompletionRoute {
        self.invocation.completion_route(registry)
    }

    pub(crate) fn error_context(&self) -> Option<tcl_registry::InlineBodyErrorContext> {
        self.invocation.facts.operation.inline_body_error_context()
    }
}

/// Resolve logical structure through the selected handler and original words.
/// Native compilation and runtime execution consume their separate receipts.
pub(crate) fn logical_structured_invocation(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    bindings: Option<&crate::command_binding::SourceCommandBindings>,
) -> Option<LogicalStructuredInvocation> {
    let context = body_assistance_context(registry, tokens)?;
    logical_structured_invocation_with_metadata_context(registry, context.into(), tokens, bindings)
}

/// Select source structure using the caller's complete immutable context.
/// Missing or foreign context refuses; this supplies no executed body/frame.
pub(crate) fn logical_structured_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    bindings: Option<&crate::command_binding::SourceCommandBindings>,
) -> Option<LogicalStructuredInvocation> {
    if !context.matches_registry(registry) {
        return None;
    }
    if bindings.is_some_and(|bindings| bindings.original_arguments_rejected_before_handler(tokens))
    {
        return None;
    }
    let selected =
        resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens)
            .or_else(|| {
                owned_body_layout_invocation(registry, context, tokens, bindings?)
                    .map(|owned| owned.invocation)
            })
            .or_else(|| original_declared_structured_invocation(registry, context, tokens))?;
    // Source-shape lowering requires each operand to retain its original slot.
    // Alias prefixes, selector rewriting and expanded elements use generic IR
    // until a lowerer explicitly accepts their composed operand projection.
    if !selected.facts.arg_roles_complete
        || selected.facts.arity_accepts_frozen_arguments() != Some(true)
        || selected.effective.words.len() != tokens.words().len()
        || !selected
            .effective
            .origins
            .iter()
            .enumerate()
            .skip(1)
            .all(|(index, origin)| *origin == InvocationWordOrigin::Written(index))
    {
        return None;
    }
    if selected.facts.successful_handler
        == Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::ExpressionArguments)
        && tokens
            .source_binding
            .as_ref()?
            .conditional_expression_evaluation(registry, tokens)
            .is_none_or(|expression| !expression.semantic_lookup_closed())
    {
        return None;
    }
    Some(LogicalStructuredInvocation {
        invocation: selected,
    })
}

/// Conditional structure of one unchanged original source invocation.
/// The shared issuer requires closed original lookup, complete roles, accepted
/// arity and direct written operand correspondence. It can describe an
/// unentered declaration body; it grants no Normal completion, native compiler
/// admission, physical frame or executed store.
#[must_use]
pub fn original_structured_invocation(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    logical_structured_invocation(registry, tokens, None).map(|selected| selected.invocation)
}

/// Select unchanged original source structure under the supplied metadata.
/// Complete source consumers validate their input with
/// [`InvocationMetadataContext::for_source_input`]. Missing ownership refuses;
/// direct operand correspondence grants no entered frame or Normal completion.
#[must_use]
pub fn original_structured_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    logical_structured_invocation_with_metadata_context(registry, context, tokens, None)
        .map(|selected| selected.invocation)
}

/// Original declaration grammar, without a completed handler or CPP receipt.
fn original_declared_structured_invocation<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    if let Some(input) = context.source_analysis_input()
        && binding
            .original_lexer_config_for_tokens(tokens)?
            .normalized()
            != input.lexer_config().normalized()
    {
        return None;
    }
    let logical_source = context.permits_logical_source_names();
    let advice = binding
        .declaration_operand_layout_advice(tokens)
        .or_else(|| {
            logical_source
                .then(|| binding.original_compilation_lookup_advice(tokens))
                .flatten()
        })?;
    // This is conditional source structure. A positively retained Logical
    // command world needs no physical variable frame; Native lookup keeps its
    // independent frame gate and cannot borrow that source-only closure.
    let closed = if logical_source {
        advice.closed_logical_source_lookup()
    } else {
        advice.closed_lookup()
    };
    if !closed {
        return None;
    }
    resolve_original_declared_layout(registry, context, tokens, &advice)
}

/// Conditional option topology of the retained original invocation. Captured
/// operands keep their effective ordinals, with no handler or Normal grant.
pub(crate) fn conditional_option_arguments_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<(
    ResolvedStatementInvocation,
    tcl_registry::AuthoredSourceOptionArguments,
)> {
    if !context.matches_registry(registry) || tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    if let Some(input) = context.source_analysis_input()
        && binding
            .original_lexer_config_for_tokens(tokens)?
            .normalized()
            != input.lexer_config().normalized()
    {
        return None;
    }
    let invocation =
        resolved_tokens_invocation_with_metadata_context(registry, Some(context), tokens)
            .or_else(|| {
                resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens)
            })
            .or_else(|| original_declared_structured_invocation(registry, context, tokens))?;
    let realm = binding.invocation_realm().or_else(|| {
        binding
            .declaration_operand_layout_advice(tokens)
            .map(|advice| advice.realm())
    })?;
    let options = invocation.with_metadata_schema(registry, context, realm, |schema| {
        schema.authored_source_possible_option_arguments()
    })?;
    Some((invocation, options))
}

/// Conditional operation recipe for the exact supplied Logical source model.
/// Original alias captures and source words remain composed by the shared
/// declaration owner. This grants no Native handler, Normal completion,
/// compiler entry, variable allocation or executable erasure permission.
pub(crate) fn original_logical_operation_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    if !context.matches_registry(registry)
        || !context.permits_logical_source_names()
        || tokens.synthetic.is_some()
    {
        return None;
    }
    let input = context.source_analysis_input()?;
    let binding = tokens.source_binding.as_ref()?;
    if binding.logical_source_name_advice_input() != Some(input)
        || binding
            .original_lexer_config_for_tokens(tokens)?
            .normalized()
            != input.lexer_config().normalized()
    {
        return None;
    }
    resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens)
        .or_else(|| original_declared_structured_invocation(registry, context, tokens))
}

/// Purity of the selected original substitution implementation. Native
/// dispatch requires a normal handler and closed execution alternatives;
/// the separate Logical branch retains its exact source operation recipe.
/// Neither branch proves operand evaluation, no-raise, observers or erasure.
pub(crate) fn original_substitution_purity_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<bool> {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    if !context.matches_registry(registry) || tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let input = context.source_analysis_input()?;
    if config.normalized() != input.lexer_config().normalized() {
        return None;
    }
    let invocation = if context.permits_logical_source_names() {
        original_logical_operation_invocation_with_metadata_context(registry, context, tokens)?
    } else {
        if binding.execution_is_unknown() || binding.execution_may_be_absent() {
            return None;
        }
        resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens)?
    };
    if invocation.facts.arity_accepts_frozen_arguments() != Some(true)
        || matches!(
            invocation.facts.subcommand,
            tcl_registry::OwnedSubcommandResolution::Unknown { .. }
                | tcl_registry::OwnedSubcommandResolution::Ambiguous { .. }
                | tcl_registry::OwnedSubcommandResolution::Indeterminate { .. }
        )
    {
        return None;
    }
    Some(
        !invocation.facts.mutator
            && invocation
                .facts
                .traits
                .intersects(tcl_registry::Traits::PURE | tcl_registry::Traits::PURE_EVALUATION),
    )
}

/// The source-role consumer chooses its purpose-specific closure before this
/// shared full-vector/schema resolution. This helper supplies no completion.
fn resolve_original_declared_layout<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
    advice: &crate::command_binding::OriginalCompilationLookupAdvice,
) -> Option<ResolvedStatementInvocation> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }
    let dialect = advice.dialect();
    let mut unanimous = None;
    for target in advice.targets() {
        if !target.registry_backed {
            return None;
        }
        let effective =
            compose_original_effective_words(tokens, &target.command, &target.prepended)?;
        let values: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm_with_metadata_context(
                registry,
                Some(context),
                &words,
                Some(dialect),
                advice.realm(),
            )
            .ok()?
        else {
            return None;
        };
        let evaluated_words = values.get(1..)?.to_vec();
        let selected = ResolvedStatementInvocation {
            dialect: Some(dialect),
            facts,
            arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
            evaluated_arguments: evaluated_words
                .iter()
                .map(|word| match word {
                    EffectiveInvocationWord::Literal(value) => Some(value.clone()),
                    _ => None,
                })
                .collect(),
            evaluated_words,
            effective,
        };
        if unanimous
            .as_ref()
            .is_some_and(|previous| previous != &selected)
        {
            return None;
        }
        unanimous = Some(selected);
    }
    unanimous
}

/// Resolve a retained invocation through the shared point owner. Synthetic
/// analysis boundaries contribute their typed effects rather than invoking a
/// command, and explicitly unproved dispatch never uses a written-head query.
#[must_use]
pub fn resolved_statement_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    statement: &crate::ir::Statement,
) -> Option<ResolvedStatementInvocation> {
    resolved_statement_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        statement,
    )
}

pub(crate) fn resolved_statement_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    statement: &crate::ir::Statement,
) -> Option<ResolvedStatementInvocation> {
    resolved_tokens_invocation_with_metadata_context(registry, context, statement.tokens()?)
}

/// Resolve an original statement under the retained availability generation.
/// Synthetic boundaries and statements without genuine invocation tokens do
/// not acquire a command operation through this metadata context.
#[must_use]
pub fn resolved_statement_invocation_in_context(
    context: &tcl_registry::model::ContextRegistry,
    statement: &crate::ir::Statement,
) -> Option<ResolvedStatementInvocation> {
    resolved_tokens_invocation_in_context(context, statement.tokens()?)
}

/// Resolve a complete retained invocation, including a nested substitution,
/// without reconstructing a statement or changing its source provenance.
#[must_use]
pub fn resolved_tokens_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    resolved_tokens_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Resolve the retained invocation under the supplied availability generation.
/// Command identity, native implementation and argument completion remain
/// independently required; this context supplies none of their missing proof.
#[must_use]
pub fn resolved_tokens_invocation_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    resolved_tokens_invocation_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

/// Select retained invocation facts under the actual supplied metadata owner.
/// Independent command, native handler and argument receipts remain required.
#[must_use]
pub fn resolved_tokens_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    if tokens.synthetic.is_some() {
        return None;
    }
    let RegistryInvocationResolution::Resolved(facts) =
        resolve_command_tokens_with_metadata_context(registry, context, tokens).ok()?
    else {
        return None;
    };
    let effective = effective_command_words(tokens)?;
    let arguments = effective.argument_presentations(tokens.argv_texts.get(1..)?)?;
    let dialect = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.variable_context.invocation_dialect)
        .or_else(|| {
            context
                .and_then(|context| context.context().environment.point())
                .map(tcl_registry::InvocationDialect::of_point)
        })
        .or_else(|| {
            registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile)
        });
    Some(ResolvedStatementInvocation {
        dialect,
        facts,
        arguments,
        evaluated_arguments: frozen_argument_values(tokens, &effective),
        evaluated_words: frozen_argument_words(tokens, &effective),
        effective,
    })
}

/// Compiler metadata of an admitted recipe; no normal effects are exposed.
#[derive(Debug)]
pub struct AdmittedNativeCompilerInvocation {
    invocation: ResolvedStatementInvocation,
    compiler_dialect: tcl_registry::InvocationDialect,
    hooks: tcl_registry::registry::NativeCompilerHooks,
    target: crate::command_binding::SourceCommandTarget,
    selection: tcl_registry::native_compilation::NativeCompilationSelection,
    hook_operand_prefix: &'static [&'static str],
}

impl AdmittedNativeCompilerInvocation {
    /// Inert logical member selectors needed by a shared backend hook.
    /// Original operands follow in their original order and retain their source.
    #[must_use]
    pub fn hook_operand_prefix(&self) -> &'static [&'static str] {
        self.hook_operand_prefix
    }
    /// Actual original compiler registration, with its slot/token identity.
    #[must_use]
    pub fn target(&self) -> &crate::command_binding::SourceCommandTarget {
        &self.target
    }
    /// Actual registration name, independent of logical ensemble metadata.
    #[must_use]
    pub fn canonical_registration_name(&self) -> &str {
        &self.target.command
    }
    /// Original effective words and their written/prefix origins.
    #[must_use]
    pub fn effective_words(&self) -> &EffectiveCommandWords {
        &self.invocation.effective
    }
    /// Number of original operands before the selected native member layout.
    #[must_use]
    pub fn argument_offset(&self) -> usize {
        self.invocation.facts.argument_offset
    }
    /// Registry-authored native compiler grammar.
    #[must_use]
    pub fn native_compilation(
        &self,
    ) -> Option<tcl_registry::native_compilation::NativeCompilationSpec> {
        self.invocation.facts.native_compilation
    }
    /// Compiler-owned error annotation for an inline body operand.
    #[must_use]
    pub fn compiler_operand_error_context(&self) -> Option<tcl_registry::InlineBodyErrorContext> {
        self.invocation.facts.operation.inline_body_error_context()
    }
    /// Logical registry command used only for compiler metadata selection.
    #[must_use]
    pub fn canonical_logical_command(&self) -> &str {
        &self.invocation.facts.canonical_command
    }
    /// Statement emitter metadata; admission remains an independent premise.
    #[must_use]
    pub fn codegen_hook(&self) -> Option<tcl_registry::hooks::CodegenHookId> {
        self.hooks.codegen
    }
    /// Value emitter metadata; admission remains an independent premise.
    #[must_use]
    pub fn inline_codegen_hook(&self) -> Option<tcl_registry::hooks::InlineCodegenHookId> {
        self.hooks.inline
    }
    /// Structured lowering metadata; no successful-handler facts accompany it.
    #[must_use]
    pub fn lowering_hook(&self) -> Option<tcl_registry::hooks::LoweringHookId> {
        self.hooks.lowering
    }
    /// Decode original source words under their retained logical value rules,
    /// then query compiler recipes under the independent physical engine.
    pub fn with_argument_words<T>(&self, apply: impl FnOnce(InvocationWords<'_>) -> T) -> T {
        let dialect = self.invocation.dialect;
        let effective = self
            .invocation
            .effective
            .words
            .iter()
            .map(|word| {
                dialect.map_or(EffectiveInvocationWord::Dynamic, |dialect| {
                    effective_invocation_word(
                        word,
                        dialect.lexer_grammar.escapes,
                        dialect.word_values,
                    )
                })
            })
            .collect::<Vec<_>>();
        let values = effective
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect::<Vec<_>>();
        apply(
            InvocationWords::structured(values[0], &values[1..])
                .with_dialect(self.compiler_dialect),
        )
    }
    /// Exact compiler environment of an original selected body operand.
    #[must_use]
    pub fn body_context(
        &self,
        enclosing: tcl_registry::native_compilation::NativeCompilationContext,
        operand: usize,
    ) -> Option<tcl_registry::native_compilation::NativeCompilationContext> {
        let spec = self.native_compilation()?;
        let word = self.effective_words().words.get(operand.checked_add(1)?)?;
        Some(self.with_argument_words(|words| {
            spec.body_context_for_invocation_operand(
                enclosing,
                self.selection,
                native_compilation_word_shape(word),
                words.arguments(),
                operand,
            )
        }))
    }
}

/// Resolve only an admitted original compiler target, even if its later guard
/// or runtime handler is uncertain. This carrier cannot license normal transfer.
#[must_use]
pub fn admitted_native_compiler_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<AdmittedNativeCompilerInvocation> {
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let target = binding
        .admitted_inline_invocation()
        .map(|proof| &proof.target)
        .or_else(|| {
            binding
                .admitted_named_invocation()?
                .dependencies
                .first()
                .map(|dependency| &dependency.target)
        })?
        .clone();
    if !target.registry_backed {
        return None;
    }
    let effective = compose_original_effective_words(tokens, &target.command, &target.prepended)?;
    let dialect = binding.native_compiler_dialect()?;
    let word_dialect = binding.compiler_word_dialect()?;
    let values = effective
        .words
        .iter()
        .map(|word| {
            effective_invocation_word(
                word,
                word_dialect.lexer_grammar.escapes,
                word_dialect.word_values,
            )
        })
        .collect::<Vec<_>>();
    let words = values
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect::<Vec<_>>();
    let RegistryInvocationResolution::Resolved(facts) = resolve_registry_words_in_realm(
        registry,
        context,
        &words,
        Some(dialect),
        binding.compiler_invocation_realm()?,
    )
    .ok()?
    else {
        return None;
    };
    let hooks = registry.native_compiler_hooks(
        InvocationWords::structured(words[0], &words[1..]).with_dialect(dialect),
    )?;
    let hook_operand_prefix = registry
        .native_registration_operand_prefix(&target.command, dialect)
        .unwrap_or(&[]);
    Some(AdmittedNativeCompilerInvocation {
        compiler_dialect: dialect,
        invocation: ResolvedStatementInvocation {
            dialect: Some(word_dialect),
            arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
            evaluated_arguments: frozen_argument_values(tokens, &effective),
            evaluated_words: frozen_argument_words(tokens, &effective),
            effective,
            facts,
        },
        hooks,
        target,
        selection: binding.native_compilation_admission_selection(),
        hook_operand_prefix,
    })
}

/// A reached handler's normal variable transfer, independent of compiler-hook
/// selection. This carrier cannot license an opcode, body evaluation or command
/// completion: its registry facts remain private to the variable transfer owner.
#[derive(Debug)]
pub struct NormalTransferInvocation {
    invocation: ResolvedStatementInvocation,
    compilation: tcl_registry::native_compilation::NativeCompilationSelection,
    alias_frame: tcl_registry::VariableAliasFrame,
    captured_outputs: bool,
    original_operands: Option<std::sync::Arc<crate::variable_bindings::OriginalVariableInvocation>>,
}

/// Element evolution of one proved normal, unobserved container store.
pub struct NormalContainerElementWrite<'a> {
    /// Authored element evolution on the normal store continuation.
    pub effect: tcl_registry::VarElementsEffect,
    /// Actual literal variable operand bound to the proved unobserved cell.
    pub target: String,
    /// Effective argv offset introduced by a selected subcommand.
    pub argument_offset: usize,
    /// Retained operand presentation for source-aware value typing. This is
    /// not evidence of evaluated key bytes.
    pub arguments: Vec<String>,
    frozen_dictionary_keys: Option<Vec<String>>,
    /// Effective source words for representation-aware value typing.
    pub words: &'a [WordExpr],
}

impl NormalContainerElementWrite<'_> {
    /// Actual dictionary update key path frozen after argument evaluation.
    /// Unknown evaluated keys cannot borrow their source presentation.
    #[must_use]
    pub fn dictionary_keys(&self) -> Option<&[String]> {
        (self.effect == tcl_registry::VarElementsEffect::SetsDictValue)
            .then_some(self.frozen_dictionary_keys.as_deref())
            .flatten()
    }
}

impl NormalTransferInvocation {
    /// Alias declarations active in the proved frame on the normal continuation.
    /// Native no-op declarations and unproved activation policies are excluded.
    /// This exposes no compiler selection, body execution, or completion facts.
    pub fn variable_alias_transitions(
        &self,
    ) -> impl Iterator<Item = &tcl_registry::state_transition::VariableCellAliasTransition> {
        self.invocation
            .facts
            .state_transitions
            .declared()
            .into_iter()
            .flat_map(tcl_registry::StateTransitions::facts)
            .filter_map(|fact| match &fact.transition {
                tcl_registry::state_transition::StateTransition::VariableCellAlias(alias)
                    if !self.captured_outputs
                        && self.invocation.dialect.and_then(|dialect| {
                            alias
                                .destination
                                .is_active_in_frame(self.alias_frame, dialect)
                        }) == Some(true) =>
                {
                    Some(alias)
                }
                _ => None,
            })
    }

    /// Decode an alias frame selector under this invocation's retained policies.
    /// Missing selector values or execution dialects retain unknown selection.
    #[must_use]
    pub fn variable_alias_frame_level(
        &self,
        frame: &tcl_registry::CallerFrameSelection,
    ) -> Option<tcl_registry::FrameLevel> {
        match frame {
            tcl_registry::CallerFrameSelection::DefaultCaller => {
                Some(tcl_registry::FrameLevel::DEFAULT)
            }
            tcl_registry::CallerFrameSelection::Explicit(level) => {
                tcl_registry::FrameLevel::parse_for_dialect(
                    level.literal()?,
                    self.invocation.dialect?,
                )
            }
        }
    }

    /// Variable trace-table transitions on the reached normal registration path.
    /// Captured output boundaries cannot register or remove an observer.
    pub fn variable_trace_transitions(
        &self,
    ) -> impl Iterator<Item = &tcl_registry::TraceTransition> {
        self.invocation
            .facts
            .state_transitions
            .declared()
            .into_iter()
            .flat_map(tcl_registry::StateTransitions::facts)
            .filter_map(|fact| match &fact.transition {
                tcl_registry::StateTransition::Trace(trace)
                    if !self.captured_outputs
                        && matches!(
                            trace,
                            tcl_registry::TraceTransition::Add {
                                target: tcl_registry::TraceTarget::Variable(_),
                                ..
                            } | tcl_registry::TraceTransition::Remove {
                                target: tcl_registry::TraceTarget::Variable(_),
                                ..
                            }
                        ) =>
                {
                    Some(trace)
                }
                _ => None,
            })
    }

    /// Select container value effects only after proving the actual store cell.
    /// This projection carries no compiler, callback-body or completion licence.
    #[must_use]
    pub fn container_element_write(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<NormalContainerElementWrite<'_>> {
        let effect = self.invocation.facts.var_elements_effect?;
        let places = self.mutation_places(state, registry);
        let [place] = places.as_slice() else {
            return None;
        };
        if place.observed || place.dynamic || place.kind == crate::place::PlaceKind::Unknown {
            return None;
        }
        let argument_offset = self.invocation.facts.argument_offset;
        Some(NormalContainerElementWrite {
            effect,
            target: self.invocation.argument_literal(argument_offset)?,
            argument_offset,
            arguments: self
                .invocation
                .arguments
                .iter()
                .cloned()
                .collect::<Option<Vec<_>>>()?,
            frozen_dictionary_keys: (argument_offset + 1
                ..self.invocation.arguments.len().checked_sub(1)?)
                .map(|index| self.invocation.argument_literal(index))
                .collect(),
            words: self.invocation.effective.words.get(1..)?,
        })
    }

    /// Select variable addresses at the phase authored by the reached handler.
    #[must_use]
    pub fn variable_binding_phase(
        &self,
    ) -> tcl_registry::native_compilation::VariableOperandBindingPhase {
        self.invocation.facts.successful_handler.map_or(
            tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments,
            tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_binding_phase,
        )
    }

    /// Authored output-address lookup protocol, without exposing compiler facts.
    #[must_use]
    pub fn variable_output_lookup(
        &self,
    ) -> Option<tcl_registry::native_compilation::VariableOutputLookup> {
        self.invocation
            .facts
            .successful_handler
            .map(tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_output_lookup)
    }

    /// Variable output indices in their observable native write order.
    /// Missing ordering evidence remains unknown rather than choosing an order.
    #[must_use]
    pub fn variable_output_arguments(&self) -> Option<Vec<usize>> {
        use tcl_registry::native_compilation::VariableOutputLookup;
        let writes = || {
            self.variable_roles()
                .into_iter()
                .filter_map(|(index, role)| {
                    (role == tcl_registry::ArgRole::VarWrite).then_some(index)
                })
                .collect::<Vec<_>>()
        };
        match self.variable_output_lookup() {
            Some(VariableOutputLookup::CatchSelected) => self.with_arguments(|arguments| {
                let dialect = self.invocation.dialect?;
                let tcl_registry::catch_invocation::CatchInvocationSelection::Valid(capture) =
                    tcl_registry::catch_invocation::select_catch_invocation(arguments, dialect)
                else {
                    return None;
                };
                Some(
                    capture
                        .output_order(dialect, self.compilation)
                        .indices(capture)?
                        .into_iter()
                        .flatten()
                        .collect(),
                )
            }),
            Some(VariableOutputLookup::Sequential | VariableOutputLookup::SingleTarget) => {
                Some(writes())
            }
            Some(VariableOutputLookup::BodyProtocol) => None,
            None => {
                let writes = writes();
                (writes.len() <= 1).then_some(writes)
            }
        }
    }

    /// Locate a setter's retained value operand on the normal handler path.
    /// An operand dependency cannot select a compiler operation or opcode.
    #[must_use]
    pub fn stored_value_argument(&self) -> Option<usize> {
        (self.invocation.facts.operation
            == tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Set,
            )
            && self.argument_count() == 2)
            .then_some(1)
    }

    /// Selected normal numeric store protocol, without a concrete value or
    /// permission to omit its operand conversion or observer schedule.
    #[must_use]
    pub fn numeric_store_production(
        &self,
    ) -> Option<tcl_registry::native_result::NativeNumericStoreProduction> {
        let contract = self.invocation.facts.native_result?;
        self.with_arguments(|arguments| {
            contract
                .normal_numeric_store_production(arguments, self.invocation.facts.argument_offset)
        })
    }

    /// Variable-access traits; these do not select script bodies or compile hooks.
    #[must_use]
    pub fn variable_traits(&self) -> tcl_registry::Traits {
        self.invocation.facts.traits.intersection(
            tcl_registry::Traits::READS_BEFORE_WRITE
                | tcl_registry::Traits::DESTROYS_VARIABLE
                | tcl_registry::Traits::CONDITIONAL_VARIABLE_WRITE
                | tcl_registry::Traits::WHOLE_ARRAY_ARG,
        )
    }

    /// Locate an original value word without reconstructing frozen argv.
    #[must_use]
    pub fn written_argument(&self, argument: usize) -> Option<usize> {
        self.invocation.effective.written_argument(argument)
    }

    /// Number of effective arguments on this reached handler continuation.
    #[must_use]
    pub fn argument_count(&self) -> usize {
        self.invocation.arguments.len()
    }

    /// Named contents reads on this handler phase, independently of alias registration.
    #[must_use]
    pub fn read_places(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<crate::place::Place> {
        if self.captured_outputs
            || self.variable_binding_phase()
                == tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol
        {
            return Vec::new();
        }
        self.with_arguments(|arguments| {
            if state.execution_name_policy.is_some() {
                return self.original_operands.as_deref().map_or_else(
                    || vec![crate::place::unknown_top()],
                    |operands| crate::variable_bindings::source_variable_read_places_with_original_operands(
                        &self.invocation.facts, arguments, state, registry, operands,
                    ),
                );
            }
            crate::variable_bindings::source_variable_read_places(
                &self.invocation.facts, arguments, state, registry,
            )
        })
    }

    /// Bound contents changes, including destruction; no runtime dispatch licence.
    #[must_use]
    pub fn mutation_places(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<crate::place::Place> {
        if self.captured_outputs {
            return self.definition_places(state, registry);
        }
        self.with_arguments(|arguments| {
            if state.execution_name_policy.is_some() {
                return self.original_operands.as_deref().map_or_else(
                    || vec![crate::place::unknown_top()],
                    |operands| crate::variable_bindings::source_variable_write_places_with_output_order_and_original_operands(
                        &self.invocation.facts, arguments, state, registry,
                        self.variable_output_arguments().as_deref(), operands,
                    ),
                );
            }
            crate::variable_bindings::source_variable_write_places_with_output_order(
                &self.invocation.facts, arguments, state, registry,
                self.variable_output_arguments().as_deref(),
            )
        })
    }

    /// Bound SSA value definitions on the successful continuation.
    #[must_use]
    pub fn definition_places(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<crate::place::Place> {
        if self.variable_binding_phase()
            == tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol
        {
            return Vec::new();
        }
        self.definition_operands(state, registry)
            .into_iter()
            .map(|(_, place)| place)
            .collect()
    }

    /// Original written output arguments paired with their selected normal
    /// definition places. Inserted and expanded operands have no written slot.
    /// Missing address/order evidence remains unknown in the returned place.
    #[must_use]
    pub fn written_definition_places(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<(usize, crate::place::Place)> {
        if self.variable_binding_phase()
            == tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol
        {
            return Vec::new();
        }
        self.definition_operands(state, registry)
            .into_iter()
            .filter_map(|(effective, place)| Some((self.written_argument(effective)?, place)))
            .collect()
    }

    fn definition_operands(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<(usize, crate::place::Place)> {
        if state.execution_name_policy.is_some() {
            let Some(operands) = self
                .original_operands
                .as_deref()
                .filter(|operands| operands.argument_count() == self.argument_count())
            else {
                return Vec::new();
            };
            return self.with_arguments(|arguments| {
                crate::variable_bindings::source_variable_definitions_with_original_operands(
                    &self.invocation.facts,
                    arguments,
                    state,
                    registry,
                    self.variable_output_arguments().as_deref(),
                    operands,
                )
            });
        }
        crate::place_bridge::resolved_invocation_variable_definitions(
            &self.invocation,
            state,
            registry,
            self.variable_output_arguments().as_deref(),
        )
    }

    /// Apply only the variable owner's normal continuation effects.
    pub fn transfer_variables(
        &self,
        state: &mut crate::var_resolve::ResolveContext,
        statement: &crate::ir::Statement,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) {
        if self.captured_outputs {
            let outputs = self.definition_places(state, registry);
            crate::variable_bindings::transfer_captured_variable_outputs(
                state,
                &outputs,
                statement.span().start(),
            );
            return;
        }
        self.with_arguments(|arguments| {
            crate::variable_bindings::transfer_resolved_invocation(
                state,
                statement,
                tokens,
                &self.invocation.facts,
                arguments,
                registry,
                (
                    self.variable_output_arguments().as_deref(),
                    self.original_operands.as_deref(),
                ),
            );
        });
    }

    /// A passthrough store value only when the selected cell has no write
    /// observer and its physical store is bounded. This says nothing about
    /// executing the invocation or eliminating its possible compiler error.
    ///
    /// `state` must describe the actual address-selection phase returned by
    /// `variable_binding_phase`, including all preceding argv/body effects.
    /// A pre-argv or freshly reconstructed context cannot establish this store.
    #[must_use]
    pub fn stored_value_operand(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<(&str, &WordExpr)> {
        let word = self.stored_value_word(state, registry)?;
        Some((self.invocation.arguments.get(1)?.as_deref()?, word))
    }

    /// Original stored word after proving an unobserved bounded normal store.
    /// Unknown evaluated bytes retain their structured source and read identity.
    #[must_use]
    pub fn stored_value_word(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<&WordExpr> {
        self.stored_value_argument()?;
        let places = self.mutation_places(state, registry);
        let [place] = places.as_slice() else {
            return None;
        };
        if place.observed || place.dynamic || place.kind == crate::place::PlaceKind::Unknown {
            return None;
        }
        self.invocation.effective.words.get(2)
    }

    /// The value frozen before later argv substitutions, after proving the
    /// same observer-free physical store as `stored_value_operand`. Its actual
    /// address-selection context precondition applies here as well.
    #[must_use]
    pub fn stored_value_literal(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<String> {
        self.stored_value_operand(state, registry)?;
        self.invocation.argument_literal(1)
    }

    /// Named variable operands, limited to storage access roles.
    #[must_use]
    pub fn variable_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        self.invocation
            .facts
            .arg_roles
            .iter()
            .filter_map(|&(index, role)| {
                matches!(
                    role,
                    tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
                )
                .then_some((
                    self.invocation.facts.argument_offset + usize::from(index),
                    role,
                ))
            })
            .collect()
    }

    /// Select one named storage operand from this retained normal invocation.
    /// Original execution uses its exact byte/compiler descriptor; the label
    /// returned by `argument_literal` is never an address producer.
    pub(crate) fn variable_operand_place(
        &self,
        argument: usize,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<crate::place::Place> {
        let role = self
            .variable_roles()
            .into_iter()
            .find_map(|(index, role)| (index == argument).then_some(role))?;
        self.with_arguments(|arguments| {
            if role == tcl_registry::ArgRole::VarWrite {
                return if state.execution_name_policy.is_some() {
                    self.original_operands.as_deref().map(|operands| {
                        crate::variable_bindings::variable_output_operand_access_with_original_operands(
                            &self.invocation.facts, arguments, argument, state, registry, operands,
                        )
                    })
                } else {
                    Some(crate::variable_bindings::variable_output_operand_access(
                        &self.invocation.facts, arguments, argument, state, registry,
                    ))
                };
            }
            if state.execution_name_policy.is_some() {
                return self.original_operands.as_deref()
                    .filter(|operands| operands.argument_count() == arguments.len())
                    .map(|operands| operands.access(argument, state, registry,
                        tcl_registry::TraceOperation::Read,
                        self.variable_traits().contains(tcl_registry::Traits::WHOLE_ARRAY_ARG)));
            }
            Some(crate::variable_bindings::variable_operand_access(
                arguments, argument, state, registry, tcl_registry::TraceOperation::Read,
                self.variable_traits().contains(tcl_registry::Traits::WHOLE_ARRAY_ARG),
            ))
        })
    }

    /// One variable operand's frozen name shape. A known array root remains
    /// bounded even when its element index is computed; source spelling and
    /// mathematical contents facts cannot replace this actual argv projection.
    #[must_use]
    pub fn variable_operand_word(&self, index: usize) -> Option<EffectiveInvocationWord> {
        self.variable_roles()
            .iter()
            .any(|&(variable_index, _)| variable_index == index)
            .then(|| self.invocation.argument_word(index))
    }

    /// A frozen variable/value operand on this normal transfer boundary.
    #[must_use]
    pub fn argument_literal(&self, index: usize) -> Option<String> {
        self.invocation.argument_literal(index)
    }

    /// Source names receiving contents on the successful continuation.
    #[must_use]
    pub fn definition_names(&self) -> Vec<String> {
        self.variable_roles()
            .into_iter()
            .filter_map(|(index, role)| {
                if role != tcl_registry::ArgRole::VarWrite {
                    return None;
                }
                let name = self.argument_literal(index)?;
                crate::variable_bindings::contents_write_operand(
                    &self.invocation.facts,
                    Some(&name),
                )
                .then_some(name)
            })
            .collect()
    }

    fn with_arguments<T>(
        &self,
        apply: impl FnOnce(tcl_registry::InvocationArguments<'_>) -> T,
    ) -> T {
        let values: Vec<_> = (0..self.argument_count())
            .map(|index| self.invocation.argument_word(index))
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let mut arguments = tcl_registry::InvocationArguments::structured(&words);
        if let Some(dialect) = self.invocation.dialect {
            arguments = arguments.with_dialect(dialect);
        }
        apply(arguments)
    }
}

/// Resolve a normal handler transfer. Strict executable resolution remains the
/// first choice. An unknown compiler protocol additionally needs an exact stable
/// handler and an authored normal-effect equivalence for its frozen argv/frame.
#[must_use]
pub fn normal_transfer_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<NormalTransferInvocation> {
    normal_transfer_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Resolve retained normal transfer metadata under the complete supplied availability generation.
/// Existing original identity, argument and handler premises remain required.
#[must_use]
pub fn normal_transfer_invocation_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<NormalTransferInvocation> {
    normal_transfer_invocation_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

/// Select successful transfer metadata with the complete supplied source owner.
/// Missing or foreign input cannot reconstruct a standalone catalogue context.
#[must_use]
pub fn normal_transfer_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<NormalTransferInvocation> {
    if tokens.synthetic == Some(crate::ir::SyntheticMarker::CapturedCatchOutputs) {
        let mut original = tokens.clone();
        original.synthetic = None;
        let mut normal =
            normal_transfer_invocation_with_metadata_context(registry, context, &original)?;
        normal.captured_outputs = true;
        return Some(normal);
    }
    if let Some(invocation) =
        resolved_tokens_invocation_with_metadata_context(registry, context, tokens)
    {
        return Some(NormalTransferInvocation {
            invocation,
            compilation: normal_output_compilation(tokens),
            alias_frame: normal_alias_frame(tokens),
            captured_outputs: false,
            original_operands: tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.original_variable_operands_for_tokens(tokens))
                .cloned(),
        });
    }
    let binding = tokens.source_binding.as_ref()?;
    let invocation = resolved_handler_invocation_with_metadata_context(registry, context, tokens)?;
    let normal = NormalTransferInvocation {
        invocation,
        compilation: normal_output_compilation(tokens),
        alias_frame: normal_alias_frame(tokens),
        captured_outputs: false,
        original_operands: binding
            .original_variable_operands_for_tokens(tokens)
            .cloned(),
    };
    normal.with_arguments(|arguments| {
        normal
            .invocation
            .facts
            .successful_handler_effects(arguments, binding.variable_context.alias_frame())
    })?;
    Some(normal)
}

/// Namespace import/export directives of one converged handler and frozen argv.
/// This metadata supplies neither a physical mutation nor an execution,
/// compiler, or successful-import proof.
#[derive(Debug)]
pub struct NamespaceDirectiveFootprint {
    transitions: Vec<tcl_registry::NamespaceTransition>,
}

impl NamespaceDirectiveFootprint {
    /// Authored directives, retaining unknown operand values and namespace axes.
    pub fn transitions(&self) -> impl Iterator<Item = &tcl_registry::NamespaceTransition> {
        self.transitions.iter()
    }
}

/// Recover directive metadata independently of native opcode eligibility.
/// A replaced or uncertain handler cannot donate its written command grammar.
#[must_use]
pub fn namespace_directive_footprint(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<NamespaceDirectiveFootprint> {
    namespace_directive_footprint_for_invocation(resolved_handler_invocation(
        registry, context, tokens,
    )?)
}

/// Namespace directive candidates under the actual supplied metadata owner.
/// Missing or foreign availability refuses; this grants no namespace effect.
pub(crate) fn namespace_directive_footprint_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<NamespaceDirectiveFootprint> {
    let context = context.filter(|context| context.matches_registry(registry))?;
    namespace_directive_footprint_for_invocation(resolved_handler_invocation_with_metadata_context(
        registry,
        Some(context),
        tokens,
    )?)
}

fn namespace_directive_footprint_for_invocation(
    invocation: ResolvedStatementInvocation,
) -> Option<NamespaceDirectiveFootprint> {
    if invocation.facts.arity_accepts_frozen_arguments() != Some(true) {
        return None;
    }
    let transitions = invocation
        .facts
        .state_transitions
        .declared()?
        .facts()
        .iter()
        .filter_map(|fact| match &fact.transition {
            tcl_registry::StateTransition::Namespace(
                transition @ (tcl_registry::NamespaceTransition::Import { .. }
                | tcl_registry::NamespaceTransition::Export { .. }),
            ) => Some(transition.clone()),
            _ => None,
        })
        .collect();
    Some(NamespaceDirectiveFootprint { transitions })
}

/// Possible alias declarations of retained handler candidates. This footprint
/// records exposure hazards, never a successful declaration or physical store.
#[derive(Debug)]
pub struct PossibleVariableAliasTransitions {
    aliases: Vec<tcl_registry::state_transition::VariableCellAliasTransition>,
    unknown_residual: bool,
}

impl PossibleVariableAliasTransitions {
    /// Candidate declarations whose activation policy does not rule them out.
    pub fn aliases(
        &self,
    ) -> impl Iterator<Item = &tcl_registry::state_transition::VariableCellAliasTransition> {
        self.aliases.iter()
    }

    /// Additional runtime handlers or operand layouts remain unenumerated.
    #[must_use]
    pub const fn unknown_residual(&self) -> bool {
        self.unknown_residual
    }
}

/// Project possible outward alias exposure from exact source candidate tokens.
/// Unknown receiver namespaces retain their lookup residual; catalogue names
/// cannot create candidates, and compiler selection grants no declaration edge.
#[must_use]
pub fn possible_variable_alias_transitions(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableAliasTransitions> {
    possible_variable_alias_transitions_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Candidate alias declarations under complete supplied source availability.
/// This grants exposure advice, never a completed link or physical cell.
#[must_use]
pub fn possible_variable_alias_transitions_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<PossibleVariableAliasTransitions> {
    possible_variable_alias_transitions_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

pub(crate) fn possible_variable_alias_transitions_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableAliasTransitions> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let dialect = binding.variable_context.invocation_dialect;
    let mut footprint = PossibleVariableAliasTransitions {
        aliases: Vec::new(),
        unknown_residual: binding.execution_is_unknown() || binding.execution_may_be_absent(),
    };
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        if !target.registry_backed {
            footprint.unknown_residual = true;
            continue;
        }
        let effective = effective_words_for_target(tokens, target)?;
        let values = frozen_argument_words(tokens, &effective);
        let mut words = vec![InvocationWord::Literal(&target.command)];
        words.extend(values.iter().map(EffectiveInvocationWord::as_registry_word));
        let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_registry_words_in_realm_with_metadata_context(
                registry,
                context,
                &words,
                dialect,
                binding.invocation_realm().unwrap_or_default(),
            )
        else {
            footprint.unknown_residual = true;
            continue;
        };
        for fact in facts
            .state_transitions
            .declared()
            .into_iter()
            .flat_map(tcl_registry::StateTransitions::facts)
        {
            let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition else {
                continue;
            };
            if alias
                .destination
                .is_active_in_frame_with_policy(binding.variable_context.alias_frame(), dialect)
                != Some(false)
                && !footprint.aliases.contains(alias)
            {
                footprint.aliases.push(alias.clone());
            }
        }
    }
    Some(footprint)
}

/// Possible variable-observer registrations from retained handler candidates.
/// This is a May exposure footprint, never a completed trace-table mutation.
#[derive(Debug)]
pub struct PossibleVariableTraceTransitions {
    traces: Vec<tcl_registry::TraceTransition>,
    unknown_residual: bool,
    metadata_unavailable: bool,
}

impl PossibleVariableTraceTransitions {
    /// Candidate observer transitions with their frozen variable operands.
    pub fn transitions(&self) -> impl Iterator<Item = &tcl_registry::TraceTransition> {
        self.traces.iter()
    }

    /// Additional handlers or operand layouts remain unenumerated.
    #[must_use]
    pub const fn unknown_residual(&self) -> bool {
        self.unknown_residual
    }

    /// An original candidate has no descriptor in the supplied generation.
    /// The observer inventory cannot be closed from a catalogue fallback.
    #[must_use]
    pub const fn metadata_unavailable(&self) -> bool {
        self.metadata_unavailable
    }
}

/// Retain observer hazards even when receiver lookup cannot prove one handler.
/// Only original candidate identities can supply this metadata.
#[must_use]
pub fn possible_variable_trace_transitions(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableTraceTransitions> {
    possible_variable_trace_transitions_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// May observer transitions under the complete supplied availability generation.
/// This does not establish a completed registration or physical observer table.
#[must_use]
pub fn possible_variable_trace_transitions_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<PossibleVariableTraceTransitions> {
    possible_variable_trace_transitions_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

pub(crate) fn possible_variable_trace_transitions_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableTraceTransitions> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let mut footprint = PossibleVariableTraceTransitions {
        traces: Vec::new(),
        unknown_residual: binding.execution_is_unknown() || binding.execution_may_be_absent(),
        metadata_unavailable: false,
    };
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        if !target.registry_backed {
            footprint.unknown_residual = true;
            continue;
        }
        let realm = binding.invocation_realm().unwrap_or_default();
        if context.is_some_and(|context| {
            context
                .context()
                .resolve_spec_in_realm(registry, &target.command, realm)
                .is_none()
        }) {
            footprint.metadata_unavailable = true;
        }
        let effective = effective_words_for_target(tokens, target)?;
        let values = frozen_argument_words(tokens, &effective);
        let mut words = vec![InvocationWord::Literal(&target.command)];
        words.extend(values.iter().map(EffectiveInvocationWord::as_registry_word));
        let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_registry_words_in_realm_with_metadata_context(
                registry,
                context,
                &words,
                binding.variable_context.invocation_dialect,
                binding.invocation_realm().unwrap_or_default(),
            )
        else {
            footprint.unknown_residual = true;
            continue;
        };
        for fact in facts
            .state_transitions
            .declared()
            .into_iter()
            .flat_map(tcl_registry::StateTransitions::facts)
        {
            let tcl_registry::StateTransition::Trace(trace) = &fact.transition else {
                continue;
            };
            if matches!(
                trace,
                tcl_registry::TraceTransition::Add {
                    target: tcl_registry::TraceTarget::Variable(_),
                    ..
                } | tcl_registry::TraceTransition::Remove {
                    target: tcl_registry::TraceTarget::Variable(_),
                    ..
                }
            ) && !footprint.traces.contains(trace)
            {
                footprint.traces.push(trace.clone());
            }
        }
    }
    Some(footprint)
}

/// Possible name accesses of retained execution candidates.
/// These are hazard dependencies only: neither a physical store nor normal
/// continuation, value, compiler eligibility or purity is established.
#[derive(Debug)]
pub struct PossibleVariableNameOperands {
    candidates: Vec<VariableNameOperands>,
    unknown_residual: bool,
}

impl PossibleVariableNameOperands {
    /// Frozen candidate operands in their actual effective argument positions.
    pub fn operands(
        &self,
    ) -> impl Iterator<Item = (tcl_registry::ArgRole, &EffectiveInvocationWord)> {
        self.candidates
            .iter()
            .flat_map(VariableNameOperands::operands)
    }

    /// Possible contents mutations, excluding proved binding-only operands of
    /// each candidate. This grants no physical write or normal completion.
    pub fn contents_write_operands(&self) -> impl Iterator<Item = &EffectiveInvocationWord> {
        self.candidates
            .iter()
            .flat_map(VariableNameOperands::contents_write_operands)
    }

    /// Possible operand lookups with their authored handler phase. The phase
    /// chooses a context for a hazard query; it cannot establish a store.
    pub fn phased_operands(
        &self,
    ) -> impl Iterator<
        Item = (
            tcl_registry::ArgRole,
            &EffectiveInvocationWord,
            tcl_registry::native_compilation::VariableOperandBindingPhase,
            bool,
        ),
    > {
        self.candidates.iter().flat_map(|candidate| {
            candidate
                .operands()
                .map(|(role, word)| (role, word, candidate.binding_phase, candidate.destroys))
        })
    }

    /// Possible role classes whose exact operand layout remains unresolved.
    pub fn unresolved_roles(&self) -> impl Iterator<Item = tcl_registry::ArgRole> + '_ {
        self.candidates
            .iter()
            .flat_map(|candidate| candidate.unresolved_roles.iter().copied())
    }

    /// Unresolved role classes paired with that candidate's destruction effect.
    /// This preserves both write and destruction hazards when selected handlers
    /// disagree; it grants no operand position or reached mutation proof.
    pub fn phased_unresolved_roles(
        &self,
    ) -> impl Iterator<Item = (tcl_registry::ArgRole, bool)> + '_ {
        self.candidates.iter().flat_map(|candidate| {
            candidate
                .unresolved_roles
                .iter()
                .map(|role| (*role, candidate.destroys))
        })
    }

    /// At least one retained candidate may destroy its selected variable.
    #[must_use]
    pub fn destroys(&self) -> bool {
        self.candidates.iter().any(VariableNameOperands::destroys)
    }

    /// Additional handlers or accesses cannot be enumerated.
    #[must_use]
    pub const fn unknown_residual(&self) -> bool {
        self.unknown_residual
    }
}

/// Union May name-access hazards from authoritative candidate identities.
/// Written command spelling and catalogue declarations cannot donate candidates.
#[must_use]
pub fn possible_variable_name_operands(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableNameOperands> {
    possible_variable_name_operands_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Resolve retained possible variable name operands under the complete supplied availability generation.
/// Existing original identity, argument and handler premises remain required.
#[must_use]
pub fn possible_variable_name_operands_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<PossibleVariableNameOperands> {
    possible_variable_name_operands_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

pub(crate) fn possible_variable_name_operands_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableNameOperands> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }

    if !matches!(
        tokens.synthetic,
        None | Some(
            crate::ir::SyntheticMarker::EvaluatedWrapper
                | crate::ir::SyntheticMarker::CapturedCatchOutputs
        )
    ) {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let dialect = binding.variable_context.invocation_dialect;
    let mut footprint = PossibleVariableNameOperands {
        candidates: Vec::new(),
        unknown_residual: binding.execution_is_unknown() || binding.execution_may_be_absent(),
    };
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        let candidate = target
            .registry_backed
            .then(|| {
                let effective = effective_words_for_target(tokens, target)?;
                let values = frozen_argument_words(tokens, &effective);
                let expansion_at = values.iter().position(|word| {
                    matches!(
                        word,
                        EffectiveInvocationWord::Expanded
                            | EffectiveInvocationWord::KnownExpansion(_) | EffectiveInvocationWord::KnownByteExpansion(_)
                    )
                });
                let mut words = vec![InvocationWord::Literal(&target.command)];
                words.extend(values.iter().map(EffectiveInvocationWord::as_registry_word));
                let Ok(RegistryInvocationResolution::Resolved(facts)) =
                    resolve_registry_words_in_realm_with_metadata_context(
                        registry,
                        context,
                        &words,
                        dialect,
                        binding.invocation_realm().unwrap_or_default(),
                    )
                else {
                    return None;
                };
                if facts.argument_offset > values.len()
                    || facts.arity_accepts_frozen_arguments() == Some(false)
                {
                    return None;
                }
                Some(VariableNameOperands {
                    binding_phase: facts.successful_handler.map_or(
                        tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments,
                        tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_binding_phase,
                    ),
                    operands: facts
                        .arg_roles
                        .iter()
                        .filter_map(|&(index, role)| {
                            matches!(
                                role,
                                tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
                            )
                            .then(|| {
                                let argument = facts.argument_offset + usize::from(index);
                                possible_name_operand_word(&values, expansion_at, argument)
                                    .map(|word| (role, word))
                            })
                            .flatten()
                        })
                        .collect(),
                    unresolved_roles: if facts.arg_roles_complete {
                        &[]
                    } else {
                        facts.arg_role_resolver_roles
                    },
                    binding_only_names: binding_only_operand_names(&facts),
                    destroys: facts
                        .traits
                        .contains(tcl_registry::Traits::DESTROYS_VARIABLE),
                })
            })
            .flatten();
        if let Some(candidate) = candidate {
            footprint.candidates.push(candidate);
        } else {
            footprint.unknown_residual = true;
        }
    }
    Some(footprint)
}

fn possible_name_operand_word(
    values: &[EffectiveInvocationWord],
    expansion_at: Option<usize>,
    argument: usize,
) -> Option<EffectiveInvocationWord> {
    // Expansion can move or supply later argv values. Earlier captured name
    // values remain fixed; neither branch grants a normal physical store.
    if expansion_at.is_some_and(|start| argument >= start) {
        Some(EffectiveInvocationWord::Dynamic)
    } else {
        values.get(argument).cloned()
    }
}

/// Possible variable-name operands of an exactly retained handler.
///
/// This footprint cannot provide variable definitions, normal store values,
/// body/control semantics or compiler eligibility. A dynamic destination retains
/// a possible name access even when successful-handler transfer cannot prove
/// its physical target. Compiler rejection may make the access unreachable;
/// that cannot establish the absence of a possible name access.
#[derive(Debug)]
pub struct VariableNameOperands {
    binding_phase: tcl_registry::native_compilation::VariableOperandBindingPhase,
    operands: Vec<(tcl_registry::ArgRole, EffectiveInvocationWord)>,
    unresolved_roles: &'static [tcl_registry::ArgRole],
    binding_only_names: std::collections::HashSet<String>,
    destroys: bool,
}

impl VariableNameOperands {
    /// Frozen evaluated name shapes, without a physical-store licence.
    pub fn operands(
        &self,
    ) -> impl Iterator<Item = (tcl_registry::ArgRole, &EffectiveInvocationWord)> {
        self.operands.iter().map(|(role, word)| (*role, word))
    }

    /// Candidate value/presence mutations, separately from alias or observer
    /// registration roles. Unknown names remain possible mutations.
    pub fn contents_write_operands(&self) -> impl Iterator<Item = &EffectiveInvocationWord> {
        self.operands.iter().filter_map(|(role, word)| {
            if *role != tcl_registry::ArgRole::VarWrite
                || matches!(word, EffectiveInvocationWord::Literal(name) if self.binding_only_names.contains(name))
            {
                None
            } else {
                Some(word)
            }
        })
    }

    /// Role classes whose exact argument positions remain unresolved.
    /// These denote possible accesses; they do not create definitions.
    #[must_use]
    pub const fn unresolved_roles(&self) -> &'static [tcl_registry::ArgRole] {
        self.unresolved_roles
    }

    /// The authored handler destroys the selected variable rather than stores it.
    #[must_use]
    pub const fn destroys(&self) -> bool {
        self.destroys
    }
}

fn binding_only_operand_names(facts: &InvocationFacts) -> std::collections::HashSet<String> {
    facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
        .filter_map(|fact| match &fact.transition {
            tcl_registry::StateTransition::VariableCellAlias(alias) => alias.local.literal(),
            tcl_registry::StateTransition::Trace(
                tcl_registry::TraceTransition::Add {
                    target: tcl_registry::TraceTarget::Variable(target),
                    ..
                }
                | tcl_registry::TraceTransition::Remove {
                    target: tcl_registry::TraceTarget::Variable(target),
                    ..
                },
            ) => target.literal(),
            _ => None,
        })
        .filter(|name| !crate::variable_bindings::contents_write_operand(facts, Some(name)))
        .map(str::to_owned)
        .collect()
}

/// Name-uncertainty projection independent of successful physical transfer.
#[must_use]
pub fn variable_name_operands(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<VariableNameOperands> {
    if tokens.synthetic.is_some() {
        return None;
    }
    let invocation = resolved_handler_invocation(registry, context, tokens)?;
    if invocation
        .effective
        .words
        .iter()
        .any(|word| matches!(word, WordExpr::Expand { .. }))
    {
        return None;
    }
    let count = invocation.effective.words.len().checked_sub(1)?;
    if invocation.facts.argument_offset > count
        || invocation.facts.arity_accepts_frozen_arguments() != Some(true)
    {
        return None;
    }
    let operands = invocation
        .facts
        .arg_roles
        .iter()
        .filter(|&&(_, role)| {
            matches!(
                role,
                tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
            )
        })
        .map(|&(index, role)| {
            (
                role,
                invocation.argument_word(invocation.facts.argument_offset + usize::from(index)),
            )
        })
        .collect();
    Some(VariableNameOperands {
        binding_phase: invocation.facts.successful_handler.map_or(
            tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments,
            tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_binding_phase,
        ),
        operands,
        unresolved_roles: if invocation.facts.arg_roles_complete {
            &[]
        } else {
            invocation.facts.arg_role_resolver_roles
        },
        binding_only_names: binding_only_operand_names(&invocation.facts),
        destroys: invocation
            .facts
            .traits
            .contains(tcl_registry::Traits::DESTROYS_VARIABLE),
    })
}

/// Representation facts on the reached handler's normal continuation.
///
/// This deliberately cannot expose native compiler, world-effect, body-flow or
/// completion facts. A converged handler may provide representation effects
/// while its compiler hook remains unknown; those effects never describe an
/// exceptional continuation. Effective words retain the original read sites.
#[derive(Debug)]
pub struct NormalRepresentationInvocation {
    proof: NormalRepresentationProof,
    native_pattern_source_argument: Option<usize>,
    pattern_operand_alternatives: Vec<(usize, Vec<String>)>,
}

/// Value retained by a successful selected variable assignment.
/// This representation projection supplies neither an opcode nor a write proof.
#[derive(Debug)]
pub struct NormalValueAssignment {
    /// Evaluated literal destination name.
    pub name: String,
    /// Original value word, retaining its positioned substitutions.
    pub value: WordExpr,
}

/// Successful native index access, without compiler or completion permission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalIndexAccessKind {
    /// Read one list element.
    ListRead,
    /// Write one list element, subject to the selected append-slot policy.
    ListWrite,
    /// Read one native string character.
    StringRead,
}

/// Original index-operation operands under a conditional native-handler
/// interpretation. This supplies no normal completion, current object class,
/// physical read version or executable operation.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ConditionalIndexAccessAdvice {
    pub(crate) kind: NormalIndexAccessKind,
    pub(crate) container: String,
    pub(crate) container_literal: Option<String>,
    pub(crate) index: String,
    pub(crate) container_word: WordExpr,
    pub(crate) index_word: WordExpr,
    pub(crate) list_set_bounds: Option<tcl_dialect::ListSetBounds>,
}

/// Original declaration-owned lookup and accepted operand layout for bounds
/// diagnostics. Every native candidate must agree. Runtime residuals remain
/// and consumers need independent source-read and interval evidence.
#[cfg(test)]
pub(crate) fn conditional_index_access_advice(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<ConditionalIndexAccessAdvice> {
    conditional_index_access_advice_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

pub(crate) fn conditional_index_access_advice_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<ConditionalIndexAccessAdvice> {
    use NormalIndexAccessKind as Kind;
    use tcl_registry::{IntrinsicId, SemanticOperationId};
    let advice = tokens
        .source_binding
        .as_ref()?
        .declaration_operand_layout_advice(tokens)?;
    let dialect = advice.dialect();
    let mut unanimous = None;
    for target in advice.targets() {
        let effective =
            compose_original_effective_words(tokens, &target.command, &target.prepended)?;
        let values: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm_with_metadata_context(
                registry,
                context,
                &words,
                Some(dialect),
                advice.realm(),
            )
            .ok()?
        else {
            return None;
        };
        let offset = facts.argument_offset;
        let operands = effective.words.get(offset + 1..)?;
        let kind = match facts.operation {
            SemanticOperationId::Intrinsic(IntrinsicId::ListIndex) if operands.len() == 2 => {
                Kind::ListRead
            }
            SemanticOperationId::Intrinsic(IntrinsicId::ListSet) if operands.len() == 3 => {
                Kind::ListWrite
            }
            SemanticOperationId::Intrinsic(IntrinsicId::StringIndex) if operands.len() == 2 => {
                Kind::StringRead
            }
            _ => return None,
        };
        let literal = match values.get(offset + 1)? {
            EffectiveInvocationWord::Literal(value) => Some(value.clone()),
            _ => None,
        };
        let selected = ConditionalIndexAccessAdvice {
            kind,
            container: if kind == Kind::ListWrite {
                literal.clone()?
            } else {
                operands[0].legacy_text().clone()
            },
            container_literal: (kind != Kind::ListWrite).then_some(literal).flatten(),
            index: operands[1].sole_variable_substitution().map_or_else(
                || operands[1].legacy_text(),
                |(spelling, _)| spelling.to_owned(),
            ),
            container_word: operands[0].clone(),
            index_word: operands[1].clone(),
            list_set_bounds: dialect.list_set_bounds(),
        };
        if unanimous
            .as_ref()
            .is_some_and(|previous| previous != &selected)
        {
            return None;
        }
        unanimous = Some(selected);
    }
    unanimous
}

/// Frozen operands of a successful selected index handler.
#[derive(Debug)]
pub struct NormalIndexAccess {
    /// Authored container operation.
    pub kind: NormalIndexAccessKind,
    /// Written container or variable-name presentation.
    pub container: String,
    /// Container value frozen before later argument effects, if proved.
    pub container_literal: Option<String>,
    /// Written index presentation.
    pub index: String,
    /// Index value frozen before later argument effects, if proved.
    pub index_literal: Option<String>,
    /// Selected native grammar for interpreting index values.
    pub index_syntax: Option<tcl_dialect::IndexSyntax>,
    /// Original effective container word, retaining its exact read site.
    pub container_word: WordExpr,
    /// Original effective index word, retaining its exact read site.
    pub index_word: WordExpr,
    /// Native list write boundary, independent of the editing catalogue.
    pub list_set_bounds: Option<tcl_dialect::ListSetBounds>,
}

#[derive(Debug)]
enum NormalRepresentationProof {
    Handler(Box<ResolvedStatementInvocation>),
    // Normal-edge operand layout only. Unknown positional arity cannot donate
    // the representation, type, security or effect contracts of Handler.
    PatternLayout(Box<ResolvedStatementInvocation>),
    Iteration {
        expected: Option<tcl_registry::TclType>,
        words: EffectiveCommandWords,
        label: String,
    },
}

impl NormalRepresentationInvocation {
    /// Frozen name and nominal class of a selected normal naming factory.
    /// This candidate grants no command allocation, physical object receipt,
    /// method implementation, completion or executable transformation.
    #[must_use]
    pub fn naming_factory_candidate(&self) -> Option<(String, &'static str)> {
        let call = self.handler()?;
        let factory = call.facts.named_object_factory?;
        let argument = call.facts.argument_offset.checked_add(factory.argument())?;
        Some((call.argument_literal(argument)?, factory.class_name()))
    }

    /// Nominal class of a successful selected callable-result factory.
    /// No written name, created command token or method implementation is proved.
    #[must_use]
    pub fn callable_result_class_candidate(&self) -> Option<&'static str> {
        let call = self.handler()?;
        (call.facts.return_type == Some(tcl_registry::TclType::Object))
            .then_some(call.facts.named_object_factory?.class_name())
    }

    /// Literal list result and its first original written operand. The selected
    /// normal handler's result contract owns the construction; public spelling,
    /// captured alias prefixes and substituted operands cannot create an anchor.
    #[must_use]
    pub fn plain_literal_list_result(&self) -> Option<(String, tcl_lexer::Span)> {
        let call = self.handler()?;
        let contract = call.facts.native_result?;
        let tcl_registry::native_result::NativeResultSelection::ListArguments { from, len } = call
            .with_argument_words(|words| {
                contract.select(words.arguments(), call.facts.argument_offset)
            })
        else {
            return None;
        };
        if len == 0 {
            return None;
        }
        let mut values = Vec::with_capacity(len);
        let mut first = None;
        for index in from..from.checked_add(len)? {
            let word_index = index.checked_add(1)?;
            if !matches!(
                call.effective.origins.get(word_index)?,
                InvocationWordOrigin::Written(_)
            ) {
                return None;
            }
            let WordExpr::Literal { text, source } = call.effective.words.get(word_index)? else {
                return None;
            };
            if text.is_empty()
                || text.contains(['{', '}', '"', '\\', '$', '[', ']'])
                || text.chars().any(char::is_whitespace)
            {
                return None;
            }
            if call.argument_literal(index).as_deref() != Some(text) {
                return None;
            }
            let end = source
                .span
                .start()
                .checked_add(u32::try_from(text.len()).ok()?)?;
            first.get_or_insert(tcl_lexer::Span::new(source.span.start(), end));
            values.push(text.as_str());
        }
        Some((values.join(" "), first?))
    }

    /// Dictionary read operands under the selected normal handler. These retain
    /// original value reads and frozen keys without selecting executable code.
    #[must_use]
    pub fn dictionary_lookup(&self) -> Option<(WordExpr, Vec<Option<String>>)> {
        let call = self.handler()?;
        if call.facts.operation
            != tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::DictGet)
        {
            return None;
        }
        let input = call.facts.argument_offset;
        let keys = (input + 1..call.arguments.len())
            .map(|index| call.argument_literal(index))
            .collect();
        Some((call.effective.words.get(input + 1)?.clone(), keys))
    }

    /// Original operands constructing a successful dictionary value, with
    /// duplicate-key order preserved and no inferred representation licence.
    #[must_use]
    pub fn dictionary_constructor_words(&self) -> Option<Vec<WordExpr>> {
        let call = self.handler()?;
        let contract = call.facts.native_result?;
        let selection = call.with_argument_words(|words| {
            contract.select(words.arguments(), call.facts.argument_offset)
        });
        let tcl_registry::native_result::NativeResultSelection::DictionaryArguments { from, len } =
            selection
        else {
            return None;
        };
        Some(call.effective.words.get(from + 1..from + len + 1)?.to_vec())
    }

    /// Ordered original operands of a selected successful List constructor.
    /// Their source spans stay separate; this grants no object representation proof.
    #[must_use]
    pub fn list_constructor_words(&self) -> Option<Vec<WordExpr>> {
        let call = self.handler()?;
        let contract = call.facts.native_result?;
        let selection = call.with_argument_words(|words| {
            contract.select(words.arguments(), call.facts.argument_offset)
        });
        let tcl_registry::native_result::NativeResultSelection::ListArguments { from, len } =
            selection
        else {
            return None;
        };
        let end = from.checked_add(len)?.checked_add(1)?;
        if !(from + 1..end).all(|index| {
            matches!(
                call.effective.origins.get(index),
                Some(InvocationWordOrigin::Written(_))
            )
        }) {
            return None;
        }
        Some(call.effective.words.get(from + 1..end)?.to_vec())
    }

    fn handler(&self) -> Option<&ResolvedStatementInvocation> {
        match &self.proof {
            NormalRepresentationProof::Handler(invocation) => Some(invocation),
            NormalRepresentationProof::PatternLayout(_)
            | NormalRepresentationProof::Iteration { .. } => None,
        }
    }

    fn pattern_handler(&self) -> Option<&ResolvedStatementInvocation> {
        match &self.proof {
            NormalRepresentationProof::Handler(call)
            | NormalRepresentationProof::PatternLayout(call) => Some(call),
            NormalRepresentationProof::Iteration { .. } => None,
        }
    }

    /// Successful assignment value for tracking representations through generic
    /// calls. Physical definitions still come from the shared SSA place owner.
    #[must_use]
    pub fn value_assignment(&self) -> Option<NormalValueAssignment> {
        let invocation = self.handler()?;
        if invocation.facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Set,
            )
            || invocation.arguments.len() != 2
        {
            return None;
        }
        Some(NormalValueAssignment {
            name: invocation.argument_literal(0)?,
            value: invocation.effective.words.get(2)?.clone(),
        })
    }

    /// Whether the selected normal handler uses no caller frame access.
    /// This escape-analysis property does not select a compiler operation,
    /// donate an ABI implementation or prove an invocation's completion.
    #[must_use]
    pub fn is_frameless_runtime(&self) -> bool {
        self.handler().is_some_and(|invocation| {
            invocation
                .facts
                .traits
                .contains(tcl_registry::Traits::FRAMELESS_RUNTIME)
        })
    }

    /// Normal index operands for diagnostic bounds analysis.
    /// This does not prove that the access succeeds or licence removing it.
    #[must_use]
    pub fn index_access(&self) -> Option<NormalIndexAccess> {
        use NormalIndexAccessKind as Kind;
        use tcl_registry::{IntrinsicId, SemanticOperationId};
        let invocation = self.handler()?;
        let offset = invocation.facts.argument_offset;
        let arguments = invocation.arguments.get(offset..)?;
        let kind = match invocation.facts.operation {
            SemanticOperationId::Intrinsic(IntrinsicId::ListIndex) if arguments.len() == 2 => {
                Kind::ListRead
            }
            SemanticOperationId::Intrinsic(IntrinsicId::ListSet) if arguments.len() == 3 => {
                Kind::ListWrite
            }
            SemanticOperationId::Intrinsic(IntrinsicId::StringIndex) if arguments.len() == 2 => {
                Kind::StringRead
            }
            _ => return None,
        };
        Some(NormalIndexAccess {
            kind,
            container: if kind == Kind::ListWrite {
                invocation.argument_literal(offset)?
            } else {
                arguments[0].clone()?
            },
            container_literal: (kind != Kind::ListWrite)
                .then(|| invocation.argument_literal(offset))
                .flatten(),
            index: arguments[1].clone()?,
            index_literal: invocation.argument_literal(offset + 1),
            index_syntax: invocation
                .dialect
                .and_then(tcl_registry::InvocationDialect::index_syntax),
            container_word: invocation.effective.words.get(offset + 1)?.clone(),
            index_word: invocation.effective.words.get(offset + 2)?.clone(),
            list_set_bounds: invocation
                .dialect
                .and_then(tcl_registry::InvocationDialect::list_set_bounds),
        })
    }

    /// Evaluated argument count, including retained alias prefixes.
    #[must_use]
    pub fn argument_count(&self) -> usize {
        self.effective_words().words.len().saturating_sub(1)
    }

    /// Selected subcommand operand offset.
    #[must_use]
    pub fn argument_offset(&self) -> usize {
        self.handler().map_or(0, |call| call.facts.argument_offset)
    }

    /// Original words and retained alias origins, for positioned value reads.
    #[must_use]
    pub fn effective_words(&self) -> &EffectiveCommandWords {
        match &self.proof {
            NormalRepresentationProof::Handler(invocation)
            | NormalRepresentationProof::PatternLayout(invocation) => &invocation.effective,
            NormalRepresentationProof::Iteration { words, .. } => words,
        }
    }

    /// Canonical diagnostic identity; this is not an execution licence.
    #[must_use]
    pub fn diagnostic_command(&self) -> &str {
        match &self.proof {
            NormalRepresentationProof::Handler(invocation)
            | NormalRepresentationProof::PatternLayout(invocation) => {
                &invocation.facts.canonical_command
            }
            NormalRepresentationProof::Iteration { label, .. } => label,
        }
    }

    /// Diagnostic label including the selected canonical subcommand.
    #[must_use]
    pub fn diagnostic_label(&self) -> String {
        let mut label = self.diagnostic_command().to_owned();
        if let Some(invocation) = self.handler()
            && let tcl_registry::OwnedSubcommandResolution::Exact { canonical_name, .. }
            | tcl_registry::OwnedSubcommandResolution::UniquePrefix { canonical_name, .. } =
                &invocation.facts.subcommand
        {
            label.push(' ');
            label.push_str(canonical_name);
        }
        label
    }

    /// Original written pattern operand selected by the normal handler grammar.
    /// Captured alias-prefix patterns have no source argument position.
    #[must_use]
    pub fn pattern_source_argument_index(&self, registry: &CommandRegistry) -> Option<usize> {
        let call = self.pattern_handler()?;
        let query = call
            .dialect
            .and_then(tcl_registry::InvocationDialect::authoring_query);
        if registry
            .get_for_surface(&call.facts.canonical_command, query)?
            .pattern_type
            != Some(tcl_registry::patterns::PatternType::Regex)
        {
            return None;
        }
        if let Some(index) = self.native_pattern_source_argument {
            return Some(index);
        }
        let alternatives = self
            .pattern_operand_alternatives
            .iter()
            .map(|(argument, values)| tcl_registry::RoleOperandAlternatives {
                argument: *argument,
                values: tcl_registry::RoleOperandValues::Closed(values),
            })
            .collect::<Vec<_>>();
        let index = call.with_argument_words(|words| {
            let roles = registry.arg_role_assignments_consensus(
                &call.facts.canonical_command,
                words.arguments(),
                &alternatives,
                &[tcl_registry::ArgRole::Pattern],
            )?;
            match roles.as_slice() {
                [(index, tcl_registry::ArgRole::Pattern)] => Some(*index),
                _ => None,
            }
        })?;
        match call.effective.origins.get(index.checked_add(1)?)? {
            InvocationWordOrigin::Written(index) => index.checked_sub(1),
            _ => None,
        }
    }

    /// Written operands that may carry a regex on an admitted runtime layout.
    /// This hazard-only projection gives no guaranteed role, result type,
    /// variable definition, native operation or execution proof.
    #[must_use]
    pub fn possible_pattern_source_argument_indices(
        &self,
        registry: &CommandRegistry,
    ) -> Option<Vec<usize>> {
        if let Some(index) = self.pattern_source_argument_index(registry) {
            return Some(vec![index]);
        }
        let call = self.pattern_handler()?;
        if let Some(index) = self.native_pattern_source_argument {
            return Some(vec![index]);
        }
        let indices = call.with_argument_words(|words| {
            registry.possible_pattern_argument_indices_words(
                &call.facts.canonical_command,
                words.arguments(),
            )
        })?;
        Some(
            indices
                .into_iter()
                .filter_map(
                    |index| match call.effective.origins.get(index.checked_add(1)?)? {
                        InvocationWordOrigin::Written(index) => index.checked_sub(1),
                        _ => None,
                    },
                )
                .collect(),
        )
    }

    /// Authored normal representation mutation/coercion contract.
    #[must_use]
    pub fn representation_effect(&self) -> tcl_registry::RepresentationEffect {
        self.handler()
            .map_or(tcl_registry::RepresentationEffect::None, |call| {
                call.facts.representation_effect
            })
    }

    /// Normal conversion policy for independently proved ordinary containers.
    /// A semantic type, unknown object or abstract list cannot supply that proof.
    /// The policy applies to operand objects, never the command's return value.
    #[must_use]
    pub fn successful_ordinary_container_coercion(
        &self,
    ) -> Option<tcl_registry::representation::OrdinaryContainerCoercion> {
        let call = self.handler()?;
        if call.facts.arity_accepts_frozen_arguments() != Some(true) {
            return None;
        }
        call.with_argument_words(|words| {
            call.facts
                .representation_effect
                .successful_ordinary_container_coercion(
                    words.arguments(),
                    call.facts.argument_offset,
                )
        })
    }

    /// Successful ordinary list-input conversions under native foreach layout.
    /// Before success, possible conversions cannot commit a Must representation.
    #[must_use]
    pub fn successful_ordinary_container_coercions(
        &self,
    ) -> Option<Vec<tcl_registry::representation::OrdinaryContainerCoercion>> {
        let call = self.handler()?;
        if call.facts.arity_accepts_frozen_arguments() != Some(true) {
            return None;
        }
        call.with_argument_words(|words| {
            call.facts
                .representation_effect
                .successful_ordinary_container_coercions(
                    words.arguments(),
                    call.facts.argument_offset,
                )
        })
    }

    /// Authored normal byte-array operand conversion contract.
    #[must_use]
    pub fn byte_array_effect(&self) -> tcl_registry::ByteArrayEffect {
        self.handler()
            .map_or(tcl_registry::ByteArrayEffect::None, |call| {
                call.facts.byte_array_effect
            })
    }

    /// Encoder data object selected by the normal handler's operand protocol.
    /// Encoding names and option values cannot supply data provenance.
    #[must_use]
    pub fn encoded_value_word(&self) -> Option<WordExpr> {
        let call = self.handler()?;
        let offset = call.facts.argument_offset;
        let count = call.arguments.len().checked_sub(offset)?;
        let data = call.facts.byte_array_effect.encoded_value_argument(count)?;
        call.effective
            .words
            .get(offset.checked_add(data)?.checked_add(1)?)
            .cloned()
    }

    /// Declared representation of the successful selected result, when known.
    /// This supplies no result bytes, native opcode or non-normal guarantee.
    #[must_use]
    pub fn result_representation_type(&self) -> Option<tcl_registry::TclType> {
        self.handler()?.facts.return_type
    }

    /// Authored element representation of the successful selected result.
    /// Input element types still require positioned reads of the retained words.
    #[must_use]
    pub fn result_elements(&self) -> Option<tcl_registry::ReturnElements> {
        self.handler()?.facts.return_elements
    }

    /// Whether a successful selected form produces a byte-array object.
    #[must_use]
    pub fn returns_byte_array(&self) -> bool {
        self.handler()
            .is_some_and(|call| call.facts.return_type == Some(tcl_registry::TclType::ByteArray))
    }

    /// Selected byte payload layout, with unknown forms excluded.
    #[must_use]
    pub fn byte_array_payload(&self) -> Option<tcl_registry::BytePayloadSpec> {
        self.handler()?.facts.byte_array_payload
    }

    /// Whether normal mutation reads the old value before writing it.
    #[must_use]
    pub fn reads_before_write(&self) -> bool {
        self.handler().is_some_and(|call| {
            call.facts
                .traits
                .contains(tcl_registry::Traits::READS_BEFORE_WRITE)
        })
    }

    /// Argument roles only for distinguishing inert words from value reads.
    /// These do not establish script entry or execution topology.
    #[must_use]
    pub fn operand_roles(&self) -> &[(u8, tcl_registry::ArgRole)] {
        self.handler()
            .map_or(&[], |call| call.facts.arg_roles.as_slice())
    }

    /// Original operands selected as deferred scripts by the actual normal
    /// handler and its frozen layout. This proves metadata only: no script
    /// entry, callback dispatch or editable source is supplied by these indices.
    /// Unknown timing coverage returns `None`. Even `Some([])` cannot prove
    /// callback absence: operands captured in an alias prefix have no writable
    /// source index in this invocation and are deliberately omitted.
    #[must_use]
    pub fn deferred_script_source_argument_indices(&self) -> Option<Vec<usize>> {
        let call = self.handler()?;
        Some(
            call.facts
                .deferred_script_argument_indices()?
                .iter()
                .filter_map(|&argument| call.effective.written_argument(argument))
                .collect(),
        )
    }

    /// Frozen evaluated argument knowledge, without inventing unknown bytes.
    #[must_use]
    pub fn argument_word(&self, index: usize) -> EffectiveInvocationWord {
        self.handler()
            .map_or(EffectiveInvocationWord::Dynamic, |call| {
                call.argument_word(index)
            })
    }

    /// Known evaluated value, including an alias prefix's captured value.
    #[must_use]
    pub fn argument_literal(&self, index: usize) -> Option<String> {
        self.handler()?.argument_literal(index)
    }

    /// Registry-owned positional representation hint selected with frozen argv.
    /// Genuine synthetic iterators carry their producer's input representation.
    #[must_use]
    pub fn argument_type_hint(
        &self,
        index: usize,
    ) -> Option<&'static tcl_registry::hooks::ArgTypeHint> {
        use tcl_registry::{TclType, hooks::ArgTypeHint};
        const LIST: ArgTypeHint = ArgTypeHint {
            expected: Some(TclType::List),
            shimmers: true,
            transparent_from: &[],
        };
        const DICT: ArgTypeHint = ArgTypeHint {
            expected: Some(TclType::Dict),
            shimmers: true,
            transparent_from: &[],
        };
        match &self.proof {
            NormalRepresentationProof::Handler(call) => call.facts.argument_type_hint(index),
            NormalRepresentationProof::Iteration { expected, .. }
                if index < self.argument_count() =>
            {
                match expected {
                    Some(TclType::List) => Some(&LIST),
                    Some(TclType::Dict) => Some(&DICT),
                    _ => None,
                }
            }
            NormalRepresentationProof::PatternLayout(_)
            | NormalRepresentationProof::Iteration { .. } => None,
        }
    }

    /// Conditional normal cache disposition for the selected stock Length
    /// operand. The original current stock class must be authenticated by the
    /// caller; semantic types and result types cannot supply that evidence.
    /// This is conversion-cost metadata, not success, value or erasure proof.
    #[must_use]
    pub fn stock_length_operand_cache(
        &self,
        index: usize,
        class: tcl_registry::native_stock_list::NativeStockListInputClass,
        bytes: Option<&[u8]>,
    ) -> Option<tcl_registry::native_stock_list::NativeStockListCacheDisposition> {
        let call = self.handler()?;
        call.with_argument_words(|words| {
            let protocol = call.facts.stock_list_length_protocol(words.arguments())?;
            if protocol.argument() != index {
                return None;
            }
            protocol.normal_cache_disposition(class, bytes)
        })
    }

    /// Normal expression read projection under the caller's retained full grammar.
    pub(crate) fn source_expression_with_syntax_context(
        &self,
        parser: tcl_syntax::expr::parser::ExprParseContext,
        span: tcl_lexer::Span,
        parent: Option<crate::command_binding::CommandAllocationSite>,
    ) -> Option<crate::word_subst::LiftedSourceExpression> {
        crate::word_subst::source_expression_from_invocation_with_syntax_context(
            self.handler()?,
            parser,
            span,
            parent,
        )
    }
}

/// Select a representation contract for a reached normal handler.
/// Compiler uncertainty never becomes an opcode or body-execution proof here.
///
/// ```no_run
/// use tcl_compiler::{ir::CommandTokens, registry_invocation::normal_representation_invocation};
/// use tcl_registry::CommandRegistry;
/// fn successful_result(registry: &CommandRegistry, tokens: &CommandTokens) {
///     if let Some(normal) = normal_representation_invocation(registry, None, tokens) {
///         let result_representation = normal.result_representation_type();
///         // This hint supplies neither result bytes nor an opcode.
///         let _ = result_representation;
///     }
/// }
/// ```
#[must_use]
pub fn normal_representation_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<NormalRepresentationInvocation> {
    normal_representation_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Successful representation metadata under the actual availability generation.
/// This retains all normal-handler and operand requirements of the typed owner;
/// availability does not create a successful handler or a native invocation.
#[must_use]
pub fn normal_representation_invocation_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<NormalRepresentationInvocation> {
    normal_representation_invocation_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

/// Successful representation metadata with the caller's complete supplied
/// availability and source-policy context. Selection remains independent of
/// successful completion, original value identity and Native execution entry.
#[must_use]
pub fn normal_representation_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<NormalRepresentationInvocation> {
    if let Some(crate::ir::SyntheticMarker::IterationBindings(expected)) = tokens.synthetic {
        // Minted only by the typed iterator lowering. This selects input value
        // conversion, never an invocation of the synthetic diagnostic label.
        return Some(NormalRepresentationInvocation {
            proof: NormalRepresentationProof::Iteration {
                expected,
                words: EffectiveCommandWords {
                    words: tokens.words().to_vec(),
                    origins: (0..tokens.words().len())
                        .map(InvocationWordOrigin::Written)
                        .collect(),
                    binding_prefix: Vec::new(),
                },
                label: tokens.argv_texts.first()?.clone(),
            },
            native_pattern_source_argument: None,
            pattern_operand_alternatives: Vec::new(),
        });
    }
    let invocation = resolved_tokens_invocation_with_metadata_context(registry, context, tokens)
        .or_else(|| resolved_handler_invocation_with_metadata_context(registry, context, tokens))?;
    // The public ensemble's identity does not select an unavailable or
    // indeterminate member. Parent facts retain that uncertainty for other
    // purposes; they cannot supply a successful value-handler contract here.
    if matches!(
        invocation.facts.subcommand,
        tcl_registry::OwnedSubcommandResolution::Unknown { .. }
            | tcl_registry::OwnedSubcommandResolution::Ambiguous { .. }
            | tcl_registry::OwnedSubcommandResolution::Indeterminate { .. }
    ) {
        return None;
    }
    // Representation contracts require accepted arity. A conditional Pattern
    // projection may retain unknown positional arity under a separate carrier.
    // Expansion still cannot fabricate ordinary operand positions.
    if invocation
        .effective
        .words
        .iter()
        .any(|word| matches!(word, WordExpr::Expand { .. }))
    {
        return None;
    }
    if invocation.facts.argument_offset > invocation.arguments.len() {
        return None;
    }
    let accepted = invocation.facts.arity_accepts_frozen_arguments();
    if accepted == Some(false)
        || (accepted.is_none() && !has_regex_pattern_contract(&invocation, registry))
    {
        return None;
    }
    let native_pattern_source_argument = compiled_pattern_source_argument(tokens);
    let pattern_operand_alternatives = pattern_operand_alternatives(tokens, &invocation, registry);
    let invocation = Box::new(invocation);
    Some(NormalRepresentationInvocation {
        native_pattern_source_argument,
        pattern_operand_alternatives,
        proof: if accepted == Some(true) {
            NormalRepresentationProof::Handler(invocation)
        } else {
            NormalRepresentationProof::PatternLayout(invocation)
        },
    })
}

fn has_regex_pattern_contract(
    invocation: &ResolvedStatementInvocation,
    registry: &CommandRegistry,
) -> bool {
    let query = invocation
        .dialect
        .and_then(tcl_registry::InvocationDialect::authoring_query);
    registry
        .get_for_surface(&invocation.facts.canonical_command, query)
        .and_then(|spec| spec.pattern_type)
        == Some(tcl_registry::patterns::PatternType::Regex)
}

// Values come from each original read boundary, before later argv callbacks.
// They are used only to compare operand layouts, never to freeze an argv value.
fn pattern_operand_alternatives(
    tokens: &CommandTokens,
    invocation: &ResolvedStatementInvocation,
    registry: &CommandRegistry,
) -> Vec<(usize, Vec<String>)> {
    if !has_regex_pattern_contract(invocation, registry) {
        return Vec::new();
    }
    invocation
        .effective
        .words
        .iter()
        .skip(1)
        .enumerate()
        .filter(|(index, _)| invocation.argument_literal(*index).is_none())
        .filter_map(|(index, word)| {
            let (_, source) = word.sole_variable_substitution()?;
            let access = crate::command_binding::SourceVariableAccess::find_at_site(
                &tokens.variable_accesses,
                source,
            )?;
            closed_read_literals(access, registry).map(|values| (index, values))
        })
        .collect()
}

fn closed_read_literals(
    access: &crate::command_binding::SourceVariableAccess,
    registry: &CommandRegistry,
) -> Option<Vec<String>> {
    if access.context_residual() != crate::command_binding::SourceVariableReadResidual::Closed
        || access.context_alternatives().is_empty()
    {
        return None;
    }
    let mut values = std::collections::BTreeSet::new();
    for context in access.context_alternatives() {
        let crate::literal_contents::LiteralContentsAlternatives::Closed(contents) =
            context.substitution_literal_alternatives(&access.original_spelling, registry)
        else {
            return None;
        };
        values.extend(contents.values().iter().cloned());
        if values.len() > 8 || values.iter().map(String::len).sum::<usize>() > 8192 {
            return None;
        }
    }
    Some(values.into_iter().collect())
}

fn compiled_pattern_source_argument(tokens: &CommandTokens) -> Option<usize> {
    let binding = tokens.source_binding.as_ref()?;
    let proof = binding.native_operand_layout()?;
    // Layout and normal handler identities have independent authority. An
    // argv mutation or conflicting registration cannot borrow this layout.
    if binding.proved_handler_target()? != proof.target()
        || proof.compilation_site().offset != tokens.words().first()?.source().span.start()
    {
        return None;
    }
    let tcl_registry::native_compilation::NativeCompilationOperandLayout::Pattern {
        argument_index,
        ..
    } = proof.layout();
    let effective = compose_original_effective_words(
        tokens,
        &proof.target().command,
        &proof.target().prepended,
    )?;
    match effective.origins.get(argument_index.checked_add(1)?)? {
        InvocationWordOrigin::Written(index) => index.checked_sub(1),
        _ => None,
    }
}

/// Successful result security properties of the exact selected normal handler.
/// No compiler operation, completion or catalogue candidate is exposed.
#[derive(Debug)]
pub struct NormalTaintInvocation {
    invocation: Box<ResolvedStatementInvocation>,
}

impl NormalTaintInvocation {
    /// Authored source colour after actual frozen argc selection. Safety bits
    /// require this selected implementation and never come from its spelling.
    #[must_use]
    pub fn source_colour(&self) -> Option<tcl_registry::taint::TaintColour> {
        self.invocation.facts.taint_source
    }
    /// Declared bounded successful numeric result, without an execution licence.
    #[must_use]
    pub fn is_sanitiser(&self) -> bool {
        matches!(
            self.invocation.facts.return_type,
            Some(tcl_registry::TclType::Int | tcl_registry::TclType::Boolean)
        )
    }

    /// Authored successful-result transform proven from frozen evaluated argv.
    #[must_use]
    pub fn transform_colour(&self) -> Option<tcl_registry::taint::TaintColour> {
        self.invocation.facts.taint_transform
    }
}

/// Resolve security metadata through the same accepted normal handler proof as
/// representation queries, retaining private worker and vendor prerequisites.
#[must_use]
pub fn normal_taint_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<NormalTaintInvocation> {
    let normal = normal_representation_invocation(registry, context, tokens)?;
    let NormalRepresentationProof::Handler(invocation) = normal.proof else {
        return None;
    };
    Some(NormalTaintInvocation { invocation })
}

/// Select normal result metadata through the supplied generation and original
/// handler proof. Missing or foreign availability never reconstructs a label.
pub(crate) fn normal_taint_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<NormalTaintInvocation> {
    let context = context.filter(|context| context.matches_registry(registry))?;
    let normal =
        normal_representation_invocation_with_metadata_context(registry, Some(context), tokens)?;
    let NormalRepresentationProof::Handler(invocation) = normal.proof else {
        return None;
    };
    Some(NormalTaintInvocation { invocation })
}

/// Statement convenience projection retaining the original token proof.
#[must_use]
pub fn normal_statement_representation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    statement: &crate::ir::Statement,
) -> Option<NormalRepresentationInvocation> {
    normal_statement_representation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        statement,
    )
}

pub(crate) fn normal_statement_representation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    statement: &crate::ir::Statement,
) -> Option<NormalRepresentationInvocation> {
    normal_representation_invocation_with_metadata_context(registry, context, statement.tokens()?)
}

/// Possible effect regions on the selected handler's normal route. No
/// completion, compiler, purity or unknown-residual obligation is withdrawn.
pub(crate) fn possible_normal_handler_effect_regions_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<(
    crate::side_effects::EffectRegion,
    crate::side_effects::EffectRegion,
)> {
    let invocation = resolved_handler_invocation_with_metadata_context(registry, context, tokens)?;
    Some(crate::side_effects::normal_handler_effect_regions(
        &invocation.facts.effects,
    ))
}

/// Exact original argv layout of the independently proved handler. This
/// borrows its retained semantic context and implementation dependencies, but
/// does not require an expression preparation, successful execution or CPP.
/// Consumers close any value/evaluation obligations through their own owner.
pub(crate) fn original_selected_handler_layout_invocation(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    let context = body_assistance_context(registry, tokens)?;
    resolved_handler_invocation(registry, Some(context), tokens)
}

/// Selected callback grammar from an actual handler or its authentic
/// declaration observation. This supplies no successful handler or CPP entry.
pub(crate) fn original_callback_invocation(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    let context = body_assistance_context(registry, tokens)?;
    original_callback_invocation_with_metadata_context(registry, context.into(), tokens)
}

pub(crate) fn original_callback_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    if !context.matches_registry(registry) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens)
        .or_else(|| original_declared_structured_invocation(registry, context, tokens))
}

pub(crate) fn resolved_handler_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    resolved_handler_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

pub(crate) fn resolved_handler_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    use tcl_registry::native_compilation::NormalHandlerImplementationLookup;
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let target = binding
        .proved_handler_target()
        .filter(|target| target.registry_backed)?;
    let effective = effective_words_for_target(tokens, target)?;
    let dialect = binding.variable_context.invocation_dialect.or_else(|| {
        registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile)
    });
    let RegistryInvocationResolution::Resolved(facts) =
        resolve_effective_tokens_with_metadata_context(
            registry, context, tokens, &effective, dialect,
        )
        .ok()?
    else {
        return None;
    };
    let invocation = ResolvedStatementInvocation {
        dialect,
        arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
        evaluated_arguments: frozen_argument_values(tokens, &effective),
        evaluated_words: frozen_argument_words(tokens, &effective),
        effective,
        facts,
    };
    match invocation
        .facts
        .normal_handler_implementation_lookup(dialect)
    {
        NormalHandlerImplementationLookup::NoneRequired => {}
        NormalHandlerImplementationLookup::Required(lookup) => {
            if !binding.proves_native_implementation_lookup(&lookup) {
                return None;
            }
        }
        NormalHandlerImplementationLookup::RequiredPath(paths) => {
            let closed = invocation.with_argument_words(|words| {
                binding.proves_native_handler_path(
                    paths,
                    words.arguments(),
                    invocation.facts.argument_offset,
                )
            });
            if !closed {
                return None;
            }
        }
        NormalHandlerImplementationLookup::Unknown => return None,
    }
    Some(invocation)
}

fn normal_alias_frame(tokens: &CommandTokens) -> tcl_registry::VariableAliasFrame {
    tokens
        .source_binding
        .as_ref()
        .map_or(tcl_registry::VariableAliasFrame::Unknown, |binding| {
            binding.variable_context.alias_frame()
        })
}

fn normal_output_compilation(
    tokens: &CommandTokens,
) -> tcl_registry::native_compilation::NativeCompilationSelection {
    tokens.source_binding.as_ref().map_or(
        tcl_registry::native_compilation::NativeCompilationSelection::Unknown,
        crate::command_binding::SourceInvocationBinding::native_compilation_selection,
    )
}

impl ResolvedStatementInvocation {
    /// Map selected argument roles to original written operands. This keeps
    /// the selected invocation's proof strength; captured prefix values and
    /// materialized expansion elements have no editable source operand.
    #[must_use]
    pub fn written_argument_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        mapped_written_argument_roles(
            &self.effective,
            self.facts.argument_offset,
            &self.facts.arg_roles,
        )
    }

    /// Query registry metadata with the canonical selected head and evaluated
    /// argv knowledge. Captured values keep their slots without invented bytes;
    /// the retained native dialect travels with the same argument view.
    pub fn with_argument_words<T>(
        &self,
        apply: impl FnOnce(tcl_registry::InvocationWords<'_>) -> T,
    ) -> T {
        let values: Vec<_> = (0..self.arguments.len())
            .map(|index| self.argument_word(index))
            .collect();
        let arguments: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let mut words = tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(&self.facts.canonical_command),
            &arguments,
        );
        if let Some(dialect) = self.dialect {
            words = words.with_dialect(dialect);
        }
        apply(words)
    }

    /// Effective evaluated argument knowledge, including bounded variable-name shape.
    /// A frozen complete value takes precedence over its original structural shape.
    #[must_use]
    pub fn argument_word(&self, argument: usize) -> EffectiveInvocationWord {
        if let Some(word) = self.evaluated_words.get(argument) {
            return word.clone();
        }
        if let Some(value) = self.argument_literal(argument) {
            return EffectiveInvocationWord::Literal(value);
        }
        match (
            self.dialect,
            argument
                .checked_add(1)
                .and_then(|index| self.effective.words.get(index)),
        ) {
            (Some(dialect), Some(word)) => {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            }
            _ => EffectiveInvocationWord::Dynamic,
        }
    }

    /// Query retained selected argv under the caller's whole metadata context.
    /// The context selects availability only; it grants no new handler proof.
    pub fn with_metadata_schema<T>(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        realm: tcl_dialect::model::InvocationRealm,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        if !context.matches_registry(registry) {
            return None;
        }
        self.with_argument_words(|words| {
            let selected =
                tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                    registry,
                    Some(context.context()),
                    words,
                    realm,
                )
                .resolved()?;
            apply(&selected)
        })
    }

    /// Completion of this selected native implementation after the original
    /// words evaluated. Frozen values travel with their effective arguments;
    /// dynamic options and argv expansion retain the registry's unknown route.
    #[must_use]
    pub fn completion_route(
        &self,
        registry: &CommandRegistry,
    ) -> tcl_registry::completion_route::InvocationCompletionRoute {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        if self
            .effective
            .words
            .iter()
            .any(|word| matches!(word, WordExpr::Expand { .. }))
        {
            return Route::Unknown;
        }
        let values: Vec<_> = (0..self.arguments.len())
            .map(|index| self.argument_literal(index))
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(|value| {
                value.as_deref().map_or(
                    tcl_registry::InvocationWord::Dynamic,
                    tcl_registry::InvocationWord::Literal,
                )
            })
            .collect();
        let mut arguments = tcl_registry::InvocationArguments::structured(&words);
        if let Some(dialect) = self.dialect {
            arguments = arguments.with_dialect(dialect);
        }
        registry
            .invocation_completion_route(
                &self.facts.canonical_command,
                arguments,
                self.dialect
                    .and_then(tcl_registry::InvocationDialect::authoring_query)
                    .or_else(|| {
                        registry
                            .profile()
                            .map(tcl_dialect::DialectProfile::surface_query)
                    }),
            )
            .unwrap_or(Route::Unknown)
    }

    /// Decode one argument under this invocation's retained grammar. A missing
    /// grammar exposes only values that do not depend on dialect rules.
    #[must_use]
    pub fn argument_literal(&self, argument: usize) -> Option<String> {
        if let Some(Some(value)) = self.evaluated_arguments.get(argument) {
            return Some(value.clone());
        }
        if let Some(dialect) = self.dialect {
            return self.effective.argument_literal(
                argument,
                dialect.lexer_grammar.escapes,
                dialect.word_values,
            );
        }
        match self.effective.words.get(argument.checked_add(1)?)? {
            WordExpr::Literal { text, .. } => Some(text.clone()),
            WordExpr::BracedLiteral { text, .. } if !text.contains("\\\n") => Some(text.clone()),
            WordExpr::Template { parts, .. }
                if parts.iter().all(
                    |part| matches!(part, WordPart::Text { text, .. } if !text.contains('\\')),
                ) =>
            {
                Some(
                    parts
                        .iter()
                        .filter_map(|part| match part {
                            WordPart::Text { text, .. } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect(),
                )
            }
            _ => None,
        }
    }
}

/// Document-declared argument roles applicable at a retained lookup site.
/// These are advisory facts: they cannot establish stores, effects, completion,
/// purity or executable dispatch and are deliberately separate from runtime facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredInvocationAssistance<'a> {
    /// Authored declaration with its original trust provenance.
    pub declaration: &'a tcl_registry::model::DeclaredCommand,
    /// Declared role positions in the written argument sequence.
    pub roles: Vec<(usize, tcl_registry::ArgRole)>,
}

/// Query declaration assistance through its owner after the shared lookup
/// kernel establishes slot applicability. Mutation or argument expansion
/// cannot recover an applicable contract by written-head matching.
#[must_use]
pub fn resolved_declared_assistance<'a>(
    surface: &tcl_registry::model::DocumentCommandSurface<'a>,
    tokens: &CommandTokens,
) -> Option<DeclaredInvocationAssistance<'a>> {
    let candidate = tokens.source_binding.as_ref()?.declared_command.as_ref()?;
    let declaration = surface.declared_command(&candidate.name)?;
    if tokens
        .words()
        .iter()
        .any(|word| matches!(word, WordExpr::Expand { .. } | WordExpr::Opaque { .. }))
    {
        return None;
    }
    let supplied = tokens.words().len().checked_sub(1)?;
    let mut roles = Vec::new();
    for argument in &declaration.arguments {
        for index in declaration.arg_indices_for_role(argument.role, supplied) {
            if !roles.contains(&(index, argument.role)) {
                roles.push((index, argument.role));
            }
        }
    }
    roles.sort_by_key(|(index, _)| *index);
    Some(DeclaredInvocationAssistance { declaration, roles })
}

impl EffectiveCommandWords {
    /// Project effective argument spellings without evaluating source words.
    ///
    /// Written arguments keep their original spelling; binding-prefix arguments
    /// use their retained literal values. A mismatched source snapshot declines
    /// rather than dropping a word and changing the invocation's arity.
    #[must_use]
    pub fn argument_spellings(&self, original_args: &[String]) -> Option<Vec<String>> {
        self.argument_presentations(original_args)?
            .into_iter()
            .collect()
    }

    /// Retain every effective argument slot without inventing captured bytes.
    #[must_use]
    pub fn argument_presentations(&self, original_args: &[String]) -> Option<Vec<Option<String>>> {
        if self.words.len() != self.origins.len() {
            return None;
        }
        self.origins
            .iter()
            .zip(&self.words)
            .skip(1)
            .map(|(origin, word)| match origin {
                InvocationWordOrigin::Written(index) => {
                    original_args.get(index.checked_sub(1)?).cloned().map(Some)
                }
                InvocationWordOrigin::BindingPrefix(index) => Some(
                    self.binding_prefix
                        .get(*index)?
                        .as_registry_word()
                        .literal()
                        .map(str::to_owned),
                ),
                InvocationWordOrigin::ExpandedElement { .. } => match word {
                    WordExpr::Opaque { text, .. } => Some(Some(text.clone())),
                    _ => None,
                },
                InvocationWordOrigin::ResolvedHead => None,
            })
            .collect()
    }

    /// Decode one effective argument only when its source representation is static.
    /// No variable substitution, script evaluation, or expansion is performed.
    #[must_use]
    pub fn argument_literal(
        &self,
        argument: usize,
        escapes: EscapeSyntax,
        rules: WordValueRules,
    ) -> Option<String> {
        match effective_invocation_word(self.words.get(argument.checked_add(1)?)?, escapes, rules) {
            EffectiveInvocationWord::Literal(value) => Some(value),
            EffectiveInvocationWord::ByteLiteral(_)
            | EffectiveInvocationWord::Dynamic
            | EffectiveInvocationWord::ArrayElementName { .. }
            | EffectiveInvocationWord::Expanded
            | EffectiveInvocationWord::KnownExpansion(_)
            | EffectiveInvocationWord::KnownByteExpansion(_)
            | EffectiveInvocationWord::Opaque => None,
        }
    }

    /// Map an effective argument to a written argument, when it has one.
    #[must_use]
    pub fn written_argument(&self, argument: usize) -> Option<usize> {
        match self.origins.get(argument + 1)? {
            InvocationWordOrigin::Written(index) => index.checked_sub(1),
            InvocationWordOrigin::ResolvedHead
            | InvocationWordOrigin::BindingPrefix(_)
            | InvocationWordOrigin::ExpandedElement { .. } => None,
        }
    }
}

/// Compose effective execution words exactly once from the shared point binding.
/// Native compiled selection can differ from the later live lookup target.
/// An explicitly uncertain binding never falls back to source spelling.
#[must_use]
pub fn effective_command_words(tokens: &CommandTokens) -> Option<EffectiveCommandWords> {
    let Some(binding) = &tokens.source_binding else {
        return Some(EffectiveCommandWords {
            words: tokens.words().to_vec(),
            origins: (0..tokens.words().len())
                .map(InvocationWordOrigin::Written)
                .collect(),
            binding_prefix: Vec::new(),
        });
    };
    effective_words_for_target(tokens, binding.proved_execution_target()?)
}

/// Retain original Tcl word shape independently from its evaluated value.
#[must_use]
pub fn native_compilation_word_shape(
    word: &WordExpr,
) -> tcl_registry::native_compilation::NativeCompilationWordShape {
    use tcl_registry::native_compilation::NativeCompilationWordShape as Shape;
    match word {
        WordExpr::BracedLiteral { .. } => Shape::BracedLiteral,
        WordExpr::Literal { .. } => Shape::Literal,
        WordExpr::Expand { .. } => Shape::Expanded,
        WordExpr::Opaque { .. } => Shape::Opaque,
        WordExpr::Template { parts, .. }
            if matches!(parts.as_slice(), [WordPart::Text { text, .. }]
                if native_simple_text_component(text)) =>
        {
            // A quoted SIMPLE_WORD retains its literal compiler token. A
            // decoded backslash run is a Parts word even when its value is
            // known, so it cannot acquire that compiler capability.
            Shape::QuotedLiteral
        }
        WordExpr::Template { parts, .. }
            if !parts.is_empty()
                && parts
                    .iter()
                    .all(|part| matches!(part, WordPart::Text { .. })) =>
        {
            Shape::BackslashLiteral
        }
        _ => Shape::Substituted,
    }
}

fn native_simple_text_component(text: &str) -> bool {
    use tcl_lexer::word_parts::{ExecutablePart, ExecutablePartArena, ExecutableText, SubstFlags};
    let Ok(length) = u32::try_from(text.len()) else {
        return false;
    };
    let Ok(arena) = ExecutablePartArena::decompose(
        tcl_lexer::SourceImage::native(text.as_bytes()),
        tcl_lexer::Span::new(0, length),
        SubstFlags::default(),
        tcl_lexer::LexerConfig::default(),
    ) else {
        return false;
    };
    matches!(arena.list(arena.root()), [part] if matches!(part.part, ExecutablePart::Text(ExecutableText::Original)))
}

/// Query original source compiler grammar. This establishes syntax capability
/// only; callers still need actual token selection and runtime entry validation.
#[must_use]
pub fn native_compilation_syntax(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    dialect: tcl_registry::InvocationDialect,
    context: tcl_registry::native_compilation::NativeCompilationContext,
) -> Option<(
    InvocationFacts,
    tcl_registry::native_compilation::NativeCompilationSelection,
)> {
    if tokens
        .words()
        .first()
        .and_then(|word| native_compilation_word_shape(word).compiler_head(Some(dialect)))
        != Some(true)
    {
        return None;
    }
    let values: Vec<_> = tokens
        .words()
        .iter()
        .map(|word| {
            effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
        })
        .collect();
    let words: Vec<_> = values
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect();
    let (head, arguments) = words.split_first()?;
    let invocation = InvocationWords::structured(*head, arguments).with_dialect(dialect);
    let resolution = registry.resolve_structured_invocation(invocation, dialect.authoring_query());
    let resolved = resolution.resolved()?;
    let native = resolved.semantics.native_compilation?;
    let shapes: Vec<_> = tokens
        .words()
        .iter()
        .skip(1)
        .map(native_compilation_word_shape)
        .collect();
    let facts = resolved.facts();
    let original = tokens.source_binding.as_ref().and_then(|binding| {
        let (image, original_words, offset) = binding.original_compiler_source(tokens)?;
        original_native_compilation(
            native,
            OriginalNativeCompilerInvocation {
                image,
                words: original_words,
                offset,
                config: binding.original_lexer_config_for_tokens(tokens)?,
                source_protocol: binding.compiler_source_protocol(),
                compiler_dialect: binding
                    .native_compiler_dialect()
                    .filter(|original| original.has_same_execution_policy(dialect)),
                context,
                operand_from: native.original_operand_from_for_facts(&facts)?,
            },
        )
    });
    let selection = original.map_or_else(
        || {
            if (native.namespace_binding_kind().is_some()
                || native.grammar
                    == tcl_registry::native_compilation::NativeCompilationGrammar::Switch)
                && tokens.source_binding.is_some()
            {
                tcl_registry::native_compilation::NativeCompilationSelection::Unknown
            } else {
                native.select_for_facts(invocation, &shapes, &facts, Some(dialect), context)
            }
        },
        |(selection, _)| selection,
    );
    Some((facts, selection))
}

/// Runtime validation boundary of a selected native operation. Ordinary
/// executable specialisations retain live before-arguments validation.
///
/// Handler convergence alone does not establish that such an operation exists.
/// Use [`proved_native_admitted_inline_operation`] before emitting a guarded opcode.
#[must_use]
pub fn command_binding_guard(tokens: &CommandTokens) -> tcl_runtime_api::CommandBindingGuard {
    use tcl_runtime_api::CommandBindingGuard;
    let Some(binding) = &tokens.source_binding else {
        return CommandBindingGuard::BeforeArguments;
    };
    binding
        .admitted_inline_invocation()
        .map_or(CommandBindingGuard::BeforeArguments, |proof| {
            native_command_binding_guard(proof.guard)
        })
}

/// Exact native operation selected at this original invocation's compilation.
/// A known successful handler or a generic late call cannot license an opcode.
#[must_use]
pub fn proved_native_inline_operation(
    tokens: &CommandTokens,
) -> Option<(
    tcl_registry::SemanticOperationId,
    tcl_registry::native_compilation::NativeCompilationGuard,
)> {
    match tokens
        .source_binding
        .as_ref()?
        .native_compilation_selection()
    {
        tcl_registry::native_compilation::NativeCompilationSelection::Inline {
            operation,
            guard,
        } => Some((operation, guard)),
        _ => None,
    }
}

/// Opcode recipe retained at chunk admission, before runtime callback effects.
/// This licenses guarded emission and original-source replay only. It cannot
/// establish a reached handler, normal effects, or permission to erase work.
#[must_use]
pub fn proved_native_admitted_inline_operation(
    tokens: &CommandTokens,
) -> Option<(
    tcl_registry::SemanticOperationId,
    tcl_registry::native_compilation::NativeCompilationGuard,
)> {
    match tokens
        .source_binding
        .as_ref()?
        .native_compilation_admission_selection()
    {
        tcl_registry::native_compilation::NativeCompilationSelection::Inline {
            operation,
            guard,
        } => Some((operation, guard)),
        _ => None,
    }
}

/// Whether a folded same-frame script must retain its runtime invocation.
/// A logical body region does not supply an actual native expansion recipe.
#[must_use]
pub fn block_requires_runtime_script(statement: &Statement) -> bool {
    let Statement::Block {
        error_context: Some(context),
        tokens,
        ..
    } = statement
    else {
        return false;
    };
    tokens
        .as_ref()
        .and_then(proved_native_admitted_inline_operation)
        .is_none_or(|(operation, _)| operation.inline_body_error_context() != Some(*context))
}

/// Restore a folded script's complete original invocation without flattening
/// its body into the caller's local table, literal pool or exception ranges.
/// Missing original words cannot be reconstructed from the advisory body.
#[must_use]
pub fn original_block_runtime_invocation(statement: &Statement) -> Option<Statement> {
    let Statement::Block {
        span,
        tokens: Some(tokens),
        ..
    } = statement
    else {
        return None;
    };
    if tokens.synthetic.is_some() || !tokens.words_align_with_argv_text() {
        return None;
    }
    let (command, arguments) = tokens.argv_texts.split_first()?;
    Some(Statement::Barrier {
        span: *span,
        reason: "separate same-frame script compilation".to_owned(),
        command: command.clone(),
        canonical_command: None,
        args: arguments.to_vec(),
        tokens: Some(tokens.clone()),
    })
}

/// Compiler environment of one proved inline script operand. This consumes
/// the same frozen argv and authored native range descriptor as source preflight;
/// a spelling or normal-handler envelope cannot supply an exception range.
/// `None` means that retained invocation, operand, or compiler metadata is
/// insufficient; it does not authorise inheriting a default child context.
#[must_use]
pub fn proved_native_inline_body_context(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    enclosing: tcl_registry::native_compilation::NativeCompilationContext,
    operand: usize,
) -> Option<tcl_registry::native_compilation::NativeCompilationContext> {
    proved_native_admitted_inline_operation(tokens)?;
    admitted_native_compiler_invocation(registry, None, tokens)?.body_context(enclosing, operand)
}

/// Retain a site's existing opcode guard only when its source selected Inline.
/// Source-less compatibility sites keep their explicitly authored requirement;
/// this projection does not supply missing opcode or source evidence.
#[must_use]
pub fn native_site_binding_requirement(
    site: &crate::ir::CommandBindingSite,
) -> Option<&tcl_runtime_api::CommandBindingIdentity> {
    site.source_tokens
        .as_deref()
        .is_none_or(|tokens| {
            tokens.source_binding.is_none()
                || proved_native_admitted_inline_operation(tokens).is_some()
        })
        .then_some(&site.binding)
}

/// Original invocation and premises of an already proved native operation.
/// This is a guard/replay plan, not a captured command token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeOperationSelectionPlan {
    /// Exact native operation whose selection the range preserves.
    pub operation: tcl_registry::SemanticOperationId,
    /// Exact source-instance owner; equal bytes and spans cannot substitute.
    pub compilation_site: crate::command_binding::CommandAllocationSite,
    /// Original command and auxiliary implementation lookup premises.
    pub requirements: Vec<tcl_runtime_api::CommandBindingIdentity>,
    /// Actual raw compiler registration selected independently of the handler.
    pub compiler_prerequisite: Option<
        std::sync::Arc<tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite>,
    >,
    /// Selection phase, separate from the requirements' validation phases.
    pub guard: tcl_runtime_api::CommandBindingGuard,
    /// Exact original written command for replay before any argument executes.
    pub source: String,
    /// That command's coordinates in its retained source instance.
    pub span: tcl_lexer::Span,
    /// Original rooted command lookup namespace.
    pub namespace: String,
    /// Exact owner used by replay; display spelling cannot reconstruct it.
    pub namespace_context: Option<tcl_runtime_api::CompiledNamespaceContext>,
}

/// A proved operation lacks the original carrier needed for safe replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOperationSelectionDecline {
    /// The singleton selected-operation witness is unavailable.
    MissingOperationWitness,
    /// Original source words cannot be recovered in their retained instance.
    MissingOriginalSource,
    /// Original coordinates cannot represent the complete command.
    MissingSourceCoordinates,
    /// The selected command has no proved static lookup value.
    MissingStaticHead,
}

/// Carry exact original namespace identity across compiler/runtime consumers.
pub(crate) fn compiled_namespace_context(
    tokens: &CommandTokens,
) -> Option<tcl_runtime_api::CompiledNamespaceContext> {
    tokens
        .source_binding
        .as_ref()?
        .lookup_namespace_key
        .to_compiled_context()
}

/// Preserve an inline operation's selection across its original arguments.
/// Procedure-header operations use their independent actual-header plan.
/// `Ok(None)` identifies an invocation without an applicable inline plan.
///
/// # Errors
/// Returns the missing carrier when a proved operation cannot retain exact
/// replay. Executable consumers must preserve a provider obligation.
pub fn native_operation_selection_plan(
    tokens: &CommandTokens,
    escapes: EscapeSyntax,
    word_rules: WordValueRules,
) -> Result<Option<NativeOperationSelectionPlan>, NativeOperationSelectionDecline> {
    use NativeOperationSelectionDecline as Decline;
    let Some((operation, guard)) = proved_native_admitted_inline_operation(tokens) else {
        return Ok(None);
    };
    if operation
        == tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::ProcedureNoOp)
    {
        return Ok(None);
    }
    let binding = tokens
        .source_binding
        .as_ref()
        .ok_or(Decline::MissingOperationWitness)?;
    let selected = binding
        .admitted_inline_invocation()
        .ok_or(Decline::MissingOperationWitness)?;
    let source = native_compiler_replay_source(tokens, &selected.compilation_site)
        .ok_or(Decline::MissingOriginalSource)?;
    let length = u32::try_from(source.len()).map_err(|_| Decline::MissingSourceCoordinates)?;
    let end = selected
        .compilation_site
        .offset
        .checked_add(length)
        .ok_or(Decline::MissingSourceCoordinates)?;
    let guard = native_command_binding_guard(guard);
    let mut requirements = if selected.compiler_prerequisite.is_some() {
        Vec::new()
    } else {
        vec![
            tcl_runtime_api::CommandBindingIdentity::in_rooted_namespace(
                &binding.lookup_namespace,
                static_command_word(tokens, escapes, word_rules)
                    .ok_or(Decline::MissingStaticHead)?,
                selected
                    .target
                    .command
                    .strip_prefix("::")
                    .unwrap_or(&selected.target.command),
            )
            .with_namespace_context(compiled_namespace_context(tokens))
            .with_guard(guard),
        ]
    };
    requirements.extend(
        selected
            .lookup_dependencies
            .iter()
            .filter_map(native_implementation_dependency),
    );
    Ok(Some(NativeOperationSelectionPlan {
        operation,
        compilation_site: selected.compilation_site.clone(),
        compiler_prerequisite: selected.compiler_prerequisite.clone(),
        requirements,
        guard,
        source,
        span: tcl_lexer::Span::new(selected.compilation_site.offset, end),
        namespace: binding.lookup_namespace.clone(),
        namespace_context: compiled_namespace_context(tokens),
    }))
}

/// Decode a statically known original command word under the retained grammar.
///
/// This returns a lookup value only. It does not prove a handler, compiler hook,
/// or validation boundary; callers must retain those independent proofs. A
/// dynamic or expanded original word never becomes a static lookup here.
#[must_use]
pub fn static_command_word(
    tokens: &CommandTokens,
    escapes: EscapeSyntax,
    word_rules: WordValueRules,
) -> Option<String> {
    match effective_invocation_word(tokens.words().first()?, escapes, word_rules) {
        EffectiveInvocationWord::Literal(value) => Some(value),
        EffectiveInvocationWord::ByteLiteral(_)
        | EffectiveInvocationWord::ArrayElementName { .. }
        | EffectiveInvocationWord::Dynamic
        | EffectiveInvocationWord::Expanded
        | EffectiveInvocationWord::KnownExpansion(_)
        | EffectiveInvocationWord::KnownByteExpansion(_)
        | EffectiveInvocationWord::Opaque => None,
    }
}

/// Project the registry's native compiler validation boundary to the runtime ABI.
/// This shared conversion carries no additional command identity evidence.
#[must_use]
pub const fn native_command_binding_guard(
    guard: tcl_registry::native_compilation::NativeCompilationGuard,
) -> tcl_runtime_api::CommandBindingGuard {
    match guard {
        tcl_registry::native_compilation::NativeCompilationGuard::ChunkEntry => {
            tcl_runtime_api::CommandBindingGuard::ChunkEntry
        }
        tcl_registry::native_compilation::NativeCompilationGuard::BeforeArguments => {
            tcl_runtime_api::CommandBindingGuard::BeforeArguments
        }
    }
}

/// Preserve every auxiliary implementation lookup selected by the exact
/// native invocation proof. These are runtime guard requirements, not source
/// command boundaries or a replacement for the entered command's identity.
#[must_use]
pub fn native_implementation_dependencies(
    tokens: &CommandTokens,
) -> Vec<tcl_runtime_api::CommandBindingIdentity> {
    let Some(binding) = tokens.source_binding.as_ref() else {
        return Vec::new();
    };
    if let Some(named) = binding.admitted_named_invocation() {
        return named
            .dependencies
            .iter()
            .filter_map(native_implementation_dependency)
            .collect();
    }
    binding
        .admitted_inline_invocation()
        .into_iter()
        .flat_map(|proof| &proof.lookup_dependencies)
        .filter_map(native_implementation_dependency)
        .collect()
}

/// One shared conversion for selected-operation and compiler-failure lookup
/// dependencies. The registry owns timing; source proof owns lookup identity.
#[must_use]
pub fn native_implementation_dependency(
    dependency: &crate::command_binding::SourceNativeCompilationDependency,
) -> Option<tcl_runtime_api::CommandBindingIdentity> {
    if dependency.compiler_prerequisite.is_some() {
        return None;
    }
    Some(
        tcl_runtime_api::CommandBindingIdentity::in_rooted_namespace(
            &dependency.namespace,
            &dependency.head,
            dependency
                .target
                .command
                .strip_prefix("::")
                .unwrap_or(&dependency.target.command),
        )
        .with_namespace_context(dependency.namespace_key.to_compiled_context())
        .with_guard(native_command_binding_guard(dependency.guard)),
    )
}

/// Preserve the actual ordinary or ensemble compiler registration as issued.
#[must_use]
pub fn native_compiler_dependency(
    dependency: &crate::command_binding::SourceNativeCompilationDependency,
) -> Option<tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite> {
    dependency.compiler_prerequisite.as_ref().map(|required| {
        tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::from_command_registration(
            std::sync::Arc::clone(required),
        )
    })
}

/// Independent registration and handler axes consumed by compilation.
pub(crate) enum NativeCompilationDependency {
    Compiler(tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite),
    Implementation(tcl_runtime_api::CommandBindingIdentity),
}

pub(crate) fn native_compilation_dependency(
    dependency: &crate::command_binding::SourceNativeCompilationDependency,
) -> NativeCompilationDependency {
    if let Some(required) = native_compiler_dependency(dependency) {
        NativeCompilationDependency::Compiler(required)
    } else {
        NativeCompilationDependency::Implementation(
            native_implementation_dependency(dependency).expect("implementation dependency"),
        )
    }
}

/// Compose a single candidate using the same ordering as proved dispatch.
pub(crate) fn effective_words_for_target(
    tokens: &CommandTokens,
    target: &crate::command_binding::SourceCommandTarget,
) -> Option<EffectiveCommandWords> {
    let mut effective = compose_effective_words(tokens, &target.command, &target.prepended)?;
    if let Some(named) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.named_invocation())
    {
        discard_named_selection_operands(&mut effective, named.arguments_from);
    }
    Some(effective)
}

/// Replay prerequisites for an actually selected empty procedure compiler.
/// Retains opaque callable identity rather than borrowing a registry identity.
pub struct NativeProcedureNoOpPlan {
    /// Actual header compiler and allocation selected before argv.
    pub prerequisite: tcl_runtime_api::native_compilation::NativeProcedureHeaderPrerequisite,
    /// Exact written command in its own source instance.
    pub source: String,
    /// Original command lookup namespace, separate from its slot's namespace.
    pub namespace: String,
    /// Exact original command lookup owner.
    pub namespace_context: Option<tcl_runtime_api::CompiledNamespaceContext>,
}

/// A complete execution/replay plan; unknown source or header evidence declines.
#[must_use]
pub fn native_procedure_noop_plan(tokens: &CommandTokens) -> Option<NativeProcedureNoOpPlan> {
    let binding = tokens.source_binding.as_ref()?;
    if !matches!(
        binding.native_compilation_admission_selection(),
        tcl_registry::native_compilation::NativeCompilationSelection::Inline {
            operation: tcl_registry::SemanticOperationId::Intrinsic(
                tcl_registry::IntrinsicId::ProcedureNoOp
            ),
            ..
        }
    ) {
        return None;
    }
    let selected = binding.admitted_inline_invocation()?;
    let prerequisite = selected.procedure_header_prerequisite.as_deref()?.clone();
    if tokens.words().len() != tokens.argv_texts.len()
        || tokens
            .words()
            .iter()
            .any(|word| matches!(word, WordExpr::Expand { .. } | WordExpr::Opaque { .. }))
    {
        return None;
    }
    let source = native_compiler_replay_source(tokens, &selected.compilation_site)?;
    Some(NativeProcedureNoOpPlan {
        prerequisite,
        source,
        namespace: binding.lookup_namespace.clone(),
        namespace_context: compiled_namespace_context(tokens),
    })
}

/// Recover one exact original written command for compiler-selection replay.
/// This does not grant implementation identity or native compilation eligibility.
#[must_use]
pub fn native_compiler_replay_source(
    tokens: &CommandTokens,
    site: &crate::command_binding::CommandAllocationSite,
) -> Option<String> {
    if tokens.words().len() != tokens.argv_texts.len() {
        return None;
    }
    let text = site.source.try_text().ok()?;
    let first = tcl_lexer::word_span_at(text, tokens.words().first()?.source().span);
    let last = tcl_lexer::word_span_at(text, tokens.words().last()?.source().span);
    if first.start() != site.offset || first.start() > last.end() {
        return None;
    }
    text.get(first.start() as usize..last.end() as usize)
        .map(str::to_owned)
}

/// Compiler-selected private name with original argument words. This authorizes
/// late dispatch only, including an unknown or absent late handler; it never
/// licenses the original handler's inline opcodes or semantic effects.
#[must_use]
pub fn native_named_command_words(tokens: &CommandTokens) -> Option<EffectiveCommandWords> {
    let named = tokens
        .source_binding
        .as_ref()?
        .admitted_named_invocation()?;
    let mut effective = compose_original_effective_words(tokens, named.captured_name(), &[])?;
    discard_named_selection_operands(&mut effective, named.arguments_from);
    Some(effective)
}

fn discard_named_selection_operands(effective: &mut EffectiveCommandWords, arguments_from: usize) {
    let mut index = 0;
    effective.words.retain(|_| {
        let retain = !matches!(effective.origins[index], InvocationWordOrigin::Written(written) if written <= arguments_from);
        index += 1;
        retain
    });
    effective.origins.retain(|origin| !matches!(origin, InvocationWordOrigin::Written(written) if *written <= arguments_from));
}

fn compose_effective_words(
    tokens: &CommandTokens,
    command: &str,
    prepended: &[EffectiveInvocationWord],
) -> Option<EffectiveCommandWords> {
    let original = compose_original_effective_words(tokens, command, prepended)?;
    let Some(frozen) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.frozen_written_words())
    else {
        return Some(original);
    };
    if frozen.len() != tokens.words().len() {
        return None;
    }
    let mut effective = EffectiveCommandWords {
        words: Vec::new(),
        origins: Vec::new(),
        binding_prefix: original.binding_prefix.clone(),
    };
    for (word, origin) in original.words.into_iter().zip(original.origins) {
        if !matches!(origin, InvocationWordOrigin::Written(_)) {
            effective.words.push(word);
            effective.origins.push(origin);
        }
    }
    let mut needs_head = true;
    for (written, (word, frozen)) in tokens.words().iter().zip(frozen).enumerate() {
        if let Some(count) = frozen.expansion_len() {
            if count == 0 {
                continue;
            }
            append_expansion_elements(
                &mut effective,
                word,
                written,
                frozen,
                usize::from(needs_head),
            );
        } else if matches!(frozen, EffectiveInvocationWord::Expanded) {
            return None;
        } else if !needs_head {
            effective.words.push(word.clone());
            effective
                .origins
                .push(InvocationWordOrigin::Written(written));
        }
        needs_head = false;
    }
    if needs_head {
        return None;
    }
    Some(effective)
}

fn append_expansion_elements(
    effective: &mut EffectiveCommandWords,
    word: &WordExpr,
    written: usize,
    elements: &EffectiveInvocationWord,
    from: usize,
) {
    for element in from..elements.expansion_len().unwrap_or(0) {
        let text = elements
            .expansion_element(element)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .unwrap_or_default();
        effective.words.push(WordExpr::Opaque {
            text: text.to_owned(),
            source: crate::ir::SourceSite::opaque(word.source().span),
            reason: crate::ir::WordOpacity::LossySnapshot,
        });
        effective
            .origins
            .push(InvocationWordOrigin::ExpandedElement { written, element });
    }
}

pub(crate) fn compose_original_effective_words(
    tokens: &CommandTokens,
    command: &str,
    prepended: &[EffectiveInvocationWord],
) -> Option<EffectiveCommandWords> {
    let span = tokens.words().first()?.source().span;
    let mut words = vec![WordExpr::Literal {
        text: command.to_owned(),
        source: crate::ir::SourceSite::opaque(span),
    }];
    let mut origins = vec![InvocationWordOrigin::ResolvedHead];
    for (index, value) in prepended.iter().enumerate() {
        words.push(match value {
            EffectiveInvocationWord::Literal(text) => WordExpr::Literal {
                text: text.clone(),
                source: crate::ir::SourceSite::opaque(span),
            },
            _ => WordExpr::Opaque {
                text: String::new(),
                source: crate::ir::SourceSite::opaque(span),
                reason: crate::ir::WordOpacity::LossySnapshot,
            },
        });
        origins.push(InvocationWordOrigin::BindingPrefix(index));
    }
    for (index, word) in tokens.words().iter().enumerate().skip(1) {
        words.push(word.clone());
        origins.push(InvocationWordOrigin::Written(index));
    }
    Some(EffectiveCommandWords {
        words,
        origins,
        binding_prefix: prepended.to_vec(),
    })
}

/// Values retained before later argument substitutions change interpreter state.
/// Expansion and recovery words cannot acquire a fixed argument cardinality.
fn frozen_argument_values(
    tokens: &CommandTokens,
    effective: &EffectiveCommandWords,
) -> Vec<Option<String>> {
    effective
        .words
        .iter()
        .zip(&effective.origins)
        .skip(1)
        .map(|(word, origin)| {
            if let InvocationWordOrigin::ExpandedElement { written, element } = origin {
                let values = tokens
                    .source_binding
                    .as_ref()?
                    .frozen_written_words()?
                    .get(*written)?;
                return values
                    .expansion_element(*element)
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    .map(str::to_owned);
            }
            if matches!(word, WordExpr::Expand { .. } | WordExpr::Opaque { .. }) {
                return None;
            }
            match origin {
                InvocationWordOrigin::BindingPrefix(_) => match word {
                    WordExpr::Literal { text, .. } => Some(text.clone()),
                    _ => None,
                },
                InvocationWordOrigin::Written(index) => tokens
                    .source_binding
                    .as_ref()?
                    .evaluated_argument_values
                    .get(index.checked_sub(1)?)
                    .cloned()
                    .flatten(),
                InvocationWordOrigin::ExpandedElement { .. }
                | InvocationWordOrigin::ResolvedHead => None,
            }
        })
        .collect()
}

/// Readonly evaluated arguments aligned with the supplied effective origins.
/// Captured alias operands and retained native expansion children preserve
/// their exact bytes; no value is re-evaluated and no source geometry, handler
/// execution or completion proof is issued by this projection.
#[must_use]
pub fn frozen_argument_words(
    tokens: &CommandTokens,
    effective: &EffectiveCommandWords,
) -> Vec<EffectiveInvocationWord> {
    effective
        .words
        .iter()
        .zip(&effective.origins)
        .skip(1)
        .map(|(word, origin)| match origin {
            InvocationWordOrigin::Written(index) => tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.evaluated_argument_words.get(index - 1))
                .cloned()
                .unwrap_or_else(|| {
                    if let Some(dialect) = tokens
                        .source_binding
                        .as_ref()
                        .and_then(|binding| binding.variable_context.invocation_dialect)
                    {
                        return effective_invocation_word(
                            word,
                            dialect.lexer_grammar.escapes,
                            dialect.word_values,
                        );
                    }
                    match invocation_word(word) {
                        InvocationWord::Literal(text) => {
                            EffectiveInvocationWord::Literal(text.to_owned())
                        }
                        InvocationWord::KnownBytes(value) => {
                            EffectiveInvocationWord::from_bytes(value)
                        }
                        InvocationWord::Expanded => EffectiveInvocationWord::Expanded,
                        InvocationWord::Opaque => EffectiveInvocationWord::Opaque,
                        _ => EffectiveInvocationWord::Dynamic,
                    }
                }),
            InvocationWordOrigin::ExpandedElement { written, element } => tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.frozen_written_words())
                .and_then(|words| words.get(*written))
                .and_then(|word| word.expansion_element(*element))
                .map_or(
                    EffectiveInvocationWord::Dynamic,
                    EffectiveInvocationWord::from_bytes,
                ),
            InvocationWordOrigin::BindingPrefix(index) => effective
                .binding_prefix
                .get(*index)
                .cloned()
                .unwrap_or(EffectiveInvocationWord::Dynamic),
            InvocationWordOrigin::ResolvedHead => EffectiveInvocationWord::Dynamic,
        })
        .collect()
}

/// Nominal metadata for one possible registry implementation. This type
/// deliberately has no execution effects, completion or purity proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryInvocationShape {
    /// Possible registry implementation identity.
    pub command: String,
    /// Nominal semantic operation, for advisory interpretation only.
    pub operation: tcl_registry::SemanticOperationId,
    /// Possible roles relative to the selected command or member operands.
    pub roles: Vec<(u8, tcl_registry::ArgRole)>,
    /// Selected member operands' offset in effective post-head argv, including
    /// captured alias prefixes. This is independent of compiler selector removal.
    pub argument_offset: usize,
    /// Whether the candidate's role grammar was determinate.
    pub roles_complete: bool,
    /// Traits of this candidate, never a guarantee about the actual invocation.
    pub possible_traits: tcl_registry::Traits,
    /// Declared candidate result type, never an established runtime value type.
    pub nominal_return_type: Option<tcl_registry::TclType>,
    /// Alias-composed source words and original argument indices.
    pub effective: EffectiveCommandWords,
}

impl RegistryInvocationShape {
    fn from_facts(facts: &InvocationFacts, effective: EffectiveCommandWords) -> Self {
        Self {
            command: facts.canonical_command.clone(),
            operation: facts.operation,
            roles: facts.arg_roles.clone(),
            argument_offset: facts.argument_offset,
            roles_complete: facts.arg_roles_complete,
            possible_traits: facts.traits,
            nominal_return_type: facts.return_type,
            effective,
        }
    }

    /// Original written operands mentioned by this possible candidate.
    /// These roles grant assistance only, not reads, stores or body execution.
    #[must_use]
    pub fn written_argument_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        mapped_written_argument_roles(&self.effective, self.argument_offset, &self.roles)
    }
}

fn mapped_written_argument_roles(
    effective: &EffectiveCommandWords,
    argument_offset: usize,
    roles: &[(u8, tcl_registry::ArgRole)],
) -> Vec<(usize, tcl_registry::ArgRole)> {
    let mut mapped = Vec::new();
    for &(index, role) in roles {
        if let Some(effective_index) = argument_offset.checked_add(usize::from(index))
            && let Some(written) = effective.written_argument(effective_index)
            && !mapped.contains(&(written, role))
        {
            mapped.push((written, role));
        }
    }
    mapped
}

/// Advisory envelope retaining every uncertainty of shared command lookup.
/// Consumers may explain candidates but must not infer stores or optimise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryInvocationAssistance {
    /// Registry candidates for which nominal metadata is available.
    pub candidates: Vec<RegistryInvocationShape>,
    /// At least one possible implementation has no registry shape.
    pub unknown_residual: bool,
    /// Lookup can fail at runtime.
    pub may_be_absent: bool,
}

/// Command words shared by every closed advisory lookup alternative.
/// This projection grants no execution, role, mutation or completion facts.
#[derive(Debug, Clone, Copy)]
pub struct AdvisoryCommandWords<'a> {
    command: &'a str,
    effective: &'a EffectiveCommandWords,
}

impl AdvisoryCommandWords<'_> {
    /// Registry command identity shared by the advisory alternatives.
    #[must_use]
    pub fn command(&self) -> &str {
        self.command
    }

    /// Original words, alias prefix and origins shared by all alternatives.
    #[must_use]
    pub fn effective(&self) -> &EffectiveCommandWords {
        self.effective
    }
}

impl RegistryInvocationAssistance {
    /// Project unanimous advisory words without selecting an implementation.
    /// Any absent, unknown or disagreeing lookup alternative withdraws the view.
    #[must_use]
    pub fn unanimous_command_words(&self) -> Option<AdvisoryCommandWords<'_>> {
        if self.unknown_residual || self.may_be_absent {
            return None;
        }
        let first = self.candidates.first()?;
        if first.effective.words.is_empty()
            || first.effective.words.len() != first.effective.origins.len()
            || first.effective.origins.iter().any(|origin| {
                matches!(origin, InvocationWordOrigin::BindingPrefix(index)
                    if *index >= first.effective.binding_prefix.len())
            })
        {
            return None;
        }
        self.candidates
            .iter()
            .all(|candidate| {
                candidate.command == first.command && candidate.effective == first.effective
            })
            .then_some(AdvisoryCommandWords {
                command: &first.command,
                effective: &first.effective,
            })
    }
}

/// A possible value-store shape for assistance. It cannot establish a write,
/// a variable's runtime type, or a constant-propagation fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvisoryValueAssignment {
    /// Literal destination mentioned by the candidate operation.
    pub name: String,
    /// Authored or alias-inserted value word.
    pub value: WordExpr,
}

/// Project possible scalar value stores from the shared May envelope. A
/// document procedure named `set` contributes no stock assignment contract.
#[must_use]
pub fn advisory_value_assignments(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Vec<AdvisoryValueAssignment> {
    advisory_value_assignments_with_metadata_context(registry, None, tokens)
}

/// Possible scalar stores under the complete supplied availability generation.
/// The May envelope supplies no actual write, type or constant value.
#[must_use]
pub fn advisory_value_assignments_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Vec<AdvisoryValueAssignment> {
    advisory_value_assignments_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

pub(crate) fn advisory_value_assignments_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Vec<AdvisoryValueAssignment> {
    let Some(assistance) =
        registry_invocation_assistance_with_metadata_context(registry, context, tokens)
    else {
        return Vec::new();
    };
    let dialect = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.variable_context.invocation_dialect)
        .or_else(|| {
            context.map_or_else(
                || {
                    registry
                        .profile()
                        .map(tcl_registry::InvocationDialect::of_profile)
                },
                |context| {
                    context
                        .context()
                        .environment
                        .point()
                        .map(tcl_registry::InvocationDialect::of_point)
                },
            )
        });
    let Some(dialect) = dialect else {
        return Vec::new();
    };
    assistance
        .candidates
        .into_iter()
        .filter_map(|shape| {
            if shape.operation
                != tcl_registry::SemanticOperationId::StructuredLowering(
                    tcl_registry::hooks::LoweringHookId::Set,
                )
                || shape.effective.words.len() != 3
            {
                return None;
            }
            Some(AdvisoryValueAssignment {
                name: shape.effective.argument_literal(
                    0,
                    dialect.lexer_grammar.escapes,
                    dialect.word_values,
                )?,
                value: shape.effective.words.get(2)?.clone(),
            })
        })
        .collect()
}

/// Possible original operands of a default return operation. These are
/// result candidates only: no completion, returned type or reached caller is
/// established. Option-bearing forms require their own return-options recipe.
#[must_use]
pub fn advisory_return_values(registry: &CommandRegistry, tokens: &CommandTokens) -> Vec<WordExpr> {
    advisory_return_values_with_metadata_context(registry, None, tokens)
}

/// Possible return operands selected under the retained availability context.
/// These source candidates provide no completion or returned runtime type.
#[must_use]
pub fn advisory_return_values_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Vec<WordExpr> {
    advisory_return_values_with_metadata_context(context.commands(), Some(context.into()), tokens)
}

pub(crate) fn advisory_return_values_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Vec<WordExpr> {
    let mut values = Vec::new();
    for shape in registry_invocation_assistance_with_metadata_context(registry, context, tokens)
        .into_iter()
        .flat_map(|assistance| assistance.candidates)
    {
        if shape.operation
            == tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Return,
            )
            && shape.effective.words.len() == 2
            && let Some(value) = shape.effective.words.get(1)
            && !values.contains(value)
        {
            values.push(value.clone());
        }
    }
    values
}

/// Nominal catalogue assistance for an unchanged logical lookup slot. The
/// provider need not be loaded; this cannot establish actual result types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogueInvocationAssistance {
    /// Slot applicability established by the source lookup owner.
    pub candidate: crate::command_binding::DeclaredCommandCandidate,
    /// Available nominal metadata, selected under the document's dialect.
    pub shape: RegistryInvocationShape,
}

/// Obtain catalogue assistance separately from execution proofs. A command
/// mutation, document definition or unknown lookup invalidates applicability;
/// catalogue availability alone never restores the live implementation.
#[must_use]
pub fn catalogue_invocation_assistance<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Option<CatalogueInvocationAssistance> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let candidate = binding.catalogue_command.as_ref()?;
    context.context().resolve_spec(registry, &candidate.name)?;
    let effective = compose_effective_words(tokens, &candidate.name, &[])?;
    let RegistryInvocationResolution::Resolved(facts) =
        resolve_effective_tokens_with_metadata_context(
            registry,
            Some(context),
            tokens,
            &effective,
            binding.variable_context.invocation_dialect,
        )
        .ok()?
    else {
        return None;
    };
    Some(CatalogueInvocationAssistance {
        candidate: candidate.clone(),
        shape: RegistryInvocationShape::from_facts(&facts, effective),
    })
}

/// Query possible registry shapes without upgrading a May binding into Must.
/// Document implementations retain an unknown residual and cannot inherit a
/// stock command contract from their written name.
#[must_use]
pub fn registry_invocation_assistance(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    registry_invocation_assistance_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Query possible Registry shapes under the retained availability generation.
/// The original binding supplies candidates and unresolved alternatives; this
/// context neither selects a candidate nor supplies missing execution proof.
#[must_use]
pub fn registry_invocation_assistance_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    registry_invocation_assistance_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

pub(crate) fn registry_invocation_assistance_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let dialect = binding.variable_context.invocation_dialect.or_else(|| {
        registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile)
    });
    let mut assistance = RegistryInvocationAssistance {
        candidates: Vec::new(),
        unknown_residual: binding.execution_is_unknown(),
        may_be_absent: binding.execution_may_be_absent(),
    };
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        if !target.registry_backed {
            assistance.unknown_residual = true;
            continue;
        }
        let effective = effective_words_for_target(tokens, target)?;
        let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_effective_tokens_with_metadata_context(
                registry, context, tokens, &effective, dialect,
            )
        else {
            assistance.unknown_residual = true;
            continue;
        };
        assistance
            .candidates
            .push(RegistryInvocationShape::from_facts(&facts, effective));
    }
    Some(assistance)
}

/// Readonly Registry metadata at an unchanged original command site.
/// A unanimous retained execution envelope is preferred. Otherwise the genuine
/// original declaration layout may supply conditional command words before
/// operand evaluation, including an uncalled procedure body. Runtime lookup
/// uncertainty, Normal completion, stores and compiler admission are separate;
/// this projection changes none of their receipts.
#[must_use]
pub fn original_registry_invocation_assistance(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    original_registry_invocation_assistance_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Read conditional original Registry syntax with the supplied metadata owner.
/// Source consumers retain complete input and reject missing ownership before
/// calling; this readonly advice supplies no entered invocation or edit grant.
#[must_use]
pub fn original_registry_invocation_assistance_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    // Implementation contract: naming.source.original-registry-header-advice
    // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
    tokens
        .source_binding
        .as_ref()?
        .original_lexer_config_for_tokens(tokens)?;
    let reached = registry_invocation_assistance_with_metadata_context(registry, context, tokens);
    if reached
        .as_ref()
        .is_some_and(|advice| advice.unanimous_command_words().is_some())
    {
        return reached;
    }
    let context = context
        .or_else(|| body_assistance_context(registry, tokens).map(InvocationMetadataContext::from));
    let declared = context
        .and_then(|context| original_declared_structured_invocation(registry, context, tokens));
    let Some(declared) = declared else {
        return reached;
    };
    Some(RegistryInvocationAssistance {
        candidates: vec![RegistryInvocationShape::from_facts(
            &declared.facts,
            declared.effective,
        )],
        unknown_residual: false,
        may_be_absent: false,
    })
}

/// Possible Registry grammar in the unchanged literal prefix of an open quoted
/// procedure body. The shared source owner retains a distinct incomplete header
/// observation; this creates no complete body, installed definition, formal
/// activation, executable invocation, Normal or native compiler capability.
#[must_use]
pub fn original_incomplete_body_registry_invocation_assistance(
    registry: &CommandRegistry,
    realm: &crate::realm::CommandBindingRealm,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    // Implementation contract: naming.source.incomplete-body-header-metadata
    // docs/design/analysis/name-resolution-proofs/incomplete-body-header-metadata.md
    let advice = realm
        .source_bindings_ref()
        .incomplete_body_operand_layout_advice(tokens, registry)?;
    let dialect = advice.dialect();
    let mut candidates = Vec::new();
    for target in advice.targets() {
        let effective = effective_words_for_target(tokens, target)?;
        let values: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm(registry, None, &words, Some(dialect), advice.realm())
                .ok()?
        else {
            return None;
        };
        candidates.push(RegistryInvocationShape::from_facts(&facts, effective));
    }
    (!candidates.is_empty()).then_some(RegistryInvocationAssistance {
        candidates,
        unknown_residual: false,
        may_be_absent: false,
    })
}

/// Source role assistance from retained lookup candidates and applicable
/// declarations. Original operand mapping is owned here; captured alias
/// prefixes and expanded values cannot manufacture editable source indices.
/// This union preserves lookup uncertainty and grants no execution, read,
/// store, completion or optimisation proof.
#[must_use]
pub fn invocation_argument_role_assistance<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    tokens: &CommandTokens,
) -> Vec<(usize, tcl_registry::ArgRole)> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return Vec::new();
    }
    let mut roles = Vec::new();
    if let Some(assistance) =
        registry_invocation_assistance_with_metadata_context(registry, Some(context), tokens)
    {
        for candidate in assistance.candidates {
            roles.extend(candidate.written_argument_roles());
        }
    }
    if let Some(catalogue) = catalogue_invocation_assistance(registry, context, tokens) {
        roles.extend(catalogue.shape.written_argument_roles());
    }
    if let Some(declared) = resolved_declared_assistance(surface, tokens) {
        roles.extend(declared.roles);
    }
    let mut unique = Vec::new();
    for role in roles {
        if !unique.contains(&role) {
            unique.push(role);
        }
    }
    unique.sort_by_key(|(index, _)| *index);
    unique
}

/// Definite syntactic roles shared by every actual retained implementation.
/// An opaque, absent, incomplete or unaccepted alternative withdraws the
/// entire projection. Catalogue and declared navigation do not establish a
/// grammar obligation. This grants neither execution nor successful effects.
#[must_use]
pub fn invocation_argument_role_consensus<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Vec<(usize, tcl_registry::ArgRole)> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return Vec::new();
    }
    let Some(binding) = tokens.source_binding.as_ref() else {
        return Vec::new();
    };
    if tokens.synthetic.is_some()
        || binding.execution_is_unknown()
        || binding.execution_may_be_absent()
    {
        return Vec::new();
    }
    let dialect = binding.variable_context.invocation_dialect;
    let mut consensus: Option<Vec<(usize, tcl_registry::ArgRole)>> = None;
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        if !target.registry_backed {
            return Vec::new();
        }
        let Some(effective) = effective_words_for_target(tokens, target) else {
            return Vec::new();
        };
        let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_effective_tokens_with_metadata_context(
                registry,
                Some(context),
                tokens,
                &effective,
                dialect,
            )
        else {
            return Vec::new();
        };
        if !facts.arg_roles_complete || facts.arity_accepts_frozen_arguments() != Some(true) {
            return Vec::new();
        }
        let roles =
            mapped_written_argument_roles(&effective, facts.argument_offset, &facts.arg_roles);
        if let Some(shared) = &mut consensus {
            shared.retain(|role| roles.contains(role));
        } else {
            consensus = Some(roles);
        }
    }
    consensus.unwrap_or_default()
}

/// Body navigation and definite grammar are separate projections of the same
/// original invocation. Possible roles never establish an entered frame,
/// successful execution, variable store or scoped command definition.
pub(crate) struct InvocationBodyAssistance {
    pub possible_roles: Vec<(usize, tcl_registry::ArgRole)>,
    pub definite_roles: Vec<(usize, tcl_registry::ArgRole)>,
    pub possible_traits: tcl_registry::Traits,
    pub definite_traits: tcl_registry::Traits,
    pub definite_scope: Option<&'static tcl_registry::scoped::ScopedCommandEnv>,
    pub possible_definition_grammars: Vec<&'static tcl_registry::definer::DefinitionBodyGrammar>,
    pub possible_case_lists: Vec<(tcl_registry::spec::CaseListSpec, usize)>,
    pub possible_operations: Vec<(tcl_registry::SemanticOperationId, EffectiveCommandWords)>,
    pub definite_invocation: Option<ResolvedStatementInvocation>,
    /// Accepted own-body layout, conditional on this declaration's entry.
    /// It never establishes caller depth, actual argument values or stores.
    conditional_invocation: Option<ConditionalBodyInvocation>,
    /// Source-only receiver trait recipe. Runtime namespace and entry stay
    /// unavailable, and no definite role or execution projection uses this.
    receiver_trait_invocation: Option<ReceiverTraitInvocation>,
    pub declared_roles: Vec<(usize, tcl_registry::ArgRole)>,
    pub unknown_residual: bool,
    pub may_be_absent: bool,
    /// May usage of formals by retained implementation candidates, excluding
    /// nominal catalogue assistance. This grants no copy, frame or effects.
    parameter_advice: ParameterRoleAdvice,
}

#[derive(Default)]
pub(crate) struct ParameterRoleAdvice {
    candidates: Vec<RegistryInvocationShape>,
    closed: bool,
}

impl ParameterRoleAdvice {
    pub(crate) fn is_closed(&self) -> bool {
        self.closed
    }

    pub(crate) fn candidates(&self) -> &[RegistryInvocationShape] {
        &self.candidates
    }

    pub(crate) fn written_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        self.candidates
            .iter()
            .flat_map(RegistryInvocationShape::written_argument_roles)
            .collect()
    }

    pub(crate) fn may_execute_body(&self) -> bool {
        self.candidates.iter().any(|candidate| {
            candidate
                .possible_traits
                .contains(tcl_registry::Traits::PERFORMS_SUBSTITUTION)
                || candidate.roles.iter().any(|(_, role)| {
                    matches!(
                        role,
                        tcl_registry::ArgRole::Body
                            | tcl_registry::ArgRole::LambdaLiteral
                            | tcl_registry::ArgRole::CommandPrefix
                    )
                })
        })
    }
}

struct ReceiverTraitInvocation {
    entry: std::sync::Arc<crate::command_binding::SourceDeclaredReceiverBodyEntry>,
    invocation: ResolvedStatementInvocation,
}

struct ConditionalBodyInvocation {
    entry: std::sync::Arc<crate::command_binding::SourceConditionalBodyEntry>,
    invocation: ResolvedStatementInvocation,
}

impl InvocationBodyAssistance {
    pub(crate) fn parameter_role_advice(&self) -> &ParameterRoleAdvice {
        &self.parameter_advice
    }

    /// The own-body caller-name layout is a symbolic callee template. It
    /// identifies no caller cell and authorises no successful alias or store.
    pub(crate) fn caller_name_template_invocation(&self) -> Option<&ResolvedStatementInvocation> {
        self.definite_invocation
            .as_ref()
            .or_else(|| {
                self.conditional_invocation
                    .as_ref()
                    .map(|conditional| &conditional.invocation)
            })
            .or_else(|| {
                self.receiver_trait_invocation
                    .as_ref()
                    .map(|owned| &owned.invocation)
            })
    }

    /// Symbolic local-copy and alias traits use the declaration's original
    /// formal recipe. Symbolic caller-name roles may use it too; actual caller
    /// cells and effects require entered-frame proof and `definite_invocation`.
    pub(crate) fn parameter_invocation(
        &self,
        parameters: &[&str],
    ) -> Option<&ResolvedStatementInvocation> {
        self.definite_invocation
            .as_ref()
            .or_else(|| {
                let conditional = self.conditional_invocation.as_ref()?;
                conditional
                    .entry
                    .matches_parameters(parameters)
                    .then_some(&conditional.invocation)
            })
            .or_else(|| {
                let owned = self.receiver_trait_invocation.as_ref()?;
                owned
                    .entry
                    .matches_trait_parameters(parameters)
                    .then_some(&owned.invocation)
            })
    }
}

/// Heap-owned only during this original invocation's preparation. Different
/// May and exact-actual dialect inputs retain independent selected facts.
struct PreparedBodyCandidate {
    effective: EffectiveCommandWords,
    possible_facts: Box<InvocationFacts>,
    exact_facts: Option<Box<InvocationFacts>>,
    same_dialect: bool,
    selected_execution: bool,
}

impl PreparedBodyCandidate {
    fn exact_facts(&self) -> Option<&InvocationFacts> {
        if self.same_dialect {
            Some(&self.possible_facts)
        } else {
            self.exact_facts.as_deref()
        }
    }
}

struct PreparedBodyCandidates {
    candidates: Vec<PreparedBodyCandidate>,
    unknown_residual: bool,
    may_be_absent: bool,
}

#[inline(never)]
fn prepare_body_candidates<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Option<Box<PreparedBodyCandidates>> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let actual = binding.variable_context.invocation_dialect;
    let possible = actual.or_else(|| {
        registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile)
    });
    let mut prepared = Box::new(PreparedBodyCandidates {
        candidates: Vec::new(),
        unknown_residual: binding.execution_is_unknown(),
        may_be_absent: binding.execution_may_be_absent(),
    });
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        if !target.registry_backed {
            prepared.unknown_residual = true;
            continue;
        }
        let effective = effective_words_for_target(tokens, target)?;
        let Ok(RegistryInvocationResolution::Resolved(possible_facts)) =
            resolve_effective_tokens_with_metadata_context(
                registry,
                Some(context),
                tokens,
                &effective,
                possible,
            )
        else {
            prepared.unknown_residual = true;
            continue;
        };
        let same_dialect = actual.is_some() && actual == possible;
        let exact_facts = if same_dialect || actual.is_none() {
            None
        } else if let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_effective_tokens_with_metadata_context(
                registry,
                Some(context),
                tokens,
                &effective,
                actual,
            )
        {
            Some(facts)
        } else {
            None
        };
        prepared.candidates.push(PreparedBodyCandidate {
            effective,
            possible_facts,
            exact_facts,
            same_dialect,
            selected_execution: binding.proved_execution_target() == Some(target),
        });
    }
    Some(prepared)
}

#[inline(never)]
pub(crate) fn invocation_body_assistance<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    tokens: &CommandTokens,
) -> Box<InvocationBodyAssistance> {
    let context = context.into();
    body_assistance_with_entry(registry, context, surface, tokens, None)
}

#[inline(never)]
fn body_assistance_with_entry<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    tokens: &CommandTokens,
    entry: Option<std::sync::Arc<crate::command_binding::SourceConditionalBodyEntry>>,
) -> Box<InvocationBodyAssistance> {
    let context = context.into();
    let prepared = prepare_body_candidates(registry, context, tokens);
    let mut result = Box::new(InvocationBodyAssistance {
        possible_roles: Vec::new(),
        definite_roles: Vec::new(),
        possible_traits: tcl_registry::Traits::empty(),
        definite_traits: tcl_registry::Traits::empty(),
        definite_scope: None,
        possible_definition_grammars: Vec::new(),
        possible_case_lists: Vec::new(),
        possible_operations: Vec::new(),
        definite_invocation: None,
        conditional_invocation: None,
        receiver_trait_invocation: None,
        declared_roles: Vec::new(),
        unknown_residual: prepared.as_ref().is_none_or(|view| view.unknown_residual),
        may_be_absent: prepared.as_ref().is_none_or(|view| view.may_be_absent),
        parameter_advice: ParameterRoleAdvice::default(),
    });
    result.parameter_advice.closed = !result.unknown_residual && !result.may_be_absent;
    publish_possible_body_context(
        registry,
        context,
        surface,
        tokens,
        prepared.as_deref(),
        &mut result,
    );
    if result.unknown_residual || result.may_be_absent || tokens.synthetic.is_some() {
        return result;
    }
    let prepared = prepared.expect("closed assistance has prepared candidates");
    publish_unanimous_body_context(registry, context, &prepared, &mut result);
    if let Some(entry) = entry {
        retain_conditional_body_invocation(
            entry,
            registry,
            context,
            tokens,
            &prepared,
            &mut result,
        );
    }
    result.definite_invocation = prepared
        .candidates
        .iter()
        .find(|candidate| candidate.selected_execution)
        .filter(|candidate| {
            candidate.exact_facts().is_some_and(|facts| {
                facts.arg_roles_complete && facts.arity_accepts_frozen_arguments() == Some(true)
            })
        })
        .and_then(|candidate| prepared_body_invocation(registry, context, tokens, candidate));
    result
}

fn publish_possible_body_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    tokens: &CommandTokens,
    prepared: Option<&PreparedBodyCandidates>,
    result: &mut InvocationBodyAssistance,
) {
    if !context.matches_registry(registry) {
        return;
    }
    if let Some(prepared) = prepared {
        for candidate in &prepared.candidates {
            let facts = &candidate.possible_facts;
            let roles = mapped_written_argument_roles(
                &candidate.effective,
                facts.argument_offset,
                &facts.arg_roles,
            );
            result
                .parameter_advice
                .candidates
                .push(RegistryInvocationShape::from_facts(
                    facts,
                    candidate.effective.clone(),
                ));
            result.possible_roles.extend(roles);
            result.possible_traits |= facts.traits;
            collect_body_navigation(
                registry,
                context,
                tokens,
                (
                    &facts.canonical_command,
                    facts.operation,
                    &candidate.effective,
                ),
                result,
            );
        }
    }
    if let Some(catalogue) = catalogue_invocation_assistance(registry, context, tokens) {
        result
            .possible_roles
            .extend(catalogue.shape.written_argument_roles());
        result.possible_traits |= catalogue.shape.possible_traits;
        collect_body_navigation(
            registry,
            context,
            tokens,
            (
                &catalogue.shape.command,
                catalogue.shape.operation,
                &catalogue.shape.effective,
            ),
            result,
        );
    }
    if let Some(declared) = resolved_declared_assistance(surface, tokens) {
        result.declared_roles.clone_from(&declared.roles);
        result.possible_roles.extend(declared.roles);
    }
    let mut unique = Vec::new();
    for role in std::mem::take(&mut result.possible_roles) {
        if !unique.contains(&role) {
            unique.push(role);
        }
    }
    unique.sort_by_key(|(index, _)| *index);
    result.possible_roles = unique;
}

fn publish_unanimous_body_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    prepared: &PreparedBodyCandidates,
    result: &mut InvocationBodyAssistance,
) {
    let mut roles: Option<Vec<(usize, tcl_registry::ArgRole)>> = None;
    let mut traits: Option<tcl_registry::Traits> = None;
    let mut scope: Option<Option<&'static tcl_registry::scoped::ScopedCommandEnv>> = None;
    for candidate in &prepared.candidates {
        let Some(facts) = candidate.exact_facts().filter(|facts| {
            facts.arg_roles_complete && facts.arity_accepts_frozen_arguments() == Some(true)
        }) else {
            return;
        };
        let mapped = mapped_written_argument_roles(
            &candidate.effective,
            facts.argument_offset,
            &facts.arg_roles,
        );
        if let Some(shared) = &mut roles {
            shared.retain(|role| mapped.contains(role));
        } else {
            roles = Some(mapped);
        }
        traits = Some(traits.map_or(facts.traits, |previous| previous.intersection(facts.traits)));
        let selected_scope = context
            .context()
            .resolve_spec(registry, &facts.canonical_command)
            .and_then(|spec| spec.body_scope);
        scope = Some(match scope {
            None => selected_scope,
            Some(Some(previous))
                if selected_scope.is_some_and(|next| std::ptr::eq(previous, next)) =>
            {
                Some(previous)
            }
            _ => None,
        });
    }
    result.definite_roles = roles.unwrap_or_default();
    result.definite_traits = traits.unwrap_or_default();
    result.definite_scope = scope.flatten();
}

fn prepared_body_invocation<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
    candidate: &PreparedBodyCandidate,
) -> Option<ResolvedStatementInvocation> {
    let context = context.into();
    let effective = candidate.effective.clone();
    let dialect = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.variable_context.invocation_dialect)
        .or_else(|| {
            context
                .context()
                .environment
                .point()
                .map(tcl_registry::InvocationDialect::of_point)
        })
        .or_else(|| {
            registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile)
        });
    Some(ResolvedStatementInvocation {
        dialect,
        facts: Box::new(candidate.exact_facts()?.clone()),
        arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
        evaluated_arguments: frozen_argument_values(tokens, &effective),
        evaluated_words: frozen_argument_words(tokens, &effective),
        effective,
    })
}

fn collect_body_navigation(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    candidate: (
        &str,
        tcl_registry::SemanticOperationId,
        &EffectiveCommandWords,
    ),
    result: &mut InvocationBodyAssistance,
) {
    let (command, operation, effective) = candidate;
    let operation = (operation, effective.clone());
    if !result.possible_operations.contains(&operation) {
        result.possible_operations.push(operation);
    }
    if let Some(grammar) = context
        .context()
        .resolve_spec(registry, command)
        .and_then(|spec| spec.definition_body)
        && !result
            .possible_definition_grammars
            .iter()
            .any(|previous| std::ptr::eq(*previous, grammar))
    {
        result.possible_definition_grammars.push(grammar);
    }
    let Some(arguments) = effective.argument_spellings(&tokens.argv_texts[1..]) else {
        return;
    };
    let arguments: Vec<_> = arguments.iter().map(String::as_str).collect();
    // CaseInvocation indices are already relative to the complete post-head
    // argv supplied below, unlike selected member ArgRole indices.
    if let Some((spec, invocation)) = registry.case_invocation(
        command,
        &arguments,
        Some(context.context().authoring_query()),
    ) && let Some(index) = invocation
        .clause_list_index
        .and_then(|index| effective.written_argument(index))
    {
        result.possible_case_lists.push((spec, index));
    }
}

/// Use the retained execution point before the catalogue's separate authoring
/// view. The version-to-environment label is owned by the typed Tcl release;
/// an unknown point cannot manufacture a runtime or a default context.
fn body_assistance_context(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<SemanticContext> {
    let authoring = registry.profile().map(SemanticContext::for_profile);
    if let Some(packaged) =
        authoring.filter(|context| !context.context().authoring_query().packages.is_empty())
    {
        // Package placements are authoring availability, independently of the
        // actual engine passed to native facts and the retained invocation realm.
        return Some(packaged);
    }
    tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.variable_context.invocation_dialect)
        .and_then(tcl_registry::InvocationDialect::execution_point)
        .and_then(|point| {
            point
                .tcl_version()
                .map(|version| SemanticContext::for_environment(version.dialect_profile_name()))
                .or_else(|| {
                    (point.family() == tcl_dialect::model::Family::Jim)
                        .then(|| SemanticContext::for_environment(point.family().name()))
                })
        })
        .or(authoring)
}

/// Reconstruct only original words in this source buffer, then attach the
/// caller's retained source inventory. The affine base is supplied by the
/// source owner; this helper never analyses a detached body as a fresh entry.
pub(crate) fn segmented_body_assistance(
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    bindings: &crate::command_binding::SourceCommandBindings,
    source: &str,
    config: tcl_lexer::LexerConfig,
    command: &crate::segmenter::SegmentedCommand,
    base: u32,
    metadata: Option<&tcl_registry::model::ContextRegistry>,
) -> Option<Box<InvocationBodyAssistance>> {
    let registry = surface.commands();
    let original = match bindings.source_origin()?.kind() {
        crate::command_binding::SourceOriginKind::Authored(text)
        | crate::command_binding::SourceOriginKind::Loaded { source: text, .. }
        | crate::command_binding::SourceOriginKind::Derived { source: text, .. } => text,
    };
    let start = usize::try_from(base).ok()?;
    if original.get(start..start.checked_add(source.len())?) != Some(source.as_bytes()) {
        return None;
    }
    let mut tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, command);
    crate::lattice_rebase::rebase_command_tokens(&mut tokens, i64::from(base));
    bindings.stamp_original_tokens(&mut tokens);
    bindings.attach_declared_body_assistance(&mut tokens, surface);
    let context = match metadata {
        Some(context) => InvocationMetadataContext::from(context),
        None => body_assistance_context(registry, &tokens)?.into(),
    };
    if !context.matches_registry(registry) {
        return None;
    }
    let mut view = body_assistance_with_entry(registry, context, surface, &tokens, None);
    if let Some(owned) = owned_body_layout_invocation(registry, context, &tokens, bindings) {
        publish_conditional_parameter_advice(owned, &mut view);
    } else if let Some(owned) =
        receiver_source_trait_invocation(registry, context, &tokens, bindings)
    {
        view.parameter_advice = ParameterRoleAdvice {
            candidates: vec![RegistryInvocationShape::from_facts(
                &owned.invocation.facts,
                owned.invocation.effective.clone(),
            )],
            closed: true,
        };
        view.receiver_trait_invocation = Some(owned);
    }
    Some(view)
}

/// Conditional traits from the complete original receiver declaration. Its
/// source namespace does not claim the future receiver's private namespace.
fn receiver_source_trait_invocation<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
) -> Option<ReceiverTraitInvocation> {
    // naming.tcloo.original-declared-receiver-caller-traits
    // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md
    let binding = tokens.source_binding.as_ref()?;
    let site = binding.invocation_site()?;
    let entry = bindings.declared_receiver_body_entry_at(&site.source, site.offset)?;
    native_compiler_replay_source(tokens, site)?;
    let advice = binding.declaration_operand_layout_advice(tokens)?;
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_CALLER_FRAME").is_some() {
        eprintln!(
            "ORIGINAL_RECEIVER_TRAITS offset={} closed={} original_closed={} opaque={} targets={}",
            site.offset,
            advice.closed_receiver_trait_lookup(&entry),
            advice.closed_lookup(),
            advice.has_opaque_handler_alternatives(),
            advice.targets().len()
        );
    }
    if !advice.closed_receiver_trait_lookup(&entry) {
        return None;
    }
    let invocation = resolve_original_declared_layout(registry, context, tokens, &advice)?;
    if !invocation.facts.arg_roles_complete
        || invocation.facts.arity_accepts_frozen_arguments() != Some(true)
    {
        return None;
    }
    Some(ReceiverTraitInvocation { entry, invocation })
}

/// One declaration's conditional handler layout. Compiled execution residuals
/// remain independent; they cannot erase a separately proved logical handler.
fn owned_body_layout_invocation<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
) -> Option<ConditionalBodyInvocation> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }
    let site = tokens.source_binding.as_ref()?.invocation_site()?;
    let entry = bindings.conditional_body_entry_at(&site.source, site.offset)?;
    native_compiler_replay_source(tokens, site)?;
    let mut conditional = tokens.clone();
    conditional.source_binding =
        Some(bindings.conditional_invocation_at_source(&entry, site.offset));
    if !entry.owns_invocation(conditional.source_binding.as_ref()?) {
        return None;
    }
    let invocation =
        resolved_handler_invocation_with_metadata_context(registry, Some(context), &conditional)
            .or_else(|| {
                let prepared = prepare_body_candidates(registry, context, &conditional)?;
                if prepared.unknown_residual || prepared.may_be_absent {
                    return None;
                }
                let mut unanimous = None;
                for candidate in &prepared.candidates {
                    let facts = candidate.exact_facts()?;
                    if !facts.arg_roles_complete
                        || facts.arity_accepts_frozen_arguments() != Some(true)
                    {
                        return None;
                    }
                    let selected =
                        prepared_body_invocation(registry, context, &conditional, candidate)?;
                    if unanimous
                        .as_ref()
                        .is_some_and(|previous| previous != &selected)
                    {
                        return None;
                    }
                    unanimous = Some(selected);
                }
                unanimous
            })?;
    if !invocation.facts.arg_roles_complete
        || invocation.facts.arity_accepts_frozen_arguments() != Some(true)
    {
        return None;
    }
    Some(ConditionalBodyInvocation { entry, invocation })
}

fn publish_conditional_parameter_advice(
    owned: ConditionalBodyInvocation,
    view: &mut InvocationBodyAssistance,
) {
    view.parameter_advice = ParameterRoleAdvice {
        candidates: vec![RegistryInvocationShape::from_facts(
            &owned.invocation.facts,
            owned.invocation.effective.clone(),
        )],
        closed: true,
    };
    view.conditional_invocation = Some(owned);
}

/// Build a purpose-restricted layout only from converged original candidates.
/// The declaration recipe is independent of entered-script frame observations.
fn retain_conditional_body_invocation(
    entry: std::sync::Arc<crate::command_binding::SourceConditionalBodyEntry>,
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    prepared: &PreparedBodyCandidates,
    view: &mut InvocationBodyAssistance,
) {
    let Some(site) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.invocation_site())
    else {
        return;
    };
    if !entry.owns_source(&site.source, site.offset)
        || !tokens
            .source_binding
            .as_ref()
            .is_some_and(|binding| entry.owns_invocation(binding))
    {
        return;
    }
    let mut unanimous = None;
    for candidate in &prepared.candidates {
        let Some(facts) = candidate.exact_facts() else {
            return;
        };
        if !facts.arg_roles_complete || facts.arity_accepts_frozen_arguments() != Some(true) {
            return;
        }
        let Some(selected) = prepared_body_invocation(registry, context, tokens, candidate) else {
            return;
        };
        if unanimous
            .as_ref()
            .is_some_and(|previous| previous != &selected)
        {
            return;
        }
        unanimous = Some(selected);
    }
    if let Some(invocation) = unanimous {
        publish_conditional_parameter_advice(ConditionalBodyInvocation { entry, invocation }, view);
    }
}

/// Offset-free syntax assistance joins the retained lookup worlds. It has no
/// original invocation point and therefore never returns definite grammar,
/// entered scope or state-bearing alias/value-copy evidence.
pub(crate) fn unpositioned_body_assistance(
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    bindings: &crate::command_binding::SourceCommandBindings,
    source: &str,
    config: tcl_lexer::LexerConfig,
    command: &crate::segmenter::SegmentedCommand,
    metadata: Option<&tcl_registry::model::ContextRegistry>,
) -> Option<Box<InvocationBodyAssistance>> {
    let registry = surface.commands();
    let mut tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, command);
    tokens.source_binding = Some(bindings.invocation_unpositioned(command.name()));
    bindings.attach_declared_body_assistance(&mut tokens, surface);
    let context = match metadata {
        Some(context) => Some(InvocationMetadataContext::from(context)),
        None => body_assistance_context(registry, &tokens).map(InvocationMetadataContext::from),
    };
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }
    let Some(context) = context else {
        let declared = resolved_declared_assistance(surface, &tokens)?;
        return Some(Box::new(InvocationBodyAssistance {
            possible_roles: declared.roles.clone(),
            declared_roles: declared.roles,
            definite_roles: Vec::new(),
            possible_traits: tcl_registry::Traits::empty(),
            definite_traits: tcl_registry::Traits::empty(),
            definite_scope: None,
            possible_definition_grammars: Vec::new(),
            possible_case_lists: Vec::new(),
            possible_operations: Vec::new(),
            definite_invocation: None,
            conditional_invocation: None,
            receiver_trait_invocation: None,
            unknown_residual: true,
            may_be_absent: true,
            parameter_advice: ParameterRoleAdvice::default(),
        }));
    };
    let mut view = invocation_body_assistance(registry, context, surface, &tokens);
    view.definite_roles.clear();
    view.definite_traits = tcl_registry::Traits::empty();
    view.definite_scope = None;
    view.definite_invocation = None;
    view.unknown_residual = true;
    view.may_be_absent = true;
    Some(view)
}

/// Owned source-aware word fact for callers that must decode a static Tcl
/// word before lending it to the registry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EffectiveInvocationWord {
    /// A Tcl value proven static after applying the active escape syntax.
    Literal(String),
    /// A decoded native string whose exact bytes are not Unicode.
    /// This remains one argv operand and grants no text, name or object proof.
    ByteLiteral(std::sync::Arc<[u8]>),
    /// One argv word produced by substitution.
    Dynamic,
    /// A computed variable name whose evaluated root and array form are proved.
    ArrayElementName {
        /// Root before the first opening parenthesis; the element remains unknown.
        root: String,
    },
    /// An expansion with an unknown resulting argv length.
    Expanded,
    /// Native list elements retained at a written expansion evaluation.
    /// This grants cardinality and bytes, never object-representation proof.
    KnownExpansion(Vec<String>),
    /// Native list elements whose payloads cannot all be represented as Unicode.
    /// Each element remains one argv operand, with no object or source-word grant.
    KnownByteExpansion(Vec<std::sync::Arc<[u8]>>),
    /// A compatibility/recovery word whose source meaning is unavailable.
    Opaque,
}

/// Whether a source word may name a compiled local-variable slot.
///
/// This is deliberately stricter than [`EffectiveInvocationWord`]: a bare
/// word with backslash processing has a static *value*, but C Tcl still emits
/// the stack form for it and does not intern that value in the LVT.  Braced
/// and quoted/plain text words whose spelling needs no backslash processing
/// retain the direct-name form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompiledLocalNameWord {
    /// The source form is eligible for a direct local-name opcode.
    Direct,
    /// The word must be evaluated or decoded before it can name a variable.
    Stack,
}

/// Classify one source word for bytecode local-name selection.
///
/// The word-parts owner has already retained substitution and raw backslash
/// facts in [`WordExpr`].  Consumers must ask this adapter rather than infer
/// source provenance from compatibility argv text.  In particular, `{p\\x}`
/// is direct (the backslash is part of the name), while bare `p\\x` is stack
/// (the backslash is decoded before lookup).
#[must_use]
pub fn compiled_local_name_word(word: &WordExpr) -> CompiledLocalNameWord {
    match word {
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => CompiledLocalNameWord::Direct,
        WordExpr::Template { parts, .. }
            if parts.iter().all(
                |part| matches!(part, WordPart::Text { text, .. } if !text.contains('\\')),
            ) =>
        {
            // A quoted literal reaches this arm: grouping quotes are outside
            // its text parts, while a quoted backslash remains in one.
            CompiledLocalNameWord::Direct
        }
        WordExpr::Variable { .. }
        | WordExpr::CommandSubstitution { .. }
        | WordExpr::Template { .. }
        | WordExpr::Expand { .. }
        | WordExpr::Opaque { .. } => CompiledLocalNameWord::Stack,
    }
}

/// Return the evaluated name for a source word eligible for a compiled local
/// opcode. The same source owner supplies both the eligibility decision and
/// the literal value so brace grouping cannot reach an opcode as data.
#[must_use]
pub fn compiled_local_name_value(
    word: &WordExpr,
    escapes: EscapeSyntax,
    word_rules: WordValueRules,
) -> Option<String> {
    if compiled_local_name_word(word) != CompiledLocalNameWord::Direct {
        return None;
    }
    match effective_invocation_word(word, escapes, word_rules) {
        EffectiveInvocationWord::Literal(value) => Some(value),
        EffectiveInvocationWord::ByteLiteral(_)
        | EffectiveInvocationWord::Dynamic
        | EffectiveInvocationWord::ArrayElementName { .. }
        | EffectiveInvocationWord::Expanded
        | EffectiveInvocationWord::KnownExpansion(_)
        | EffectiveInvocationWord::KnownByteExpansion(_)
        | EffectiveInvocationWord::Opaque => None,
    }
}

impl EffectiveInvocationWord {
    /// Cardinality retained by a proved native list expansion.
    #[must_use]
    pub fn expansion_len(&self) -> Option<usize> {
        match self {
            Self::KnownExpansion(values) => Some(values.len()),
            Self::KnownByteExpansion(values) => Some(values.len()),
            _ => None,
        }
    }

    /// Exact native bytes of one proved expansion child.
    #[must_use]
    pub fn expansion_element(&self, ordinal: usize) -> Option<&[u8]> {
        match self {
            Self::KnownExpansion(values) => values.get(ordinal).map(String::as_bytes),
            Self::KnownByteExpansion(values) => values.get(ordinal).map(AsRef::as_ref),
            _ => None,
        }
    }

    /// Retain decoded native bytes with a checked Unicode projection.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        std::str::from_utf8(bytes).map_or_else(
            |_| Self::ByteLiteral(std::sync::Arc::from(bytes)),
            |text| Self::Literal(text.to_owned()),
        )
    }

    /// Exact payload when this word has a known native string value.
    #[must_use]
    pub fn literal_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Literal(text) => Some(text.as_bytes()),
            Self::ByteLiteral(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// Lend this owned fact to the registry's allocation-free vocabulary.
    #[must_use]
    pub fn as_registry_word(&self) -> InvocationWord<'_> {
        match self {
            Self::Literal(value) => InvocationWord::Literal(value),
            Self::ByteLiteral(value) => InvocationWord::KnownBytes(value),
            Self::Dynamic => InvocationWord::Dynamic,
            Self::ArrayElementName { root } => InvocationWord::ArrayElementName { root },
            Self::Expanded | Self::KnownExpansion(_) | Self::KnownByteExpansion(_) => {
                InvocationWord::Expanded
            }
            Self::Opaque => InvocationWord::Opaque,
        }
    }
}

/// An owned, target-neutral result from registry invocation resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryInvocationResolution {
    /// The registry selected complete common facts for the invocation.
    Resolved(Box<InvocationFacts>),
    /// The command head could not safely select a registry descriptor.
    Unresolved(OwnedInvocationResolutionUnresolved),
}

/// A typed unresolved command-head outcome retained across the registry's
/// borrowing boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedInvocationResolutionUnresolved {
    /// The command head was substituted, expanded, or opaque.
    ComputedHead {
        /// Source-word knowledge that prevented registry lookup.
        word_kind: InvocationWordKind,
    },
    /// A literal head was absent for the explicitly selected dialect profile.
    UnknownLiteralHead {
        /// Literal source spelling consulted in the registry.
        spelling: String,
    },
    /// Point-specific lookup cannot prove one live registry implementation.
    UnprovedBinding {
        /// Original command spelling, retained for presentation only.
        spelling: String,
        /// At least one represented execution path may have no command.
        may_be_absent: bool,
        /// Some execution path has no bounded target implementation.
        unknown: bool,
    },
    /// A selected semantic implementation requires an original private slot
    /// and dispatch mapping whose live identity is not established.
    UnprovedNativeImplementationLookup {
        /// Exact registry-authored lookup; catalogue presence is insufficient.
        lookup: tcl_registry::native_compilation::NativeCompilerImplementationLookup,
    },
}

impl OwnedInvocationResolutionUnresolved {
    fn from_registry(unresolved: InvocationResolutionUnresolved<'_>) -> Self {
        match unresolved {
            InvocationResolutionUnresolved::ComputedHead { word_kind } => {
                Self::ComputedHead { word_kind }
            }
            InvocationResolutionUnresolved::UnknownLiteralHead { spelling } => {
                Self::UnknownLiteralHead {
                    spelling: spelling.to_owned(),
                }
            }
        }
    }
}

/// Why no registry invocation outcome could be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryInvocationDecline {
    /// The semantic word sequence contained no command head.
    MissingCommandHead,
    /// The registry violated its structured-resolution contract by producing
    /// neither a resolved invocation nor an unresolved reason.
    IncompleteResolution,
}

/// Convert one compiler source word to the registry's value-conservative
/// invocation vocabulary.
///
/// Literal and braced-literal words expose their Tcl value. Substitutions and
/// compound templates contribute exactly one argv word but never expose their
/// source spelling as a value. Expansion keeps only its argv-shape fact, and
/// compatibility recovery remains opaque.
#[must_use]
pub fn invocation_word(word: &WordExpr) -> InvocationWord<'_> {
    match word {
        WordExpr::Literal { text, .. } | WordExpr::BracedLiteral { text, .. } => {
            InvocationWord::Literal(text)
        }
        WordExpr::Variable { .. }
        | WordExpr::CommandSubstitution { .. }
        | WordExpr::Template { .. } => InvocationWord::Dynamic,
        WordExpr::Expand { .. } => InvocationWord::Expanded,
        WordExpr::Opaque { .. } => InvocationWord::Opaque,
    }
}

/// Preserve a proved non-option prefix without exposing a substituted value.
/// Known evaluated bytes take precedence; template prefix bytes are decoded
/// under the original ingress escape grammar.
#[must_use]
pub fn invocation_word_with_source<'a>(
    source: &WordExpr,
    effective: &'a EffectiveInvocationWord,
    escapes: EscapeSyntax,
) -> InvocationWord<'a> {
    if matches!(effective, EffectiveInvocationWord::Dynamic)
        && let WordExpr::Template { parts, .. } = source
    {
        let prefix: String = parts
            .iter()
            .take_while(|part| matches!(part, WordPart::Text { .. }))
            .filter_map(|part| match part {
                WordPart::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        if tcl_syntax::backslash::decode_bytes_in(prefix.as_bytes(), escapes)
            .first()
            .is_some_and(|first| *first != b'-')
        {
            return InvocationWord::DynamicNonOption;
        }
    }
    effective.as_registry_word()
}

/// Evaluate the statically-known portion of a source word under the active
/// escape grammar. Braced words keep their backslashes except for the
/// dialect-defined line-continuation collapse.
#[must_use]
pub fn effective_invocation_word(
    word: &WordExpr,
    escapes: EscapeSyntax,
    word_rules: WordValueRules,
) -> EffectiveInvocationWord {
    match word {
        WordExpr::Literal { text, .. } => EffectiveInvocationWord::Literal(text.clone()),
        WordExpr::BracedLiteral { text, .. } => {
            EffectiveInvocationWord::Literal(word_rules.collapse_braced_word(text).into_owned())
        }
        WordExpr::Template { parts, .. }
            if parts
                .iter()
                .all(|part| matches!(part, WordPart::Text { .. })) =>
        {
            let raw: String = parts
                .iter()
                .map(|part| match part {
                    WordPart::Text { text, .. } => text.as_str(),
                    _ => unreachable!("guard admits text parts only"),
                })
                .collect();
            EffectiveInvocationWord::from_bytes(&tcl_syntax::backslash::decode_bytes_in(
                raw.as_bytes(),
                escapes,
            ))
        }
        WordExpr::Template { .. } => evaluated_array_element_root(word, escapes)
            .map_or(EffectiveInvocationWord::Dynamic, |root| {
                EffectiveInvocationWord::ArrayElementName { root }
            }),
        WordExpr::Variable { .. } | WordExpr::CommandSubstitution { .. } => {
            EffectiveInvocationWord::Dynamic
        }
        WordExpr::Expand { .. } => EffectiveInvocationWord::Expanded,
        WordExpr::Opaque { .. } => EffectiveInvocationWord::Opaque,
    }
}

fn evaluated_array_element_root(word: &WordExpr, escapes: EscapeSyntax) -> Option<String> {
    let WordExpr::Template { parts, .. } = word else {
        return None;
    };
    if parts
        .iter()
        .any(|part| matches!(part, WordPart::Opaque { .. }))
    {
        return None;
    }
    let prefix: String = parts
        .iter()
        .take_while(|part| matches!(part, WordPart::Text { .. }))
        .filter_map(|part| match part {
            WordPart::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let suffix: String = parts
        .iter()
        .rev()
        .take_while(|part| matches!(part, WordPart::Text { .. }))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .filter_map(|part| match part {
            WordPart::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let prefix_bytes = tcl_syntax::backslash::decode_bytes_in(prefix.as_bytes(), escapes);
    let suffix_bytes = tcl_syntax::backslash::decode_bytes_in(suffix.as_bytes(), escapes);
    let prefix = std::str::from_utf8(&prefix_bytes).ok()?;
    let suffix = std::str::from_utf8(&suffix_bytes).ok()?;
    if !suffix.ends_with(')') {
        return None;
    }
    let opening = prefix.find('(')?;
    Some(prefix[..opening].to_owned())
}

/// Convert a command-token snapshot into source-aware effective argv facts.
#[must_use]
pub fn effective_command_arguments(
    tokens: &CommandTokens,
    escapes: EscapeSyntax,
    word_rules: WordValueRules,
) -> Vec<EffectiveInvocationWord> {
    tokens
        .words()
        .get(1..)
        .unwrap_or_default()
        .iter()
        .map(|word| effective_invocation_word(word, escapes, word_rules))
        .collect()
}

/// Borrow source-aware post-head words from a segmented command.
///
/// The segmenter's compatibility text is still the literal value bridge for
/// callers that do not need escape decoding. Its lossless fragment list is
/// what prevents a `$word` / `[command]` / `{*}word` from becoming a literal
/// option or operand during a registry query, while retaining the distinct
/// safe shape of `prefix-$word`: it remains dynamic as a value but cannot
/// start a leading option.
#[must_use]
pub fn segmented_command_arguments(command: &SegmentedCommand) -> Vec<InvocationWord<'_>> {
    command
        .texts
        .iter()
        .enumerate()
        .skip(1)
        .map(|(index, text)| {
            if command
                .expand_word
                .as_ref()
                .and_then(|markers| markers.get(index))
                == Some(&true)
            {
                return InvocationWord::Expanded;
            }
            let Some(fragments) = command.word_fragments.get(index) else {
                return InvocationWord::Opaque;
            };
            if fragments.iter().any(|fragment| {
                matches!(
                    fragment.token.kind,
                    tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
                )
            }) {
                // A non-empty literal prefix before the first substitution
                // proves the evaluated word cannot begin with `-`. Keep that
                // narrower source fact so option-dependent descriptors may
                // still locate its positional embedded language; it is never
                // exposed as a literal value to registry hooks.
                let literal_prefix = fragments
                    .iter()
                    .take_while(|fragment| {
                        !matches!(
                            fragment.token.kind,
                            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
                        )
                    })
                    .map(|fragment| fragment.text.as_str())
                    .collect::<String>();
                // `word_piece` retains a quoted word's opening quote in an
                // empty leading `Esc` fragment, and a leading backslash can
                // decode to `-`; neither proves a non-option value. Every
                // other direct first character is stable under Tcl word
                // substitution.
                let proves_non_option = literal_prefix
                    .chars()
                    .next()
                    .is_some_and(|first| !matches!(first, '-' | '\\' | '"'));
                if proves_non_option {
                    InvocationWord::DynamicNonOption
                } else {
                    InvocationWord::Dynamic
                }
            } else {
                InvocationWord::Literal(text)
            }
        })
        .collect()
}

/// Resolve semantic source words through the registry in `context`.
///
/// The borrowed registry resolution is materialised immediately into owned
/// facts or an owned typed unresolved outcome, making this suitable for
/// common IR and analysis state.
///
/// `context` is `None` for a caller that carries no environment (a unit
/// harness, a shape-only query); the selection is then dialect-blind, exactly
/// as the retired `None` argument behaved.
pub fn resolve_word_exprs(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    words: &[WordExpr],
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    let dialect = context
        .and_then(|context| context.context().environment.point())
        .map(tcl_registry::InvocationDialect::of_point)
        .or_else(|| {
            registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile)
        });
    resolve_word_exprs_with_dialect(registry, context, words, dialect)
}

/// Resolve source words under explicitly retained interpreter policies. This
/// keeps escape and frame grammars independent of catalogue availability.
pub fn resolve_word_exprs_with_dialect(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    words: &[WordExpr],
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_word_exprs_with_dialect_and_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        words,
        dialect,
    )
}

fn resolve_word_exprs_with_dialect_and_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    words: &[WordExpr],
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    if words.is_empty() {
        return Err(RegistryInvocationDecline::MissingCommandHead);
    }
    let effective: Option<Vec<_>> = dialect.map(|dialect| {
        words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect()
    });
    let facts: Vec<_> = if let Some(effective) = &effective {
        effective
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect()
    } else {
        words.iter().map(invocation_word).collect()
    };
    resolve_registry_words_in_realm_with_metadata_context(
        registry,
        context,
        &facts,
        dialect,
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
}

pub(crate) fn resolve_registry_words_in_realm(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    facts: &[InvocationWord<'_>],
    dialect: Option<tcl_registry::InvocationDialect>,
    realm: tcl_dialect::model::InvocationRealm,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_registry_words_in_realm_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        facts,
        dialect,
        realm,
    )
}

pub(crate) fn resolve_registry_words_in_realm_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    facts: &[InvocationWord<'_>],
    dialect: Option<tcl_registry::InvocationDialect>,
    realm: tcl_dialect::model::InvocationRealm,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return Err(RegistryInvocationDecline::IncompleteResolution);
    }
    let Some(head) = facts.first().copied() else {
        return Err(RegistryInvocationDecline::MissingCommandHead);
    };
    let mut invocation = InvocationWords::structured(head, &facts[1..]);
    if let Some(dialect) = dialect {
        invocation = invocation.with_dialect(dialect);
    }
    let resolution =
        tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
            registry,
            context.map(InvocationMetadataContext::context),
            invocation,
            realm,
        );
    if let Some(resolved) = resolution.resolved() {
        return Ok(RegistryInvocationResolution::Resolved(Box::new(
            resolved.facts(),
        )));
    }
    if context.is_none_or(|context| !context.is_actual())
        && let Some(facts) = registry.native_registration_invocation_facts(invocation)
    {
        return Ok(RegistryInvocationResolution::Resolved(Box::new(facts)));
    }
    let Some(unresolved) = resolution.unresolved() else {
        return Err(RegistryInvocationDecline::IncompleteResolution);
    };
    Ok(RegistryInvocationResolution::Unresolved(
        OwnedInvocationResolutionUnresolved::from_registry(unresolved),
    ))
}

/// Resolve a source command-token snapshot through the registry.
///
/// [`CommandTokens::words`] is the compiler's canonical ordered source-word
/// representation. This adapter never falls back to the lossy argv text
/// arrays, so dynamic words cannot become accidental literal command or
/// subcommand values.
pub fn resolve_command_tokens(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_command_tokens_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

pub(crate) fn resolve_command_tokens_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    let Some(binding) = &tokens.source_binding else {
        let dialect = context
            .and_then(|context| context.context().environment.point())
            .map(tcl_registry::InvocationDialect::of_point)
            .or_else(|| {
                registry
                    .profile()
                    .map(tcl_registry::InvocationDialect::of_profile)
            });
        let resolution = resolve_word_exprs_with_dialect_and_metadata_context(
            registry,
            context,
            tokens.words(),
            dialect,
        )?;
        let mut resolution = require_native_implementation_lookup(tokens, resolution, dialect);
        if let RegistryInvocationResolution::Resolved(facts) = &mut resolution {
            let words = (0..tokens.words().len().saturating_sub(1))
                .map(|_| InvocationWord::Dynamic)
                .collect::<Vec<_>>();
            let mut arguments = tcl_registry::InvocationArguments::structured(&words);
            if let Some(dialect) = dialect {
                arguments = arguments.with_dialect(dialect);
            }
            crate::command_binding::object_callbacks::refine_facts(facts, arguments, None);
        }
        return Ok(resolution);
    };
    let Some(target) = binding
        .proved_execution_target()
        .filter(|target| target.registry_backed)
    else {
        return Ok(RegistryInvocationResolution::Unresolved(
            OwnedInvocationResolutionUnresolved::UnprovedBinding {
                spelling: tokens.argv_texts.first().cloned().unwrap_or_default(),
                may_be_absent: binding.may_be_absent,
                unknown: binding.unknown,
            },
        ));
    };
    // Selection above also establishes why an effective projection exists.
    let _ = target;
    let effective =
        effective_command_words(tokens).ok_or(RegistryInvocationDecline::IncompleteResolution)?;
    resolve_effective_tokens_with_metadata_context(
        registry,
        context,
        tokens,
        &effective,
        binding.variable_context.invocation_dialect.or_else(|| {
            registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile)
        }),
    )
}

fn resolve_effective_tokens_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
    effective: &EffectiveCommandWords,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    let frozen = frozen_argument_words(tokens, effective);
    let values: Vec<_> = effective
        .words
        .iter()
        .enumerate()
        .map(|(index, word)| {
            if let Some(value) = index
                .checked_sub(1)
                .and_then(|argument| frozen.get(argument))
            {
                return value.clone();
            }
            dialect.map_or_else(
                || match invocation_word(word) {
                    InvocationWord::Literal(value) => {
                        EffectiveInvocationWord::Literal(value.to_owned())
                    }
                    InvocationWord::KnownBytes(value) => EffectiveInvocationWord::from_bytes(value),
                    InvocationWord::Expanded => EffectiveInvocationWord::Expanded,
                    InvocationWord::Opaque => EffectiveInvocationWord::Opaque,
                    _ => EffectiveInvocationWord::Dynamic,
                },
                |dialect| {
                    effective_invocation_word(
                        word,
                        dialect.lexer_grammar.escapes,
                        dialect.word_values,
                    )
                },
            )
        })
        .collect();
    let facts: Vec<_> = values
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect();
    let realm = tokens
        .source_binding
        .as_ref()
        .and_then(crate::command_binding::SourceInvocationBinding::invocation_realm)
        .unwrap_or_default();
    let resolution = resolve_registry_words_in_realm_with_metadata_context(
        registry, context, &facts, dialect, realm,
    )?;
    let mut resolution = require_native_implementation_lookup(tokens, resolution, dialect);
    if let RegistryInvocationResolution::Resolved(resolved) = &mut resolution {
        let mut arguments = tcl_registry::InvocationArguments::structured(&facts[1..]);
        if let Some(dialect) = dialect {
            arguments = arguments.with_dialect(dialect);
        }
        crate::command_binding::object_callbacks::refine_facts(
            resolved,
            arguments,
            tokens.source_binding.as_ref(),
        );
    }
    Ok(resolution)
}

/// Private ensemble implementation proof belongs to the semantic adapter,
/// including callers that carry source words without an interpreter snapshot.
/// Such callers may query syntax/assistance separately; they cannot borrow the
/// public ensemble's identity to specialize an unproved private implementation.
fn require_native_implementation_lookup(
    tokens: &CommandTokens,
    resolution: RegistryInvocationResolution,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> RegistryInvocationResolution {
    if tokens
        .source_binding
        .as_ref()
        .is_some_and(|binding| binding.named_invocation().is_some())
    {
        return resolution;
    }
    if let RegistryInvocationResolution::Resolved(facts) = &resolution
        && let Some(lookup) = match facts.normal_handler_implementation_lookup(dialect) {
            tcl_registry::native_compilation::NormalHandlerImplementationLookup::Required(
                lookup,
            ) => Some(lookup),
            _ => dialect.and_then(|dialect| {
                facts
                    .native_compilation
                    .and_then(|native| native.implementation_lookup(dialect))
            }),
        }
        && !tokens
            .source_binding
            .as_ref()
            .is_some_and(|binding| binding.proves_native_implementation_lookup(&lookup))
    {
        return RegistryInvocationResolution::Unresolved(
            OwnedInvocationResolutionUnresolved::UnprovedNativeImplementationLookup { lookup },
        );
    }
    resolution
}

/// Purpose-limited possible body inventory of a converged handler. It exposes
/// no registry effects, purity, completion or executable opcode permission.
pub struct PossibleBodyInvocation {
    /// Logical source grammar of this selected layout, independently of runtime dispatch.
    pub dialect: Option<tcl_registry::InvocationDialect>,
    /// Registry-authored topology of possible script entry.
    pub topology: tcl_registry::native_compilation::PossibleBodyTopology,
    /// Frozen native expression operands, preserving unknown evaluated values.
    pub condition_values: Vec<(usize, Option<String>)>,
    /// Original/alias operand correspondence; original words still evaluate once.
    pub effective: EffectiveCommandWords,
}

/// Completion of the selected normal handler for diagnostic flow only.
/// Native compiler uncertainty and its independent pre-entry failure remain in
/// the execution carrier; this route cannot license CFG pruning or an opcode.
#[must_use]
pub fn normal_handler_completion_route(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> tcl_registry::completion_route::InvocationCompletionRoute {
    resolved_handler_invocation(registry, context, tokens).map_or(
        tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
        |invocation| invocation.completion_route(registry),
    )
}

/// Procedure caller operands of a converged handler, independently of its
/// compiler protocol. This carries no effects of the called procedure.
pub struct NormalUserProcedureInvocation {
    /// Frozen procedure name at the handler's selected operand.
    pub target: String,
    /// Frozen actual values passed to the procedure, preserving unknown slots.
    pub arguments: Vec<Option<String>>,
    /// Actual runtime dispatch uncertainty, separate from compiler uncertainty.
    pub unknown_runtime: bool,
}

/// Project only an authored converged procedure-caller contract. Catalogue
/// spelling and an unknown actual command binding cannot provide this proof.
#[must_use]
pub fn normal_user_procedure_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<NormalUserProcedureInvocation> {
    normal_user_procedure_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

pub(crate) fn normal_user_procedure_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<NormalUserProcedureInvocation> {
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let invocation = resolved_handler_invocation_with_metadata_context(registry, context, tokens)?;
    let layout = invocation.with_argument_words(|words| {
        invocation
            .facts
            .successful_handler_user_procedure_call(words.arguments())
    })?;
    Some(NormalUserProcedureInvocation {
        target: invocation.argument_literal(layout.target_at)?,
        arguments: (layout.arguments_from..invocation.arguments.len())
            .map(|index| invocation.argument_literal(index))
            .collect(),
        unknown_runtime: binding.unknown || binding.may_be_absent,
    })
}

/// Query candidate bodies independently of native compiler selection. Only the
/// shared source kernel's converged handler and exact frozen argv/frame contract
/// can supply this inventory; written spellings do not provide fallback proof.
#[must_use]
pub fn possible_body_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<PossibleBodyInvocation> {
    possible_body_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Candidate handler-body topology under the supplied complete availability.
/// The original handler and frame owner remain independently required.
pub(crate) fn possible_body_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<PossibleBodyInvocation> {
    if context.is_some_and(|context| !context.matches_registry(registry))
        || tokens.synthetic.is_some()
    {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let target = binding
        .proved_handler_target()
        .filter(|target| target.registry_backed)?;
    let effective = effective_words_for_target(tokens, target)?;
    let dialect = binding.variable_context.invocation_dialect;
    let RegistryInvocationResolution::Resolved(facts) =
        resolve_effective_tokens_with_metadata_context(
            registry, context, tokens, &effective, dialect,
        )
        .ok()?
    else {
        return None;
    };
    let invocation = ResolvedStatementInvocation {
        dialect,
        arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
        evaluated_arguments: frozen_argument_values(tokens, &effective),
        evaluated_words: frozen_argument_words(tokens, &effective),
        effective,
        facts,
    };
    let topology = invocation.with_argument_words(|words| {
        invocation
            .facts
            .possible_handler_body_flow(
                registry,
                words.arguments(),
                binding.variable_context.alias_frame(),
            )?
            .topology()
    })?;
    let condition_values = match &topology {
        tcl_registry::native_compilation::PossibleBodyTopology::Conditional(branches) => branches
            .iter()
            .filter_map(|(condition, _)| *condition)
            .map(|argument| (argument, invocation.argument_literal(argument)))
            .collect(),
        _ => Vec::new(),
    };
    Some(PossibleBodyInvocation {
        dialect: invocation.dialect,
        topology,
        condition_values,
        effective: invocation.effective,
    })
}

/// Conditional topology from an original declaration's owned invocation
/// layout. This is lexical diagnostic advice only; it grants neither entered
/// phases nor successful dispatch and retains every runtime residual.
pub(crate) fn conditional_body_topology_advice_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
) -> Option<PossibleBodyInvocation> {
    if !context.matches_registry(registry) {
        return None;
    }
    conditional_runtime_body_topology_advice(registry, context, tokens, bindings).or_else(|| {
        original_compilation_body_topology_advice(registry, context, tokens).or_else(|| {
            original_layout_body_topology_advice(
                registry,
                context,
                tokens,
                &bindings.declaration_operand_layout_advice(tokens)?,
            )
        })
    })
}

fn conditional_runtime_body_topology_advice(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
) -> Option<PossibleBodyInvocation> {
    let binding = tokens.source_binding.as_ref()?;
    let invocation = if let Some(owned) =
        owned_body_layout_invocation(registry, context, tokens, bindings)
    {
        owned.invocation
    } else {
        // Known candidates retain lexical possibilities alongside an opaque
        // alternative. They cannot publish entered phases or close dispatch.
        let site = binding.invocation_site()?;
        native_compiler_replay_source(tokens, site)?;
        let entry = bindings.conditional_body_entry_at(&site.source, site.offset)?;
        let mut conditional = tokens.clone();
        conditional.source_binding =
            Some(bindings.conditional_invocation_at_source(&entry, site.offset));
        if !entry.owns_invocation(conditional.source_binding.as_ref()?) {
            return None;
        }
        let prepared = prepare_body_candidates(registry, context, &conditional)?;
        let mut unanimous = None;
        for candidate in &prepared.candidates {
            let selected = prepared_body_invocation(registry, context, &conditional, candidate)?;
            if unanimous
                .as_ref()
                .is_some_and(|previous| previous != &selected)
            {
                return None;
            }
            unanimous = Some(selected);
        }
        if let Some(invocation) = unanimous {
            invocation
        } else {
            return original_compilation_body_topology_advice(registry, context, tokens);
        }
    };
    let topology = invocation.with_argument_words(|words| {
        invocation
            .facts
            .possible_handler_body_flow(
                registry,
                words.arguments(),
                binding.variable_context.alias_frame(),
            )?
            .topology()
    })?;
    let condition_values = match &topology {
        tcl_registry::native_compilation::PossibleBodyTopology::Conditional(branches) => branches
            .iter()
            .filter_map(|(condition, _)| *condition)
            .map(|argument| (argument, invocation.argument_literal(argument)))
            .collect(),
        _ => Vec::new(),
    };
    Some(PossibleBodyInvocation {
        dialect: invocation.dialect,
        topology,
        condition_values,
        effective: invocation.effective,
    })
}

/// An unchanged original expression operand for conditional diagnostics only.
/// Neither a normal handler nor successful operand evaluation is asserted.
pub(crate) struct OriginalExpressionOperandAdvice {
    pub(crate) expression: crate::expr_ast::ExprNode,
    pub(crate) evaluation: tcl_registry::conditional_expression::ConditionalExpressionEvaluation,
    pub(crate) pool: tcl_registry::conditional_expression::ConditionalExpressionPoolState,
    pub(crate) expression_base: u32,
    pub(crate) expression_text: String,
    pub(crate) lookup_closed: bool,
    pub(crate) frame: crate::var_resolve::VariableExecutionFrame,
    pub(crate) namespace_key: crate::command_binding::SourceNamespaceKey,
    pub(crate) span: tcl_lexer::Span,
}

/// Original declaration/compiler lookup candidates must agree on the expression
/// operation and the same written operand. Runtime uncertainty remains intact.
pub(crate) fn original_expression_operand_advice(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<OriginalExpressionOperandAdvice> {
    original_expression_operand_advice_selected_word(registry, tokens, None)
}

/// Conditional checked expression topology for one actual written operand.
/// The selected descriptor owns its Expression role; source geometry alone
/// supplies neither this role nor an evaluation or successful handler.
pub(crate) fn original_expression_operand_advice_for_word(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    written: usize,
) -> Option<OriginalExpressionOperandAdvice> {
    original_expression_operand_advice_selected_word(registry, tokens, Some(written))
}

/// Independently selected conditional expression grammar at one unchanged
/// original written operand. This is a parser purpose, never Normal or edits.
pub(crate) struct OriginalExpressionContextAdvice {
    pub(crate) parser: tcl_syntax::expr::parser::ExprParseContext,
    pub(crate) dialect: tcl_registry::InvocationDialect,
    pub(crate) written: usize,
    pub(crate) lookup_closed: bool,
    pool: tcl_registry::conditional_expression::ConditionalExpressionPoolState,
    frame: crate::var_resolve::VariableExecutionFrame,
    namespace_key: crate::command_binding::SourceNamespaceKey,
}

/// Literal native bytes and logical source text borrow the same original
/// candidate/role/ordinal/configuration selector; neither reselects a parser.
pub(crate) fn original_literal_expression_context_advice(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<OriginalExpressionContextAdvice> {
    original_expression_context_advice_selected_word(registry, tokens, None)
}

fn original_expression_context_advice_selected_word(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    written: Option<usize>,
) -> Option<OriginalExpressionContextAdvice> {
    let binding = tokens.source_binding.as_ref()?;
    let advice = binding
        .declaration_operand_layout_advice(tokens)
        .or_else(|| binding.original_compilation_lookup_advice(tokens))?;
    if advice.has_opaque_handler_alternatives() {
        return None;
    }
    let context = body_assistance_context(registry, tokens)?;
    let dialect = advice.dialect();
    let mut original = None;
    for target in advice.targets() {
        if !target.registry_backed {
            return None;
        }
        let effective =
            compose_original_effective_words(tokens, &target.command, &target.prepended)?;
        let words: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let words: Vec<_> = words
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) = resolve_registry_words_in_realm(
            registry,
            Some(context),
            &words,
            Some(dialect),
            advice.realm(),
        )
        .ok()?
        else {
            return None;
        };
        if !facts.arg_roles_complete {
            return None;
        }
        let selected = if let Some(written) = written {
            let mut selected = facts.arg_roles.iter().filter_map(|(argument, role)| {
                if *role != tcl_registry::arg_role::ArgRole::Expr { return None; }
                let effective_index = facts.argument_offset.checked_add(usize::from(*argument))?.checked_add(1)?;
                matches!(effective.origins.get(effective_index), Some(InvocationWordOrigin::Written(index))
                    if *index == written).then_some(effective_index)
            });
            let first = selected.next()?;
            if selected.next().is_some() {
                return None;
            }
            first
        } else {
            if facts.successful_handler
                != Some(
                    tcl_registry::native_compilation::SuccessfulHandlerSpec::ExpressionArguments,
                )
                || effective.words.len() != 2
            {
                return None;
            }
            1
        };
        let InvocationWordOrigin::Written(index) = *effective.origins.get(selected)? else {
            return None;
        };
        if original.is_some_and(|previous| previous != index) {
            return None;
        }
        original = Some(index);
    }
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let mut parser = dialect.expression_parse_context(None);
    parser.lexer_grammar = config.grammar_over(parser.lexer_grammar);
    Some(OriginalExpressionContextAdvice {
        parser,
        dialect,
        written: original?,
        lookup_closed: advice.closed_lookup(),
        pool: advice.expression_pool_state(),
        frame: advice.frame().clone(),
        namespace_key: advice.namespace().clone(),
    })
}

fn original_expression_operand_advice_selected_word(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    written: Option<usize>,
) -> Option<OriginalExpressionOperandAdvice> {
    let context = original_expression_context_advice_selected_word(registry, tokens, written)?;
    let binding = tokens.source_binding.as_ref()?;
    let index = context.written;
    let (WordExpr::BracedLiteral { text, source } | WordExpr::Literal { text, source }) =
        tokens.words().get(index)?
    else {
        return None;
    };
    if source.provenance != crate::ir::Provenance::Source {
        return None;
    }
    let base = crate::lowering_hooks::word_content_base(
        *tokens.argv.get(index)?,
        *tokens.single_token_word.get(index)?,
        text,
    )?;
    let end = base.checked_add(u32::try_from(text.len()).ok()?)?;
    let site = binding.invocation_site()?;
    if site
        .source
        .source_image()
        .bytes()
        .get(base as usize..end as usize)
        != Some(text.as_bytes())
    {
        return None;
    }
    let parser = context.parser;
    let evaluation =
        tcl_registry::conditional_expression::ConditionalExpressionEvaluation::prepare(
            text, &parser,
        )?;
    let expression = evaluation.tree().clone();
    Some(OriginalExpressionOperandAdvice {
        expression,
        evaluation,
        pool: context.pool,
        expression_base: base,
        expression_text: text.clone(),
        lookup_closed: context.lookup_closed,
        frame: context.frame,
        namespace_key: context.namespace_key,
        span: tcl_lexer::Span::new(
            tokens.words().first()?.source().span.start(),
            source.span.end(),
        ),
    })
}

/// Original compilation-table candidates select lexical topology only. Values
/// are decoded from unchanged source words under the captured logical grammar;
/// this never supplies runtime argument values or selected normal facts.
fn original_compilation_body_topology_advice<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Option<PossibleBodyInvocation> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }

    let advice = tokens
        .source_binding
        .as_ref()?
        .original_compilation_lookup_advice(tokens)?;
    original_layout_body_topology_advice(registry, context, tokens, &advice)
}

fn original_layout_body_topology_advice<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
    advice: &crate::command_binding::OriginalCompilationLookupAdvice,
) -> Option<PossibleBodyInvocation> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }

    let dialect = advice.dialect();
    let mut unanimous = None;
    for target in advice.targets() {
        let effective =
            compose_original_effective_words(tokens, &target.command, &target.prepended)?;
        let words: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let registry_words: Vec<_> = words
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm_with_metadata_context(
                registry,
                Some(context),
                &registry_words,
                Some(dialect),
                advice.realm(),
            )
            .ok()?
        else {
            return None;
        };
        let invocation =
            InvocationWords::structured(*registry_words.first()?, &registry_words[1..])
                .with_dialect(dialect);
        let frame = if advice.alias_frame() == tcl_registry::VariableAliasFrame::Unknown {
            tokens
                .source_binding
                .as_ref()?
                .declaration_alias_frame_advice(tokens)?
        } else {
            advice.alias_frame()
        };
        let topology = facts
            .possible_handler_body_flow(registry, invocation.arguments(), frame)?
            .topology()?;
        let condition_values = match &topology {
            tcl_registry::native_compilation::PossibleBodyTopology::Conditional(branches) => {
                branches
                    .iter()
                    .filter_map(|(condition, _)| *condition)
                    .map(|argument| {
                        (
                            argument,
                            words.get(argument + 1).and_then(|word| {
                                if let EffectiveInvocationWord::Literal(value) = word {
                                    Some(value.clone())
                                } else {
                                    None
                                }
                            }),
                        )
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        let selected = PossibleBodyInvocation {
            dialect: Some(dialect),
            topology,
            condition_values,
            effective,
        };
        if unanimous
            .as_ref()
            .is_some_and(|previous: &PossibleBodyInvocation| {
                previous.topology != selected.topology
                    || previous.condition_values != selected.condition_values
                    || previous.effective != selected.effective
            })
        {
            return None;
        }
        unanimous = Some(selected);
    }
    unanimous
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::StateTransition;

    use crate::ir::{SourceSite, WordOpacity};
    use crate::segmenter::segment_commands;

    fn logical_source_tokens(
        source: &str,
        registry: &CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> CommandTokens {
        let image = tcl_lexer::SourceImage::document(source);
        let config = input.lexer_config();
        let bindings =
            crate::command_binding::SourceCommandBindings::analyse_image_in_frame_with_options(
                &image,
                &crate::var_resolve::VariableExecutionFrame::Unknown,
                config,
                registry,
                crate::command_binding::SourceAnalysisOptions::for_logical_source(input).unwrap(),
            )
            .unwrap();
        let segment =
            crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
                .unwrap()
                .pop()
                .unwrap();
        let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &segment);
        bindings.stamp_original_tokens(&mut tokens);
        tokens
    }

    #[test]
    fn logical_source_structure_uses_original_lookup_without_a_physical_frame() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Conditional source roles remain separate from Native handler and frame entry.
        let owner =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&owner),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let registry = owner.commands();
        let tokens = logical_source_tokens("if {1} {set value VALUE}", registry, &input);
        let binding = tokens.source_binding.as_ref().unwrap();
        let advice = binding.original_compilation_lookup_advice(&tokens).unwrap();
        assert!(!advice.closed_lookup());
        assert!(advice.closed_logical_source_lookup());
        let metadata = InvocationMetadataContext::for_analysis_input(registry, &input).unwrap();
        let shape =
            logical_structured_invocation_with_metadata_context(registry, metadata, &tokens, None)
                .expect("genuine Logical source body shape");
        assert_eq!(
            shape.lowering_hook(),
            Some(tcl_registry::hooks::LoweringHookId::If)
        );
        assert!(
            shape
                .written_roles()
                .contains(&(1, tcl_registry::ArgRole::Body))
        );
        assert!(
            resolved_handler_invocation_with_metadata_context(registry, Some(metadata), &tokens)
                .is_none()
        );
        assert!(binding.native_compilation_admission.is_none());
    }

    #[test]
    fn logical_source_structure_refuses_old_availability_stale_grammar_and_replacements() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let owner =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&owner),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let registry = owner.commands();
        let tokens = logical_source_tokens("try {set value VALUE}", registry, &input);
        let current = InvocationMetadataContext::for_analysis_input(registry, &input).unwrap();
        assert!(original_declared_structured_invocation(registry, current, &tokens).is_some());
        let older = tcl_registry::model::ingress::resolve_environment("tcl8.4")
            .default_context_registry()
            .with_command_store(registry.snapshot().shared_registry());
        let older = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(older),
            input.lexer_config(),
        );
        let older = InvocationMetadataContext::for_analysis_input(registry, &older).unwrap();
        assert!(original_declared_structured_invocation(registry, older, &tokens).is_none());
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        let stale = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        let stale = InvocationMetadataContext::for_analysis_input(registry, &stale).unwrap();
        assert!(original_declared_structured_invocation(registry, stale, &tokens).is_none());
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let foreign = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            foreign,
            input.lexer_config(),
        );
        assert!(InvocationMetadataContext::for_analysis_input(registry, &foreign).is_none());
        let replaced = logical_source_tokens(
            "proc try {args} {}; try {set value VALUE}",
            registry,
            &input,
        );
        assert!(original_declared_structured_invocation(registry, current, &replaced).is_none());
        let mut missing = tokens.clone();
        missing.source_binding = None;
        assert!(original_declared_structured_invocation(registry, current, &missing).is_none());
    }

    #[test]
    fn logical_source_shape_cannot_substitute_for_missing_native_entry() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let owner =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&owner),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let registry = owner.commands();
        let tokens = logical_source_tokens("if {1} {set value VALUE}", registry, &input);
        let native_profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let native = crate::analyser::ResolvedAnalysisInput::new(
            native_profile,
            native_profile,
            std::sync::Arc::clone(&owner),
            input.lexer_config(),
        );
        let native = InvocationMetadataContext::for_analysis_input(registry, &native).unwrap();
        assert!(original_declared_structured_invocation(registry, native, &tokens).is_none());
        assert!(
            resolved_handler_invocation_with_metadata_context(registry, Some(native), &tokens)
                .is_none()
        );
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .native_compilation_admission
                .is_none()
        );
    }

    fn test_context() -> SemanticContext {
        SemanticContext::for_environment("tcl8.6")
    }

    fn literal(text: &str) -> WordExpr {
        WordExpr::Literal {
            text: text.to_owned(),
            source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
        }
    }

    #[test]
    fn expression_operand_advice_keeps_original_sites_and_replaced_handlers_separate() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (source, expected) in [
            ("proc f {i} {set y [expr {$i + 0}]}", 1),
            (
                "interp alias {} arithmetic {} expr; proc f {i} {set y [arithmetic {$i + 0}]}",
                1,
            ),
            (
                "rename expr original_expr; proc expr args {list PLAIN}; proc f {i} {set y [expr {$i + 0}]}",
                0,
            ),
            ("proc f {i} {set y [unknown {$i + 0}]}", 0),
        ] {
            let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
                source,
                registry,
                false,
                registry.profile().unwrap(),
            );
            let mut count = 0;
            crate::ir::for_each_statement(
                &unit.ir_module.procedures["::f"].body,
                &mut |statement| {
                    for call in crate::word_subst::lifted_calls(
                        statement.tokens(),
                        tcl_lexer::LexerConfig::for_profile(registry.profile()),
                    ) {
                        if let Some(tokens) = call.tokens
                            && let Some(advice) =
                                original_expression_operand_advice(registry, &tokens)
                        {
                            assert!(
                                source
                                    .get(advice.span.as_range())
                                    .is_some_and(|text| text.contains("$i + 0"))
                            );
                            count += 1;
                        }
                    }
                },
            );
            assert_eq!(count, expected, "{source}");
        }
    }

    #[test]
    fn default_return_operand_advice_preserves_generic_calls_and_replacements() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (source, expected) in [
            ("proc make {} {return [list VALUE]}", vec!["[list VALUE]"]),
            ("proc make {} {return -code error VALUE}", vec![]),
            (
                "rename return stock_return; proc return args {list PLAIN}; proc make {} {return [list VALUE]}",
                vec![],
            ),
        ] {
            let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
                source,
                registry,
                false,
                registry.profile().unwrap(),
            );
            let procedure = &unit.ir_module.procedures["::make"];
            let mut values = Vec::new();
            crate::ir::for_each_statement(&procedure.body, &mut |statement| {
                if let Some(tokens) = procedure
                    .body
                    .retained_source_tokens_for_statement(statement)
                {
                    values.extend(
                        advisory_return_values(registry, tokens)
                            .into_iter()
                            .map(|word| word.legacy_text()),
                    );
                }
            });
            assert_eq!(values, expected, "{source}");
        }
    }

    #[test]
    fn decoded_native_byte_words_preserve_cardinality_and_checked_text() {
        let bytes = EffectiveInvocationWord::from_bytes(&[0xff]);
        assert_eq!(bytes.literal_bytes(), Some([0xff].as_slice()));
        // Implementation contract: naming.invocation.known-native-byte-values
        // docs/design/analysis/name-resolution-proofs/known-native-byte-values.md
        assert_eq!(
            bytes.as_registry_word(),
            InvocationWord::KnownBytes(&[0xff])
        );
        assert_eq!(bytes.as_registry_word().literal(), None);
        assert!(bytes.as_registry_word().has_exactly_one_argv_entry());
        let unicode = EffectiveInvocationWord::from_bytes("ÿ".as_bytes());
        assert_eq!(unicode.as_registry_word(), InvocationWord::Literal("ÿ"));
        assert_ne!(bytes, unicode);
    }

    #[test]
    fn static_command_word_decodes_only_original_static_values() {
        let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
        let rules = WordValueRules::from_config(&config);
        for (source, expected) in [
            (r"se\x74 x 1", Some("set")),
            (r"{se\x74} x 1", Some(r"se\x74")),
            ("\"set\" x 1", Some("set")),
            ("$head x 1", None),
            ("[gethead] x 1", None),
            ("{*}$head x 1", None),
        ] {
            let segment =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .pop()
                    .expect("one invocation");
            let tokens =
                CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
            assert_eq!(
                static_command_word(&tokens, config.escapes, rules).as_deref(),
                expected,
                "{source}"
            );
        }
    }

    fn last_source_tokens(source: &str) -> CommandTokens {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let bindings =
            crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .unwrap();
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
            .with_source_binding(
                bindings.invocation_at_source("ignored-authored-head", segment.span.start()),
            )
    }

    #[test]
    fn possible_name_operands_preserve_alias_prefix_and_missing_binding_residual() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut tokens = last_source_tokens("interp alias {} store {} set target; store VALUE");
        let possible = possible_variable_name_operands(registry, None, &tokens).unwrap();
        assert!(
            possible
                .operands()
                .any(|(role, word)| role == tcl_registry::ArgRole::VarWrite
                    && word == &EffectiveInvocationWord::Literal("target".to_owned()))
        );
        let mut unknown_binding = crate::command_binding::SourceInvocationBinding::default();
        unknown_binding.unknown = true;
        tokens.source_binding = Some(unknown_binding);
        let unknown = possible_variable_name_operands(registry, None, &tokens).unwrap();
        assert!(unknown.unknown_residual());
        assert!(unknown.operands().next().is_none());
        tokens.source_binding = Some(crate::command_binding::SourceInvocationBinding::default());
        let closed = possible_variable_name_operands(registry, None, &tokens).unwrap();
        assert!(!closed.unknown_residual());
        assert!(closed.operands().next().is_none());
        tokens.source_binding = None;
        assert!(possible_variable_name_operands(registry, None, &tokens).is_none());
    }

    #[test]
    fn possible_name_operands_keep_expansion_cardinality_separate_from_name_values() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        for (command, dynamic, expected_roles) in [
            ("set {*}$n VALUE", true, 2),
            ("set fixed {*}$n", false, 2),
            ("set {*}{fixed} VALUE", false, 1),
            ("set fixed VALUE EXTRA", false, 0),
        ] {
            let source = format!("proc f {{n}} {{{command}; return ok}}");
            let bindings =
                crate::command_binding::SourceCommandBindings::analyse(&source, config, registry);
            let offset = source.find(command).unwrap();
            let offset_u32 = u32::try_from(offset).unwrap();
            let segment = crate::segmenter::segment_commands_with_offset_and_config(
                &source[offset..],
                offset_u32,
                config,
            )
            .remove(0);
            let tokens = CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(&source),
                config,
                &segment,
            )
            .with_source_binding(bindings.invocation_at_source("unused-written-head", offset_u32));
            let possible = possible_variable_name_operands(registry, None, &tokens).unwrap();
            let mut operands = Vec::new();
            for operand in possible.operands() {
                if !operands.contains(&operand) {
                    operands.push(operand);
                }
            }
            assert_eq!(operands.len(), expected_roles, "{command}");
            assert_eq!(
                operands
                    .iter()
                    .any(|(_, word)| matches!(word, EffectiveInvocationWord::Dynamic)),
                dynamic,
                "{command}"
            );
            if dynamic {
                assert!(normal_transfer_invocation(registry, None, &tokens).is_none());
            }
        }
    }

    #[test]
    fn evaluated_wrapper_retains_only_possible_output_names_and_lookup_phase() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let mut tokens = f5_source_tokens("catch {foo} ::err");
        tokens.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedWrapper);
        let possible = possible_variable_name_operands(registry, None, &tokens).unwrap();
        assert!(possible.unknown_residual());
        assert!(possible.phased_operands().any(|(role, word, phase, destroys)| {
            role == tcl_registry::ArgRole::VarWrite
                && !destroys
                && word == &EffectiveInvocationWord::Literal("::err".to_owned())
                && phase
                    == tcl_registry::native_compilation::VariableOperandBindingPhase::NormalContinuation
        }));
        assert!(normal_transfer_invocation(registry, None, &tokens).is_none());
        assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
        tokens.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedArguments);
        assert!(possible_variable_name_operands(registry, None, &tokens).is_none());

        let mut replaced = f5_source_tokens("proc catch args {return CUSTOM}; catch BODY ::err");
        replaced.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedWrapper);
        assert!(
            possible_variable_name_operands(registry, None, &replaced)
                .unwrap()
                .operands()
                .next()
                .is_none()
        );
    }

    #[test]
    fn possible_method_name_operands_do_not_create_normal_store_proof() {
        use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
        let source = "oo::class create C {method m {} {set state VALUE}}";
        let config = tcl_lexer::LexerConfig::default();
        let registry = CommandRegistry::build_default();
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V9_1,
                )),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..SourceAnalysisOptions::default()
            },
        );
        let offset = u32::try_from(source.find("set state").unwrap()).unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            "set state VALUE",
            offset,
            config,
        )
        .pop()
        .unwrap();
        let tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
                .with_source_binding(bindings.invocation_at_source("set", offset));
        let possible = possible_variable_name_operands(&registry, None, &tokens).unwrap();
        assert!(possible.unknown_residual());
        assert!(
            possible
                .operands()
                .any(|(role, word)| role == tcl_registry::ArgRole::VarWrite
                    && word == &EffectiveInvocationWord::Literal("state".to_owned()))
        );
        assert!(normal_transfer_invocation(&registry, None, &tokens).is_none());
    }

    #[test]
    fn native_operation_plan_retains_original_words_and_reports_missing_source() {
        let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
        let rules = WordValueRules::from_config(&config);
        let source = r"se\x74 value 7";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            crate::command_binding::SourceAnalysisOptions {
                invocation_dialect: Some(crate::environment_ingress::authoring_invocation_dialect(
                    registry,
                    registry.profile(),
                    config,
                )),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                    ..crate::environment_ingress::authoring_native_compilation()
                },
                ..crate::command_binding::SourceAnalysisOptions::default()
            },
        );
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .unwrap();
        let mut tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
                .with_source_binding(bindings.invocation_at_source("set", 0));
        let plan = native_operation_selection_plan(&tokens, config.escapes, rules)
            .unwrap()
            .unwrap();
        assert_eq!(plan.source, r"se\x74 value 7");
        assert_eq!(plan.requirements[0].name, "set");
        assert_eq!(
            plan.guard,
            tcl_runtime_api::CommandBindingGuard::BeforeArguments
        );
        tokens.argv_texts.pop();
        assert_eq!(
            native_operation_selection_plan(&tokens, config.escapes, rules),
            Err(NativeOperationSelectionDecline::MissingOriginalSource),
        );
        let generic = last_source_tokens("pid");
        assert_eq!(
            native_operation_selection_plan(&generic, config.escapes, rules),
            Ok(None)
        );
    }

    fn f5_source_tokens(source: &str) -> CommandTokens {
        f5_source_tokens_in_realm(source, tcl_dialect::model::InvocationRealm::RuleLoader)
    }

    fn f5_source_tokens_in_realm(
        source: &str,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> CommandTokens {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let config = tcl_lexer::LexerConfig::for_dialect("f5-irules");
        let analysis = crate::command_binding::SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            crate::command_binding::SourceAnalysisOptions {
                invocation_realm: realm,
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                    tcl_dialect::DialectProfile::find("f5-irules").expect("selected fixture entry"),
                )),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                    catch_depth: Some(0),
                    loop_depth: 0,
                },
                ..crate::command_binding::SourceAnalysisOptions::default()
            },
        );
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .unwrap();
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
            .with_source_binding(analysis.invocation_at_source("ignored", segment.span.start()))
    }

    #[test]
    fn normal_ensemble_leaf_requires_the_actual_release_worker() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
            for (command, introduced) in [
                ("string toupper hello", "tcl8.5"),
                ("binary format a* hello", "tcl8.6"),
            ] {
                let worker = if command.starts_with("string") {
                    "::tcl::string::toupper"
                } else {
                    "::tcl::binary::format"
                };
                for replaced in [false, true] {
                    let source = if replaced {
                        format!(
                            "namespace eval ::tcl::string {{}}; namespace eval ::tcl::binary {{}}; proc {worker} args {{return CUSTOM}}; {command}"
                        )
                    } else {
                        command.to_owned()
                    };
                    let bindings = crate::command_binding::SourceCommandBindings::analyse(
                        &source, config, registry,
                    );
                    let segment = crate::segmenter::segment_commands_with_offset_and_config(
                        &source, 0, config,
                    )
                    .pop()
                    .unwrap();
                    let tokens = CommandTokens::from_segmented(
                        &tcl_lexer::SourceMap::new(&source),
                        config,
                        &segment,
                    )
                    .with_source_binding(
                        bindings.invocation_at_source("ignored", segment.span.start()),
                    );
                    let expects_worker = profile != "jim" && profile >= introduced;
                    assert_eq!(
                        normal_representation_invocation(registry, None, &tokens).is_some(),
                        !replaced || !expects_worker,
                        "{profile}: {source}"
                    );
                }
            }
        }
    }

    #[test]
    fn f5_value_provider_proves_normal_representation_without_c_workers() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        for command in ["string toupper hello", "binary format a* hello"] {
            let tokens = f5_source_tokens(command);
            assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
            assert!(normal_representation_invocation(registry, None, &tokens).is_some());
            let replaced = f5_source_tokens(&format!(
                "proc {} args {{return CUSTOM}}; {command}",
                command.split_whitespace().next().unwrap()
            ));
            assert!(normal_representation_invocation(registry, None, &replaced).is_none());
        }
        let tokens = f5_source_tokens_in_realm(
            "namespace eval ::tcl::string {}; proc ::tcl::string::toupper args {return CUSTOM}; string toupper hello",
            tcl_dialect::model::InvocationRealm::InterpreterRuntime,
        );
        assert!(normal_representation_invocation(registry, None, &tokens).is_some());
        let malformed = f5_source_tokens(
            "proc ::tcl::string::toupper args {return CUSTOM}; string toupper hello",
        );
        assert!(normal_representation_invocation(registry, None, &malformed).is_none());
    }

    #[test]
    fn normal_completion_retains_handler_return_without_licensing_unknown_compilation() {
        let f5 = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let tokens = f5_source_tokens("return VALUE");
        assert!(!normal_handler_completion_route(f5, None, &tokens).normal_possible());
        assert!(resolved_tokens_invocation(f5, None, &tokens).is_none());
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .compiled_execution_unknown()
        );
        let replaced = f5_source_tokens("proc return args {set x CUSTOM}; return VALUE");
        assert!(normal_handler_completion_route(f5, None, &replaced).normal_possible());
        let c = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let tokens = last_source_tokens("return VALUE");
        assert!(!normal_handler_completion_route(c, None, &tokens).normal_possible());
        assert!(resolved_tokens_invocation(c, None, &tokens).is_some());
    }

    #[test]
    fn f5_connection_controls_preserve_following_normal_return_identity() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        for command in ["drop", "discard", "reject", "DNS::return"] {
            let tokens = f5_source_tokens(&format!("{command}; return"));
            assert!(
                !normal_handler_completion_route(registry, None, &tokens).normal_possible(),
                "{command}"
            );
            assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
            assert!(
                tokens
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .compiled_execution_unknown()
            );
        }
        let replaced =
            f5_source_tokens("proc drop {} {proc return args {set x CUSTOM}}; drop; return");
        assert!(normal_handler_completion_route(registry, None, &replaced).normal_possible());
    }

    #[test]
    fn representation_contract_preserves_handler_proof_without_opcode_proof() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let tokens = f5_source_tokens("llength {a b}");
        assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
        let normal = normal_representation_invocation(registry, None, &tokens).unwrap();
        assert_eq!(normal.argument_literal(0).as_deref(), Some("a b"));
        assert_eq!(
            normal.argument_type_hint(0).unwrap().expected,
            Some(tcl_registry::TclType::List)
        );
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .compiled_execution_unknown()
        );
    }

    #[test]
    fn unavailable_f5_string_member_cannot_donate_a_value_handler_contract() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let tokens = f5_source_tokens("string cat a b");
        assert!(normal_representation_invocation(registry, None, &tokens).is_none());
    }

    #[test]
    fn unavailable_f5_binary_member_cannot_donate_a_value_handler_contract() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let tokens = f5_source_tokens("binary encode hex DATA");
        assert!(normal_representation_invocation(registry, None, &tokens).is_none());
    }

    #[test]
    fn normal_assignment_keeps_value_words_without_an_opcode_or_custom_handler_proof() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let tokens = f5_source_tokens("set p [TCP::payload]");
        let normal = normal_representation_invocation(registry, None, &tokens).unwrap();
        let assignment = normal.value_assignment().unwrap();
        assert_eq!(assignment.name, "p");
        assert_eq!(assignment.value, tokens.words()[2]);
        assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
        for source in ["set p", "proc set args {return CUSTOM}; set p value"] {
            let tokens = f5_source_tokens(source);
            assert!(
                normal_representation_invocation(registry, None, &tokens)
                    .and_then(|normal| normal.value_assignment())
                    .is_none()
            );
        }
    }

    #[test]
    fn representation_contract_declines_replaced_and_unproved_handlers() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        for source in [
            "proc llength args {return CUSTOM}; llength {a b}",
            "set selected mystery; $selected {a b}",
        ] {
            let tokens = f5_source_tokens(source);
            assert!(normal_representation_invocation(registry, None, &tokens).is_none());
        }
    }

    #[test]
    fn normal_taint_source_properties_require_the_selected_getter() {
        use tcl_registry::taint::TaintColour;
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let getter = f5_source_tokens("HTTP::uri");
        let source = normal_taint_invocation(registry, None, &getter).unwrap();
        let colour = source.source_colour().unwrap();
        assert!(colour.contains(TaintColour::TAINTED | TaintColour::PATH_PREFIXED));
        assert!(resolved_tokens_invocation(registry, None, &getter).is_none());

        let setter = f5_source_tokens("HTTP::uri /changed");
        let colour = normal_taint_invocation(registry, None, &setter)
            .unwrap()
            .source_colour()
            .unwrap();
        assert!(colour.contains(TaintColour::TAINTED));
        assert!(!colour.contains(TaintColour::PATH_PREFIXED));
    }

    #[test]
    fn compiled_dynamic_regexp_pattern_retains_its_original_operand_slot() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let cu = crate::compilation_unit::CompilationUnit::build_for(
                "proc p {pattern subject} {regexp $pattern $subject}",
                registry,
                false,
            );
            let statement = &cu.ir_module.procedures["::p"].body.statements[0];
            let tokens = statement.tokens().unwrap();
            let normal = normal_representation_invocation(registry, None, tokens).unwrap();
            assert_eq!(
                normal.pattern_source_argument_index(registry),
                (profile != "tcl8.4").then_some(0),
                "{profile}"
            );
        }
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let tokens = last_source_tokens("regexp -about aa");
        assert_eq!(
            normal_representation_invocation(registry, None, &tokens)
                .unwrap()
                .pattern_source_argument_index(registry),
            Some(1)
        );
    }

    #[test]
    fn normal_pattern_layout_uses_closed_original_read_values() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        for (source, expected) in [
            (
                "proc p {outer inner s} {set re seed; if {$outer} {if {$inner} {set re left}}; regsub -command $re $s callback out}",
                Some(1),
            ),
            (
                "proc p {flag s} {set re seed; if {$flag} {set re -command}; regsub $re $s callback out}",
                None,
            ),
            ("proc p {re s} {regsub -command $re $s callback out}", None),
            (
                "proc p {flag s} {set re seed; if {$flag} {set re left}; proc observer args {}; trace add variable re read observer; regsub -command $re $s callback out}",
                None,
            ),
        ] {
            let cu = crate::compilation_unit::CompilationUnit::build_for(source, registry, false);
            let statement = cu.ir_module.procedures["::p"]
                .body
                .statements
                .last()
                .unwrap();
            let normal =
                normal_representation_invocation(registry, None, statement.tokens().unwrap());
            assert_eq!(
                normal
                    .as_ref()
                    .and_then(|normal| normal.pattern_source_argument_index(registry)),
                expected,
                "{source}"
            );
            if let Some(normal) = normal
                && matches!(normal.proof, NormalRepresentationProof::PatternLayout(_))
            {
                assert!(normal.argument_type_hint(1).is_none());
                assert_eq!(normal.operand_roles(), []);
                assert_eq!(
                    normal.representation_effect(),
                    tcl_registry::RepresentationEffect::None
                );
                assert!(normal.value_assignment().is_none());
            }
        }
    }

    #[test]
    fn normal_taint_numeric_results_do_not_require_opcode_proof() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let tokens = f5_source_tokens("string length VALUE");
        assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
        assert!(
            normal_taint_invocation(registry, None, &tokens)
                .unwrap()
                .is_sanitiser()
        );
        let replaced = f5_source_tokens("proc string args {return CUSTOM}; string length VALUE");
        assert!(normal_taint_invocation(registry, None, &replaced).is_none());
    }

    #[test]
    fn normal_taint_transform_requires_frozen_mapping_values() {
        use tcl_registry::taint::TaintColour;
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        for source in [
            r#"string map {"\r" "" "\n" ""} VALUE"#,
            r#"set mapping {"\r" "" "\n" ""}; string map -nocase $mapping VALUE"#,
        ] {
            let tokens = f5_source_tokens(source);
            assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
            assert_eq!(
                normal_taint_invocation(registry, None, &tokens)
                    .unwrap()
                    .transform_colour(),
                Some(TaintColour::CRLF_FREE),
                "{source}"
            );
        }
        for source in [
            r#"set mapping [HTTP::uri]; string map $mapping {"\r" "" "\n" ""}"#,
            r#"string map {"\r" "" "\n" "x\ny"} VALUE"#,
            r#"string map {"\r" "" "\n"} VALUE"#,
        ] {
            let tokens = f5_source_tokens(source);
            assert!(
                normal_taint_invocation(registry, None, &tokens)
                    .and_then(|normal| normal.transform_colour())
                    .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn normal_taint_source_properties_decline_replaced_or_unproved_handlers() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        for source in [
            "proc HTTP::uri args {return CUSTOM}; HTTP::uri",
            "set selected mystery; $selected",
        ] {
            let tokens = f5_source_tokens(source);
            assert!(normal_taint_invocation(registry, None, &tokens).is_none());
        }
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let tokens = last_source_tokens("HTTP::uri");
        assert!(normal_taint_invocation(registry, None, &tokens).is_none());
    }

    #[test]
    fn normal_handler_transfer_preserves_unknown_compiler_protocol() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let tokens = f5_source_tokens("set connection_value 17");
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .compiled_execution_unknown()
        );
        assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
        let normal = normal_transfer_invocation(registry, None, &tokens).unwrap();
        assert_eq!(normal.definition_names(), ["connection_value"]);
        assert_eq!(normal.written_argument(1), Some(1));
        let selected = &tokens.source_binding.as_ref().unwrap().variable_context;
        assert_eq!(
            normal.stored_value_literal(selected, registry).as_deref(),
            Some("17")
        );
        let mut observed = selected.as_ref().clone();
        observed.dynamic_traces = true;
        assert!(normal.stored_value_literal(&observed, registry).is_none());
        let statement = crate::ir::Statement::Call {
            span: tokens.words().first().unwrap().source().span,
            command: "set".into(),
            canonical_command: None,
            args: vec!["connection_value".into(), "17".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            foreach_groups: None,
            tokens: Some(tokens),
        };
        assert_eq!(
            crate::ssa::defs_of_with_registry(&statement, Some(registry)),
            ["connection_value"]
        );
        assert!(resolved_statement_invocation(registry, None, &statement).is_none());
    }

    #[test]
    fn normal_store_uses_frozen_target_and_post_argv_observers() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let tokens = f5_source_tokens("set n x; set x OLD; set $n [set n NEW]");
        let normal = normal_transfer_invocation(registry, None, &tokens).unwrap();
        let selected = &tokens.source_binding.as_ref().unwrap().variable_context;
        assert_eq!(normal.definition_names(), ["x"]);
        assert_eq!(
            normal.stored_value_literal(selected, registry).as_deref(),
            Some("NEW")
        );
        assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());

        let source = "proc observer {args} {set ::x CHANGED}; set x OLD; set x [trace add variable x write observer]";
        let tokens = f5_source_tokens(source);
        assert!(normal_transfer_invocation(registry, None, &tokens).is_none());
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let tokens = last_source_tokens(source);
        let normal = normal_transfer_invocation(registry, None, &tokens).unwrap();
        let selected = &tokens.source_binding.as_ref().unwrap().variable_context;
        assert!(normal.stored_value_operand(selected, registry).is_none());
        assert!(normal.stored_value_literal(selected, registry).is_none());
        assert!(resolved_tokens_invocation(registry, None, &tokens).is_some());
    }

    #[test]
    fn normal_store_resolves_actual_alias_destination_after_arguments() {
        let source = "set x OLD; set y NEW; upvar 0 x a; set a [upvar 0 y a]";
        for dialect in ["f5-irules", "tcl8.6"] {
            let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
            let tokens = if dialect == "f5-irules" {
                f5_source_tokens(source)
            } else {
                last_source_tokens(source)
            };
            let normal = normal_transfer_invocation(registry, None, &tokens).unwrap();
            let selected = &tokens.source_binding.as_ref().unwrap().variable_context;
            let places = normal.mutation_places(selected, registry);
            assert_eq!(places.len(), 1);
            let target = crate::var_resolve::resolve_place("y", selected, false, registry);
            assert_eq!(places[0].kind, crate::place::PlaceKind::Scalar);
            assert!(!places[0].observed && !places[0].dynamic);
            assert!(places[0].cell.is_some());
            assert_eq!(places[0].cell, target.cell);
            assert_eq!(
                crate::var_resolve::canonical_binding_value_name(&places[0]),
                Some("::y".to_owned())
            );
            assert_eq!(
                normal.stored_value_literal(selected, registry).as_deref(),
                Some("")
            );
            assert_eq!(
                resolved_tokens_invocation(registry, None, &tokens).is_some(),
                dialect == "tcl8.6"
            );
        }
    }

    #[test]
    fn normal_handler_transfer_rejects_mutation_and_body_licences() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        for source in [
            "set x [proc set args {return CUSTOM}]",
            "if {$flag} {set x 1}",
            "proc set args {return CUSTOM}; set x 1",
            "set x 1 extra",
        ] {
            let tokens = f5_source_tokens(source);
            assert!(
                normal_transfer_invocation(registry, None, &tokens).is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn argument_values_are_frozen_before_later_substitutions() {
        let tokens = last_source_tokens("set v OLD; list $v [set v NEW]");
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let invocation =
            resolved_tokens_invocation(registry, Some(test_context()), &tokens).unwrap();
        assert_eq!(invocation.argument_literal(0).as_deref(), Some("OLD"));
        assert_eq!(invocation.arguments[0].as_deref(), Some("${v}"));
        assert!(matches!(
            invocation.effective.words[1],
            WordExpr::Variable { .. }
        ));
        assert_eq!(
            invocation.effective.origins[1],
            InvocationWordOrigin::Written(1)
        );
        assert_eq!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .variable_context
                .substitution_literal_value("$v", registry),
            Some("NEW")
        );
    }

    #[test]
    fn possible_registry_shapes_do_not_establish_execution_facts() {
        let tokens = last_source_tokens("if {$flag} {proc set args {return CUSTOM}}; set x 1");
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        assert!(resolved_tokens_invocation(registry, Some(test_context()), &tokens).is_none());
        let assistance =
            registry_invocation_assistance(registry, Some(test_context()), &tokens).unwrap();
        assert!(assistance.unknown_residual);
        assert!(
            assistance
                .candidates
                .iter()
                .any(|candidate| candidate.command == "set")
        );
        assert_eq!(
            assistance
                .candidates
                .iter()
                .find(|candidate| candidate.command == "set")
                .unwrap()
                .effective
                .written_argument(0),
            Some(0)
        );
    }

    #[test]
    fn unanimous_advisory_words_keep_all_lookup_residuals() {
        let tokens = last_source_tokens("set target VALUE");
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let original = registry_invocation_assistance(registry, Some(test_context()), &tokens)
            .expect("stock command has advisory candidates");
        let first = original
            .candidates
            .first()
            .expect("stock candidate")
            .clone();
        let mut assistance = RegistryInvocationAssistance {
            candidates: vec![first.clone(), first.clone()],
            unknown_residual: false,
            may_be_absent: false,
        };
        let view = assistance
            .unanimous_command_words()
            .expect("unanimous words");
        assert_eq!(view.command(), first.command);
        assert_eq!(view.effective(), &first.effective);
        // Role agreement is a separate purpose and cannot be borrowed here.
        assistance.candidates[1].roles.clear();
        assistance.candidates[1].roles_complete = false;
        assert!(assistance.unanimous_command_words().is_some());
        for (unknown_residual, may_be_absent) in [(true, false), (false, true), (true, true)] {
            assistance.unknown_residual = unknown_residual;
            assistance.may_be_absent = may_be_absent;
            assert!(assistance.unanimous_command_words().is_none());
        }
        assistance.unknown_residual = false;
        assistance.may_be_absent = false;
        assistance.candidates.clear();
        assert!(assistance.unanimous_command_words().is_none());
        for malformed in [
            EffectiveCommandWords {
                words: Vec::new(),
                ..first.effective.clone()
            },
            EffectiveCommandWords {
                origins: Vec::new(),
                ..first.effective.clone()
            },
            EffectiveCommandWords {
                origins: vec![InvocationWordOrigin::BindingPrefix(0); first.effective.words.len()],
                binding_prefix: Vec::new(),
                ..first.effective.clone()
            },
        ] {
            assistance.candidates.push(RegistryInvocationShape {
                effective: malformed,
                ..first.clone()
            });
            assert!(assistance.unanimous_command_words().is_none());
            assistance.candidates.clear();
        }
    }

    #[test]
    fn unanimous_advisory_words_require_identity_values_and_origins() {
        let tokens = last_source_tokens("set target VALUE");
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let original = registry_invocation_assistance(registry, Some(test_context()), &tokens)
            .expect("stock command has advisory candidates");
        let first = original
            .candidates
            .first()
            .expect("stock candidate")
            .clone();
        let mut alternatives = Vec::new();
        let mut renamed = first.clone();
        renamed.command.push_str("_other");
        alternatives.push(renamed);
        let mut changed_word = first.clone();
        changed_word.effective.words[1] = literal("other_target");
        alternatives.push(changed_word);
        let mut changed_origin = first.clone();
        changed_origin.effective.origins[1] = InvocationWordOrigin::BindingPrefix(0);
        alternatives.push(changed_origin);
        let mut changed_prefix = first.clone();
        changed_prefix
            .effective
            .binding_prefix
            .push(EffectiveInvocationWord::Dynamic);
        alternatives.push(changed_prefix);
        for alternative in alternatives {
            let assistance = RegistryInvocationAssistance {
                candidates: vec![first.clone(), alternative],
                unknown_residual: false,
                may_be_absent: false,
            };
            assert!(assistance.unanimous_command_words().is_none());
        }
    }

    #[test]
    fn role_assistance_maps_alias_prefixes_to_only_original_operands() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
        for (source, expected) in [
            (
                "proc p {} {}; info body p",
                vec![(1, tcl_registry::ArgRole::CommandName)],
            ),
            (
                "proc p {} {}; interp alias {} inspect {} info; inspect body p",
                vec![(1, tcl_registry::ArgRole::CommandName)],
            ),
            (
                "proc p {} {}; interp alias {} inspect {} info body; inspect p",
                vec![(0, tcl_registry::ArgRole::CommandName)],
            ),
            (
                "proc p {} {}; interp alias {} inspect {} info body p; inspect",
                vec![],
            ),
            (
                "interp alias {} declare {} proc p; declare {x} {}",
                vec![
                    (0, tcl_registry::ArgRole::ParamList),
                    (1, tcl_registry::ArgRole::Body),
                ],
            ),
            (
                "interp alias {} declare {} proc p {x}; declare {}",
                vec![(0, tcl_registry::ArgRole::Body)],
            ),
            (
                "interp alias {} inspect {} info body; rename inspect {}; proc inspect args {}; inspect p",
                vec![],
            ),
        ] {
            let tokens = last_source_tokens(source);
            let roles =
                invocation_argument_role_assistance(registry, test_context(), &surface, &tokens);
            assert_eq!(roles, expected, "{source}");
        }
    }

    #[test]
    fn body_assistance_keeps_navigation_separate_from_unanimous_context() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
        let tokens = last_source_tokens("interp alias {} declare {} proc p {x}; declare {}");
        let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
        assert!(
            view.possible_roles
                .contains(&(0, tcl_registry::ArgRole::Body))
        );
        assert!(
            view.definite_roles
                .contains(&(0, tcl_registry::ArgRole::Body))
        );
        assert!(
            !view
                .possible_roles
                .iter()
                .any(|(_, role)| *role == tcl_registry::ArgRole::ParamList)
        );

        let tokens = last_source_tokens(
            "if {$flag} {rename if original; proc if args {return ordinary}}; if {$other} {set x 1}",
        );
        let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
        assert!(
            view.possible_roles
                .contains(&(1, tcl_registry::ArgRole::Body))
        );
        assert!(view.unknown_residual);
        assert!(view.definite_roles.is_empty());
        assert!(view.definite_traits.is_empty());
        assert!(view.definite_scope.is_none());

        let tokens = last_source_tokens(
            "interp alias {} declare {} proc p; rename declare {}; proc declare args {return ordinary}; declare {} {}",
        );
        let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
        assert!(view.possible_roles.is_empty());

        let tokens = last_source_tokens(
            "rename if original; proc if args {return ordinary}; if {$other} {set x 1}",
        );
        assert!(catalogue_invocation_assistance(registry, test_context(), &tokens).is_none());
        let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
        assert!(view.possible_roles.is_empty());
        assert!(view.definite_roles.is_empty());
        assert!(view.definite_invocation.is_none());

        let tokens = last_source_tokens(
            "if {$flag} {rename proc original; original proc args {return ordinary}}; proc p {x} {}",
        );
        assert!(catalogue_invocation_assistance(registry, test_context(), &tokens).is_none());
        let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
        assert!(
            view.possible_roles
                .contains(&(2, tcl_registry::ArgRole::Body))
        );
        assert!(view.definite_roles.is_empty());
        assert!(view.definite_invocation.is_none());
    }

    #[test]
    fn body_assistance_does_not_promote_authoring_facts_without_an_actual_axis() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
        let original = last_source_tokens("proc p {x} {}");
        let actual = invocation_body_assistance(registry, test_context(), &surface, &original);
        assert!(actual.definite_invocation.is_some());
        let mut binding = original.source_binding.as_ref().unwrap().clone();
        std::sync::Arc::make_mut(&mut binding.variable_context).invocation_dialect = None;
        let tokens = original.with_source_binding(binding);
        let advisory = invocation_body_assistance(registry, test_context(), &surface, &tokens);
        assert!(
            advisory
                .possible_roles
                .contains(&(2, tcl_registry::ArgRole::Body))
        );
        assert!(advisory.definite_roles.is_empty());
        assert!(advisory.definite_traits.is_empty());
        assert!(advisory.definite_invocation.is_none());
        assert!(advisory.conditional_invocation.is_none());
    }

    #[test]
    fn prepared_body_views_preserve_residual_and_selected_projection_boundaries() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
        for source in [
            "proc p {x} {}",
            "interp alias {} declare {} proc p; declare {x} {}",
            "interp alias {} declare {} proc p {x}; declare {}",
            "proc p {} {}; interp alias {} inspect {} info body; inspect p",
            "if {$flag} {rename proc original; original proc args {return ordinary}}; proc p {x} {}",
            "rename proc original; original proc args {return ordinary}; proc p {x} {}",
            "proc p {*}$unknown",
            "external p {x} {}",
        ] {
            let tokens = last_source_tokens(source);
            let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
            assert_eq!(
                view.possible_roles,
                invocation_argument_role_assistance(registry, test_context(), &surface, &tokens),
                "possible original roles: {source}"
            );
            assert_eq!(
                view.definite_roles,
                invocation_argument_role_consensus(registry, test_context(), &tokens),
                "unanimous original roles: {source}"
            );
            let selected = resolved_tokens_invocation(registry, Some(test_context()), &tokens)
                .filter(|_| !view.unknown_residual && !view.may_be_absent)
                .filter(|invocation| {
                    invocation.facts.arg_roles_complete
                        && invocation.facts.arity_accepts_frozen_arguments() == Some(true)
                });
            assert_eq!(
                view.definite_invocation, selected,
                "selected implementation: {source}"
            );
            if view.unknown_residual || view.may_be_absent {
                assert!(view.definite_roles.is_empty(), "{source}");
                assert!(view.definite_traits.is_empty(), "{source}");
                assert!(view.definite_scope.is_none(), "{source}");
            }
        }
    }

    #[test]
    fn packaged_body_assistance_keeps_availability_separate_from_native_entry() {
        let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
        let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
        let source = "bind .widget click {return event}";
        let config = tcl_lexer::LexerConfig::for_dialect("tk");
        let bindings =
            crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
        let command = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .unwrap();
        let mut tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &command);
        bindings.stamp_original_tokens(&mut tokens);
        let context = body_assistance_context(registry, &tokens).unwrap();
        assert_eq!(
            context,
            SemanticContext::for_profile(registry.profile().unwrap())
        );
        assert!(!context.context().authoring_query().packages.is_empty());
        let view =
            segmented_body_assistance(&surface, &bindings, source, config, &command, 0, None)
                .unwrap();
        assert!(
            view.possible_roles
                .contains(&(2, tcl_registry::ArgRole::Body))
        );
        let opaque =
            tokens.with_source_binding(crate::command_binding::SourceInvocationBinding::unknown());
        let view = invocation_body_assistance(registry, context, &surface, &opaque);
        assert!(view.definite_roles.is_empty());
        assert!(view.definite_traits.is_empty());
        assert!(view.definite_scope.is_none());
        assert!(view.definite_invocation.is_none());
    }

    #[test]
    fn unprofiled_body_assistance_uses_retained_entry_without_definite_offset_free_facts() {
        let registry = CommandRegistry::build_default();
        assert!(registry.profile().is_none());
        let surface = tcl_registry::model::DocumentCommandSurface::new(&registry, None);
        let source = "eval {$script}";
        let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
        let bindings =
            crate::command_binding::SourceCommandBindings::analyse(source, config, &registry);
        let command = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .unwrap();
        let view =
            segmented_body_assistance(&surface, &bindings, source, config, &command, 0, None)
                .unwrap();
        assert!(
            view.possible_roles
                .contains(&(0, tcl_registry::ArgRole::Body))
        );
        assert!(
            view.possible_traits
                .contains(tcl_registry::Traits::DYNAMIC_EVAL_BODY)
        );
        let view =
            unpositioned_body_assistance(&surface, &bindings, source, config, &command, None)
                .unwrap();
        assert!(
            view.possible_roles
                .contains(&(0, tcl_registry::ArgRole::Body))
        );
        assert!(view.definite_roles.is_empty());
        assert!(view.definite_traits.is_empty());
        assert!(view.definite_invocation.is_none());
        let unknown = crate::command_binding::SourceCommandBindings::analyse_with_options(
            source,
            config,
            &registry,
            crate::command_binding::SourceAnalysisOptions::default(),
        );
        assert!(
            segmented_body_assistance(&surface, &unknown, source, config, &command, 0, None)
                .is_none()
        );
        assert!(
            unpositioned_body_assistance(&surface, &unknown, source, config, &command, None)
                .is_none()
        );
    }

    #[test]
    fn definite_roles_require_closed_actual_alternatives_and_original_positions() {
        use tcl_registry::ArgRole::{Body, Name, ParamList};
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (source, expected) in [
            ("proc p {x} {}", vec![(0, Name), (1, ParamList), (2, Body)]),
            (
                "interp alias {} declare {} proc p; declare {x} {}",
                vec![(0, ParamList), (1, Body)],
            ),
            (
                "interp alias {} declare {} proc p {x}; declare {}",
                vec![(0, Body)],
            ),
            (
                "rename proc original; original proc args {return ordinary}; proc p {{a b c}} {}",
                vec![],
            ),
            (
                "if {$unknown} {rename proc original; original proc args {return ordinary}}; proc p {{a b c}} {}",
                vec![],
            ),
            ("external p {{a b c}} {}", vec![]),
            ("proc p {*}$unknown", vec![]),
        ] {
            let tokens = last_source_tokens(source);
            assert_eq!(
                invocation_argument_role_consensus(registry, test_context(), &tokens),
                expected,
                "{source}"
            );
        }
        let opaque = last_source_tokens("proc p {{a b c}} {}")
            .with_source_binding(crate::command_binding::SourceInvocationBinding::unknown());
        assert_eq!(
            invocation_argument_role_consensus(registry, test_context(), &opaque),
            vec![]
        );
    }

    #[test]
    fn unloaded_catalogue_assistance_never_restores_a_live_implementation() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let tokens = last_source_tokens("::struct::tree tree");
        assert!(resolved_tokens_invocation(registry, Some(test_context()), &tokens).is_none());
        let assistance = catalogue_invocation_assistance(registry, test_context(), &tokens)
            .expect("catalogue metadata at an unchanged qualified slot");
        assert_eq!(assistance.shape.command, "struct::tree");
        let shadowed = last_source_tokens(
            "namespace eval ::struct {}; proc ::struct::tree args {return ordinary}; ::struct::tree tree",
        );
        assert!(catalogue_invocation_assistance(registry, test_context(), &shadowed).is_none());
        let unregistered = last_source_tokens("::external::factory tree");
        assert!(catalogue_invocation_assistance(registry, test_context(), &unregistered).is_none());
    }

    #[test]
    fn explicit_point_uncertainty_never_recovers_written_builtin_facts() {
        let registry = CommandRegistry::build_default();
        let command = segment_commands("set x 1").remove(0);
        let tokens = CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new("set x 1"),
            tcl_lexer::LexerConfig::default(),
            &command,
        )
        .with_source_binding(crate::command_binding::SourceInvocationBinding::unknown());
        assert!(effective_command_words(&tokens).is_none());
        assert!(matches!(
            resolve_command_tokens(&registry, Some(test_context()), &tokens),
            Ok(RegistryInvocationResolution::Unresolved(
                OwnedInvocationResolutionUnresolved::UnprovedBinding { unknown: true, .. }
            ))
        ));
    }

    #[test]
    fn declared_roles_remain_separate_from_runtime_semantics() {
        use tcl_registry::model::DocumentCommandSurface;
        let registry = CommandRegistry::build_default();
        let config = tcl_lexer::LexerConfig::default();
        let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub custom {destination:var}\n# tcl-lsp: stubs-end\ncustom result";
        let declared = crate::analyser::utils::document_declared_surface(source, None, "tcl8.6");
        let bindings =
            crate::command_binding::SourceCommandBindings::analyse_in_namespace_with_options(
                source,
                "::",
                config,
                &registry,
                crate::command_binding::SourceAnalysisOptions {
                    declared_commands: Some(&declared),
                    ..Default::default()
                },
            );
        let segment = segment_commands(source).remove(0);
        let tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
                .with_source_binding(bindings.invocation_at_source("custom", segment.span.start()));
        let surface = DocumentCommandSurface::new(&registry, Some(&declared));
        let assistance = resolved_declared_assistance(&surface, &tokens)
            .expect("untouched declared lookup slot");
        assert_eq!(assistance.roles, [(0, tcl_registry::ArgRole::VarWrite)]);
        assert!(matches!(
            resolve_command_tokens(&registry, None, &tokens),
            Ok(RegistryInvocationResolution::Unresolved(_))
        ));
        assert!(resolved_tokens_invocation(&registry, None, &tokens).is_none());
    }

    #[test]
    fn nested_dispatch_inheritance_never_recovers_a_missing_point() {
        let source = "set r [expr {1 + 2}]";
        let config = tcl_lexer::LexerConfig::default();
        let segment = segment_commands(source).remove(0);
        let parent =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
                .with_source_binding(crate::command_binding::SourceInvocationBinding::unknown());
        let word = &parent.words()[2];
        let mut nested =
            crate::word_subst::nested_command_words(&word.legacy_text(), word.source(), config)
                .unwrap();
        nested.inherit_nested_bindings(&parent);
        assert!(nested.has_unproved_source_binding());
        assert!(
            resolved_tokens_invocation(
                &CommandRegistry::build_default(),
                Some(test_context()),
                &nested
            )
            .is_none()
        );
    }

    #[test]
    fn alias_composition_preserves_original_evaluation_and_inserted_origins() {
        let registry = CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir(
            "interp alias {} inner {} list second\ninterp alias {} outer {} inner first\nouter $value",
            &registry,
        );
        let tokens = module
            .top_level
            .statements
            .last()
            .unwrap()
            .tokens()
            .unwrap();
        let effective = effective_command_words(tokens).expect("alias has a unique target");
        assert_eq!(tokens.argv_texts, ["outer", "${value}"]);
        assert_eq!(
            effective
                .argument_spellings(&tokens.argv_texts[1..])
                .unwrap(),
            ["second", "first", "${value}"]
        );
        assert_eq!(
            effective.origins,
            [
                InvocationWordOrigin::ResolvedHead,
                InvocationWordOrigin::BindingPrefix(0),
                InvocationWordOrigin::BindingPrefix(1),
                InvocationWordOrigin::Written(1)
            ]
        );
        assert_eq!(effective.written_argument(0), None);
        assert_eq!(effective.written_argument(2), Some(0));
        assert!(matches!(
            effective.words.last(),
            Some(WordExpr::Variable { .. })
        ));
        assert!(matches!(
            effective.words[1].source().provenance,
            crate::ir::Provenance::Opaque
        ));
    }

    #[test]
    fn document_target_composes_arguments_without_registry_semantics() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = "proc custom {prefix value} {return $value}\ninterp alias {} invoke {} custom FIXED\ninvoke $value";
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let bindings =
            crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .unwrap();
        let tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
                .with_source_binding(bindings.invocation_at_source("invoke", segment.span.start()));
        let effective =
            effective_command_words(&tokens).expect("unique document procedure through alias");
        assert_eq!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .proved_target()
                .unwrap()
                .command,
            "::custom"
        );
        assert_eq!(
            effective
                .argument_spellings(&tokens.argv_texts[1..])
                .unwrap(),
            ["FIXED", "${value}"]
        );
        assert_eq!(effective.written_argument(0), None);
        assert_eq!(effective.written_argument(1), Some(0));
        assert!(
            resolved_tokens_invocation(registry, Some(test_context()), &tokens).is_none(),
            "composing a document callee is not registry implementation proof"
        );
    }

    #[test]
    fn captured_unknown_prefix_preserves_count_without_literal_presentation() {
        let source = "alias written";
        let config = tcl_lexer::LexerConfig::default();
        let segment = segment_commands(source).remove(0);
        let tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
        let effective = compose_effective_words(
            &tokens,
            "list",
            &[
                EffectiveInvocationWord::Dynamic,
                EffectiveInvocationWord::Literal("known".to_owned()),
            ],
        )
        .unwrap();
        assert_eq!(
            effective.argument_presentations(&tokens.argv_texts[1..]),
            Some(vec![
                None,
                Some("known".to_owned()),
                Some("written".to_owned())
            ])
        );
        assert!(
            effective
                .argument_spellings(&tokens.argv_texts[1..])
                .is_none()
        );
        let words = frozen_argument_words(&tokens, &effective);
        assert_eq!(words[0], EffectiveInvocationWord::Dynamic);
        assert_eq!(words.len(), 3);
    }

    #[test]
    fn effective_argument_projection_declines_inconsistent_source_ownership() {
        let projection = EffectiveCommandWords {
            words: vec![literal("list"), literal("arg")],
            origins: vec![InvocationWordOrigin::Written(0)],
            binding_prefix: Vec::new(),
        };
        assert_eq!(projection.argument_spellings(&["arg".into()]), None);
        let projection = EffectiveCommandWords {
            words: vec![literal("list"), literal("arg")],
            origins: vec![
                InvocationWordOrigin::Written(0),
                InvocationWordOrigin::Written(1),
            ],
            binding_prefix: Vec::new(),
        };
        assert_eq!(projection.argument_spellings(&[]), None);
        assert_eq!(
            projection.argument_spellings(&["arg".into()]),
            Some(vec!["arg".into()])
        );
    }

    #[test]
    fn literal_words_materialise_registry_transition_facts() {
        let words = [literal("global"), literal("::shared")];
        let resolution = resolve_word_exprs(
            &CommandRegistry::build_default(),
            Some(test_context()),
            &words,
        )
        .expect("literal command has a registry outcome");

        let RegistryInvocationResolution::Resolved(facts) = resolution else {
            panic!("literal global must resolve");
        };
        let transitions = facts
            .state_transitions
            .declared()
            .expect("global declares transition facts");
        assert!(matches!(
            transitions.facts(),
            [fact]
                if matches!(
                    &fact.transition,
                    StateTransition::VariableCellAlias(alias)
                        if alias.local.literal() == Some("shared")
                            && alias.local.argument_index() == Some(0)
                )
        ));
    }

    #[test]
    fn segmented_arguments_preserve_a_static_non_option_template_prefix() {
        let command = segment_commands("regexp \"abc$part.*\" $subject\n")
            .into_iter()
            .next()
            .expect("one command");
        assert_eq!(
            segmented_command_arguments(&command),
            vec![InvocationWord::DynamicNonOption, InvocationWord::Dynamic],
            "the first template remains dynamic but cannot be a leading option"
        );
    }

    #[test]
    fn compiler_word_shape_preserves_quoted_simple_words_without_decoding_escapes() {
        use tcl_registry::native_compilation::NativeCompilationWordShape as Shape;

        for (source, expected) in [
            ("list plain", Shape::Literal),
            ("list \"plain\"", Shape::QuotedLiteral),
            ("list \"\"", Shape::QuotedLiteral),
            ("list {plain}", Shape::BracedLiteral),
            (r"list \p\lain", Shape::BackslashLiteral),
            (r#"list "\160lain""#, Shape::BackslashLiteral),
            ("list \"$value\"", Shape::Substituted),
        ] {
            let segment = segment_commands(source).pop().unwrap();
            let tokens = CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(source),
                tcl_lexer::LexerConfig::default(),
                &segment,
            );
            assert_eq!(
                native_compilation_word_shape(&tokens.words()[1]),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn effective_words_keep_braces_and_decode_bare_escapes() {
        let source = SourceSite::source(tcl_lexer::Span::new(0, 0));
        assert_eq!(
            effective_invocation_word(
                &WordExpr::Template {
                    parts: vec![WordPart::Text {
                        text: r"\x32".to_owned(),
                        source: source.clone()
                    }],
                    source: source.clone(),
                    rejected: None,
                },
                EscapeSyntax::Tcl86,
                WordValueRules::TCL,
            ),
            EffectiveInvocationWord::Literal("2".to_owned())
        );
        assert_eq!(
            effective_invocation_word(
                &WordExpr::BracedLiteral {
                    text: r"\x32".to_owned(),
                    source
                },
                EscapeSyntax::Tcl86,
                WordValueRules::TCL,
            ),
            EffectiveInvocationWord::Literal(r"\x32".to_owned())
        );
    }

    #[test]
    fn compiled_local_name_requires_a_direct_source_word() {
        let source = SourceSite::source(tcl_lexer::Span::new(0, 0));
        let raw_braced = WordExpr::BracedLiteral {
            text: r"p\x75b".to_owned(),
            source: source.clone(),
        };
        let bare_escape = WordExpr::Template {
            parts: vec![WordPart::Text {
                text: r"p\x75b".to_owned(),
                source: source.clone(),
            }],
            source: source.clone(),
            rejected: None,
        };
        let quoted_plain = WordExpr::Template {
            parts: vec![WordPart::Text {
                text: "pub".to_owned(),
                source: source.clone(),
            }],
            source: source.clone(),
            rejected: None,
        };
        let dynamic = WordExpr::Variable {
            spelling: "$name".to_owned(),
            source: source.clone(),
        };

        assert_eq!(
            compiled_local_name_word(&raw_braced),
            CompiledLocalNameWord::Direct,
            "braces preserve the backslash as part of the variable name"
        );
        assert_eq!(
            compiled_local_name_value(
                &WordExpr::BracedLiteral {
                    text: "{zz}".to_owned(),
                    source: source.clone(),
                },
                EscapeSyntax::Tcl90,
                WordValueRules::TCL,
            ),
            Some("{zz}".to_owned()),
            "the source owner removes only the grouping brace pair"
        );
        assert_eq!(
            compiled_local_name_word(&bare_escape),
            CompiledLocalNameWord::Stack,
            "bare escapes decode before variable lookup"
        );
        assert_eq!(
            compiled_local_name_word(&quoted_plain),
            CompiledLocalNameWord::Direct,
            "quotes group a direct name without backslash processing"
        );
        assert_eq!(
            compiled_local_name_word(&dynamic),
            CompiledLocalNameWord::Stack
        );
    }

    #[test]
    fn compiled_local_name_collapses_only_braced_continuations() {
        let source = SourceSite::source(tcl_lexer::Span::new(0, 0));
        let continuation = WordExpr::BracedLiteral {
            text: "p\\\n  ub".to_owned(),
            source: source.clone(),
        };
        let raw_escape = WordExpr::BracedLiteral {
            text: r"p\x75b".to_owned(),
            source,
        };

        assert_eq!(
            compiled_local_name_value(&continuation, EscapeSyntax::Tcl90, WordValueRules::TCL),
            Some("p ub".to_owned())
        );
        assert_eq!(
            compiled_local_name_value(&raw_escape, EscapeSyntax::Tcl90, WordValueRules::TCL),
            Some(r"p\x75b".to_owned())
        );
    }

    #[test]
    fn dynamic_and_expanded_words_never_become_literal_registry_values() {
        let dynamic = [
            literal("global"),
            WordExpr::Variable {
                spelling: "$name".to_owned(),
                source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
            },
        ];
        let RegistryInvocationResolution::Resolved(dynamic_facts) = resolve_word_exprs(
            &CommandRegistry::build_default(),
            Some(test_context()),
            &dynamic,
        )
        .expect("dynamic global has a registry outcome") else {
            panic!("literal global head resolves");
        };
        assert!(
            dynamic_facts
                .state_transitions
                .declared()
                .expect("global declares transition facts")
                .facts()
                .iter()
                .any(|fact| matches!(fact.transition, StateTransition::Widen(_)))
        );

        let expanded = [
            literal("proc"),
            literal("p"),
            WordExpr::Expand {
                source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
                word: Box::new(WordExpr::Opaque {
                    text: "args".to_owned(),
                    source: SourceSite::opaque(tcl_lexer::Span::new(0, 0)),
                    reason: WordOpacity::LossySnapshot,
                }),
            },
            literal("body"),
        ];
        let RegistryInvocationResolution::Resolved(expanded_facts) = resolve_word_exprs(
            &CommandRegistry::build_default(),
            Some(test_context()),
            &expanded,
        )
        .expect("expanded proc has a registry outcome") else {
            panic!("literal proc head resolves");
        };
        assert!(
            expanded_facts
                .state_transitions
                .declared()
                .expect("proc declares transition facts")
                .facts()
                .iter()
                .all(|fact| !matches!(fact.transition, StateTransition::CommandBinding(_)))
        );
    }

    #[test]
    fn dynamic_head_stays_typed_unresolved() {
        let words = [WordExpr::Variable {
            spelling: "$command".to_owned(),
            source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
        }];
        assert!(matches!(
            resolve_word_exprs(
                &CommandRegistry::build_default(),
                Some(test_context()),
                &words,
            ),
            Ok(RegistryInvocationResolution::Unresolved(
                OwnedInvocationResolutionUnresolved::ComputedHead {
                    word_kind: InvocationWordKind::Dynamic,
                }
            ))
        ));
    }

    #[test]
    fn original_metadata_context_keeps_actual_availability_and_generation() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = generation.commands();
        let old = tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(registry));
        let tokens = last_source_tokens("dict for {key value} {} {puts nested}");
        let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
        assert!(
            !invocation_argument_role_assistance(registry, generation, &surface, &tokens)
                .is_empty()
        );
        assert!(invocation_argument_role_assistance(registry, &old, &surface, &tokens).is_empty());
        assert!(invocation_argument_role_consensus(registry, &old, &tokens).is_empty());
        let body = invocation_body_assistance(registry, &old, &surface, &tokens);
        assert!(body.possible_roles.is_empty() && body.definite_roles.is_empty());
        assert!(body.unknown_residual && body.definite_invocation.is_none());
        let set = last_source_tokens("set target value");
        assert!(original_variable_write_advice(registry, generation, &set).is_some());
        let foreign =
            generation.with_command_store(std::sync::Arc::new(CommandRegistry::build_default()));
        assert_ne!(
            foreign.commands().snapshot().semantic_key(),
            registry.snapshot().semantic_key()
        );
        assert!(invocation_argument_role_assistance(registry, &foreign, &surface, &set).is_empty());
        assert!(invocation_argument_role_consensus(registry, &foreign, &set).is_empty());
        assert!(original_variable_write_advice(registry, &foreign, &set).is_none());
        assert!(
            original_symbol_declaration_advice(
                registry,
                &foreign,
                &last_source_tokens("proc p {} {}")
            )
            .is_none()
        );
    }
    #[test]
    fn retained_script_metadata_keeps_actual_context_and_effective_ordinals() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
        for (source, argument, expected) in [
            (
                "trace add variable v write changed",
                4,
                tcl_registry::ScriptTiming::Deferred,
            ),
            (
                "trace remove variable v write changed",
                4,
                tcl_registry::ScriptTiming::ReferenceOnly,
            ),
            (
                "lsort -command compare {b a}",
                1,
                tcl_registry::ScriptTiming::SameInvocation,
            ),
            (
                "interp alias {} install {} trace add variable v write; install changed",
                4,
                tcl_registry::ScriptTiming::Deferred,
            ),
        ] {
            let tokens = last_source_tokens(source);
            let invocation = resolved_tokens_invocation_in_context(generation, &tokens)
                .expect("authentic selected invocation");
            let realm = tokens
                .source_binding
                .as_ref()
                .unwrap()
                .invocation_realm()
                .unwrap();
            assert_eq!(
                invocation.with_metadata_schema(
                    generation.commands(),
                    generation.into(),
                    realm,
                    |selected| selected.authored_source_script_timing_at(argument)
                ),
                Some(expected),
                "{source}"
            );
            assert!(
                invocation
                    .with_metadata_schema(
                        generation.commands(),
                        foreign.into(),
                        realm,
                        |selected| selected.authored_source_script_timing_at(argument)
                    )
                    .is_none()
            );
        }
    }

    #[test]
    fn original_body_metadata_keeps_supplied_availability_and_generation() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(generation.commands()));
        let source = "dict for {key value} {one two} {set found $value}";
        let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
        let bindings = crate::command_binding::SourceCommandBindings::analyse(
            source,
            config,
            generation.commands(),
        );
        let surface = tcl_registry::model::DocumentCommandSurface::new(generation.commands(), None);
        let command =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let current = segmented_body_assistance(
            &surface,
            &bindings,
            source,
            config,
            &command,
            0,
            Some(generation),
        )
        .unwrap();
        assert!(
            current
                .possible_roles
                .contains(&(3, tcl_registry::ArgRole::Body))
        );
        let unavailable = segmented_body_assistance(
            &surface,
            &bindings,
            source,
            config,
            &command,
            0,
            Some(&older),
        )
        .unwrap();
        assert!(unavailable.possible_roles.is_empty());
        assert!(unavailable.unknown_residual);
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
        assert!(
            segmented_body_assistance(
                &surface,
                &bindings,
                source,
                config,
                &command,
                0,
                Some(foreign)
            )
            .is_none()
        );
        assert!(
            unpositioned_body_assistance(
                &surface,
                &bindings,
                source,
                config,
                &command,
                Some(foreign)
            )
            .is_none()
        );
    }

    #[test]
    fn retained_invocation_context_keeps_availability_without_donating_handler_proof() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(generation.commands()));
        let tokens = last_source_tokens("dict create key value");
        assert!(resolved_tokens_invocation_in_context(generation, &tokens).is_some());
        assert!(resolved_tokens_invocation_in_context(&older, &tokens).is_none());
        assert!(normal_representation_invocation_in_context(generation, &tokens).is_some());
        assert!(normal_representation_invocation_in_context(&older, &tokens).is_none());
        assert!(normal_transfer_invocation_in_context(generation, &tokens).is_some());
        assert!(normal_transfer_invocation_in_context(&older, &tokens).is_none());
        let names = possible_variable_name_operands_in_context(generation, &tokens).unwrap();
        assert!(!names.candidates.is_empty());
        let unavailable_names =
            possible_variable_name_operands_in_context(&older, &tokens).unwrap();
        assert!(unavailable_names.candidates.is_empty() && unavailable_names.unknown_residual());
        let assistance = registry_invocation_assistance_in_context(generation, &tokens).unwrap();
        assert!(!assistance.candidates.is_empty());
        let unavailable = registry_invocation_assistance_in_context(&older, &tokens).unwrap();
        assert!(unavailable.candidates.is_empty() && unavailable.unknown_residual);
        let replaced = last_source_tokens("proc dict args {}; dict create key value");
        assert!(resolved_tokens_invocation_in_context(generation, &replaced).is_none());
        assert!(normal_representation_invocation_in_context(generation, &replaced).is_none());
        assert!(normal_transfer_invocation_in_context(generation, &replaced).is_none());
        let replaced_names =
            possible_variable_name_operands_in_context(generation, &replaced).unwrap();
        assert!(replaced_names.candidates.is_empty() && replaced_names.unknown_residual());
        let replaced_assistance =
            registry_invocation_assistance_in_context(generation, &replaced).unwrap();
        assert!(replaced_assistance.candidates.is_empty() && replaced_assistance.unknown_residual);
        assert!(
            logical_structured_invocation_with_metadata_context(
                generation.commands(),
                generation.into(),
                &tokens,
                None
            )
            .is_some()
        );
        assert!(
            logical_structured_invocation_with_metadata_context(
                generation.commands(),
                (&older).into(),
                &tokens,
                None
            )
            .is_none()
        );
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
        assert!(
            logical_structured_invocation_with_metadata_context(
                generation.commands(),
                foreign.into(),
                &tokens,
                None
            )
            .is_none()
        );
        let mut unproved = tokens;
        unproved.source_binding = None;
        assert!(resolved_tokens_invocation_in_context(generation, &unproved).is_none());
        assert!(normal_representation_invocation_in_context(generation, &unproved).is_none());
        assert!(normal_transfer_invocation_in_context(generation, &unproved).is_none());
        assert!(possible_variable_name_operands_in_context(generation, &unproved).is_none());
        assert!(registry_invocation_assistance_in_context(generation, &unproved).is_none());
        assert!(
            logical_structured_invocation_with_metadata_context(
                generation.commands(),
                generation.into(),
                &unproved,
                None
            )
            .is_none()
        );
        assert!(
            original_callback_invocation_with_metadata_context(
                generation.commands(),
                generation.into(),
                &unproved
            )
            .is_none()
        );
    }
    fn advisory_metadata_tokens(source: &str, registry: &CommandRegistry) -> CommandTokens {
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let bindings =
            crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
        let command = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .unwrap();
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &command)
            .with_source_binding(
                bindings.invocation_at_source("unused-reporting-head", command.span.start()),
            )
    }

    #[test]
    fn advisory_value_metadata_shares_supplied_availability_and_store_refusal() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Registry May candidates only; no actual assignment or returned object.
        let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let mut registry = baseline
            .commands()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        for name in ["set", "return"] {
            let mut descriptor = registry.get(name).unwrap().clone();
            descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
            registry.insert(descriptor);
        }
        let generation = baseline.with_command_store(std::sync::Arc::new(registry));
        let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(generation.commands()));
        let registry = generation.commands();
        assert!(std::sync::Arc::ptr_eq(older.commands(), registry));
        let mut assignment = advisory_metadata_tokens("set target VALUE", registry);
        let mut returning = advisory_metadata_tokens("return VALUE", registry);
        let current = advisory_value_assignments_in_context(&generation, &assignment);
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].name, "target");
        assert_eq!(current[0].value, assignment.words()[2]);
        assert_eq!(
            advisory_return_values_in_context(&generation, &returning),
            vec![returning.words()[1].clone()]
        );
        assert!(advisory_value_assignments_in_context(&older, &assignment).is_empty());
        assert!(advisory_return_values_in_context(&older, &returning).is_empty());
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
        assert!(
            advisory_value_assignments_with_metadata_context(
                registry,
                Some(foreign.into()),
                &assignment
            )
            .is_empty()
        );
        assert!(
            advisory_return_values_with_metadata_context(
                registry,
                Some(foreign.into()),
                &returning
            )
            .is_empty()
        );
        assignment.source_binding = None;
        returning.source_binding = None;
        assert!(advisory_value_assignments_in_context(&generation, &assignment).is_empty());
        assert!(advisory_return_values_in_context(&generation, &returning).is_empty());
    }
}
