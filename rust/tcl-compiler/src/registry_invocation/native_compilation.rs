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

//! Native compilation of an invocation: compiler admission, inline operations and dependencies.

use super::effective_words::{
    compose_original_effective_words, discard_named_selection_operands, frozen_argument_values,
    frozen_argument_words,
};
use super::native_compilation_source::{
    OriginalNativeCompilerInvocation, original_native_compilation,
};
use super::word_resolution::{
    EffectiveInvocationWord, RegistryInvocationResolution, effective_invocation_word,
    resolve_registry_words_in_realm,
};
use super::{AdmittedNativeCompilerInvocation, EffectiveCommandWords, ResolvedStatementInvocation};
use crate::ir::CommandTokens;
use crate::ir::Statement;
use tcl_dialect::EscapeSyntax;
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{CommandRegistry, InvocationFacts, InvocationWords};
use tcl_source::words::WordExpr;
use tcl_source::words::WordPart;
use tcl_syntax::word_rules::WordValueRules;

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

    /// Exact metadata key retained by the already admitted original compiler.
    /// The key supplies no additional handler, frame, completion or admission.
    #[must_use]
    pub fn descriptor_key(&self) -> tcl_registry::CanonicalCommandDescriptorKey<'_> {
        self.invocation.facts.descriptor_key()
    }
    /// Compiler metadata from the already admitted original descriptor.
    /// This does not establish successful-handler effects.
    #[must_use]
    pub fn traits(&self) -> tcl_registry::Traits {
        self.invocation.facts.traits
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
            authored_case_body_selection: None,
        },
        hooks,
        target,
        selection: binding.native_compilation_admission_selection(),
        hook_operand_prefix,
    })
}

/// Separate original compiler registration requirements from Logical metadata.
#[derive(Clone, Copy)]
pub(crate) enum SelectedLoweringBindingIdentity<'a> {
    Native(&'a AdmittedNativeCompilerInvocation),
    Logical(tcl_registry::CanonicalCommandDescriptorKey<'a>),
    HostedSchema(tcl_registry::CanonicalCommandDescriptorKey<'a>),
    NativeSourceSchema(tcl_registry::CanonicalCommandDescriptorKey<'a>),
    LogicalSourceSchema(tcl_registry::CanonicalCommandDescriptorKey<'a>),
    AuthoredSourceSchema(tcl_registry::CanonicalCommandDescriptorKey<'a>),
    AuthoredSimulationSourceSchema(tcl_registry::CanonicalCommandDescriptorKey<'a>),
}

impl<'a> SelectedLoweringBindingIdentity<'a> {
    pub(crate) fn descriptor_key(self) -> tcl_registry::CanonicalCommandDescriptorKey<'a> {
        match self {
            Self::Native(invocation) => invocation.descriptor_key(),
            Self::Logical(key)
            | Self::HostedSchema(key)
            | Self::NativeSourceSchema(key)
            | Self::LogicalSourceSchema(key)
            | Self::AuthoredSimulationSourceSchema(key)
            | Self::AuthoredSourceSchema(key) => key,
        }
    }

    pub(crate) fn binding_identity(self) -> &'a str {
        let name = match self {
            Self::Native(invocation) => invocation.canonical_registration_name(),
            Self::Logical(key)
            | Self::HostedSchema(key)
            | Self::NativeSourceSchema(key)
            | Self::LogicalSourceSchema(key)
            | Self::AuthoredSimulationSourceSchema(key)
            | Self::AuthoredSourceSchema(key) => key.label(),
        };
        tcl_syntax::naming::unroot_rooted_key(name).unwrap_or(name)
    }
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
        // dialect-drift-ok: `text` is a Text part already split under the
        // document's grammar (no substitution); only its backslash runs are
        // asked about, which every grammar decodes alike
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
    dialect: tcl_native_model::InvocationDialect,
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
    pub compilation_site: tcl_names::command::CommandAllocationSite,
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
#[doc(hidden)]
#[must_use]
pub fn compiled_namespace_context(
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
                tcl_syntax::naming::unroot_rooted_key(&selected.target.command)
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
            tcl_syntax::naming::unroot_rooted_key(&dependency.target.command)
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
#[doc(hidden)]
pub enum NativeCompilationDependency {
    Compiler(tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite),
    Implementation(tcl_runtime_api::CommandBindingIdentity),
}

#[doc(hidden)]
#[must_use]
pub fn native_compilation_dependency(
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
    site: &tcl_names::command::CommandAllocationSite,
) -> Option<String> {
    if tokens.words().len() != tokens.argv_texts.len() {
        return None;
    }
    let text = site.source.try_text().ok()?;
    let first = tokens.words().first()?.original_written_span(text)?;
    let last = tokens.words().last()?.original_written_span(text)?;
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
