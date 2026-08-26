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

//! The C-ABI runtime's dialect ingress — this engine's face of the one
//! shared seam, [`tcl_registry::model::ingress`] (centralisation contract
//! R-a, retirement-ledger rows C2/B1; P1-F wave 3, the backend lane).
//!
//! The same three helpers `tcl-vm`'s `crate::environment` carries, for the
//! same reason: both engines used to resolve a dialect *name* with
//! `DialectProfile::by_name` (or, in `codegen_abi`, with a raw
//! `DialectSet::parse`), reach the registry with `registry_for_profile`,
//! and read `profile.availability_mask` at each availability question.
//! All three now go through the resolved environment.
//!
//! Behaviour is unchanged by construction. The only dialect names this
//! runtime accepts are the closed set [`TclVersion::dialect_profile_name`]
//! spells, whose environments are their same-named catalogue entries, so
//! [`profile_for_dialect`] returns the profile `by_name` returned; the
//! generation's command store is the very `Arc` the old `(profile,
//! overlay)` cache owns; and the document authoring mask is test-pinned
//! equal to the threaded profile's `availability_mask` for every profile
//! an ingress can produce.
//!
//! Post-P1-G (which deleted the name validators and old cache doors):
//! the `&'static DialectProfile` these helpers take and hand back
//! retires with ledger C1's re-type, and the interpreter's pin then
//! carries a [`tcl_registry::model::DocumentEnvironment`] instead.
//!
//! [`TclVersion::dialect_profile_name`]: tcl_dialect::TclVersion::dialect_profile_name

use tcl_dialect::{DialectProfile, DialectSet};
use tcl_registry::CommandRegistry;

/// Resolve a dialect **name** to the profile this interpreter pins.
///
/// The environment-model form of the retired
/// `DialectProfile::by_name(name)`: the resolved environment's
/// [`unit_profile`], which is its same-named catalogue profile for every
/// release name this runtime pins and the permissive fallback for the
/// lenient and unknown spellings.
///
/// [`unit_profile`]: tcl_registry::model::DocumentEnvironment::unit_profile
pub(crate) fn profile_for_dialect(name: &str) -> &'static DialectProfile {
    tcl_registry::model::resolve_environment(name).unit_profile()
}

/// The command **store** for `profile` — the resolved environment's
/// registry generation, replacing the retired
/// `tcl_registry::cache::registry_for_profile(profile)`.
///
/// A profile's canonical name **is** a canonical environment id, so this
/// is an id-keyed generation lookup rather than a re-parse, and the
/// generation's store is the same allocation the old per-profile cache
/// published. The `&'static` promotion leaks a clone of one `Arc` per
/// environment, never a second assembly — which is what lets the
/// interpreter keep caching this handle on its pin.
pub(crate) fn store_for_profile(profile: &'static DialectProfile) -> &'static CommandRegistry {
    tcl_registry::model::static_context_for_profile(profile).commands()
}

/// The availability mask a command-surface, subcommand, or option
/// question about `profile` is answered under — its environment's
/// **document authoring mask**, replacing the direct
/// `profile.availability_mask` read.
///
/// Equal to `profile.availability_mask` for every profile an ingress can
/// produce, pinned by `tcl_registry::model::ingress`'s
/// `the_document_mask_is_the_threaded_profiles_mask`.
pub(crate) fn surface_mask(profile: &'static DialectProfile) -> DialectSet {
    tcl_registry::model::static_document_context_for_profile(profile).authoring_mask()
}

/// The mask a dialect **name** is gated under, `None` when the name is
/// not a declared environment — the fail-closed form, replacing
/// `codegen_abi`'s raw `DialectSet::parse` ingress (ledger row C2's last
/// backend caller).
///
/// The decline arm is kept rather than collapsed onto the lenient
/// fallback [`surface_mask`] would give: an intrinsic whose dialect
/// cannot be established must not enter the guarded fast path under the
/// permissive mask. The acceptance set is unchanged for every name that
/// reaches here — the closed release set
/// [`tcl_dialect::TclVersion::dialect_profile_name`] spells, whose
/// environments' authoring masks are the single release bits `parse`
/// returned — and the seam's `the_validator_accepts_every_retired_validators_name`
/// pins that no spelling `DialectSet::parse` accepted is rejected here.
pub(crate) fn known_surface_mask_for_dialect(name: &str) -> Option<DialectSet> {
    tcl_registry::model::resolve_known_environment(name)
        .map(|environment| environment.document_authoring_mask())
}
