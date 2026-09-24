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
//! time (rung 1), or a builtin's specialisation reached through a pack
//! command's `alias_of` (rung 2) — records a [`SiteClaim`] carrying the
//! [`PackFactStamp`] of the pack facts behind it, so a changed pack
//! invalidates the artefact rather than silently changing its meaning. The
//! VM admits a unit only when every claim's stamp is one it holds for the
//! pack set it runs under. Rung 0, generic dispatch, records nothing: there
//! is nothing a generic dispatch can get wrong.

use crate::CommandBindingIdentity;

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
}

impl SiteClaim {
    /// The pack facts the claim rests on — what admission compares against
    /// the facts the VM holds.
    #[must_use]
    pub fn facts(&self) -> &PackFactStamp {
        match self {
            Self::PackFacts(facts) | Self::BuiltinAlias { facts, .. } => facts,
        }
    }
}
