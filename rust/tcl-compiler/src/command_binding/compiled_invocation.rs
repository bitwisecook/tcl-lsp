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
    Arc, BTreeSet, BindingKind, CommandAllocationSite, MayBinding, ModuleCommandBindings,
    SourceCommandTarget, SourceInvocationBinding, SourceLookupSnapshot,
};
use tcl_registry::native_compilation::{
    NativeCompilationContext, NativeCompilationGuard, NativeCompilationMode,
    NativeCompilationSelection, NativeCompilationWordShape,
};

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

/// Immutable command table at a native compilation boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCompilationSnapshot {
    pub(super) table: Arc<SourceLookupSnapshot>,
    pub(super) generic_fallbacks: BTreeSet<CommandAllocationSite>,
    pub(super) source_locals:
        Option<Arc<super::compiled_preflight::ordered_locals::SourceCompilerLocalInventory>>,
}

/// Compiler-hook-capable tokens found by the shared command lookup owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCompilerTargets {
    /// Original or imported native tokens; name aliases never forward compiler hooks.
    pub targets: Vec<SourceCommandTarget>,
    /// Lookup may instead reach a token without a native compiler hook.
    pub may_be_generic: bool,
    /// The table or lookup route contains an unbounded alternative.
    pub unknown: bool,
}

impl NativeCompilationSnapshot {
    pub(super) fn compiler_targets_for_original_input(
        &self,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        namespace: &super::SourceNamespaceKey,
    ) -> NativeCompilerTargets {
        self.table
            .state
            .native_compiler_targets_for_original_input(input, namespace)
    }

    /// Query the same immutable lookup paths used by ordinary command resolution.
    #[must_use]
    pub fn compiler_targets(&self, head: &str, namespace: &str) -> NativeCompilerTargets {
        self.compiler_targets_in_namespace(head, namespace)
    }

    pub(super) fn compiler_targets_in_namespace(
        &self,
        head: &str,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
    ) -> NativeCompilerTargets {
        self.table.state.native_compiler_targets(head, namespace)
    }
}

