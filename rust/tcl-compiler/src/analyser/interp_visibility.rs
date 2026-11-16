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

//! Conditional interpreter source visibility, independently of native lookup.

#[cfg(test)]
use super::Analyser;
use super::ResolvedAnalysisInput;
use crate::signature_scan::scope::{
    SignatureNamespaceScope, SignatureSourceNameInput, SignatureSourceNameKey,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use tcl_core_types::{ByteCommandSlot, NameBytes};
#[cfg(test)]
use tcl_lexer::Span;
use tcl_lexer::{NativeWord, SourceImage};
use tcl_syntax::naming::{
    NamePolicyProtocol, NativeNameContext, NativeNameProtocol, NativeNameQualification,
};

mod child_visibility;
mod command_transitions;
mod lambda_bodies;
mod source_loads;
mod source_schemas;

pub(super) use lambda_bodies::OriginalInterpreterLambdaBody;
pub use source_loads::OriginalInterpreterSourceLoad;

/// Reporting domain of a positively owned original child-interpreter body.
/// The complete declaration and input remain retained. This is a source label
/// domain, never a native interpreter, namespace token or selected callable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalInterpreterSourceDomain {
    label: String,
    input: ResolvedAnalysisInput,
    declaration: Vec<NativeWord>,
}
impl OriginalInterpreterSourceDomain {
    pub(super) fn reported_namespace(&self, namespace: &SignatureNamespaceScope) -> Option<String> {
        let SignatureNamespaceScope::C(path) = namespace else {
            return None;
        };
        let bytes = tcl_syntax::naming::native_namespace_full_name_bytes(path);
        let suffix = std::str::from_utf8(bytes.get(2..)?).ok()?;
        let domain = tcl_syntax::naming::root_unrooted_key(&self.label);
        Some(if suffix.is_empty() {
            domain
        } else {
            tcl_syntax::naming::rooted_member_key(&domain, suffix)
        })
    }

    pub(super) fn reported_command(&self, slot: &ByteCommandSlot) -> Option<String> {
        let bytes = tcl_syntax::naming::native_command_full_name_bytes(slot);
        let suffix = std::str::from_utf8(bytes.get(2..)?).ok()?;
        Some(tcl_syntax::naming::rooted_member_key(
            &tcl_syntax::naming::root_unrooted_key(&self.label),
            suffix,
        ))
    }
}

