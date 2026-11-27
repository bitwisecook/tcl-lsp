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

//! Command identity: the token that survives slot movement, the allocation
//! that created it, the command-table key and the binding lattice element.

use crate::namespace::SourceNamespaceKey;
use crate::origin::{SourceOriginId, SourceOriginKind};
use std::hash::Hash;
use std::sync::Arc;
use tcl_core_types::NameBytes;

/// Object identity survives movement of its command-table slot. Imports keep
/// this identity; interpreter aliases perform target-name lookup instead.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandIdentity {
    /// Actual interpreter token supplied by a live runtime entry snapshot.
    pub runtime: Option<RuntimeCommandTokenIdentity>,
    /// Slot in which this token was originally created.
    pub origin: String,
    /// Source creation site; zero denotes an explicit entry implementation.
    pub declaration: u32,
    /// Allocation identity; source offsets alone do not identify derived code
    /// or a command recreated by another execution of the same source site.
    pub allocation: Option<CommandAllocation>,
}

/// Stable runtime command identity; generations belong to implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeCommandTokenIdentity {
    /// Interpreter owner and non-reused interpreter arena identity.
    pub interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    /// Non-reused command token within the interpreter.
    pub token: u64,
}

/// An allocation instruction in one authored or materialised source instance.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandAllocationSite {
    /// Source instance; derived offsets are never authored document offsets.
    pub source: Arc<SourceOriginId>,
    /// Offset within that source instance.
    pub offset: u32,
}

impl CommandAllocationSite {
    /// Compact storage projection for this exact source allocation instruction.
    /// Callers retain the typed site as the equality proof and append their
    /// allocation incarnation and implementation generation independently.
    #[must_use]
    pub fn variable_storage_identity(&self) -> String {
        format!(
            "source:{}:{}",
            SourceOriginId::activation_key(&self.source),
            self.offset
        )
    }
}

/// Bounded incarnation of an allocation instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AllocationIncarnation {
    /// First reached execution.
    First,
    /// Second reached execution, distinct from an earlier captured token.
    Second,
    /// Further executions; no unique runtime token is asserted.
    RepeatedFresh,
}

/// Command allocation, distinct from its current name and implementation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandAllocation {
    /// Instruction allocating the token or implementation.
    pub site: CommandAllocationSite,
    /// Repeated executions must not alias a prior captured runtime identity.
    pub incarnation: AllocationIncarnation,
    /// Presentation of the evaluated output slot, without lookup authority.
    pub command: String,
    /// Exact originally selected slot; never recovered from the reporting name.
    pub slot: SourceCommandKey,
    /// Exact namespace owning the allocation, independent of its presentation.
    pub namespace: SourceNamespaceKey,
}

impl CommandAllocation {
    /// Correlate a reached implementation with an analysed declaration's bytes
    /// and offset. Loaded files retain their own identity; this value-purpose
    /// match grants no edit, native compiler, frame or object-erasure licence.
    #[must_use]
    pub fn matches_source_declaration(&self, source: &str, offset: u32) -> bool {
        self.matches_source_declaration_image(&tcl_lexer::SourceImage::document(source), offset)
    }

    /// Match the exact source bytes and input channel of a declaration.
    #[must_use]
    pub fn matches_source_declaration_image(
        &self,
        source: &tcl_lexer::SourceImage,
        offset: u32,
    ) -> bool {
        self.site.offset == offset
            && self.incarnation != AllocationIncarnation::RepeatedFresh
            && match self.site.source.kind() {
                SourceOriginKind::Authored(text) => text == source,
                SourceOriginKind::Loaded {
                    source: text,
                    implementation_id,
                    ..
                } => !implementation_id.is_empty() && text == source,
                SourceOriginKind::InheritedPoint { .. }
                | SourceOriginKind::Derived { .. }
                | SourceOriginKind::ConditionalField { .. } => false,
            }
    }
}

