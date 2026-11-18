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

//! What a compiled invocation proves about execution: handler path, lookup, targets and joins.

use super::super::{Arc, BindingKind, MayBinding, SourceCommandTarget, SourceInvocationBinding};
use super::target_selection::implementation_dependency;
use super::{
    CompiledExecutionCertainty, CompiledExecutionResidual, NativeInlineRejection,
    SourceCompiledInvocationProof, SourceNativeCompilerAdmission,
};
use tcl_registry::native_compilation::{NativeCompilationGuard, NativeCompilationSelection};

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
    pub fn admitted_named_invocation(&self) -> Option<&super::super::SourceNamedInvocationProof> {
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
    ) -> Option<&super::super::SourceNamedInvocationProof> {
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
        self.runtime_reachability == super::super::SourceRuntimeReachability::Conditional
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

    #[doc(hidden)]
    pub fn zz_proof(&self) -> String {
        format!("reach={:?} unknown={} live={} cands={:?} named={}", self.runtime_reachability, self.compiled_execution_unknown(), self.may_use_live_dispatch, self.compiled_candidates.iter().map(|c| format!("{:?}", c.certainty)).collect::<Vec<_>>(), self.compiled_named_candidates.len())
    }

    /// Actual execution's unique target, retaining native selection separately
    /// from the command that a later generic lookup would have found.
    #[must_use]
    pub fn proved_execution_target(&self) -> Option<&SourceCommandTarget> {
        if self.runtime_reachability == super::super::SourceRuntimeReachability::Conditional {
            return None;
        }
        self.closed_selected_execution_target()
    }

    pub(in super::super) fn closed_selected_execution_target(
        &self,
    ) -> Option<&SourceCommandTarget> {
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

    pub(in super::super) fn join_compiled_execution(&mut self, other: &Self) {
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

pub(in super::super) fn join_argument_words(
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
