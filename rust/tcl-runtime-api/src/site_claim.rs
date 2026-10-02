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

//! What a specialised site claims about the spec-pack facts its emitted code
//! rests on — `docs/design/compiler/registry-consumer-contracts.md`
//! § *What the artefact records per rung*.
//!
//! A pack may claim; only the runtime may attest. A site whose emitted code
//! depends on a pack — a constant a pack's `const_fold` computed at compile
//! time (rung 1), a builtin's specialisation reached through a pack
//! command's `alias_of` (rung 2), or a body the pack's `runtime_backing`
//! says defines a command, inlined at its call (rung 3) — records a
//! [`SiteClaim`] carrying the [`PackFactStamp`] of the pack facts behind it,
//! so a changed pack invalidates the artefact rather than silently changing
//! its meaning. The VM admits a unit only when every claim's stamp is one it
//! holds for the pack set it runs under. Rung 0, generic dispatch, records
//! nothing: there is nothing a generic dispatch can get wrong.

use crate::{CommandBindingIdentity, ProcedureBindingIdentity};

/// Which pack facts a specialised site rests on.
///
/// Two stamps are the same facts exactly when every field agrees; the VM's
/// admission compares them whole.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackFactStamp {
    /// The pack's name as the pack set holds it — its `speclib` name.
    pub pack: String,
    /// The content hash of the pack file that declared the command: the
    /// `u64` xxh3 of its bytes, the value that file's `EvalSnapshotKey`
    /// interns, folded with every fragment an `include` row brought in.
    pub content_hash: u64,
    /// The loader's vocabulary version (`tcl_spectcl::VOCABULARY_VERSION`,
    /// the one the snapshot key interns beside the content hash), so a
    /// change to what a word means invalidates even at an unchanged hash.
    pub vocabulary_version: String,
    /// The registry overlay generation the site compiled under — the pack
    /// set's content key.
    pub overlay_generation: u64,
    /// The evaluator revision the site compiled under
    /// (`tcl_registry::pack_hooks::evaluator_generation`): which host served
    /// any declared implementation the site consumed.
    pub evaluator_revision: u64,
}

/// What one specialised site claims, by rung.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SiteClaim {
    /// Rung 1: the pack facts this site's specialisation rests on — a
    /// constant a pack's `const_fold` computed at compile time.
    PackFacts(PackFactStamp),
    /// Rung 2: the builtin the pack said this command is, as the identity
    /// the VM's alias hop resolves — the target's, never the pack command's
    /// own name — and the pack facts behind the claim. The binding is also
    /// one of the unit's command bindings, checked as any other is.
    BuiltinAlias {
        /// The site's binding: the pack spelling, resolving to the target.
        binding: CommandBindingIdentity,
        /// The pack facts that made the target admissible.
        facts: PackFactStamp,
    },
    /// Rung 3: the body of a command a pack declares as `TclBody`-backed,
    /// inlined at this site, and the kind of backing that makes a procedure
    /// the right thing to compare against at all. The procedure binding is
    /// also one of the unit's procedure bindings, checked as any other is;
    /// what the claim adds is the backing, which the VM requires to be
    /// [`BackingKind::TclBody`], and the pack facts behind the declaration.
    ReferenceBody {
        /// The site's binding: the invocation as written, the creation name,
        /// parameters and body text of the definition copied into the unit.
        procedure: ProcedureBindingIdentity,
        /// The kind of backing the command was declared with when the body
        /// was inlined.
        backing: BackingKind,
        /// The pack facts that declared the backing.
        facts: PackFactStamp,
    },
}

/// How a described command's behaviour reaches the runtime, without the
/// payload of where — `tcl_registry::RuntimeBacking`'s variants by name. A
/// claim states the kind and the VM holds the rung to it: an exact body
/// match is a true statement about a procedure and says nothing about
/// whether the command *is* that procedure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BackingKind {
    /// A shipped builtin, attested by its registry identity.
    ShippedBuiltin,
    /// A Tcl body — the only kind a procedure is a faithful stand-in for.
    TclBody,
    /// A command the host registered natively, attested by a guard identity
    /// and never by a procedure definition.
    HostNative,
    /// Nothing executes the command in the target runtime.
    None,
}

impl SiteClaim {
    /// The pack facts the claim rests on — what admission compares against
    /// the facts the VM holds.
    #[must_use]
    pub fn facts(&self) -> &PackFactStamp {
        match self {
            Self::PackFacts(facts)
            | Self::BuiltinAlias { facts, .. }
            | Self::ReferenceBody { facts, .. } => facts,
        }
    }
}