/// Owner of interned source command references. The descriptor retains original
/// component boundaries and allocation provenance; reporting labels are excluded.
#[derive(Debug)]
pub struct CommandRefInterner {
    identities:
        tcl_core_types::IdentityInterner<SourceCommandDescriptor, tcl_core_types::CommandRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SourceCommandDescriptor {
    site: CommandAllocationSite,
    namespace: SourceNamespaceKey,
    simple: tcl_core_types::NameBytes,
}

impl CommandRefInterner {
    /// Retain a source-analysis identity domain. Independently created interners
    /// cannot alias even when their source instances have equal bytes.
    #[must_use]
    pub fn new(source: &Arc<SourceOriginId>) -> Self {
        Self {
            identities: tcl_core_types::IdentityInterner::new(
                tcl_core_types::IdentityDomain::Analysis(SourceOriginId::activation_key(source)),
            ),
        }
    }
}

impl CommandIdentity {
    /// Project a retained allocation into the shared typed identity vocabulary.
    /// Runtime tokens retain their actual owner. Repeated or symbolic source
    /// allocations abstain instead of manufacturing an identity from a label.
    pub fn command_ref(
        &self,
        interner: &mut CommandRefInterner,
    ) -> tcl_core_types::Answer<tcl_core_types::CommandRef> {
        use tcl_core_types::{
            Answer, IdentityDomain, IdentityKey, IdentityProvenance, NamingAuthority,
            NamingProvenance, SemanticRef, UnknownReason,
        };
        if let Some(runtime) = self.runtime {
            return Answer::proven(tcl_core_types::CommandRef::from_key(IdentityKey {
                domain: IdentityDomain::Runtime {
                    owner: runtime.interpreter.owner,
                    interpreter: runtime.interpreter.interpreter,
                },
                token: runtime.token,
                generation: 0,
                issuer: None,
            }))
            .with_evidence(
                NamingProvenance::Runtime {
                    owner: runtime.interpreter.owner,
                    interpreter: runtime.interpreter.interpreter,
                },
                NamingAuthority::Navigation,
            );
        }
        let Some(allocation) = &self.allocation else {
            return Answer::unknown(UnknownReason::UnavailableContext);
        };
        let generation = match allocation.incarnation {
            AllocationIncarnation::First => 1,
            AllocationIncarnation::Second => 2,
            AllocationIncarnation::RepeatedFresh => {
                return Answer::unknown(UnknownReason::RepeatedAllocation);
            }
        };
        let SourceCommandKey::Slot { namespace, simple } = &allocation.slot else {
            return Answer::unknown(UnknownReason::UnavailableContext);
        };
        let instance = SourceOriginId::activation_key(&allocation.site.source);
        let offset = allocation.site.offset;
        let reference = interner.identities.intern(
            SourceCommandDescriptor {
                site: allocation.site.clone(),
                namespace: namespace.clone(),
                simple: simple.clone(),
            },
            generation,
            IdentityProvenance::Source { instance, offset },
        );
        Answer::proven(reference).with_evidence(
            NamingProvenance::Source { instance, offset },
            NamingAuthority::Navigation,
        )
    }
}

/// Primary command-table key. A written source spelling cannot impersonate an
/// actual native slot, even when it equals that slot's display spelling.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc(hidden)]
pub enum SourceCommandKey {
    /// Symbolic command key in an authored document's namespace model.
    Authored(String),
    /// Exact counted simple key in a retained or allocated namespace table.
    Slot {
        /// Namespace incarnation, independent of its printed full name.
        namespace: SourceNamespaceKey,
        /// Already selected native simple-name bytes.
        simple: NameBytes,
    },
}

impl SourceCommandKey {
    #[must_use]
    pub fn authored(key: impl Into<String>) -> Self {
        Self::Authored(key.into())
    }

    #[doc(hidden)]
    #[must_use]
    pub fn slot(namespace: SourceNamespaceKey, simple: NameBytes) -> Self {
        Self::Slot { namespace, simple }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn holder(&self) -> std::borrow::Cow<'_, SourceNamespaceKey> {
        match self {
            Self::Slot { namespace, .. } => std::borrow::Cow::Borrowed(namespace),
            Self::Authored(text) => {
                let (holder, _) = tcl_syntax::naming::key_holder_and_tail(text);
                std::borrow::Cow::Owned(SourceNamespaceKey::authored(if holder.is_empty() {
                    "::"
                } else {
                    holder
                }))
            }
        }
    }

    /// Symbolic document spelling only. Native slots require an independent
    /// callable-spelling receipt rather than a rendered path.
    #[must_use]
    pub fn authored_spelling(&self) -> Option<&str> {
        match self {
            Self::Authored(text) => Some(text),
            Self::Slot { .. } => None,
        }
    }
}

impl From<String> for SourceCommandKey {
    fn from(value: String) -> Self {
        Self::authored(value)
    }
}
impl From<&str> for SourceCommandKey {
    fn from(value: &str) -> Self {
        Self::authored(value)
    }
}

/// The lattice element a command name resolves to.
///
/// Height-3 join lattice: [`BindingKind::Bottom`] (⊥) is the identity,
/// a concrete binding joined with itself is unchanged, and two
/// *different* bindings rise to [`BindingKind::Unknown`] (⊤).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BindingKind {
    /// ⊥ — identity for join (no contribution yet).
    Bottom,
    /// The original core/registry command, unperturbed.
    Builtin,
    /// A user procedure (`target` = its canonical qname).
    Proc,
    /// A concrete registry-described command whose narrower identity is not
    /// relevant to this lattice.
    Command,
    /// A `TclOO`/snit/itcl class or instance command created by a
    /// registry-described definer (`target` = its canonical qname).
    /// Distinct from [`Self::Proc`] so `NAME destroy` — the universal
    /// object method — is only modelled as a deletion for names that
    /// actually denote objects.
    Class,
    /// An instance of a registry object class (a Tk widget's path), created
    /// by the constructor command `factory`. A call through it dispatches
    /// that class's declared instance methods; it is never a `TclOO` class,
    /// and its methods include no `destroy` deletion.
    Instance {
        /// The constructor's canonical registry command.
        factory: &'static str,
    },
    /// An `interp alias` (`target` = the alias target name).
    Alias,
    /// Renamed/deleted-away or never-defined → dispatches to `unknown`.
    Opaque,
    /// ⊤ — conflicting bindings at a merge, or dynamic mutation.
    Unknown,
}

impl BindingKind {
    /// The binding a registry definition installs before any instance
    /// refinement: an instance is first published as an object command.
    #[must_use]
    pub const fn of_definition(kind: tcl_registry::CommandBindingDefinitionKind) -> Self {
        match kind {
            tcl_registry::CommandBindingDefinitionKind::Procedure => Self::Proc,
            tcl_registry::CommandBindingDefinitionKind::Object
            | tcl_registry::CommandBindingDefinitionKind::Instance => Self::Class,
            tcl_registry::CommandBindingDefinitionKind::Command => Self::Command,
        }
    }
}
