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

//! Native opcode selection is independent of the invocation's later live lookup.

use super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, SourceCommandTarget, SourceInvocationBinding,
};
#[cfg(test)]
use super::{BTreeSet, BindingKind, MayBinding};
use tcl_registry::native_compilation::{
    NativeCompilationContext, NativeCompilationGuard, NativeCompilationMode,
    NativeCompilationSelection, NativeCompilationWordShape,
};

mod compiler_targets;
mod execution_proofs;
mod registered_compiler;
pub(super) use registered_compiler::original_named_selection_is_closed;
mod target_selection;

pub use compiler_targets::{NativeCompilationSnapshot, NativeCompilerTargets};
pub(super) use execution_proofs::join_argument_words;
pub(super) use registered_compiler::{
    compiler_dependencies, original_fixed_compiler_lookup_preserves_names,
    original_registration_descriptor,
};
use target_selection::select_target;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CompiledInvocationSelection {
    pub proofs: Vec<SourceCompiledInvocationProof>,
    pub named: Vec<super::SourceNamedInvocationProof>,
    /// Immutable implementation shared by every possible compiler protocol.
    /// This licenses no opcode, body-compilation, or successful-entry claim.
    pub stable_handler: Option<SourceCommandTarget>,
    /// Every possible compiler preserves tracked names before argv. This
    /// closes no native opcode, variable receiver or handler completion.
    pub(super) compiler_name_preservation: super::SourceCompilerNameClosure,
    pub admission: Option<NativeCompilationSelection>,
    pub admitted: Option<Arc<SourceNativeCompilerAdmission>>,
    pub policy: Option<Arc<super::compiler_inventory::SourceNativeCompilerPolicy>>,
    pub operand_layout: Option<Arc<super::SourceNativeOperandLayoutProof>>,
    pub namespace_bindings: Option<Arc<SourceNativeNamespaceBindingPreparation>>,
    pub switch: Option<Arc<SourceNativeSwitchPreparation>>,
    pub structured: Option<Arc<SourceNativeStructuredPreparation>>,
    pub original_words: Option<Arc<SourceOriginalCompilerWords>>,
    pub live: bool,
    pub unknown: bool,
    pub compile_error: bool,
    pub rejection: NativeInlineRejection,
}

impl Default for CompiledInvocationSelection {
    fn default() -> Self {
        Self {
            proofs: Vec::new(),
            named: Vec::new(),
            stable_handler: None,
            compiler_name_preservation: super::SourceCompilerNameClosure::Preserved,
            admission: Some(NativeCompilationSelection::Generic),
            admitted: None,
            policy: None,
            operand_layout: None,
            namespace_bindings: None,
            switch: None,
            structured: None,
            original_words: None,
            live: true,
            unknown: false,
            compile_error: false,
            rejection: NativeInlineRejection::None,
        }
    }
}

impl CompiledInvocationSelection {
    pub(super) fn compiler_preserves_names(&self) -> bool {
        self.compiler_name_preservation.is_preserved()
    }

    pub(super) fn stable_handler_after_original_arguments(
        &self,
        head: Option<&crate::signature_scan::scope::SignatureSourceNameInput>,
        state: &ModuleCommandBindings,
        namespace: &super::SourceNamespaceKey,
    ) -> Option<&SourceCommandTarget> {
        let target = self.stable_handler.as_ref()?;
        let after = super::source_binding_from_original_input(state, head?, namespace)?;
        (after.proved_target() == Some(target)).then_some(target)
    }