impl ModuleCommandBindings {
    /// Compiler-hook projection of the actual byte table cells. Callable
    /// target resolution cannot forward an alias's compiler registration.
    pub(super) fn native_compiler_targets_for_original_input(
        &self,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        namespace: &super::SourceNamespaceKey,
    ) -> NativeCompilerTargets {
        self.trace_original_compiler_world(input, namespace);
        if self.source_step_observed()
            || self
                .baseline
                .native_entry
                .as_ref()
                .is_some_and(|entry| entry.inline_compilation_disabled)
        {
            return NativeCompilerTargets {
                targets: Vec::new(),
                may_be_generic: true,
                unknown: false,
            };
        }
        let Some(selection) = self.original_targets_for_input(
            input,
            namespace,
            super::CommandTargetLookup::CompilerSlots,
        ) else {
            if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
                eprintln!(
                    "NATIVE_ORIGINAL_COMPILER_LOOKUP head={:?} reason=target-unavailable namespace={namespace:?}",
                    input.bytes()
                );
            }
            return NativeCompilerTargets {
                targets: Vec::new(),
                may_be_generic: true,
                unknown: true,
            };
        };
        let Some(keys) = self.original_command_keys_for_input(namespace, input) else {
            if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
                eprintln!(
                    "NATIVE_ORIGINAL_COMPILER_LOOKUP head={:?} reason=keys-unavailable namespace={namespace:?}",
                    input.bytes()
                );
            }
            return NativeCompilerTargets {
                targets: Vec::new(),
                may_be_generic: true,
                unknown: true,
            };
        };
        let mut eligible = BTreeSet::new();
        let mut generic = selection.may_be_absent;
        let mut unknown = selection.unknown;
        if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
            eprintln!(
                "NATIVE_ORIGINAL_COMPILER_LOOKUP head={:?} selected_unknown={} absent={} keys={}",
                input.bytes(),
                selection.unknown,
                selection.may_be_absent,
                keys.len(),
            );
        }
        for key in keys {
            let bindings = self.original_bindings_for_key(&key);
            self.trace_original_compiler_cell(&key, bindings.as_ref());
            let Some(bindings) = bindings else {
                unknown = true;
                continue;
            };
            for binding in &bindings {
                // A missing earlier cell falls through along the retained
                // lookup path; total absence is a separate selection axis.
                if matches!(binding, MayBinding::Missing) {
                    continue;
                }
                self.collect_native_compiler_tokens(
                    binding,
                    &mut eligible,
                    &mut generic,
                    &mut unknown,
                    &mut BTreeSet::new(),
                    None,
                );
            }
        }
        let targets = self.current_original_compiler_targets(eligible, &mut generic);
        NativeCompilerTargets {
            may_be_generic: generic || unknown || targets.is_empty(),
            targets,
            unknown,
        }
    }

    fn current_original_compiler_targets(
        &self,
        eligible: BTreeSet<super::ResolvedCommandTarget>,
        generic: &mut bool,
    ) -> Vec<SourceCommandTarget> {
        eligible
            .into_iter()
            .filter_map(|target| {
                if self.source_execution_observed(target.token.as_ref()) {
                    *generic = true;
                    return None;
                }
                Some(SourceCommandTarget {
                    runtime_implementation_generation: self
                        .runtime_implementation_generation(target.token.as_ref()),
                    command: target.command,
                    prepended: target.prepended,
                    original_prepended: None,
                    logical_original_prefix: None,
                    registry_backed: target.registry_backed,
                    kind: target.kind,
                    identity: target.token,
                    implementation_generation: target.implementation_generation,
                    implementation_allocation: target.implementation_allocation,
                })
            })
            .collect::<Vec<_>>()
    }

    fn trace_original_compiler_world(
        &self,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        namespace: &super::SourceNamespaceKey,
    ) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
            let mut world = (*self.original_command_world).clone();
            let policy = world.select_policy(self);
            eprintln!(
                "NATIVE_ORIGINAL_COMPILER_WORLD head={:?} opaque={} policy={:?} input_policy={:?} scope={:?} root={:?} namespace_rows={:?}",
                input.bytes(),
                self.has_opaque_domain(),
                policy,
                input.policy(),
                policy.and_then(|policy| world.scope(namespace, policy)),
                self.source_root_namespace_key(),
                self.baseline.native_entry.as_ref().map(|entry| entry
                    .namespaces
                    .iter()
                    .map(|row| (
                        row.token,
                        row.path.is_root(),
                        row.jim_namespace_object.is_some()
                    ))
                    .collect::<Vec<_>>()),
            );
        }
    }

    fn trace_original_compiler_cell(
        &self,
        key: &super::SourceCommandKey,
        bindings: Option<&BTreeSet<MayBinding>>,
    ) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
            eprintln!(
                "NATIVE_ORIGINAL_COMPILER_CELL key={key:?} alternatives={:?}",
                bindings.as_ref().map(|alternatives| alternatives
                    .iter()
                    .map(|binding| {
                        let target = match binding {
                            MayBinding::Target(target) => Some(target),
                            _ => None,
                        };
                        (
                            match binding {
                                MayBinding::Target(_) => "target",
                                MayBinding::Imported(_) => "imported",
                                MayBinding::Missing => "missing",
                                MayBinding::Unknown => "unknown",
                            },
                            target.map(|target| target.command.as_str()),
                            target.map(|target| target.registry_backed),
                            target
                                .and_then(|target| target.token.as_ref())
                                .and_then(|token| token.runtime),
                            target.and_then(|target| self.installed_compiler_hook(target)),
                        )
                    })
                    .collect::<Vec<_>>()),
            );
        }
    }

    /// Freeze actual point knowledge; a catalogue alone never supplies an entry contract.
    #[must_use]
    pub fn native_compilation_snapshot(&self) -> NativeCompilationSnapshot {
        self.native_compilation_snapshot_in_realm(self.baseline.invocation_realm)
    }

    pub(super) fn native_compilation_snapshot_in_realm(
        &self,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> NativeCompilationSnapshot {
        #[cfg(any(test, debug_assertions))]
        let _timing = super::root_owner_work::Timing::at(super::root_owner_work::Query::Snapshot);
        NativeCompilationSnapshot {
            table: Arc::new(SourceLookupSnapshot::in_realm(self.clone(), realm)),
            generic_fallbacks: BTreeSet::new(),
            source_locals: None,
        }
    }

    pub(super) fn native_compiler_targets(
        &self,
        head: &str,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
    ) -> NativeCompilerTargets {
        if self.source_step_observed()
            || self
                .baseline
                .native_entry
                .as_ref()
                .is_some_and(|entry| entry.inline_compilation_disabled)
        {
            return NativeCompilerTargets {
                targets: Vec::new(),
                may_be_generic: true,
                unknown: false,
            };
        }
        let mut eligible = BTreeSet::new();
        let mut generic = false;
        // Execution observers widen the eventual dispatch, but a closed
        // compilation table still proves their compiler-hook suppression.
        let mut unknown = !self.source_lookup_is_closed(head, namespace);
        for slot in self.source_keys(head, namespace) {
            let bindings = self.binding_alternatives(&slot);
            for binding in &bindings {
                self.collect_native_compiler_tokens(
                    binding,
                    &mut eligible,
                    &mut generic,
                    &mut unknown,
                    &mut BTreeSet::new(),
                    None,
                );
            }
        }
        // Compiler targets are a separate projection. An imported token can
        // retain a compiler registration after its callable origin changes.
        let targets: Vec<_> = eligible
            .into_iter()
            .map(|target| SourceCommandTarget {
                runtime_implementation_generation: self
                    .runtime_implementation_generation(target.token.as_ref()),
                command: target.command,
                prepended: target.prepended,
                original_prepended: None,
                logical_original_prefix: None,
                registry_backed: target.registry_backed,
                kind: target.kind,
                identity: target.token,
                implementation_generation: target.implementation_generation,
                implementation_allocation: target.implementation_allocation,
            })
            .collect();
        let targets = targets
            .into_iter()
            .filter(|target| {
                if self.source_execution_observed(
                    self.compiler_identity_at_lookup(head, namespace)
                        .or(target.identity.as_ref()),
                ) {
                    generic = true;
                    false
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();
        NativeCompilerTargets {
            may_be_generic: generic || unknown || targets.is_empty(),
            targets,
            unknown,
        }
    }

    fn collect_native_compiler_tokens(
        &self,
        binding: &MayBinding,
        eligible: &mut BTreeSet<super::ResolvedCommandTarget>,
        generic: &mut bool,
        unknown: &mut bool,
        visiting: &mut BTreeSet<super::CommandIdentity>,
        raw_import: Option<&super::CommandIdentity>,
    ) {
        if let (MayBinding::Target(target), Some(raw)) = (binding, raw_import) {
            self.collect_original_imported_compiler(target, raw, eligible, unknown);
            return;
        }
        if let MayBinding::Target(target) = binding
            && target.terminal
            && target.implementation_generation == 0
            && target.implementation_allocation.is_none()
            && let Some(raw) = target.token.as_ref()
            && let Some(row) = self.runtime_compiler_row(raw)
            && row.compiler.is_some()
        {
            match self.runtime_compiler_hook(Some(raw)) {
                Some(true) => self.collect_native_compiler_tokens(
                    binding,
                    eligible,
                    generic,
                    unknown,
                    visiting,
                    Some(raw),
                ),
                Some(false) => *generic = true,
                None => *unknown = true,
            }
            return;
        }
        if let MayBinding::Target(target) = binding
            && target.kind == BindingKind::Alias
            && !target.terminal
            && target.implementation_generation == 0
            && target.implementation_allocation.is_none()
            && let Some(identity) = target.token.as_ref()
            && let Some(row) = self.runtime_compiler_row(identity)
            && matches!(row.implementation,
                tcl_runtime_api::native_compilation::NativeCommandImplementation::Alias { .. }
                    | tcl_runtime_api::native_compilation::NativeCommandImplementation::PartialAlias { .. })
        {
            match self.runtime_compiler_hook(Some(identity)) {
                Some(false) => *generic = true,
                Some(true) | None => *unknown = true,
            }
            return;
        }
        match binding {
            MayBinding::Target(target)
                if self
                    .runtime_noop_header(
                        target.token.as_ref(),
                        target.implementation_generation,
                        target.implementation_allocation.as_ref(),
                    )
                    .is_some() =>
            {
                eligible.insert(target.clone());
            }
            MayBinding::Target(target)
                if self
                    .runtime_ensemble_compiler_for_identity(
                        target.token.as_ref(),
                        target.implementation_generation,
                        target.implementation_allocation.as_ref(),
                    )
                    .is_some() =>
            {
                eligible.insert(target.clone());
            }
            MayBinding::Target(target)
                if target.terminal && self.installed_compiler_hook(target) == Some(false) =>
            {
                *generic = true;
            }
            MayBinding::Target(target)
                if target.terminal
                    && target.kind == BindingKind::Builtin
                    && target.registry_backed =>
            {
                eligible.insert(target.clone());
            }
            MayBinding::Target(target)
                if target.terminal
                    && target
                        .token
                        .as_ref()
                        .is_some_and(|token| token.runtime.is_some()) =>
            {
                if self.runtime_compiler_hook(target.token.as_ref()) == Some(false) {
                    *generic = true;
                } else {
                    // An opaque runtime implementation proves no semantic
                    // descriptor. Only its independent actual no-hook axis
                    // closes compilation as a generic invocation.
                    *unknown = true;
                }
            }
            MayBinding::Target(_) | MayBinding::Missing => *generic = true,
            MayBinding::Unknown => *unknown = true,
            MayBinding::Imported(import) => self.collect_native_compiler_import(
                import, eligible, generic, unknown, visiting, raw_import,
            ),
        }
    }

    fn collect_original_imported_compiler(
        &self,
        target: &super::ResolvedCommandTarget,
        raw: &super::CommandIdentity,
        eligible: &mut BTreeSet<super::ResolvedCommandTarget>,
        unknown: &mut bool,
    ) {
        let Some(row) = self.runtime_compiler_row(raw) else {
            *unknown = true;
            return;
        };
        let mut compiler = target.clone();
        compiler.token = Some(raw.clone());
        compiler.prepended.clear();
        compiler.implementation_generation = 0;
        compiler.implementation_allocation = None;
        compiler.terminal = true;
        if let Some(registration) = &row.compiler {
            compiler.command.clone_from(&registration.registry_identity);
            compiler.registry_backed = true;
            compiler.kind = BindingKind::Builtin;
            eligible.insert(compiler);
        } else if self.runtime_noop_header(Some(raw), 0, None).is_some() {
            // This header has no registry handler identity to donate.
            compiler.registry_backed = false;
            compiler.kind = BindingKind::Command;
            eligible.insert(compiler);
        } else {
            *unknown = true;
        }
    }
    fn collect_native_compiler_import(
        &self,
        import: &super::ImportedCommandBinding,
        eligible: &mut BTreeSet<super::ResolvedCommandTarget>,
        generic: &mut bool,
        unknown: &mut bool,
        visiting: &mut BTreeSet<super::CommandIdentity>,
        raw_import: Option<&super::CommandIdentity>,
    ) {
        let raw = raw_import.or(import.compiler.as_ref());
        if let Some(raw) = raw {
            match self.runtime_compiler_hook(Some(raw)) {
                Some(false) => {
                    *generic = true;
                    return;
                }
                None => {
                    *unknown = true;
                    return;
                }
                Some(true) => {}
            }
        } else if import.origin.runtime.is_some() {
            // A new import has no actual raw-token row in this closed
            // runtime entry. Its origin cannot donate that missing axis.
            *unknown = true;
            return;
        }
        if let Some(raw) = raw
            && let Some(row) = self.runtime_compiler_row(raw)
            && (row.compiler.is_some() || self.runtime_noop_header(Some(raw), 0, None).is_some())
        {
            let target = super::ResolvedCommandTarget {
                command: raw.origin.clone(),
                prepended: Vec::new(),
                registry_backed: false,
                kind: BindingKind::Command,
                implementation_generation: 0,
                implementation_allocation: None,
                terminal: true,
                target_lookup: tcl_registry::AliasTargetLookup::Global,
                token: Some(raw.clone()),
            };
            self.collect_native_compiler_tokens(
                &MayBinding::Target(target),
                eligible,
                generic,
                unknown,
                visiting,
                Some(raw),
            );
            return;
        }
        let token = &import.origin;
        if !visiting.insert(token.clone()) {
            *unknown = true;
            return;
        }
        if let Some(bindings) = self.objects.get(token) {
            for binding in bindings {
                self.collect_native_compiler_tokens(
                    binding, eligible, generic, unknown, visiting, raw,
                );
            }
        } else {
            *unknown = true;
        }
        visiting.remove(token);
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

pub(super) fn select_invocation(
    words: &[crate::ir::WordExpr],
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
    offset: u32,
) -> CompiledInvocationSelection {
    // The positively retained source-only input requests lexical continuation,
    // not a Native compiler outcome. Its validated ingress excludes Native,
    // hosted and independently authored execution policies.
    if state.logical_source_name_advice_input().is_some() {
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
        return CompiledInvocationSelection::default();
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

fn select_target(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
    result: &mut CompiledInvocationSelection,
) {
    if let Some(prerequisite) = state.runtime_noop_header(
        state
            .compiler_identity_at_lookup(selected.head, &context.namespace_identity())
            .or(target.identity.as_ref()),
        target.implementation_generation,
        target.implementation_allocation.as_ref(),
    ) {
        select_noop_target(target, selected, prerequisite, state, context, result);
        return;
    }
    let registered = registered_compiler_selection(target, selected, state, context);
    if let Some(snapshot) = context.compilation_snapshot
        && let Some(plan) = snapshot.table.state.actual_ensemble_plan(
            target,
            selected.head,
            &context.namespace_identity(),
            state.current_source_origin.as_ref().map(|origin| (
                crate::registry_invocation::OriginalNativeCompilerInvocation {
                    image: origin.source_image(), words: selected.words, offset: selected.offset,
                    config: context.config,
                    source_protocol: super::compiler_inventory::SourceNativeCompilerPolicy::source_protocol_of(state),
                    compiler_dialect: state.baseline.compilation_dialect(), context: context.compilation(), operand_from: 2,
                }, context.registry,
            )),
        )
    {
        // A stock public ensemble can delegate to a replaced opaque worker.
        // Its actual configuration then selects a named invocation independently
        // of the registry implementation prerequisites for the original opcode.
        if !target.registry_backed
            || !matches!(
                plan,
                super::ensemble_compilation::ActualEnsemblePlan::Unknown
            )
        {
            result.compiler_name_preservation = super::SourceCompilerNameClosure::Unknown;
            select_actual_ensemble_target(target, selected, plan, state, context, result);
            return;
        }
    }
    let Some(registered) = registered else {
        // `TclCompileScript` invokes a command without a compile procedure;
        // the runtime certifies that this registration has none.
        if closed_surface_catalogue_dispatch(target, state)
            || state.runtime_compiler_hook(target.identity.as_ref()) == Some(false)
            || basic_member_invoked_by_its_private_name(target, state, context)
        {
            result.live = true;
            result.admit(NativeCompilationSelection::Generic);
            return;
        }
        result.compiler_name_preservation = super::SourceCompilerNameClosure::Unknown;
        result.unknown = true;
        result.admit(NativeCompilationSelection::Unknown);
        return;
    };
    result
        .compiler_name_preservation
        .join(registered.compiler_preserves_names.into());
    result.operand_layout = registered.operand_layout;
    result.namespace_bindings = registered.namespace_bindings;
    result.switch = registered.switch;
    result.structured = registered.structured;
    result.admit(registered.admission);
    let spec = registered.spec;
    let admission = registered.admission;
    match admission {
        NativeCompilationSelection::NamedInvocation {
            lookup,
            arguments_from,
            protocol,
        } => {
            select_named_target(
                target,
                selected,
                RegisteredNamedInvocation {
                    preparations: result.structured.as_ref().and_then(|prepared|match prepared.recipe() {tcl_registry::native_instruction_plan::NativeInstructionPlan::NamedInvocation(recipe)=>Some(recipe.preparations.clone()),_=>None}).unwrap_or_default(),
                    lookup,
                    arguments_from,
                    protocol,
                    dependencies: registered.dependencies,
                },
                state,
                context,
                result,
            );
        }
        NativeCompilationSelection::Generic => result.live = true,
        NativeCompilationSelection::Unknown => result.unknown = true,
        NativeCompilationSelection::CompileError => result.compile_error = true,
        NativeCompilationSelection::Inline { operation, guard } => {
            select_inline_target(
                target,
                selected,
                (spec, operation, guard),
                state,
                context,
                result,
            );
        }
    }
}

struct RegisteredCompilerSelection {
    spec: tcl_registry::native_compilation::NativeCompilationSpec,
    admission: NativeCompilationSelection,
    compiler_preserves_names: bool,
    dependencies: Vec<super::SourceNativeCompilationDependency>,
    operand_layout: Option<Arc<super::SourceNativeOperandLayoutProof>>,
    namespace_bindings: Option<Arc<SourceNativeNamespaceBindingPreparation>>,
    switch: Option<Arc<SourceNativeSwitchPreparation>>,
    structured: Option<Arc<SourceNativeStructuredPreparation>>,
}

/// A closed load-time surface ([`Family::closed_load_time_resolution`])
/// resolves a literal catalogue head when the rule loads. Its vendor compiler
/// owns no tracked command or namespace name and dispatches to the catalogued
/// handler, as the surface's direct evaluation does, so an undescribed hook
/// selects that handler generically after argv.
///
/// [`Family::closed_load_time_resolution`]: tcl_dialect::model::Family::closed_load_time_resolution
/// A registered member whose compiler is `TclCompileBasicNArgCmd`, reached by
/// its own private name where the registry resolves no invocation facts (a
/// nested worker such as `::tcl::binary::encode::hex`). The basic compiler and
/// its fallback both push the written words and emit `invokeStk`; tclsh
/// 8.6.18 and 9.0.4 disassemble `::tcl::binary::encode::hex X` to
/// `push ::tcl::binary::encode::hex; push X; invokeStk1 2`.
fn basic_member_invoked_by_its_private_name(
    target: &SourceCommandTarget,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
) -> bool {
    if !target.registry_backed || !target.prepended.is_empty() {
        return false;
    }
    let Some(dialect) = state.baseline.compilation_dialect() else {
        return false;
    };
    context
        .registry
        .native_compilation_for_registration(&target.command, dialect)
        .and_then(tcl_registry::native_compilation::NativeCompilationSpec::named_member_slot)
        .is_some_and(|slot| {
            tcl_syntax::naming::unroot_rooted_key(slot).unwrap_or(slot)
                == tcl_syntax::naming::unroot_rooted_key(&target.command).unwrap_or(&target.command)
        })
}

fn closed_surface_catalogue_dispatch(
    target: &SourceCommandTarget,
    state: &ModuleCommandBindings,
) -> bool {
    target.registry_backed
        && target.kind == BindingKind::Builtin
        && target.prepended.is_empty()
        && state
            .baseline
            .dialect
            .and_then(tcl_registry::InvocationDialect::family)
            .is_some_and(tcl_dialect::model::Family::closed_load_time_resolution)
}

fn registered_compiler_selection(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
) -> Option<RegisteredCompilerSelection> {
    let CompilerInvocation {
        arguments, shapes, ..
    } = selected;
    let mut invocation = tcl_registry::InvocationWords::structured(
        tcl_registry::InvocationWord::Literal(&target.command),
        arguments,
    );
    if let Some(dialect) = state.baseline.compilation_dialect() {
        invocation = invocation.with_dialect(dialect);
    }
    let facts = super::resolve_source_invocation_facts(
        context.registry,
        context
            .registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile),
        invocation,
        context.realm(),
    );
    let Some(facts) = facts.as_ref() else {
        trace_missing_compiler_facts(target, state, context);
        return None;
    };
    let descriptor = original_registration_descriptor(
        selected.words,
        selected.head,
        selected.offset,
        facts,
        state,
        context,
    );
    let (spec, argument_offset) = descriptor?;
    let original =
        original_registered_compiler_preparation(spec, selected, state, context, argument_offset);
    let admission = original.as_ref().map_or_else(
        || registered_fallback_admission(spec, selected, facts, state, context, invocation),
        |(selection, _)| *selection,
    );
    let dependencies = compiler_dependencies(spec, target, state, context);
    let dependency_closed = dependencies.is_some();
    let compiler_preserves_names = original_compiler_preserves_names(
        spec,
        selected,
        state,
        context,
        argument_offset,
        dependency_closed,
    );
    let (original_site, original_dependency) =
        original_compilation_point(target, selected, state, context, admission);
    let preparation = dependency_closed
        .then(|| original.as_ref()?.1.as_ref())
        .flatten();
    let OriginalCompilerPreparations {
        namespace_bindings,
        switch,
        structured,
    } = retain_original_preparations(preparation, original_site.as_ref(), &original_dependency);
    let operand_layout = super::operand_layout::retain_layout(
        spec,
        super::operand_layout::LayoutInvocation {
            invocation,
            shapes,
            target,
            head: selected.head,
            offset: selected.offset,
        },
        dependency_closed,
        state,
        context,
    );
    Some(RegisteredCompilerSelection {
        spec,
        compiler_preserves_names,
        dependencies: dependencies.unwrap_or_default(),
        admission: if dependency_closed {
            admission
        } else {
            NativeCompilationSelection::Unknown
        },
        operand_layout,
        namespace_bindings,
        switch,
        structured,
    })
}

fn registered_fallback_admission(
    spec: tcl_registry::native_compilation::NativeCompilationSpec,
    selected: CompilerInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
    invocation: tcl_registry::InvocationWords<'_>,
) -> NativeCompilationSelection {
    let shapes = selected.shapes;
    if original_named_selection_is_closed(selected, facts, state, context) {
        spec.select_for_original_facts(
            invocation,
            shapes,
            facts,
            state.baseline.compilation_dialect(),
            context.compilation(),
        )
    } else {
        spec.select_for_facts(
            invocation,
            shapes,
            facts,
            state.baseline.compilation_dialect(),
            context.compilation(),
        )
    }
}

/// Whether the retained original command may select a named ensemble
/// compiler from its resolved facts. The handler must declare its transitions:
/// none at all, or only the namespace pattern statements whose closed transfer
/// and premises the namespace-binding seal owns. An undeclared or other
/// transition keeps the compiler selection unknown, and so does an earlier
/// ensemble creation, which may have replaced a stock ensemble command.
fn original_named_selection_is_closed(
    selected: CompilerInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
) -> bool {
    if state.namespace_resolution.ensembles.created {
        return false;
    }
    let Some(origin) = state.current_source_origin.as_ref() else {
        return false;
    };
    if origin
        .original_lexical_command(selected.words, selected.offset, context.config)
        .is_none()
    {
        return false;
    }
    // The compileProc selects its named invocation from the original vector
    // alone (`namespace import` is `TclCompileBasicMin0ArgCmd` in 8.6–9.1,
    // like `export` and `forget`). Whether the invocation's own transitions
    // then close (an `auto_import` prelude, an occupied destination) is the
    // handler's obligation after dispatch, as in a directly evaluated
    // script; it does not make the compiler's selection unknown.
    facts.state_transitions.is_declared()
}

fn original_compilation_point(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
    admission: NativeCompilationSelection,
) -> (
    Option<CommandAllocationSite>,
    super::SourceNativeCompilationDependency,
) {
    let original_site = state
        .current_source_origin
        .as_ref()
        .map(|source| CommandAllocationSite {
            source: Arc::clone(source),
            offset: selected.offset,
        });
    let preparation_guard = match admission {
        NativeCompilationSelection::Inline { guard, .. } => guard,
        _ => NativeCompilationGuard::ChunkEntry,
    };
    let dependency =
        original_compilation_dependency(target, selected, state, context, preparation_guard);
    (original_site, dependency)
}

fn trace_missing_compiler_facts(
    target: &SourceCommandTarget,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
        eprintln!(
            "NATIVE_ORIGINAL_COMPILER_FACTS_MISSING command={:?} registry={} compiler_hook={:?}",
            target.command,
            target.registry_backed,
            context
                .compilation_snapshot
                .map_or(state, |snapshot| &snapshot.table.state)
                .runtime_compiler_hook(target.identity.as_ref()),
        );
    }
}

fn original_compilation_dependency(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
    preparation_guard: NativeCompilationGuard,
) -> super::SourceNativeCompilationDependency {
    super::SourceNativeCompilationDependency {
        compiler_prerequisite: state
            .runtime_command_compiler_prerequisite(
                selected.head,
                &context.namespace_identity(),
                match preparation_guard {
                    NativeCompilationGuard::ChunkEntry => {
                        tcl_runtime_api::CommandBindingGuard::ChunkEntry
                    }
                    NativeCompilationGuard::BeforeArguments => {
                        tcl_runtime_api::CommandBindingGuard::BeforeArguments
                    }
                },
            )
            .map(Arc::new),
        target: target.clone(),
        namespace: context.namespace.to_owned(),
        namespace_key: context.namespace_identity(),
        head: selected.head.to_owned(),
        guard: preparation_guard,
    }
}

fn original_compiler_preserves_names(
    spec: tcl_registry::native_compilation::NativeCompilationSpec,
    selected: CompilerInvocation<'_>,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
    argument_offset: usize,
    dependency_closed: bool,
) -> bool {
    let Some(origin) = state.current_source_origin.as_ref() else {
        return false;
    };
    let Some(projection) =
        origin.original_lexical_command(selected.words, selected.offset, context.config)
    else {
        return false;
    };
    let Some(protocol) =
        super::compiler_inventory::SourceNativeCompilerPolicy::source_protocol_of(state)
    else {
        return false;
    };
    let Ok(captured) = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        projection.native_words(),
        protocol,
    ) else {
        return false;
    };
    let dialect = state.baseline.compilation_dialect();
    let compiler_state = context
        .compilation_snapshot
        .map_or(state, |snapshot| &snapshot.table.state);
    match spec.original_argument_name_effects_in_context(
        &captured,
        argument_offset + 1,
        dialect,
        context.compilation(),
    ) {
        tcl_registry::native_compilation::NativeOriginalCompilerNameEffects::Preserved => {
            // A selected null compileProc cannot enter a private compiler worker.
            dialect.is_some_and(|dialect| spec.compiler_hook_presence(dialect) == Some(false))
                || dependency_closed
        }
        tcl_registry::native_compilation::NativeOriginalCompilerNameEffects::FixedLookup(
            lookup,
        ) => {
            dependency_closed
                && original_fixed_compiler_lookup_preserves_names(
                    compiler_state,
                    lookup,
                    compiler_state
                        .baseline
                        .execution_name_policy
                        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe),
                )
        }
        tcl_registry::native_compilation::NativeOriginalCompilerNameEffects::FixedLookupThenVisits { lookup, visits } => {
            dependency_closed
                && original_fixed_compiler_lookup_preserves_names(
                    compiler_state,
                    lookup,
                    compiler_state.baseline.execution_name_policy.and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe),
                )
                && super::original_compiler_effects::preserves_visits(&captured, visits, compiler_state, context, origin)
        }
        tcl_registry::native_compilation::NativeOriginalCompilerNameEffects::Visits(visits) => {
            dependency_closed
                && super::original_compiler_effects::preserves_visits(
                    &captured,
                    visits,
                    compiler_state,
                    context,
                    origin,
                )
        }
        tcl_registry::native_compilation::NativeOriginalCompilerNameEffects::Unknown => false,
    }
}

pub(super) fn original_fixed_compiler_lookup_preserves_names(
    state: &ModuleCommandBindings,
    lookup: &tcl_registry::native_compilation::NativeCompilerImplementationLookup,
    policy: Option<tcl_syntax::naming::NamePolicyProtocol>,
) -> bool {
    if state.baseline.unknown_entry {
        return false;
    }
    // Actual runtime lookup absence has its own same-entry issuer. Closed
    // tables and the authored initial world cannot supply this callback axis.
    if let Some(entry) = &state.baseline.native_entry
        && !entry
            .command_resolvers
            .is_some_and(|inventory| inventory.permits_no_callbacks(entry))
    {
        return false;
    }
    let Some(policy) = policy else {
        return false;
    };
    let Some(root) = state.source_root_namespace_key() else {
        return false;
    };
    let Some(target) = state.original_registry_metadata_target(&root, lookup.slot, policy) else {
        return false;
    };
    target.registry_backed
        && target.terminal
        && target.kind == BindingKind::Builtin
        && target.implementation_generation == 0
        && target.prepended.is_empty()
        && tcl_registry::command_descriptor_labels_match(&target.command, lookup.slot)
}

/// A monolithic compileProc owns the complete original command vector. The
/// runtime member's body and effect descriptor does not replace that compiler.
pub(super) fn original_registration_descriptor(
    words: &[crate::ir::WordExpr],
    head: &str,
    offset: u32,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
) -> Option<(
    tcl_registry::native_compilation::NativeCompilationSpec,
    usize,
)> {
    let compiler_state = context
        .compilation_snapshot
        .map_or(state, |snapshot| &snapshot.table.state);
    let registered = compiler_state.runtime_command_compiler_prerequisite(
        head,
        &context.namespace_identity(),
        tcl_runtime_api::CommandBindingGuard::ChunkEntry,
    );
    if let Some(registered) = registered
        && registered.compiler.ensemble.is_none()
    {
        let origin = state.current_source_origin.as_ref()?;
        let original = origin.original_lexical_command(words, offset, context.config)?;
        let protocol =
            super::compiler_inventory::SourceNativeCompilerPolicy::source_protocol_of(state)?;
        let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            original.native_words(),
            protocol,
        )
        .ok()?;
        let spec = context
            .registry
            .native_compilation_for_original_registration(
                &registered.compiler.registry_identity,
                &captured,
                1,
                state.baseline.compilation_dialect()?,
            )?;
        return Some((spec, 0));
    }
    let spec = facts.native_compilation?;
    let argument_offset = spec
        .original_operand_from_for_facts(facts)?
        .checked_sub(1)?;
    Some((spec, argument_offset))
}

fn original_registered_compiler_preparation(
    spec: tcl_registry::native_compilation::NativeCompilationSpec,
    selected: CompilerInvocation<'_>,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
    argument_offset: usize,
) -> Option<(
    NativeCompilationSelection,
    Option<crate::registry_invocation::OriginalNativeCompilerPreparation>,
)> {
    let origin = state.current_source_origin.as_ref()?;
    crate::registry_invocation::original_native_compilation(
        spec,
        crate::registry_invocation::OriginalNativeCompilerInvocation {
            image: origin.source_image(),
            words: selected.words,
            offset: selected.offset,
            config: context.config,
            source_protocol:
                super::compiler_inventory::SourceNativeCompilerPolicy::source_protocol_of(state),
            compiler_dialect: state.baseline.compilation_dialect(),
            context: context.compilation(),
            operand_from: argument_offset + 1,
        },
    )
}

struct OriginalCompilerPreparations {
    namespace_bindings: Option<Arc<SourceNativeNamespaceBindingPreparation>>,
    switch: Option<Arc<SourceNativeSwitchPreparation>>,
    structured: Option<Arc<SourceNativeStructuredPreparation>>,
}

fn retain_original_preparations(
    preparation: Option<&crate::registry_invocation::OriginalNativeCompilerPreparation>,
    site: Option<&CommandAllocationSite>,
    dependency: &super::SourceNativeCompilationDependency,
) -> OriginalCompilerPreparations {
    let namespace_bindings = preparation.and_then(|preparation| {
        Some(Arc::new(SourceNativeNamespaceBindingPreparation {
            compilation_site: site?.clone(),
            dependency: dependency.clone(),
            recipe: preparation.namespace_bindings()?.clone(),
        }))
    });
    let switch = preparation.and_then(|preparation| {
        Some(Arc::new(SourceNativeSwitchPreparation {
            compilation_site: site?.clone(),
            dependency: dependency.clone(),
            recipe: preparation.switch()?.clone(),
        }))
    });
    let structured = preparation.and_then(|preparation| {
        Some(Arc::new(SourceNativeStructuredPreparation {
            compilation_site: site?.clone(),
            dependency: dependency.clone(),
            recipe: preparation.structured()?.clone(),
        }))
    });
    OriginalCompilerPreparations {
        namespace_bindings,
        switch,
        structured,
    }
}

pub(super) fn compiler_dependencies(
    spec: tcl_registry::native_compilation::NativeCompilationSpec,
    target: &SourceCommandTarget,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
) -> Option<Vec<super::SourceNativeCompilationDependency>> {
    let lookups = spec.implementation_prerequisites(state.baseline.compilation_dialect()?)?;
    if lookups.is_empty() {
        return Some(Vec::new());
    }
    let snapshot = context.compilation_snapshot?;
    let mut current = target.clone();
    let mut dependencies = Vec::with_capacity(lookups.len());
    for lookup in &lookups {
        let mapping_closed = snapshot
            .table
            .state
            .original_ensemble_mapping_holds(&current, lookup);
        let dependency = mapping_closed
            .then(|| {
                implementation_dependency(
                    lookup,
                    &snapshot.table.state,
                    NativeCompilationGuard::ChunkEntry,
                )
            })
            .flatten();
        let dependency = dependency?;
        current = dependency.target.clone();
        dependencies.push(dependency);
    }
    Some(dependencies)
}

fn select_actual_ensemble_target(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    plan: super::ensemble_compilation::ActualEnsemblePlan,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
    result: &mut CompiledInvocationSelection,
) {
    use super::ensemble_compilation::ActualEnsemblePlan;
    match plan {
        ActualEnsemblePlan::Operation {
            selection,
            recipe,
            prerequisite,
        } => {
            result.admit(selection);
            let Some((site, possible)) =
                original_ensemble_selection_site(target, selected, state, result)
            else {
                return;
            };
            let prerequisite = Arc::from(prerequisite);
            result.structured = Some(Arc::new(SourceNativeStructuredPreparation {
                compilation_site: site.clone(),
                dependency: super::SourceNativeCompilationDependency {
                    compiler_prerequisite: Some(Arc::clone(&prerequisite)),
                    target: target.clone(),
                    namespace: context.namespace.to_owned(),
                    namespace_key: context.namespace_identity(),
                    head: selected.head.to_owned(),
                    guard: NativeCompilationGuard::BeforeArguments,
                },
                recipe: *recipe,
            }));
            if let NativeCompilationSelection::Inline { operation, guard } = selection {
                let proof = SourceCompiledInvocationProof {
                    target: target.clone(),
                    operation,
                    guard,
                    compilation_site: site,
                    certainty: CompiledExecutionCertainty::May,
                    lookup_dependencies: Vec::new(),
                    compiler_prerequisite: Some(prerequisite),
                    procedure_header_prerequisite: None,
                };
                result.admitted = Some(Arc::new(SourceNativeCompilerAdmission::Inline(Box::new(
                    proof.clone(),
                ))));
                if possible {
                    result.proofs.push(proof);
                }
            }
        }
        ActualEnsemblePlan::Generic => {
            result.admit(NativeCompilationSelection::Generic);
            result.live = true;
        }
        ActualEnsemblePlan::Unknown => {
            result.admit(NativeCompilationSelection::Unknown);
            result.unknown = true;
        }
        ActualEnsemblePlan::Named {
            preparations,
            name,
            replacement_words,
            prerequisite,
            protocol,
            arguments_from,
        } => {
            let Some((site, possible)) =
                original_ensemble_selection_site(target, selected, state, result)
            else {
                return;
            };
            let proof = super::SourceNamedInvocationProof {
                preparations,
                lookup: None,
                captured_name: name,
                compiler_prerequisite: Some(Arc::from(prerequisite)),
                arguments_from,
                protocol,
                replacement_words,
                compilation_site: site,
                certainty: CompiledExecutionCertainty::May,
                dependencies: Vec::new(),
                targets: Vec::new(),
                unknown: false,
                may_be_absent: false,
            };
            result.admitted = Some(Arc::new(SourceNativeCompilerAdmission::Named(
                proof.clone(),
            )));
            if possible {
                result.named.push(proof);
            }
        }
    }
}

fn original_ensemble_selection_site(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    state: &ModuleCommandBindings,
    result: &mut CompiledInvocationSelection,
) -> Option<(CommandAllocationSite, bool)> {
    let proved = selected.before.proved_target() == Some(target)
        || (state
            .compiler_identity_at_lookup(selected.head, &selected.before.lookup_namespace_key)
            == target.identity.as_ref()
            && !state.source_execution_observed(target.identity.as_ref()));
    result.live |= !proved;
    let possible = proved || selected.before.unknown || selected.before.targets.contains(target);
    let Some(source) = &state.current_source_origin else {
        result.unknown = true;
        return None;
    };
    Some((
        CommandAllocationSite {
            source: Arc::clone(source),
            offset: selected.offset,
        },
        possible,
    ))
}

fn select_inline_target(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    selection: (
        tcl_registry::native_compilation::NativeCompilationSpec,
        tcl_registry::SemanticOperationId,
        NativeCompilationGuard,
    ),
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
    result: &mut CompiledInvocationSelection,
) {
    let (spec, operation, guard) = selection;
    let CompilerInvocation { before, offset, .. } = selected;
    let mut lookup_dependencies = Vec::new();
    if let Some(lookup) = state
        .baseline
        .compilation_dialect()
        .and_then(|dialect| spec.implementation_lookup(dialect))
    {
        let Some(snapshot) = context.compilation_snapshot else {
            result.unknown = true;
            return;
        };
        let Some(dependency) = implementation_dependency(&lookup, &snapshot.table.state, guard)
        else {
            result.live = true;
            return;
        };
        lookup_dependencies.push(dependency);
    }
    let compiler_prerequisite = context
        .compilation_snapshot
        .and_then(|snapshot| {
            snapshot.table.state.runtime_command_compiler_prerequisite(
                selected.head,
                &context.namespace_identity(),
                crate::registry_invocation::native_command_binding_guard(guard),
            )
        })
        .map(Arc::new);
    let raw_guard = compiler_prerequisite.is_some()
        && state.compiler_identity_at_lookup(selected.head, &context.namespace_identity())
            == target.identity.as_ref()
        && !state.source_execution_observed(target.identity.as_ref());
    let proved_guard = guard == NativeCompilationGuard::ChunkEntry
        || raw_guard
        || before.proved_target() == Some(target);
    let possible_guard = proved_guard || before.unknown || before.targets.contains(target);
    result.live |= !proved_guard;
    let Some(source) = &state.current_source_origin else {
        result.unknown = true;
        return;
    };
    let proof = SourceCompiledInvocationProof {
        target: target.clone(),
        operation,
        guard,
        compilation_site: CommandAllocationSite {
            source: Arc::clone(source),
            offset,
        },
        certainty: CompiledExecutionCertainty::May,
        lookup_dependencies,
        compiler_prerequisite,
        procedure_header_prerequisite: None,
    };
    result.admitted = Some(Arc::new(SourceNativeCompilerAdmission::Inline(Box::new(
        proof.clone(),
    ))));
    let private_guard = proof.lookup_dependencies.iter().all(|dependency| {
        guard == NativeCompilationGuard::ChunkEntry
            || super::source_binding(state, &dependency.head, &dependency.namespace_key)
                .proved_target()
                == Some(&dependency.target)
    });
    if possible_guard && private_guard {
        result.proofs.push(proof);
    } else {
        result.live = true;
    }
}

struct RegisteredNamedInvocation {
    preparations: Vec<tcl_registry::native_control_compilation::NativeControlPreparationStep>,
    lookup: &'static tcl_registry::native_compilation::NativeCompilerImplementationLookup,
    arguments_from: usize,
    protocol: tcl_registry::native_compilation::NativeNamedInvocationProtocol,
    dependencies: Vec<super::SourceNativeCompilationDependency>,
}

fn select_named_target(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    plan: RegisteredNamedInvocation,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
    result: &mut CompiledInvocationSelection,
) {
    let compiler_prerequisite = context.compilation_snapshot.and_then(|snapshot| {
        snapshot.table.state.original_named_worker_prerequisite(
            target,
            selected.arguments,
            selected.shapes,
            selected.head,
            context.namespace,
        )
    });
    let CompilerInvocation {
        before,
        head,
        offset,
        ..
    } = selected;
    let RegisteredNamedInvocation {
        preparations,
        lookup,
        arguments_from,
        protocol,
        mut dependencies,
    } = plan;
    let proved = before.proved_target() == Some(target);
    result.live |= !proved;
    let possible = proved || before.unknown || before.targets.contains(target);
    let Some(source) = &state.current_source_origin else {
        result.unknown = true;
        return;
    };
    let captured_name = compiler_prerequisite
        .as_ref()
        .and_then(|prerequisite| prerequisite.selected_worker.as_ref())
        .and_then(|worker| {
            String::from_utf8(tcl_syntax::naming::native_command_full_name_bytes(
                &worker.slot,
            ))
            .ok()
        })
        .unwrap_or_else(|| lookup.slot.to_owned());
    let proof = super::SourceNamedInvocationProof {
        preparations,
        lookup: Some(lookup),
        captured_name,
        compiler_prerequisite: compiler_prerequisite.map(Arc::new),
        arguments_from,
        protocol,
        replacement_words: lookup
            .prepended
            .iter()
            .map(|word| (*word).to_owned())
            .collect(),
        compilation_site: CommandAllocationSite {
            source: Arc::clone(source),
            offset,
        },
        certainty: CompiledExecutionCertainty::May,
        dependencies: {
            dependencies.insert(
                0,
                super::SourceNativeCompilationDependency {
                    compiler_prerequisite: None,
                    target: target.clone(),
                    head: head.to_owned(),
                    namespace: context.namespace.to_owned(),
                    namespace_key: context.namespace_identity(),
                    guard: NativeCompilationGuard::BeforeArguments,
                },
            );
            dependencies
        },
        targets: Vec::new(),
        unknown: false,
        may_be_absent: false,
    };
    result.admitted = Some(Arc::new(SourceNativeCompilerAdmission::Named(
        proof.clone(),
    )));
    if possible {
        result.named.push(proof);
    }
}

fn select_noop_target(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    mut prerequisite: tcl_runtime_api::native_compilation::NativeProcedureHeaderPrerequisite,
    state: &ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
    result: &mut CompiledInvocationSelection,
) {
    let Some(dialect) = state.baseline.compilation_dialect() else {
        result.unknown = true;
        return;
    };
    let admission = tcl_registry::native_procedure::select_procedure_header(
        prerequisite.header,
        dialect,
        selected.shapes,
        context.compilation(),
    );
    result.admit(admission);
    match admission {
        NativeCompilationSelection::Inline { operation, guard } => {
            let proved = guard == NativeCompilationGuard::ChunkEntry
                || (state
                    .compiler_identity_at_lookup(selected.head, &context.namespace_identity())
                    == target.identity.as_ref()
                    && !state.source_execution_observed(target.identity.as_ref()))
                || selected.before.proved_target() == Some(target);
            result.live |= !proved;
            let possible =
                proved || selected.before.unknown || selected.before.targets.contains(target);
            let Some(source) = &state.current_source_origin else {
                result.unknown = true;
                return;
            };
            let Some(entry) = &state.baseline.native_entry else {
                result.unknown = true;
                return;
            };
            let namespace_key = if tcl_syntax::naming::is_absolute(selected.head) {
                state.native_root_namespace_key()
            } else {
                Some(context.namespace_identity())
            };
            let Some(namespace_context) = namespace_key
                .as_ref()
                .and_then(super::SourceNamespaceKey::native_context)
                .filter(|namespace| namespace.interpreter == entry.interpreter)
            else {
                result.unknown = true;
                return;
            };
            let Ok(namespace) = entry.namespace_context(namespace_context.token) else {
                result.unknown = true;
                return;
            };
            if namespace.path != namespace_context.path {
                result.unknown = true;
                return;
            }
            prerequisite.lookup_namespace_token = namespace.token;
            prerequisite.invocation_word = selected.head.into();
            prerequisite.guard = crate::registry_invocation::native_command_binding_guard(guard);
            let proof = SourceCompiledInvocationProof {
                target: target.clone(),
                operation,
                guard,
                compilation_site: CommandAllocationSite {
                    source: Arc::clone(source),
                    offset: selected.offset,
                },
                lookup_dependencies: Vec::new(),
                compiler_prerequisite: None,
                procedure_header_prerequisite: Some(Arc::new(prerequisite)),
                certainty: CompiledExecutionCertainty::May,
            };
            result.admitted = Some(Arc::new(SourceNativeCompilerAdmission::Inline(Box::new(
                proof.clone(),
            ))));
            if possible {
                result.proofs.push(proof);
            }
        }
        NativeCompilationSelection::Generic => result.live = true,
        _ => result.unknown = true,
    }
}

fn implementation_dependency(
    lookup: &tcl_registry::native_compilation::NativeCompilerImplementationLookup,
    state: &ModuleCommandBindings,
    guard: NativeCompilationGuard,
) -> Option<super::SourceNativeCompilationDependency> {
    let namespace_key = state
        .native_root_namespace_key()
        .unwrap_or_else(|| super::SourceNamespaceKey::authored("::"));
    let binding = super::source_binding(state, lookup.slot, &namespace_key);
    let target = binding.proved_target()?;
    if !target.registry_backed
        || target.implementation_generation != 0
        || !target.prepended.is_empty()
        || !tcl_registry::command_descriptor_labels_match(&target.command, lookup.slot)
    {
        return None;
    }
    Some(super::SourceNativeCompilationDependency {
        compiler_prerequisite: None,
        target: target.clone(),
        namespace: "::".to_owned(),
        namespace_key,
        head: lookup.slot.to_owned(),
        guard,
    })
}

impl SourceInvocationBinding {
    /// Close each reached nested ensemble edge and its actual private worker.
    /// Intermediate maps and terminal implementations retain independent proof.
    #[must_use]
    pub fn proves_native_handler_path(
        &self,
        paths: &'static tcl_registry::native_handler_path::NativeHandlerLookupPaths,
        arguments: tcl_registry::InvocationArguments<'_>,
        argument_offset: usize,
    ) -> bool {
        let Some(snapshot) = &self.lookup_state else {
            return false;
        };
        let Some(target) = self.proved_handler_target() else {
            return false;
        };
        let Some(path) = paths.select(arguments, argument_offset) else {
            return false;
        };
        if path.is_empty() {
            return false;
        }
        let mut current = target.clone();
        let start = path.iter().position(|lookup| {
            tcl_registry::command_descriptor_labels_match(&current.command, lookup.ensemble)
                || tcl_registry::command_descriptor_labels_match(&current.command, lookup.slot)
        });
        let Some(start) = start else {
            return false;
        };
        for lookup in &path[start..] {
            if !snapshot
                .state
                .original_ensemble_mapping_holds(&current, lookup)
            {
                return false;
            }
            let Some(dependency) = implementation_dependency(
                lookup,
                &snapshot.state,
                NativeCompilationGuard::BeforeArguments,
            ) else {
                return false;
            };
            current = dependency.target;
        }
        true
    }

    /// Prove the selected ensemble's original private implementation dependency.
    /// Compiled selection retains its pre-argv dependency; live dispatch checks
    /// the exact current snapshot instead.
    #[must_use]
    pub fn proves_native_implementation_lookup(
        &self,
        lookup: &tcl_registry::native_compilation::NativeCompilerImplementationLookup,
    ) -> bool {
        if self.compiled_named_candidates.iter().any(|candidate| {
            candidate.certainty == CompiledExecutionCertainty::Must
                && candidate
                    .dependencies
                    .iter()
                    .any(|dependency| dependency.head == lookup.slot)
        }) {
            return true;
        }
        if self.compiled_candidates.iter().any(|candidate| {
            candidate.certainty == CompiledExecutionCertainty::Must
                && candidate
                    .lookup_dependencies
                    .iter()
                    .any(|dependency| dependency.head == lookup.slot)
        }) {
            return true;
        }
        self.lookup_state.as_ref().is_some_and(|snapshot| {
            let Some(target) = self.proved_handler_target() else {
                return false;
            };
            if !snapshot
                .state
                .original_ensemble_mapping_holds(target, lookup)
            {
                return false;
            }
            implementation_dependency(
                lookup,
                &snapshot.state,
                NativeCompilationGuard::BeforeArguments,
            )
            .is_some()
        })
    }
}

impl SourceInvocationBinding {
    /// Selection proved against the immutable native compilation table.
    /// Later callback or command lookup uncertainty does not undo admission;
    /// execution and successful-handler proofs retain their own guards.
    #[must_use]
    pub fn native_compilation_admission_selection(&self) -> NativeCompilationSelection {
        self.native_compilation_admission
            .unwrap_or(NativeCompilationSelection::Unknown)
    }

    /// Original admitted opcode recipe, including its independent guard.
    #[must_use]
    pub fn admitted_inline_invocation(&self) -> Option<&SourceCompiledInvocationProof> {
        match self.native_compiler_admission.as_deref()? {
            SourceNativeCompilerAdmission::Inline(proof) => Some(proof),
            SourceNativeCompilerAdmission::Named(_) => None,
        }
    }

    /// Original admitted private-name recipe, before late handler lookup.
    #[must_use]
    pub fn admitted_named_invocation(&self) -> Option<&super::SourceNamedInvocationProof> {
        match self.native_compiler_admission.as_deref()? {
            SourceNativeCompilerAdmission::Named(proof) => Some(proof),
            SourceNativeCompilerAdmission::Inline(_) => None,
        }
    }

    /// Original executable admission for a captured native ensemble name.
    /// Mutable mappings have no static registry lookup and therefore no
    /// `NamedInvocation` grammar tag. Their independently retained compiler
    /// recipe supplies admission, without proving the late worker exists or
    /// granting its handler effects. Explicit unknown joins remain unknown.
    #[must_use]
    pub fn original_named_compiler_admission(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&super::SourceNamedInvocationProof> {
        if self.native_compilation_admission == Some(NativeCompilationSelection::Unknown)
            || !tokens.original_words_match_argv_text()
        {
            return None;
        }
        self.original_compiler_source(tokens)?;
        self.native_compiler_dialect()?.execution_point()?;
        self.compiler_source_protocol()?;
        let original = self.original_compiler_words.as_ref()?;
        let named = self.admitted_named_invocation()?;
        named.compiler_prerequisite.as_ref()?;
        (named.compilation_site == original.site
            && named.arguments_from <= original.words.len().checked_sub(1)?)
        .then_some(named)
    }

    /// Retained compiler selection, independent of runtime target spelling.
    /// An unresolved native alternative never acquires a generic or inline tag.
    #[must_use]
    pub fn native_compilation_selection(&self) -> NativeCompilationSelection {
        if self.lookup_state.is_none() || self.compiled_execution_unknown() {
            return NativeCompilationSelection::Unknown;
        }
        if let Some(proof) = self.named_invocation() {
            let Some(lookup) = proof.lookup else {
                return NativeCompilationSelection::Unknown;
            };
            return NativeCompilationSelection::NamedInvocation {
                lookup,
                arguments_from: proof.arguments_from,
                protocol: proof.protocol,
            };
        }
        if !self.may_use_live_dispatch
            && self.compiled_named_candidates.is_empty()
            && let [proof] = self.compiled_candidates.as_slice()
            && proof.certainty == CompiledExecutionCertainty::Must
        {
            return NativeCompilationSelection::Inline {
                operation: proof.operation,
                guard: proof.guard,
            };
        }
        if self.may_use_live_dispatch
            && self.compiled_candidates.is_empty()
            && self.compiled_named_candidates.is_empty()
        {
            NativeCompilationSelection::Generic
        } else {
            NativeCompilationSelection::Unknown
        }
    }

    /// A direct procedure slot selected at this exact post-argument lookup.
    /// Alias and import wrappers cannot establish this narrower call shape;
    /// native compiler operations retain their independent execution contract.
    #[must_use]
    pub fn direct_procedure_target(&self, _head: &str) -> Option<&SourceCommandTarget> {
        if self.native_compilation_selection() != NativeCompilationSelection::Generic {
            return None;
        }
        let target = self.proved_execution_target()?;
        let snapshot = self.lookup_state.as_ref()?;
        let keys = snapshot
            .state
            .source_keys(self.evaluated_command_word()?, &self.lookup_namespace_key);
        let [key] = keys.as_slice() else {
            return None;
        };
        let bindings = snapshot.state.bindings.get(key)?;
        if bindings.len() != 1 {
            return None;
        }
        let MayBinding::Target(slot) = bindings.first()? else {
            return None;
        };
        (slot.terminal
            && slot.kind == BindingKind::Proc
            && target.kind == BindingKind::Proc
            && slot.prepended.is_empty()
            && target.prepended.is_empty()
            && slot.token.is_some()
            && slot.token == target.identity
            && slot.implementation_generation == target.implementation_generation
            && slot.implementation_allocation == target.implementation_allocation)
            .then_some(target)
    }

    /// Kind of the exact called command slot, before alias target traversal.
    /// This diagnostic projection supplies no implementation or execution proof.
    /// Imported, conflicting and unproved lookup alternatives remain unknown.
    #[must_use]
    pub fn called_slot_kind(&self) -> Option<BindingKind> {
        let snapshot = self.lookup_state.as_ref()?;
        let keys = snapshot
            .state
            .source_keys(self.evaluated_command_word()?, &self.lookup_namespace_key);
        let [key] = keys.as_slice() else {
            return None;
        };
        let bindings = snapshot.state.bindings.get(key).map_or_else(
            || std::borrow::Cow::Owned(snapshot.state.binding_alternatives(key)),
            std::borrow::Cow::Borrowed,
        );
        if bindings.len() != 1 {
            return None;
        }
        let MayBinding::Target(slot) = bindings.first()? else {
            return None;
        };
        Some(slot.kind)
    }

    /// A registry-backed native implementation occupying the actual called slot.
    /// This retains strict execution selection and rejects alias/import wrappers.
    #[must_use]
    pub fn direct_registry_target(&self, _head: &str) -> Option<&SourceCommandTarget> {
        let target = self.proved_execution_target()?;
        let snapshot = self.lookup_state.as_ref()?;
        let keys = snapshot
            .state
            .source_keys(self.evaluated_command_word()?, &self.lookup_namespace_key);
        let [key] = keys.as_slice() else {
            return None;
        };
        let bindings = snapshot.state.bindings.get(key).map_or_else(
            || std::borrow::Cow::Owned(snapshot.state.binding_alternatives(key)),
            std::borrow::Cow::Borrowed,
        );
        if bindings.len() != 1 {
            return None;
        }
        let MayBinding::Target(slot) = bindings.first()? else {
            return None;
        };
        (slot.terminal
            && slot.kind == BindingKind::Builtin
            && slot.registry_backed
            && target.kind == BindingKind::Builtin
            && target.registry_backed
            && slot.prepended.is_empty()
            && target.prepended.is_empty()
            && slot.token.is_some()
            && slot.token == target.identity
            && slot.implementation_generation == target.implementation_generation
            && slot.implementation_allocation == target.implementation_allocation)
            .then_some(target)
    }

    /// Possible execution targets, preserving early native and late lookup alternatives.
    pub fn execution_targets(&self) -> impl Iterator<Item = &SourceCommandTarget> {
        self.compiled_candidates
            .iter()
            .map(|proof| &proof.target)
            .chain(
                self.compiled_named_candidates
                    .iter()
                    .flat_map(|proof| &proof.targets),
            )
            .chain(self.targets.iter().filter(|_| self.may_use_live_dispatch))
    }

    /// Whether actual execution retains an unbounded alternative.
    #[must_use]
    pub fn execution_is_unknown(&self) -> bool {
        self.runtime_reachability == super::SourceRuntimeReachability::Conditional
            || self.compiled_execution_unknown()
            || self
                .compiled_named_candidates
                .iter()
                .any(|proof| proof.unknown)
            || (self.may_use_live_dispatch && self.unknown)
    }

    /// Whether the late dispatch alternative can fail to find a command.
    #[must_use]
    pub fn execution_may_be_absent(&self) -> bool {
        self.compiled_named_candidates
            .iter()
            .any(|proof| proof.may_be_absent)
            || (self.may_use_live_dispatch && self.may_be_absent)
    }

    /// Whether compilation has an execution alternative with no closed descriptor.
    #[must_use]
    pub fn compiled_execution_unknown(&self) -> bool {
        self.compiled_execution_residual == CompiledExecutionResidual::Unknown
    }

    /// Actual execution's unique target, retaining native selection separately
    /// from the command that a later generic lookup would have found.
    #[must_use]
    pub fn proved_execution_target(&self) -> Option<&SourceCommandTarget> {
        if self.runtime_reachability == super::SourceRuntimeReachability::Conditional {
            return None;
        }
        self.closed_selected_execution_target()
    }

    pub(super) fn closed_selected_execution_target(&self) -> Option<&SourceCommandTarget> {
        if self.compiled_execution_unknown() {
            return None;
        }
        if !self.may_use_live_dispatch
            && self.compiled_candidates.is_empty()
            && !self.compiled_named_candidates.is_empty()
        {
            let [proof] = self.compiled_named_candidates.as_slice() else {
                return None;
            };
            return proof.proved_target();
        }
        if !self.may_use_live_dispatch {
            let [proof] = self.compiled_candidates.as_slice() else {
                return None;
            };
            return (proof.certainty == CompiledExecutionCertainty::Must).then_some(&proof.target);
        }
        (self.compiled_candidates.is_empty() && self.compiled_named_candidates.is_empty())
            .then(|| self.proved_target())
            .flatten()
    }

    /// Unique immutable handler identity, without asserting native opcode selection.
    /// Successful semantic transfer requires its own registry-authored contract.
    #[must_use]
    pub fn proved_handler_target(&self) -> Option<&SourceCommandTarget> {
        self.proved_execution_target()
            .or(self.native_handler_envelope.as_ref())
    }

    /// Name-effect closure from the actual original compiler selection. The
    /// complete retained vector must match; this supplies neither admission,
    /// normal completion nor an evaluated-argument or native-object receipt.
    pub(crate) fn original_compiler_names_preserved_for_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> bool {
        // Implementation contract: naming.compiler.original-name-effect-separation
        // docs/design/analysis/name-resolution-proofs/original-compiler-name-effect-separation.md
        self.original_compiler_preserves_names.is_preserved()
            && self.original_lexer_config_for_tokens(tokens).is_some()
    }

    pub(super) fn join_compiled_execution(&mut self, other: &Self) {
        self.original_compiler_preserves_names
            .join(other.original_compiler_preserves_names);
        if self.native_structured_preparation != other.native_structured_preparation {
            self.native_structured_preparation = None;
        }
        if self.native_switch_preparation != other.native_switch_preparation {
            self.native_switch_preparation = None;
        }
        if self.original_compiler_words != other.original_compiler_words {
            self.original_compiler_words = None;
        }
        if self.native_compiler_admission != other.native_compiler_admission {
            self.native_compiler_admission = None;
        }
        if self.native_compilation_admission != other.native_compilation_admission {
            self.native_compilation_admission = Some(NativeCompilationSelection::Unknown);
        }
        self.join_named_execution(other);
        if self.native_handler_envelope != other.native_handler_envelope {
            self.native_handler_envelope = None;
        }
        for proof in &mut self.compiled_candidates {
            let retained = other
                .compiled_candidates
                .iter()
                .find(|candidate| same_candidate(proof, candidate));
            if retained.is_none_or(|incoming| incoming.certainty == CompiledExecutionCertainty::May)
            {
                proof.certainty = CompiledExecutionCertainty::May;
            }
        }
        for proof in &other.compiled_candidates {
            if !self
                .compiled_candidates
                .iter()
                .any(|candidate| same_candidate(proof, candidate))
            {
                let mut proof = proof.clone();
                proof.certainty = CompiledExecutionCertainty::May;
                self.compiled_candidates.push(proof);
            }
        }
        self.may_use_live_dispatch |= other.may_use_live_dispatch;
        if other.compiled_execution_unknown() {
            self.compiled_execution_residual = CompiledExecutionResidual::Unknown;
        }
        if self.native_inline_rejection != other.native_inline_rejection {
            self.native_inline_rejection = NativeInlineRejection::Possible;
        }
    }
}

fn same_candidate(
    left: &SourceCompiledInvocationProof,
    right: &SourceCompiledInvocationProof,
) -> bool {
    left.target == right.target
        && left.operation == right.operation
        && left.guard == right.guard
        && left.compilation_site == right.compilation_site
        && left.lookup_dependencies == right.lookup_dependencies
        && left.compiler_prerequisite == right.compiler_prerequisite
        && left.procedure_header_prerequisite == right.procedure_header_prerequisite
}

pub(super) fn join_argument_words(
    words: &mut Arc<[crate::registry_invocation::EffectiveInvocationWord]>,
    incoming: &[crate::registry_invocation::EffectiveInvocationWord],
) {
    use crate::registry_invocation::EffectiveInvocationWord as Word;
    if words.len() != incoming.len() {
        *words = std::iter::repeat_n(Word::Opaque, words.len().max(incoming.len())).collect();
        return;
    }
    if words.iter().eq(incoming) {
        return;
    }
    let expanded = |word: &Word| {
        matches!(
            word,
            Word::Expanded | Word::KnownExpansion(_) | Word::KnownByteExpansion(_)
        )
    };
    *words = words
        .iter()
        .zip(incoming)
        .map(|(word, other)| {
            if word == other {
                word.clone()
            } else if expanded(word) || expanded(other) {
                Word::Expanded
            } else {
                Word::Dynamic
            }
        })
        .collect();
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
