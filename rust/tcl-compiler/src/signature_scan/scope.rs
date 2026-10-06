// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Authored namespace geometry for signature assistance, without runtime tokens.

use tcl_core_types::{ByteCommandSlot, ByteNamespacePath, NameBytes};
use tcl_syntax::naming::{NamePolicyProtocol, NativeNameContext, NativeNameProtocol};

/// Source namespace geometry. These values supply no namespace existence,
/// entered frame, command identity or native compilation permission.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SignatureNamespaceScope {
    /// Explicit symbolic compatibility context without an audited name policy.
    Symbolic(String),
    /// C namespace components retained before any display rendering.
    C(ByteNamespacePath),
    /// Jim's counted flat namespace value, independent of C components.
    Jim(NameBytes),
}

impl SignatureNamespaceScope {
    pub(crate) fn root(policy: Option<NamePolicyProtocol>) -> Self {
        match policy.map(NamePolicyProtocol::recipe) {
            Some(NativeNameProtocol::C(_)) => Self::C(ByteNamespacePath::root()),
            Some(NativeNameProtocol::Jim084) => Self::Jim(NameBytes::from(b"".as_slice())),
            None => Self::Symbolic("::".to_owned()),
        }
    }

    /// Presentation only; its colon boundaries cannot recover this scope.
    #[must_use]
    pub fn display(&self) -> Option<String> {
        match self {
            Self::Symbolic(key) => Some(key.clone()),
            Self::C(path) => Some(format!(
                "::{}",
                tcl_syntax::naming::checked_namespace_path_utf8(path)
                    .ok()?
                    .join("::")
            )),
            Self::Jim(value) => Some(crate::naming::root_unrooted_key(value.try_utf8().ok()?)),
        }
    }

    /// Retained projection context only; it grants no namespace existence or token.
    #[must_use]
    pub fn context(&self) -> Option<NativeNameContext<'_>> {
        match self {
            Self::C(path) => Some(NativeNameContext::new(path)),
            Self::Jim(value) => Some(NativeNameContext::with_jim_namespace(
                &ROOT,
                value.as_bytes(),
            )),
            Self::Symbolic(_) => None,
        }
    }

    fn context_for_policy(&self, policy: NamePolicyProtocol) -> Option<NativeNameContext<'_>> {
        match (self, policy.recipe()) {
            (Self::C(_), NativeNameProtocol::C(_)) | (Self::Jim(_), NativeNameProtocol::Jim084) => {
                self.context()
            }
            _ => None,
        }
    }

    /// Optional written namespace spelling checked against the retained geometry.
    #[must_use]
    pub fn source_spelling(&self, policy: Option<NamePolicyProtocol>) -> Option<String> {
        match (self, policy.map(NamePolicyProtocol::recipe)) {
            (Self::Symbolic(key), None) => Some(key.clone()),
            (Self::C(path), Some(recipe @ NativeNameProtocol::C(_))) => {
                tcl_syntax::naming::native_namespace_source_spelling(recipe, path)
            }
            (Self::Jim(value), Some(NativeNameProtocol::Jim084)) => {
                tcl_syntax::naming::native_jim_namespace_source_spelling(value.as_bytes())
            }
            _ => None,
        }
    }

    pub(crate) fn child(&self, written: &str, policy: Option<NamePolicyProtocol>) -> Option<Self> {
        let Some(policy) = policy else {
            let Self::Symbolic(namespace) = self else {
                return None;
            };
            return Some(Self::Symbolic(crate::naming::qualify_namespace(
                namespace, written,
            )));
        };
        let recipe = policy.recipe();
        match self {
            Self::C(_) => Some(Self::C(
                recipe
                    .namespace_address_path(self.context()?, written.as_bytes())
                    .ok()?,
            )),
            Self::Jim(_) => Some(Self::Jim(
                recipe
                    .jim_namespace_canonical_input(self.context()?, written.as_bytes())
                    .ok()?
                    .selected()
                    .into(),
            )),
            Self::Symbolic(_) => None,
        }
    }
}

static ROOT: ByteNamespacePath = ByteNamespacePath::root();