    pub(super) fn join(&mut self, other: &Self) {
        self.compiler_name_preservation
            .join(other.compiler_name_preservation);
        if self.policy != other.policy {
            self.policy = None;
        }
        if self.operand_layout != other.operand_layout {
            self.operand_layout = None;
        }
        if self.namespace_bindings != other.namespace_bindings {
            self.namespace_bindings = None;
        }
        if self.switch != other.switch {
            self.switch = None;
        }
        if self.structured != other.structured {
            self.structured = None;
        }
        if self.original_words != other.original_words {
            self.original_words = None;
        }
        if self.admitted != other.admitted {
            self.admitted = None;
        }
        if self.admission != other.admission {
            self.admission = Some(NativeCompilationSelection::Unknown);
        }
        if self.stable_handler != other.stable_handler {
            self.stable_handler = None;
        }
        let mut left = SourceInvocationBinding {
            compiled_candidates: self.proofs.clone(),
            compiled_named_candidates: self.named.clone(),
            may_use_live_dispatch: self.live,
            compiled_execution_residual: self.unknown.into(),
            native_inline_rejection: self.rejection,
            ..SourceInvocationBinding::default()
        };
        left.join_compiled_execution(&SourceInvocationBinding {
            compiled_candidates: other.proofs.clone(),
            compiled_named_candidates: other.named.clone(),
            may_use_live_dispatch: other.live,
            compiled_execution_residual: other.unknown.into(),
            native_inline_rejection: other.rejection,
            ..SourceInvocationBinding::default()
        });
        self.unknown = left.compiled_execution_unknown();
        self.proofs = left.compiled_candidates;
        self.named = left.compiled_named_candidates;
        self.live = left.may_use_live_dispatch;
        self.compile_error |= other.compile_error;
        self.rejection = left.native_inline_rejection;
    }

    fn admit(&mut self, selection: NativeCompilationSelection) {
        self.admission = Some(match self.admission {
            None => selection,
            Some(previous) if previous == selection => previous,
            Some(_) => NativeCompilationSelection::Unknown,
        });
    }
}

/// Certainty that execution uses a previously selected native operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompiledExecutionCertainty {
    /// Every reaching execution uses this native operation.
    Must,
    /// Native selection is possible, alongside another execution path.
    May,
}

/// Whether native execution has a remaining undescribed alternative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CompiledExecutionResidual {
    /// Every compiler alternative has a retained descriptor.
    #[default]
    Closed,
    /// A compiler alternative has no closed operation descriptor.
    Unknown,
}

/// Whether an inline compiler path was withdrawn by a selected child failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NativeInlineRejection {
    /// No selected child rejection was recorded.
    #[default]
    None,
    /// Every reaching compiler entry withdrew this inline path.
    Definite,
    /// Only some reaching compiler entries withdrew this inline path.
    Possible,
}

impl From<bool> for CompiledExecutionResidual {
    fn from(unknown: bool) -> Self {
        if unknown { Self::Unknown } else { Self::Closed }
    }
}

/// A native operation selected independently of post-argument command lookup.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceCompiledInvocationProof {
    /// Original registry implementation whose compiler hook selected the opcode.
    pub target: SourceCommandTarget,
    /// Registry-authored operation emitted by the native compiler.
    pub operation: tcl_registry::SemanticOperationId,
    /// Point at which command identity was validated, before any operand effects.
    pub guard: NativeCompilationGuard,
    /// Source instance and invocation whose compilation selected this operation.
    pub compilation_site: CommandAllocationSite,
    /// Exact private implementation bindings used by ensemble compilation.
    pub lookup_dependencies: Vec<super::SourceNativeCompilationDependency>,
    /// Exact original compiler registration, independent of the callable origin.
    pub compiler_prerequisite:
        Option<Arc<tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite>>,
    /// Actual opaque procedure header prerequisite, without registry identity donation.
    pub procedure_header_prerequisite:
        Option<Arc<tcl_runtime_api::native_compilation::NativeProcedureHeaderPrerequisite>>,
    /// Whether the native selection covers every reaching execution.
    pub certainty: CompiledExecutionCertainty,
}

/// Compiler-selected recipe, independent of its later runtime guard outcome.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceNativeCompilerAdmission {
    /// Original operation and all compiler-hook prerequisites.
    Inline(Box<SourceCompiledInvocationProof>),
    /// Captured private name and its compiler configuration prerequisites.
    Named(super::SourceNamedInvocationProof),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc(hidden)]
pub struct SourceOriginalCompilerWords {
    pub config: tcl_lexer::LexerConfig,
    pub site: CommandAllocationSite,
    pub words: Arc<[crate::ir::WordExpr]>,
}