/// The original source assumptions of an interpreter visibility diagnostic.
/// They are retained as data; none establishes entered execution or a hidden slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterpreterVisibilityObligation {
    /// The retained child creation and visibility operations complete successfully.
    SuccessfulSourceOperations,
    /// The child's initial commands agree with the retained C Registry context.
    RegistryInitialization,
    /// An unmodelled command or external mutation does not change this child.
    NoUnmodelledInterpreterMutation,
    /// Observers do not change visibility while original operands are evaluated.
    NoObserverInterference,
    /// The stored source body is entered under its retained child declaration.
    SourceBodyEntered,
    /// Original literal/list producers materialise without changing the command map.
    OriginalOperandMaterialization,
    /// The declared receiver body has no entered object namespace.
    UnknownExecutingReceiverNamespace,
    /// Conditional global fallback requires no earlier same-name receiver
    /// namespace binding or search-path target intercepting this source call.
    NoEarlierReceiverNamespaceBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SourceCommandOrigin {
    Registry(String),
    Declaration {
        name: SignatureSourceNameInput,
        kind: tcl_registry::CommandBindingDefinitionKind,
        operands: Vec<NativeWord>,
    },
    Alias {
        name: SignatureSourceNameInput,
        operands: Vec<NativeWord>,
    },
    Renamed {
        original: Box<SourceCommandOrigin>,
        operands: Vec<NativeWord>,
    },
    Exposed {
        original: Box<SourceCommandOrigin>,
        operands: Vec<NativeWord>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SourceVisibleSlot {
    Visible(SourceCommandOrigin),
    Unavailable(SourceCommandOrigin),
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SourceHiddenAllocation {
    previous_slot: ByteCommandSlot,
    origin: SourceCommandOrigin,
    operands: Vec<NativeWord>,
}

/// Hashable original source visibility context for an isolated body/cache.
/// Cloning retains the complete source and immutable editing generation.
/// No runtime hidden table, selected callable, native entry or effects are supplied.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceInterpreterVisibilitySnapshot {
    input: ResolvedAnalysisInput,
    policy: NamePolicyProtocol,
    declaration: Vec<NativeWord>,
    safe: bool,
    unavailable: BTreeMap<ByteCommandSlot, SourceCommandOrigin>,
    body: Option<NativeWord>,
    body_invocation: Vec<NativeWord>,
    slots: BTreeMap<ByteCommandSlot, SourceVisibleSlot>,
    hidden: BTreeMap<NameBytes, SourceHiddenAllocation>,
}
impl SourceInterpreterVisibilitySnapshot {
    fn initialize(
        input: ResolvedAnalysisInput,
        policy: NamePolicyProtocol,
        declaration: Vec<NativeWord>,
        safe: bool,
    ) -> Option<Self> {
        if !matches!(policy.recipe(), NativeNameProtocol::C(_)) || declaration.is_empty() {
            return None;
        }
        let context = input.context_registry();
        let mut slots = BTreeMap::new();
        let mut hidden = BTreeMap::new();
        let mut unavailable = BTreeMap::new();
        for command in context.commands().command_names() {
            let Some(spec) = context.context().resolve_spec(context.commands(), command) else {
                continue;
            };
            let slot = policy
                .recipe()
                .command_publication_slot(NativeNameContext::root(), command.as_bytes())
                .ok()?;
            let origin = SourceCommandOrigin::Registry(command.to_owned());
            if safe
                && slot.namespace.is_root()
                && spec
                    .traits
                    .contains(tcl_registry::Traits::SAFE_INTERP_HIDDEN)
            {
                let token = policy
                    .recipe()
                    .hidden_token_input(command.as_bytes())
                    .ok()?
                    .selected()
                    .into();
                hidden.insert(
                    token,
                    SourceHiddenAllocation {
                        previous_slot: slot.clone(),
                        origin: origin.clone(),
                        operands: declaration.clone(),
                    },
                );
                unavailable.insert(slot.clone(), origin.clone());
                slots.insert(slot, SourceVisibleSlot::Unavailable(origin));
            } else {
                slots.insert(slot, SourceVisibleSlot::Visible(origin));
            }
        }
        Some(Self {
            input,
            policy,
            declaration,
            safe,
            unavailable,
            body: None,
            body_invocation: Vec::new(),
            slots,
            hidden,
        })
    }
    pub(super) fn reporting_domain(&self, label: String) -> Arc<OriginalInterpreterSourceDomain> {
        Arc::new(OriginalInterpreterSourceDomain {
            label,
            input: self.input.clone(),
            declaration: self.declaration.clone(),
        })
    }
    fn image(&self) -> &SourceImage {
        self.declaration[0].image()
    }
    fn matches(&self, image: &SourceImage, input: &ResolvedAnalysisInput) -> bool {
        self.image() == image
            && self.input == *input
            && self.policy.string_protocol().escape_syntax() == input.lexer_config().escapes
    }
    fn original_input(&self, word: &NativeWord) -> Option<SignatureSourceNameInput> {
        if word.image() != self.image() || word.config() != self.input.lexer_config() {
            return None;
        }
        SignatureSourceNameKey::from_original_native_word(
            word,
            tcl_syntax::word_rules::WordValueRules::from_config(&word.config()),
            self.policy,
        )
        .map(SignatureSourceNameInput::OriginalWord)
    }
    fn hidden_token(&self, input: &SignatureSourceNameInput) -> Option<NameBytes> {
        if input.policy() != self.policy {
            return None;
        }
        let bytes = self
            .policy
            .recipe()
            .hidden_token_input(input.bytes())
            .ok()?
            .selected()
            .to_vec();
        (!tcl_syntax::naming::is_qualified(&bytes)).then(|| bytes.into())
    }
    fn hide(
        &mut self,
        visible: &SignatureSourceNameInput,
        hidden: &SignatureSourceNameInput,
        operands: &[NativeWord],
    ) -> Option<()> {
        let slot = self
            .policy
            .recipe()
            .command_lookup_slot(NativeNameContext::root(), visible.bytes())
            .ok()?;
        let Some(token) = self.hidden_token(hidden) else {
            return Some(());
        };
        if !slot.namespace.is_root() || self.hidden.contains_key(&token) {
            return Some(());
        }
        let Some(SourceVisibleSlot::Visible(origin)) = self.slots.get(&slot).cloned() else {
            return Some(());
        };
        self.unavailable.insert(slot.clone(), origin.clone());
        self.hidden.insert(
            token,
            SourceHiddenAllocation {
                previous_slot: slot.clone(),
                origin: origin.clone(),
                operands: operands.to_vec(),
            },
        );
        self.slots
            .insert(slot, SourceVisibleSlot::Unavailable(origin));
        Some(())
    }
    fn expose(
        &mut self,
        hidden: &SignatureSourceNameInput,
        visible: &SignatureSourceNameInput,
        operands: &[NativeWord],
    ) -> Option<()> {
        let token: NameBytes = self
            .policy
            .recipe()
            .hidden_token_input(hidden.bytes())
            .ok()?
            .selected()
            .into();
        let Some(allocation) = self.hidden.get(&token).cloned() else {
            return Some(());
        };
        let Some(destination) = self.hidden_token(visible) else {
            return Some(());
        };
        let slot = self
            .policy
            .recipe()
            .command_publication_slot(NativeNameContext::root(), destination.as_bytes())
            .ok()?;
        if !slot.namespace.is_root()
            || matches!(self.slots.get(&slot), Some(SourceVisibleSlot::Visible(_)))
        {
            return Some(());
        }
        self.hidden.remove(&token);
        self.unavailable.remove(&slot);
        self.slots.insert(
            slot,
            SourceVisibleSlot::Visible(SourceCommandOrigin::Exposed {
                original: Box::new(allocation.origin),
                operands: operands.to_vec(),
            }),
        );
        // The old source slot remains unavailable unless its independent new
        // declaration already installed another visible source allocation.
        Some(())
    }
    fn lookup_slot(
        &self,
        bytes: &[u8],
        namespace: &SignatureNamespaceScope,
    ) -> Option<ByteCommandSlot> {
        let context = namespace.context()?;
        let recipe = self.policy.recipe();
        let projection = recipe.command_lookup_input(context, bytes).ok()?;
        let mut slot = recipe.command_lookup_slot(context, bytes).ok()?;
        if !self.slots.contains_key(&slot)
            && projection.qualification() == NativeNameQualification::Unqualified
        {
            slot = recipe
                .command_lookup_slot(NativeNameContext::root(), bytes)
                .ok()?;
        }
        Some(slot)
    }
    fn observe(
        &self,
        name: SignatureSourceNameInput,
        namespace: SignatureNamespaceScope,
        words: Vec<NativeWord>,
    ) -> Option<ConditionalInterpreterVisibilitySubject> {
        let body = self.body.as_ref()?.content_span().ok()?;
        if words.is_empty()
            || words
                .iter()
                .any(|word| word.span().start() < body.start() || word.span().end() > body.end())
        {
            return None;
        }
        let slot = self.lookup_slot(name.bytes(), &namespace)?;
        if !slot.namespace.is_root()
            || !matches!(
                self.slots.get(&slot),
                Some(SourceVisibleSlot::Unavailable(_))
            )
        {
            return None;
        }
        Some(ConditionalInterpreterVisibilitySubject {
            snapshot: self.clone(),
            name,
            namespace: Some(namespace),
            declared_receiver: None,
            source_receiver_declaration: None,
            slot,
            words,
            obligations: vec![
                InterpreterVisibilityObligation::SuccessfulSourceOperations,
                InterpreterVisibilityObligation::RegistryInitialization,
                InterpreterVisibilityObligation::NoUnmodelledInterpreterMutation,
                InterpreterVisibilityObligation::NoObserverInterference,
                InterpreterVisibilityObligation::SourceBodyEntered,
                InterpreterVisibilityObligation::OriginalOperandMaterialization,
            ],
        })
    }
}

/// Genuine conditional source visibility subject. This is not a callable
/// identity or a definite hidden command result; possible effects remain live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalInterpreterVisibilitySubject {
    snapshot: SourceInterpreterVisibilitySnapshot,
    name: SignatureSourceNameInput,
    namespace: Option<SignatureNamespaceScope>,
    declared_receiver: Option<Arc<crate::command_binding::SourceDeclaredReceiverBodyEntry>>,
    source_receiver_declaration: Option<Arc<super::types::OriginalSourceReceiverBodyDeclaration>>,
    slot: ByteCommandSlot,
    words: Vec<NativeWord>,
    obligations: Vec<InterpreterVisibilityObligation>,
}
impl ConditionalInterpreterVisibilitySubject {
    /// Original complete word/value producer, without a reporting-name lookup.
    #[must_use]
    pub const fn name_input(&self) -> &SignatureSourceNameInput {
        &self.name
    }
    /// Conditional original C root slot; no current table or namespace existence.
    #[must_use]
    pub const fn slot(&self) -> &ByteCommandSlot {
        &self.slot
    }
    /// Source namespace geometry retained separately from the reporting label.
    #[must_use]
    pub const fn namespace(&self) -> Option<&SignatureNamespaceScope> {
        self.namespace.as_ref()
    }
    /// Authentic declared receiver source scope. Its runtime object namespace
    /// and entry remain unknown; this cannot supply a callable or receiver.
    #[must_use]
    pub fn declared_receiver_body(
        &self,
    ) -> Option<&crate::command_binding::SourceDeclaredReceiverBodyEntry> {
        self.declared_receiver.as_deref()
    }
    /// Selected original method-body declaration without a native class or
    /// receiver allocation. This source scope supplies no executing namespace.
    #[must_use]
    pub fn source_receiver_body_declaration(
        &self,
    ) -> Option<&super::types::OriginalSourceReceiverBodyDeclaration> {
        self.source_receiver_declaration.as_deref()
    }
    /// Original whole invocation or list-producer vector, without an effective
    /// head borrowing the selector/outer call's span.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.words
    }
    /// Explicit applicability requirements; consumers must not parse the message.
    #[must_use]
    pub fn obligations(&self) -> &[InterpreterVisibilityObligation] {
        &self.obligations
    }
    /// Exact full source/channel, configuration and immutable editing generation.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, input: &ResolvedAnalysisInput) -> bool {
        self.snapshot.matches(image, input)
            && self
                .source_receiver_declaration
                .as_ref()
                .is_none_or(|body| {
                    body.matches_source(image, input) && body.owns_invocation(&self.words, input)
                })
            && self.declared_receiver.as_ref().is_none_or(|body| {
                body.source().origin.source_image() == image
                    && matches!(
                        body.source().mapping,
                        crate::command_binding::ExecutedScriptMapping::Contiguous { .. }
                    )
            })
    }
}

#[cfg(test)]
mod tests;