/// A selected authored declaration slot. This retains naming policy and
/// component geometry, never an actual command token or entered lookup proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceCommandPublication {
    /// Ordinary named-command publication from original source.
    NamedCommand,
    /// TclOO object publication using its independent naming purpose.
    TclOoObject,
    /// Registry provider grammar's authored assistance, without native allocation.
    ProviderAdvice,
}

/// An original source publication and its retained naming coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignatureSourceCommand {
    publication: SourceCommandPublication,
    policy: NamePolicyProtocol,
    slot: ByteCommandSlot,
}

impl SignatureSourceCommand {
    /// Authored procedure publication in retained source namespace geometry.
    /// Name creation validation and publication remain distinct from lookup.
    #[must_use]
    pub(crate) fn procedure_in_context(
        policy: NamePolicyProtocol,
        namespace: &SignatureNamespaceScope,
        written: &str,
    ) -> Option<Self> {
        let recipe = policy.recipe();
        let context = namespace.context_for_policy(policy)?;
        if let NativeNameProtocol::C(version) = recipe {
            let selected = recipe
                .command_lookup_slot(context, written.as_bytes())
                .ok()?;
            tcl_registry::native_procedure::procedure_name_creation_error(
                tcl_registry::InvocationDialect::for_version(version),
                selected.namespace.is_root(),
                selected.simple.as_bytes(),
            )?
            .ok()?;
        }
        Some(Self::new(
            policy,
            recipe
                .command_publication_slot(context, written.as_bytes())
                .ok()?,
        ))
    }

    /// Authored TclOO object publication, without provider or object-token authority.
    #[must_use]
    pub(crate) fn object_in_context(
        policy: NamePolicyProtocol,
        namespace: &SignatureNamespaceScope,
        written: &str,
    ) -> Option<Self> {
        Some(Self {
            publication: SourceCommandPublication::TclOoObject,
            policy,
            slot: policy
                .recipe()
                .oo_object_publication_slot(
                    namespace.context_for_policy(policy)?,
                    written.as_bytes(),
                )
                .ok()?,
        })
    }

    /// Explicit registry-provider declaration advice. The byte recipe selects
    /// geometry; neither native recipe authority nor this advice proves loading.
    #[must_use]
    pub(crate) fn provider_advice_in_context(
        policy: NamePolicyProtocol,
        namespace: &SignatureNamespaceScope,
        written: &str,
    ) -> Option<Self> {
        Some(Self {
            publication: SourceCommandPublication::ProviderAdvice,
            policy,
            slot: policy
                .recipe()
                .command_publication_slot(namespace.context_for_policy(policy)?, written.as_bytes())
                .ok()?,
        })
    }

    /// The publication purpose, independent of actual provider allocation.
    #[must_use]
    pub const fn publication(&self) -> SourceCommandPublication {
        self.publication
    }

    /// Counted publication presentation, carrying no written-lookup authority.
    #[must_use]
    pub(crate) fn reported_full_name(&self) -> Option<String> {
        String::from_utf8(tcl_syntax::naming::native_command_full_name_bytes(
            &self.slot,
        ))
        .ok()
    }
    /// Select an authored alias publication from its original global-root
    /// operand. This supplies no actual publication or command-token authority.
    #[must_use]
    pub fn alias_at_root(policy: NamePolicyProtocol, written: &str) -> Option<Self> {
        let slot = policy
            .recipe()
            .alias_publication_slot(NativeNameContext::root(), written.as_bytes())
            .ok()?;
        Some(Self::new(policy, slot))
    }

    pub(super) fn new(policy: NamePolicyProtocol, slot: ByteCommandSlot) -> Self {
        Self {
            publication: SourceCommandPublication::NamedCommand,
            policy,
            slot,
        }
    }

    /// Selected authored slot, independent of the command's reported full name.
    #[must_use]
    pub fn slot(&self) -> &ByteCommandSlot {
        &self.slot
    }