impl SourceInvocationBinding {
    /// Exact retained compiler syntax for conditional point metadata only.
    /// Generic/unknown selection preserves unbounded execution; this supplies
    /// no entered handler, body owner, frame, cell or native instruction.
    pub(crate) fn original_compiler_point_words_for_source_metadata(
        &self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Vec<tcl_lexer::NativeWord>> {
        if tokens.source_binding.as_ref() != Some(self)
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
            || !self.has_original_source_layout_owner()
            || self.lookup_state.is_none()
            || self.has_original_native_source_command()
            || input.has_logical_source_name_context()
            || input.has_hosted_source_name_context()
            || !self.execution_is_unknown()
            || self.execution_targets().next().is_some()
            || self.proved_handler_target().is_some()
            || self.compiler_policy.is_none()
            || !matches!(
                self.native_compilation_admission,
                Some(NativeCompilationSelection::Generic | NativeCompilationSelection::Unknown)
            )
            || self.native_compiler_admission.is_some()
            || self.native_structured_preparation.is_some()
            || self.native_switch_preparation.is_some()
            || self.native_operand_layout.is_some()
            || self.conditional_expression_evaluations.is_some()
            || self.native_handler_envelope.is_some()
        {
            return None;
        }
        self.original_invocation_metadata_for_input(tokens, input, registry)?;
        let original = self.original_compiler_words.as_ref()?;
        let config = self.original_lexer_config_for_tokens(tokens)?;
        if original.config != config
            || config != input.lexer_config()
            || &original.site != self.invocation_site()?
            || original.words.as_ref() != tokens.words()
        {
            return None;
        }
        crate::registry_invocation::original_native_compiler_words(
            original.site.source.source_image(),
            &original.words,
            original.site.offset,
            original.config,
        )
    }
}

/// Original compiler declaration geometry, including a declined prefix.
/// This grants no reached namespace alias, variable store or emitted opcode.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceNativeNamespaceBindingPreparation {
    pub(super) compilation_site: CommandAllocationSite,
    pub(super) dependency: super::SourceNativeCompilationDependency,
    pub(super) recipe:
        tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingCompilation,
}

impl SourceNativeNamespaceBindingPreparation {
    /// Source allocation whose original compiler retained this geometry.
    #[must_use]
    pub fn compilation_site(&self) -> &CommandAllocationSite {
        &self.compilation_site
    }

    /// Declared locals and original operand visits; Generic binds never execute.
    #[must_use]
    pub fn recipe(
        &self,
    ) -> &tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingCompilation
    {
        &self.recipe
    }

    /// Original compiler registration, independent of runtime handler lookup.
    #[must_use]
    pub fn dependency(&self) -> &super::SourceNativeCompilationDependency {
        &self.dependency
    }
}

/// Original switch arms selected by the native compiler, independent of dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc(hidden)]
pub struct SourceNativeSwitchPreparation {
    pub compilation_site: CommandAllocationSite,
    pub dependency: super::SourceNativeCompilationDependency,
    pub recipe: tcl_registry::native_switch_compilation::NativeSwitchInstruction,
}

/// Exact original structured compiler preparation, independent of execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceNativeStructuredPreparation {
    #[doc(hidden)]
    pub compilation_site: CommandAllocationSite,
    pub(super) dependency: super::SourceNativeCompilationDependency,
    #[doc(hidden)]
    pub recipe: tcl_registry::native_instruction_plan::NativeInstructionPlan,
}

impl std::hash::Hash for SourceNativeStructuredPreparation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Equality retains the complete recipe. Hashing its authenticated
        // original source and registration permits no syntax reconstruction.
        std::hash::Hash::hash(&self.compilation_site, state);
        std::hash::Hash::hash(&self.dependency, state);
    }
}

impl SourceNativeStructuredPreparation {
    /// Original source allocation whose compiler supplied this preparation.
    #[must_use]
    pub fn compilation_site(&self) -> &CommandAllocationSite {
        &self.compilation_site
    }

