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

//! Registry and document-dialect ingress for the VM.
//!
//! Release names resolve through [`tcl_registry::model::resolve_environment`].
//! Registry handles come from that environment's retained command generation;
//! command visibility uses its document authoring query. Physical invocation
//! protocols and compiler admission use the VM's independently retained actual
//! engine point. A document profile alone cannot establish that engine point.
//!
//! Release-specific ensemble tables preserve native table order. They select
//! actual installed workers independently of authored command availability.

use std::sync::{Mutex, OnceLock};

use tcl_dialect::model::SurfaceQuery;
use tcl_dialect::model::surface_admits;
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_registry::CommandRegistry;
use tcl_registry::model::{PinError, PinnedContext};
use tcl_runtime_api::RuntimeContext;

/// Resolve a dialect **name** to the profile this VM pins.
///
/// The environment-model form of the name resolver: the resolved
/// environment's [`unit_profile`], which is its same-named catalogue
/// profile for every release name the VM pins and the permissive fallback
/// for the lenient and unknown spellings — exactly `by_name`'s answer at
/// every one of this engine's ingresses.
///
/// [`unit_profile`]: tcl_registry::model::DocumentEnvironment::unit_profile
pub(crate) fn profile_for_dialect(name: &str) -> &'static DialectProfile {
    tcl_registry::model::resolve_environment(name).unit_profile()
}

/// Resolve a [`RuntimeContext`] through the ingress — the environment-model
/// form of a pin. An overlay nothing has installed is an error here and never
/// the un-overlaid generation under another name.
pub(crate) fn pin_context(context: &RuntimeContext) -> Result<PinnedContext, PinError> {
    tcl_registry::model::pin(context)
}

/// The command **store** for `profile` — the resolved environment's
/// registry generation, from
/// `tcl_registry::model::static_context_for_profile(profile).commands()`.
///
/// A profile's canonical name **is** a canonical environment id, so this
/// is an id-keyed generation lookup rather than a re-parse, and the
/// generation's store is the same allocation
/// `tcl_registry::model::assembly`'s `command_store` publishes.
///
/// The `&'static` promotion is sound because the un-overlaid generation
/// axis is a closed set and those entries are retained unconditionally,
/// so the promotion leaks a clone of one `Arc`, never a second assembly.
/// That matters here — the VM caches this handle on the pin and consults
/// it on every command resolution.
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

/// The profile a `namespace` or `trace` subcommand table is gated under: the
/// profile the VM exposes commands under — the pinned dialect profile, unless
/// a host broadened the surface — as it states one, and the plain profile of
/// the release the VM emulates when that is the permissive fallback, which
/// states none.
///
/// Reading the release name alone answered for a vendor pin with the plain
/// release's table: an iRules VM, which emulates 8.4, took `trace add` from
/// `tcl8.4` though the TMM's Tcl has only the three legacy forms.
pub(crate) fn gate_profile(
    surface: &'static DialectProfile,
    release: TclVersion,
) -> &'static DialectProfile {
    if surface.is_fallback() {
        profile_for_dialect(release.dialect_profile_name())
    } else {
        surface
    }
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
    if command == "info" {
        let dialect =
            tcl_registry::InvocationDialect::of_profile(profile_for_dialect(dialect_name));
        if let Some(names) = dialect.native_jim_info_member_names() {
            return names;
        }
    }
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
                if command == "array"
                    && tcl_registry::InvocationDialect::of_profile(profile)
                        .native_array_search_member_present(name.as_bytes())
                        == Some(false)
                {
                    return false;
                }
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