    /// Explicit authored naming recipe; it authenticates no physical engine.
    #[must_use]
    pub fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }

    /// Optional globally written name selecting this exact slot.
    #[must_use]
    pub fn source_spelling(&self) -> Option<String> {
        tcl_syntax::naming::native_command_source_spelling(self.policy.recipe(), &self.slot)
    }

    pub(crate) fn body_scope(&self) -> Option<SignatureNamespaceScope> {
        match self.policy.recipe() {
            NativeNameProtocol::C(_) => {
                Some(SignatureNamespaceScope::C(self.slot.namespace.clone()))
            }
            NativeNameProtocol::Jim084 => Some(SignatureNamespaceScope::Jim(
                self.policy
                    .recipe()
                    .jim_procedure_namespace(self.slot.simple.as_bytes())
                    .ok()?
                    .into(),
            )),
        }
    }

    pub(crate) fn simple_name(&self) -> Option<String> {
        let bytes = self.slot.simple.as_bytes();
        let simple = match self.body_scope()? {
            SignatureNamespaceScope::Jim(home) if !home.as_bytes().is_empty() => {
                bytes.get(home.as_bytes().len().checked_add(2)?..)?
            }
            SignatureNamespaceScope::C(_) | SignatureNamespaceScope::Jim(_) => bytes,
            SignatureNamespaceScope::Symbolic(_) => return None,
        };
        Some(std::str::from_utf8(simple).ok()?.to_owned())
    }

    /// Whether an original written name's local lookup selects this authored
    /// slot. This supplies navigation assistance, not runtime dispatch proof.
    #[must_use]
    pub fn matches_written(&self, namespace: &SignatureNamespaceScope, written: &str) -> bool {
        let Some(context) = namespace.context() else {
            return false;
        };
        match (self.policy.recipe(), namespace) {
            (NativeNameProtocol::C(_), SignatureNamespaceScope::C(_)) => self
                .policy
                .recipe()
                .command_lookup_slot(context, written.as_bytes())
                .is_ok_and(|slot| slot == self.slot),
            (NativeNameProtocol::Jim084, SignatureNamespaceScope::Jim(_)) => self
                .policy
                .recipe()
                .jim_command_lookup_keys(context, written.as_bytes())
                .ok()
                .and_then(|keys| keys.into_iter().next())
                .is_some_and(|key| self.slot.namespace.is_root() && self.slot.simple == key),
            _ => false,
        }
    }
}

/// Original source command lookup and exact caller geometry. Candidate naming
/// supplies no command existence, class kind, token or provider survival.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignatureSourceLookup {
    policy: NamePolicyProtocol,
    namespace: SignatureNamespaceScope,
    written: String,
}

impl SignatureSourceLookup {
    /// Retain original lookup bytes in a context supported by this naming policy.
    /// Returns `None` when the original namespace context is unavailable.
    #[must_use]
    pub fn new(
        policy: NamePolicyProtocol,
        namespace: SignatureNamespaceScope,
        written: String,
    ) -> Option<Self> {
        namespace.context_for_policy(policy)?;
        Some(Self {
            policy,
            namespace,
            written,
        })
    }

    /// The independently selected naming policy for this source lookup.
    #[must_use]
    pub fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }

    /// Ordered current/global geometry, without namespace-path or import authority.
    #[must_use]
    pub fn candidates(&self) -> Option<Vec<ByteCommandSlot>> {
        let recipe = self.policy.recipe();
        let context = self.namespace.context_for_policy(self.policy)?;
        match recipe {
            NativeNameProtocol::C(_) => {
                let mut slots = vec![
                    recipe
                        .command_lookup_slot(context, self.written.as_bytes())
                        .ok()?,
                ];
                if !self.written.starts_with("::") {
                    let global = recipe
                        .command_lookup_slot(NativeNameContext::root(), self.written.as_bytes())
                        .ok()?;
                    if !slots.contains(&global) {
                        slots.push(global);
                    }
                }
                Some(slots)
            }
            NativeNameProtocol::Jim084 => Some(
                recipe
                    .jim_command_lookup_keys(context, self.written.as_bytes())
                    .ok()?
                    .into_iter()
                    .map(|simple| ByteCommandSlot {
                        namespace: ByteNamespacePath::root(),
                        simple,
                    })
                    .collect(),
            ),
        }
    }
}