    /// Shared ordered native recipe; no reached body or runtime values are implied.
    #[must_use]
    pub fn recipe(&self) -> &tcl_registry::native_instruction_plan::NativeInstructionPlan {
        &self.recipe
    }

    /// Actual compiler registration and lookup incarnation retained at this site.
    #[must_use]
    pub fn dependency(&self) -> &super::SourceNativeCompilationDependency {
        &self.dependency
    }
}

pub(super) fn word_shape(word: &crate::ir::WordExpr) -> NativeCompilationWordShape {
    crate::registry_invocation::native_compilation_word_shape(word)
}

pub(super) fn body_context<'a>(
    index: usize,
    operands: super::SourceScriptOperands<'_>,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'a>,
) -> super::SourceExecutionContext<'a> {
    let Some(dialect) = state.baseline.compilation_dialect() else {
        return super::SourceExecutionContext {
            control: (context)
                .control
                .with_source_purpose(tcl_lexer::SourceSegmentationPurpose::NestedScript)
                .with_compilation(NativeCompilationContext::default()),
            compilation_snapshot: None,
            selected_compilation: None,
            original_variable_compilation: None,
            ..context
        };
    };
    let arguments = operands.arguments.with_dialect(dialect);
    let shape = index
        .checked_sub(operands.target.prepended.len())
        .and_then(|written| operands.words.get(written + 1))
        .map_or(NativeCompilationWordShape::Substituted, word_shape);
    let compilation = operands
        .compilation_spec
        .map_or_else(Default::default, |spec| {
            spec.body_context_for_invocation_operand(
                context.compilation(),
                *operands.compilation_selection,
                shape,
                arguments,
                index,
            )
        });
    let inherited = operands.compilation_spec.is_some_and(|spec| {
        spec.body == tcl_registry::native_compilation::NativeBodyCompilation::Inherit
            && matches!(
                operands.compilation_selection,
                NativeCompilationSelection::Inline { .. }
            )
            && matches!(
                shape,
                NativeCompilationWordShape::Literal
                    | NativeCompilationWordShape::QuotedLiteral
                    | NativeCompilationWordShape::BracedLiteral
            )
    });
    super::SourceExecutionContext {
        control: (context)
            .control
            .with_source_purpose(tcl_lexer::SourceSegmentationPurpose::NestedScript)
            .with_compilation(compilation),
        compilation_snapshot: if inherited {
            context.compilation_snapshot
        } else {
            None
        },
        selected_compilation: None,
        original_variable_compilation: None,
        ..context
    }
}

/// Construct the retained selection outside recursive body interpretation.
#[inline(never)]
pub(super) fn select_invocation_boxed(
    words: &[crate::ir::WordExpr],
    state: &ModuleCommandBindings,
    context: &super::SourceExecutionContext<'_>,
    offset: u32,
) -> Box<CompiledInvocationSelection> {
    #[cfg(any(test, debug_assertions))]
    let _timing =
        super::root_owner_work::Timing::at(super::root_owner_work::Query::CompilerSelection);
    let context = *context;
    Box::new(select_invocation(words, state, context, offset))
}

