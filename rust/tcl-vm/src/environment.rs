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

//! The VM's dialect ingress — this engine's face of the one shared seam,
//! [`tcl_registry::model::ingress`] (centralisation contract R-a,
//! retirement-ledger rows C2/B1/B11; P1-F wave 3, the backend lane).
//!
//! Every dialect **name** the VM accepts (there is exactly one kind: the
//! release name a [`TclVersion`] pin spells, [`Vm::set_runtime_version`])
//! resolves here, once, through [`tcl_registry::model::resolve_environment`],
//! and every registry access and availability point the engine reads is
//! derived from the resolved environment rather than from a second lookup of
//! the string.
//!
//! Nothing in this module changes what the VM admits. The names it resolves
//! are the closed set [`TclVersion::dialect_name`] spells, whose environments
//! are their same-named catalogue entries, so [`profile_for_dialect`] returns
//! the very profile `DialectProfile::by_name` did; the generation's command
//! store is the very `Arc` the old `(profile, overlay)` cache owns, so
//! [`store_for_profile`] returns the allocation `registry_for_profile`
//! returned; and the document authoring mask is test-pinned to the threaded
//! profile's `surface_query` for every profile an ingress can produce, so
//! [`surface_point`] answers the command-availability gate exactly as the mask
//! read did.
//!
//! Post-P1-G (which deleted the name validators and old cache doors):
//! the `&'static DialectProfile` these helpers take and hand back
//! retires with ledger C1's re-type, and the VM's pin then carries a
//! [`tcl_registry::model::DocumentEnvironment`] instead.
//!
//! **Scope: the VM executes Tcl 9 semantics.** The closed set above is a
//! set of *Tcl releases*; a dialect with no Tcl ladder rung (`jim`) never
//! reaches [`Vm::set_runtime_version`] — its projected profile
//! (`DialectProfile::projected_from_point`) carries
//! `vm_runtime_version = V9_0`, so a Jim unit is *compiled* under Jim's
//! grammar and *executed* as Tcl 9. That is the intended boundary today.
//! The eventual, recorded in `docs/design/dialect-profile-model.md` §2.5,
//! is a pin that is a `tcl_dialect::DialectPoint` rather than a
//! `TclVersion`, at which point this module resolves it the same way and
//! nothing upstream changes: every consumer here already derives from the
//! resolved environment, never from the name.
//!
//! [`Vm::set_runtime_version`]: crate::Vm::set_runtime_version
//! [`TclVersion`]: tcl_dialect::TclVersion
//! [`TclVersion::dialect_name`]: tcl_dialect::TclVersion::dialect_name

use std::sync::{Mutex, OnceLock};

use tcl_dialect::DialectProfile;
use tcl_dialect::model::SurfaceQuery;
use tcl_dialect::model::surface_admits;
use tcl_registry::CommandRegistry;

/// Resolve a dialect **name** to the profile this VM pins.
///
/// The environment-model form of the retired
/// the retired name resolver: the resolved environment's
/// [`unit_profile`], which is its same-named catalogue profile for every
/// release name the VM pins and the permissive fallback for the lenient
/// and unknown spellings — exactly `by_name`'s answer at every one of
/// this engine's ingresses.
///
/// [`unit_profile`]: tcl_registry::model::DocumentEnvironment::unit_profile
pub(crate) fn profile_for_dialect(name: &str) -> &'static DialectProfile {
    tcl_registry::model::resolve_environment(name).unit_profile()
}

