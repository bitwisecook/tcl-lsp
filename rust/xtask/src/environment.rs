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

//! The sweeps' and generators' dialect ingress — `xtask`'s face of the one
//! shared seam, [`tcl_registry::model::ingress`] (centralisation contract
//! R-a; P1-F wave 3, alongside the two engines these sweeps exercise).
//!
//! Every dialect **name** an `xtask` subcommand accepts — a `--dialect`
//! flag, a projection target's canonical id, the fixed `f5-irules` the
//! iRule-test generator simulates — resolves here, once, and the registry
//! store, availability mask, and availability queries it then asks are
//! derived from the resolved environment.
//!
//! Nothing here changes what a generator emits. The catalogue names these
//! sweeps use resolve to their same-named environments, whose
//! [`unit_profile`] is the profile `by_name`/`find` returned and whose
//! generation store is the very `Arc` the old `(profile, overlay)` cache
//! owns — including the profile *stamp* the projections read back
//! ([`tcl_registry::CommandRegistry::profile`]), which the generation
//! shares by handle rather than re-deriving. The `--check` modes are the
//! gate: a projection that drifted would fail them.
//!
//! [`unit_profile`]: tcl_registry::model::DocumentEnvironment::unit_profile

use tcl_dialect::model::{SurfaceQuery};
use tcl_dialect::{DialectProfile, DialectSet};
use tcl_registry::CommandRegistry;
use tcl_registry::model::ResolvedContext;

/// Resolve a dialect **name** to the profile a projection threads — the
/// environment-model form of `DialectProfile::by_name` and of the named
/// constructors (`plain_tcl`, `irules`, `tk`).
///
/// Post-P1-G (which deleted the name validators): the threaded profile
/// handle itself retires with ledger C1's re-type, when these projections
/// read their grammar unions and labels off the environment instead.
pub fn profile_for_dialect(name: &str) -> &'static DialectProfile {
    tcl_registry::model::resolve_environment(name).unit_profile()
}

/// Resolve a dialect **name** only when it names a real environment —
/// the validator form, replacing `DialectProfile::find(name)` at the
/// ingresses that must reject an unknown spelling rather than serve the
/// lenient fallback.
pub fn known_profile_for_dialect(name: &str) -> Option<&'static DialectProfile> {
    tcl_registry::model::resolve_known_environment(name)
        .map(|environment| environment.unit_profile())
}

/// The command **store** for a dialect name — the resolved environment's
/// registry generation, replacing `tcl_registry::registry_for_dialect`.
pub fn store_for_dialect(name: &str) -> &'static CommandRegistry {
    tcl_registry::model::static_context_for(name).commands()
}

/// The command **store** for an already-resolved profile — replacing
/// `tcl_registry::registry_for_profile`. A profile's canonical name is a
/// canonical environment id, so this is an id-keyed generation lookup,
/// not a re-parse.
pub fn store_for_profile(profile: &'static DialectProfile) -> &'static CommandRegistry {
    tcl_registry::model::static_context_for_profile(profile).commands()
}

/// The **document context** a dialect name is projected under — the
/// assistance view that replaces the whole `ProfileQueries` surface
/// (ledger row F1's assistance half).
pub fn context_for_dialect(name: &str) -> &'static ResolvedContext {
    tcl_registry::model::static_document_context_for(name)
}

/// The point a dialect name's projections are filtered under — its
/// environment's document authoring point. Test-pinned equal to the
/// profile's own point for every profile an ingress can produce.
pub fn surface_point_for_dialect(name: &str) -> SurfaceQuery<'static> {
    context_for_dialect(name).authoring_query()
}