fn compiler_head_decline(
    words: &[crate::ir::WordExpr],
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Option<CompiledInvocationSelection> {
    match words
        .first()
        .and_then(|word| word_shape(word).compiler_head(dialect))
    {
        Some(true) => None,
        Some(false) => Some(CompiledInvocationSelection::default()),
        None => Some(CompiledInvocationSelection {
            compiler_name_preservation: super::SourceCompilerNameClosure::Unknown,
            unknown: true,
            admission: Some(NativeCompilationSelection::Unknown),
            ..CompiledInvocationSelection::default()
        }),
    }
}

/// A head with no original name proof is the host compiler's to judge; only
/// the rule loader's own statements are that, every other walk stays generic.
fn unproven_head_selection(
    context: &super::SourceExecutionContext<'_>,
) -> CompiledInvocationSelection {
    if !super::rule_declaration::is_rule_loader_top_level(context) {
        return CompiledInvocationSelection::default();
    }
    CompiledInvocationSelection {
        compiler_name_preservation: super::SourceCompilerNameClosure::Unknown,
        unknown: true,
        admission: Some(NativeCompilationSelection::Unknown),
        ..CompiledInvocationSelection::default()
    }
}

pub(super) fn select_invocation(
    words: &[crate::ir::WordExpr],
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
    offset: u32,
) -> CompiledInvocationSelection {
    // The positively retained source-only input requests lexical continuation,
    // not a Native compiler outcome. Its validated ingress excludes Native,
    // hosted and independently authored execution policies.
    // A hosted rule's advisory handler walk is a source interpretation too:
    // a body it enters (`if`, `foreach`) has no compiler outcome of its own.
    if state.logical_source_name_advice_input().is_some()
        || super::logical_definition::hosted_entry(state, context).is_some()
    {
        return CompiledInvocationSelection {
            admission: None,
            ..CompiledInvocationSelection::default()
        };
    }
    // This retained child is a source interpretation with no physical
    // compiler invocation. Missing compiler ingress is not a guest mutation.
    if state
        .baseline
        .source_point_entry
        .as_ref()
        .is_some_and(|point| point.is_source_only_interpretation(state, context))
    {
        return CompiledInvocationSelection {
            admission: None,
            ..CompiledInvocationSelection::default()
        };
    }
    if context.compilation().mode == NativeCompilationMode::Direct {
        return CompiledInvocationSelection::default();
    }
    if let Some(decline) = compiler_head_decline(words, state.baseline.compilation_dialect()) {
        return decline;
    }
    let written = super::source_effective_words(words, state.baseline.dialect, None);
    let Some(head_input) = super::original_name_value::original_static_command_head_input(
        words,
        offset,
        state,
        context.config,
    ) else {
        return unproven_head_selection(&context);
    };
    let head = std::str::from_utf8(head_input.bytes()).unwrap_or("");
    let Some(snapshot) = context.compilation_snapshot else {
        return CompiledInvocationSelection {
            compiler_name_preservation: super::SourceCompilerNameClosure::Unknown,
            unknown: true,
            admission: Some(NativeCompilationSelection::Unknown),
            ..CompiledInvocationSelection::default()
        };
    };
    if state.current_source_origin.as_ref().is_some_and(|source| {
        snapshot.generic_fallbacks.contains(&CommandAllocationSite {
            source: Arc::clone(source),
            offset,
        })
    }) {
        return CompiledInvocationSelection {
            rejection: NativeInlineRejection::Definite,
            admission: Some(NativeCompilationSelection::Generic),
            ..CompiledInvocationSelection::default()
        };
    }
    let targets =
        snapshot.compiler_targets_for_original_input(&head_input, &context.namespace_identity());
    trace_original_compiler_targets(&head_input, &context, &targets);
    if !head_input.is_current(&state.source_variables) {
        return CompiledInvocationSelection {
            compiler_name_preservation: super::SourceCompilerNameClosure::Unknown,
            unknown: true,
            admission: Some(NativeCompilationSelection::Unknown),
            ..Default::default()
        };
    }
    let before = super::source_binding_from_original_input(
        state,
        &head_input,
        &context.namespace_identity(),
    )
    // The immutable chunk compiler already selected its original target.
    // Unavailable later runtime occupancy removes the stable handler relation;
    // it cannot replace that independent compiler snapshot with runtime lookup.
    .unwrap_or_else(SourceInvocationBinding::unknown);
    let shapes = words.iter().skip(1).map(word_shape).collect::<Vec<_>>();
    let arguments = written
        .iter()
        .skip(1)
        .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
        .collect::<Vec<_>>();
    let closed_singleton =
        !targets.unknown && !targets.may_be_generic && targets.targets.len() == 1;
    let mut result =
        initial_compilation_selection(words, state, context, offset, &targets, &before);
    for target in targets.targets {
        select_target(
            &target,
            CompilerInvocation {
                words,
                arguments: &arguments,
                shapes: &shapes,
                before: &before,
                head,
                offset,
            },
            state,
            context,
            &mut result,
        );
    }
    finish_compiler_invocation_selection(&mut result, closed_singleton);
    result
}

fn finish_compiler_invocation_selection(
    result: &mut CompiledInvocationSelection,
    closed_singleton: bool,
) {
    if !closed_singleton {
        result.operand_layout = None;
        result.namespace_bindings = None;
        result.switch = None;
        result.admitted = None;
    }
    close_execution_certainty(result);
}

fn trace_original_compiler_targets(
    head_input: &crate::signature_scan::scope::SignatureSourceNameInput,
    context: &super::SourceExecutionContext<'_>,
    targets: &NativeCompilerTargets,
) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
        eprintln!(
            "NATIVE_ORIGINAL_COMPILER_TARGET head={:?} namespace={:?} generic={} unknown={} targets={:?}",
            head_input.bytes(),
            context.namespace_identity().display(),
            targets.may_be_generic,
            targets.unknown,
            targets
                .targets
                .iter()
                .map(|target| (
                    &target.command,
                    target.registry_backed,
                    target.kind,
                    target
                        .identity
                        .as_ref()
                        .and_then(|identity| identity.runtime),
                ))
                .collect::<Vec<_>>(),
        );
    }
}