/// The command **store** for `profile` — the resolved environment's
/// registry generation, replacing the retired
/// `tcl_registry::model::ingress::static_context_for_profile(profile).commands()`.
///
/// A profile's canonical name **is** a canonical environment id, so this
/// is an id-keyed generation lookup rather than a re-parse, and the
/// generation's store is the same allocation the old per-profile cache
/// published (`tcl_registry::model::assembly`'s `command_store`).
///
/// The `&'static` promotion is sound on the same terms the old one was:
/// the un-overlaid generation axis is a closed set and those entries are
/// retained unconditionally, so the promotion leaks a clone of one `Arc`,
/// never a second assembly. That matters here — the VM caches this handle
/// on the pin and consults it on every command resolution.
pub(crate) fn store_for_profile(profile: &'static DialectProfile) -> &'static CommandRegistry {
    tcl_registry::model::static_context_for_profile(profile).commands()
}

/// The point the builtin command-surface gate answers at for `profile` —
/// the **document authoring point** of the profile's environment, rather
/// than a direct `profile.surface_query()` read.
///
/// Equal to `profile.surface_query()` for every profile an ingress can
/// produce, pinned by `tcl_registry::model::ingress`'s
/// `the_document_point_matches_the_threaded_profile`.
pub(crate) fn surface_point(profile: &'static DialectProfile) -> SurfaceQuery<'static> {
    tcl_registry::model::static_document_context_for_profile(profile).authoring_query()
}

/// [`surface_point`] keyed by a dialect **name** — for the native command
/// handlers that gate a subcommand or option table on the emulated
/// release ([`TclVersion::dialect_profile_name`]) rather than on a pinned
/// profile handle.
///
/// One resolution of the name, not two: the old form was
/// the retired resolver's availability-mask read, and the resolved
/// environment's document authoring point is that same point.
///
/// [`TclVersion::dialect_profile_name`]: tcl_dialect::TclVersion::dialect_profile_name
pub(crate) fn surface_point_for_dialect(name: &str) -> SurfaceQuery<'static> {
    tcl_registry::model::static_document_context_for(name).authoring_query()
}

/// One memoised answer: `(command, release name, the release's slice of the
/// engine's table)`.
type SubcommandCacheEntry = (&'static str, String, &'static [&'static str]);

/// The subset of an engine ensemble `table` the emulated release actually
/// has, in the table's own order.
///
/// A `TclMakeEnsemble` table is a *release* fact: `dict getwithdefault`,
/// `array for`, `file tempdir` and `info cmdtype` arrive in Tcl 9, and a
/// handler that resolves against one release's table under every pin gets
/// two things wrong at once — it dispatches a subcommand the pinned release
/// never had, and, worse, a 9-only name silently changes an 8.x prefix
/// verdict for a name that has nothing to do with it (`dict g` is `get` on
/// 8.6 and ambiguous on 9.0). The names and their gates belong to the
/// registry, so this filters the engine's table through the selected
/// release's surface rather than duplicating the release facts here.
///
/// Only *removal* happens: a name the registry does not model (an engine
/// extra) is kept, so a table stays the engine's own list of what it
/// dispatches and its enumeration order.
///
/// This sits on the dispatch path of the hottest ensembles (`dict`, `string`,
/// `info`), and resolving the environment by name costs tens of microseconds
/// — 6x the whole cost of a `dict get` — so the answer is memoised per
/// `(command, release)`. That pair is a closed, tiny set, and each engine has
/// exactly one table per command name, so the leak is bounded by it.
fn release_subcommand_cache() -> &'static Mutex<Vec<SubcommandCacheEntry>> {
    static CACHE: OnceLock<Mutex<Vec<SubcommandCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(Vec::new()))
}

pub(crate) fn release_subcommands(
    dialect_name: &str,
    command: &'static str,
    table: &[&'static str],
) -> &'static [&'static str] {
    let cache = release_subcommand_cache();
    if let Some((_, _, hit)) = cache
        .lock()
        .expect("subcommand cache")
        .iter()
        .find(|(cmd, name, _)| *cmd == command && name == dialect_name)
    {
        return hit;
    }
    let profile = profile_for_dialect(dialect_name);
    let point = surface_point(profile);
    let filtered: Vec<&'static str> = match store_for_profile(profile).get(command) {
        Some(spec) => table
            .iter()
            .copied()
            .filter(|name| {
                spec.subcommands
                    .iter()
                    .find(|sub| sub.name == *name)
                    .is_none_or(|sub| {
                        sub.surface
                            .or(spec.surface)
                            .is_none_or(|gate| surface_admits(gate, Some(&point)))
                    })
            })
            .collect(),
        None => table.to_vec(),
    };
    let leaked: &'static [&'static str] = Vec::leak(filtered);
    cache
        .lock()
        .expect("subcommand cache")
        .push((command, dialect_name.to_string(), leaked));
    leaked
}