fn initial_compilation_selection(
    words: &[crate::ir::WordExpr],
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
    offset: u32,
    targets: &NativeCompilerTargets,
    before: &SourceInvocationBinding,
) -> CompiledInvocationSelection {
    let snapshot = context
        .compilation_snapshot
        .expect("selected original compilation snapshot");
    let stable_handler = stable_compiler_handler(targets, before);
    CompiledInvocationSelection {
        proofs: Vec::new(),
        named: Vec::new(),
        stable_handler,
        compiler_name_preservation: (!targets.unknown).into(),
        operand_layout: None,
        namespace_bindings: None,
        switch: None,
        structured: None,
        original_words: state.current_source_origin.as_ref().and_then(|origin| {
            origin.original_lexical_command(words, offset, context.config)?;
            Some(Arc::new(SourceOriginalCompilerWords {
                config: context.config,
                site: CommandAllocationSite {
                    source: Arc::clone(origin),
                    offset,
                },
                words: Arc::from(words),
            }))
        }),
        admitted: None,
        policy: Some(Arc::new(
            super::compiler_inventory::SourceNativeCompilerPolicy::of_compilation(
                &snapshot.table.state,
                context.realm(),
            ),
        )),
        admission: if targets.unknown {
            Some(NativeCompilationSelection::Unknown)
        } else {
            targets
                .may_be_generic
                .then_some(NativeCompilationSelection::Generic)
        },
        live: targets.may_be_generic,
        unknown: targets.unknown,
        compile_error: false,
        rejection: NativeInlineRejection::None,
    }
}

fn stable_compiler_handler(
    targets: &NativeCompilerTargets,
    before: &SourceInvocationBinding,
) -> Option<SourceCommandTarget> {
    let [target] = targets.targets.as_slice() else {
        return None;
    };
    (!targets.unknown && !targets.may_be_generic && before.proved_target() == Some(target))
        .then(|| target.clone())
}

fn close_execution_certainty(result: &mut CompiledInvocationSelection) {
    let certain = !result.live
        && !result.unknown
        && !result.compile_error
        && result.proofs.len() + result.named.len() == 1;
    if certain {
        if let Some(proof) = result.proofs.first_mut() {
            proof.certainty = CompiledExecutionCertainty::Must;
        }
        if let Some(proof) = result.named.first_mut() {
            proof.certainty = CompiledExecutionCertainty::Must;
        }
    }
}

#[derive(Clone, Copy)]
struct CompilerInvocation<'a> {
    words: &'a [crate::ir::WordExpr],
    arguments: &'a [tcl_registry::InvocationWord<'a>],
    shapes: &'a [NativeCompilationWordShape],
    before: &'a SourceInvocationBinding,
    head: &'a str,
    offset: u32,
}

/// A converged implementation can transfer its normal effects without proving
/// that any particular compiler operation or body traversal was selected.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HandlerSourceTransfer {
    ExpressionArguments,
    NormalEffects,
    NormalResult,
    PossibleBodies,
    UserProcedureCall,
    SubstitutionTemplate,
}

pub(super) fn successful_handler_transfer(
    target: &SourceCommandTarget,
    effective: &[crate::registry_invocation::EffectiveInvocationWord],
    state: &ModuleCommandBindings,
    registry: &tcl_registry::CommandRegistry,
    realm: tcl_dialect::model::InvocationRealm,
) -> Option<HandlerSourceTransfer> {
    if !target.registry_backed {
        return (target.kind == super::BindingKind::Proc
            && target.implementation_allocation.is_some())
        .then_some(HandlerSourceTransfer::UserProcedureCall);
    }
    let mut arguments = target
        .prepended
        .iter()
        .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
        .collect::<Vec<_>>();
    arguments.extend(
        effective
            .iter()
            .skip(1)
            .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word),
    );
    let mut invocation = tcl_registry::InvocationWords::structured(
        tcl_registry::InvocationWord::Literal(&target.command),
        &arguments,
    );
    if let Some(dialect) = state.baseline.dialect {
        invocation = invocation.with_dialect(dialect);
    }
    super::resolve_source_invocation_facts(
        registry,
        registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile),
        invocation,
        realm,
    )
    .and_then(|facts| {
        use tcl_registry::native_compilation::NormalHandlerImplementationLookup;
        match facts.normal_handler_implementation_lookup(state.baseline.dialect) {
            NormalHandlerImplementationLookup::NoneRequired => {}
            // An earlier ensemble creation may have replaced the stock
            // ensemble whose private worker this handler requires.
            NormalHandlerImplementationLookup::Required(_)
            | NormalHandlerImplementationLookup::RequiredPath(_)
                if state.namespace_resolution.ensembles.created =>
            {
                return None;
            }
            NormalHandlerImplementationLookup::Required(lookup) => {
                let namespace = state.variable_frame_namespace_key()?;
                if !effective
                    .first()
                    .and_then(|word| word.as_registry_word().literal())
                    .is_some_and(|head| {
                        super::source_binding(state, head, &namespace)
                            .proves_native_implementation_lookup(&lookup)
                    })
                {
                    return None;
                }
            }
            NormalHandlerImplementationLookup::RequiredPath(paths) => {
                let namespace = state.variable_frame_namespace_key()?;
                let head = effective.first()?.as_registry_word().literal()?;
                if !super::source_binding(state, head, &namespace).proves_native_handler_path(
                    paths,
                    invocation.arguments(),
                    facts.argument_offset,
                ) {
                    return None;
                }
            }
            NormalHandlerImplementationLookup::Unknown => return None,
        }
        let frame = state.source_variables.alias_frame();
        if facts.successful_handler
            == Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::ExpressionArguments)
            && facts.arity_accepts_frozen_arguments() == Some(true)
            && facts.arg_roles_complete
        {
            Some(HandlerSourceTransfer::ExpressionArguments)
        } else if facts.body_execution
            == Some(tcl_registry::body_execution::BodyExecutionSpec::SubstitutionTemplate)
            && invocation.arguments().exact_argv_len().is_some()
        {
            Some(HandlerSourceTransfer::SubstitutionTemplate)
        } else if facts
            .successful_handler_effects(invocation.arguments(), frame)
            .is_some()
        {
            Some(HandlerSourceTransfer::NormalEffects)
        } else if facts
            .successful_handler_user_procedure_call(invocation.arguments())
            .is_some()
        {
            Some(HandlerSourceTransfer::UserProcedureCall)
        } else if facts
            .normal_list_method_provider(invocation.arguments())
            .is_some()
        {
            Some(HandlerSourceTransfer::NormalResult)
        } else {
            facts
                .possible_handler_body_flow(registry, invocation.arguments(), frame)
                .map(|_| HandlerSourceTransfer::PossibleBodies)
        }
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_logical_compiler_purpose_tests;

#[cfg(test)]
mod compiler_point_source_metadata_tests;
