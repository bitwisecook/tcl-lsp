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

//! Per-dialect [`CommandRegistry`] cache.
//!
//! Each canonical dialect gets one lazily-built, cached registry so the
//! per-call build cost is paid once. Unparseable dialect strings collapse to
//! the plain-Tcl entry so a stream of typos cannot leak one registry per typo.
//!
//! Entries are held as [`Arc<CommandRegistry>`] and handed out as handles.
//! The **profile axis** is a closed set (the dialect catalogue), so those
//! entries live for the process either way and [`registry_for_profile`] still
//! offers them as `&'static`. The **overlay axis** is bounded by user edits
//! instead, so those generations are refcounted: when a superseded one is
//! dropped from the table and the last analysis holding it finishes, its
//! memory is actually returned. See the memory note on
//! [`registry_for_profile_with_overlay`].
//!
//! Lives in `tcl-registry` (not a consumer crate) so every downstream
//! tool — the CLI, the compiler explorer, future MCP/AI surfaces — shares
//! one cache rather than each rebuilding its own.

use std::sync::{Arc, Mutex, OnceLock};

use rustc_hash::FxHashMap;
use tcl_dialect::DialectProfile;

use crate::registry::CommandRegistry;

/// The `(profile, overlay)` lookup table's own type, named so the static, the
/// sweep, and its test all say the same thing.
type RegistryTable = FxHashMap<(&'static str, u64), Arc<CommandRegistry>>;

/// Every `(profile, overlay)` registry built so far.
///
/// Module-level rather than function-local because two entry points read it:
/// [`registry_for_profile_with_overlay`], which builds a missing entry, and
/// [`registry_for_profile_if_built`], which deliberately does not.
static REGISTRIES: OnceLock<Mutex<RegistryTable>> = OnceLock::new();

/// The process-wide **plain default** registry — exactly what
/// [`CommandRegistry::build_default`] produces, built once.
///
/// For callers that genuinely need no dialect layering and reached for
/// `build_default()` to say so.  `build_default()` is a *constructor*, not an
/// accessor: it rebuilds several hundred `CommandSpec`s (and, before issue
/// #1035, leaked the generated `tcl::mathop` / `tcl::mathfunc` ensembles) on
/// every call.  Hot paths that run per CFG build — so, per keystroke — were
/// calling it directly; they want this.
///
/// Deliberately *not* routed through [`registry_for_profile`]: that would
/// attach the `PLAIN_TCL` profile and its (currently empty) base layers, which
/// is a behavioural claim this accessor does not need to make.  Same
/// constructor, same bytes, once.
pub fn default_registry() -> &'static CommandRegistry {
    static DEFAULT: OnceLock<CommandRegistry> = OnceLock::new();
    DEFAULT.get_or_init(CommandRegistry::build_default)
}

/// The un-overlaid registry for `profile` as a **handle**, building it on
/// first use.
///
/// Same entry [`registry_for_profile`] returns — overlay `0` — just shared as
/// an [`Arc`] rather than as a `&'static`. Callers whose registry may be
/// either a plain profile registry or a pack-carrying overlay want this, so
/// both arms of that choice have one type; holding the handle is also what
/// keeps an overlay generation alive for the length of an analysis.
///
/// The profile axis is a closed set and [`prune_overlays`] retains every
/// overlay-`0` entry unconditionally, so these particular handles are never
/// retired: the table's own reference outlives the process.
#[must_use]
pub(crate) fn registry_handle_for_profile(
    profile: &'static DialectProfile,
) -> Arc<CommandRegistry> {
    registry_for_profile_with_overlay(profile, 0, |_| {})
}

/// Every profile whose handle has been promoted to a `&'static` view.
///
/// Backs [`registry_for_profile`]. The value is a leaked *clone of the
/// handle*, not a second registry, so the `&'static` and the `Arc` name one
/// allocation and cannot drift apart.
static LEAKED: OnceLock<Mutex<FxHashMap<&'static str, &'static CommandRegistry>>> = OnceLock::new();

/// Return the cached registry for `profile`, building it on first use.
///
/// The registry is the default build plus the profile's
/// [`base_layers`](DialectProfile::base_layers) command packs, keyed by the
/// profile's canonical name — so aliases share their canonical entry and
/// every unknown dialect string shares the one `PLAIN_TCL` entry.
///
/// The `&'static` is honest here and stays: the dialect catalogue is closed,
/// these entries are pinned by the table for the process lifetime, and several
/// hundred call sites across the tree are correctly process-lifetime. The
/// promotion leaks a clone of the profile's handle — one `Arc` per profile,
/// eight bytes each — rather than a copy of the registry.
#[must_use]
pub(crate) fn registry_for_profile(profile: &'static DialectProfile) -> &'static CommandRegistry {
    let leaked = LEAKED.get_or_init(|| Mutex::new(FxHashMap::default()));
    if let Some(view) = leaked
        .lock()
        .expect("registry leak map mutex")
        .get(profile.name)
        .copied()
    {
        return view;
    }
    // Built outside the lock: the miss path runs `build_default` plus the
    // profile's layers, and nothing about that needs this map held.
    let handle = registry_handle_for_profile(profile);
    let mut guard = leaked.lock().expect("registry leak map mutex");
    if let Some(view) = guard.get(profile.name).copied() {
        // Another thread promoted this profile while we were building. Its
        // handle and ours are clones of one cache entry, so either view is
        // the same allocation; keep the one already published and let ours
        // drop.
        return view;
    }
    let pinned: &'static Arc<CommandRegistry> = Box::leak(Box::new(handle));
    let view: &'static CommandRegistry = pinned.as_ref();
    guard.insert(profile.name, view);
    view
}

/// The `(profile, overlay)` registry **only if it has already been built**.
///
/// The overlay's *contents* come from a closure only its owner can write —
/// `tcl-spectcl`, which parses the packs — so a consumer that knows only the
/// overlay's identity (a `u64` handed to it as configuration, e.g. through a
/// salsa input) cannot ask for it with [`registry_for_profile_with_overlay`]:
/// on a miss it would build and permanently cache a *pack-less* registry under
/// the pack's key.
///
/// This is the read-only half of that door. A consumer passes the key it was
/// given and falls back to [`registry_for_profile`] on `None`, which is
/// exactly right: the miss means the packs have not been installed for this
/// profile yet, and a registry without them is what the process had a moment
/// ago anyway.
///
/// It exists because the **analyser** needs it. Since the EDA vendor libraries
/// became bundled loadables (`docs/design/spec-packs.md`), "which commands
/// exist" is no longer answerable from compiled-in data alone, and the
/// analyser — which resolves its own registry from its `DialectProfile` — has
/// to be able to reach the pack-carrying entry without depending on the
/// loader crate that sits above it.
#[must_use]
pub(crate) fn registry_for_profile_if_built(
    profile: &'static DialectProfile,
    overlay: u64,
) -> Option<Arc<CommandRegistry>> {
    if overlay == 0 {
        return Some(registry_handle_for_profile(profile));
    }
    let map = REGISTRIES.get_or_init(|| Mutex::new(FxHashMap::default()));
    let guard = map.lock().expect("registry cache mutex");
    guard.get(&(profile.name, overlay)).cloned()
}

/// Return the cached registry for `profile` **plus a caller-supplied overlay**,
/// building it on first use and keyed by `(profile, overlay)`.
///
/// `overlay` is an opaque content identity for whatever `extend` adds:
/// same number, same registry, and `0` means "no overlay" and is exactly
/// [`registry_for_profile`]. `extend` runs once per distinct key, after the
/// profile's own layers and before the profile stamp, so an overlaid spec
/// lands last in its name's spec list and therefore wins
/// [`CommandRegistry::get`] — which is what lets an overlay *replace* a
/// shipped command when it means to.
///
/// # Why this lives here, and takes a closure
///
/// The one caller today is `tcl-spectcl`, inserting a workspace's `SpecTcl`
/// packs (`docs/design/spec-packs.md`: packs layer into the per-profile cached
/// registry at **workspace scope**, never the per-document overlay path stubs
/// use). This crate must not depend on that one — the registry is the bottom
/// of the stack — so the extension arrives as a closure and the identity as a
/// number the caller computes. Any future workspace-scope source of specs uses
/// the same door.
///
/// # Memory
///
/// Entries are **refcounted, not leaked**. Each generation is an
/// [`Arc<CommandRegistry>`]: the table holds one reference and every consumer
/// that asked for it holds another for as long as it is using it. A
/// generation is *retired* — its memory actually returned — once
/// [`prune_overlays`] has dropped the table's handle to it **and** the last
/// analysis still walking with it has finished. Both halves matter: dropping
/// the table entry alone frees nothing while a request is mid-flight, and a
/// request finishing frees nothing while the table still indexes the key.
///
/// This is what bounds the overlay axis, which is bounded by *user edits*
/// rather than by a catalogue: each distinct pack-set content the server sees
/// costs one more generation, and saving a pack a hundred times would
/// otherwise cost a hundred resident registries until restart. Since specs are
/// interned, a generation is cheap (~261 kB, indexing shared `&'static
/// CommandSpec`s under `&'static str` keys) — but cheap is not free, and
/// unbounded multiples of cheap is the bug this closes.
///
/// The profile axis (`overlay == 0`) is a closed set, is retained
/// unconditionally by the sweep, and is additionally promoted to `&'static`
/// by [`registry_for_profile`]. Those entries are resident for the process by
/// design, which is correct: there are as many of them as there are dialects.
pub fn registry_for_profile_with_overlay(
    profile: &'static DialectProfile,
    overlay: u64,
    extend: impl FnOnce(&mut CommandRegistry),
) -> Arc<CommandRegistry> {
    let map = REGISTRIES.get_or_init(|| Mutex::new(FxHashMap::default()));
    let mut guard = map.lock().expect("registry cache mutex");
    let key = (profile.name, overlay);
    if let Some(r) = guard.get(&key) {
        return Arc::clone(r);
    }

    let mut registry = CommandRegistry::build_default();
    for &layer in profile.base_layers {
        registry.load_surface(layer);
    }
    extend(&mut registry);
    registry.set_profile(profile);
    let handle = Arc::new(registry);

    prune_overlays(&mut guard, overlay);
    guard.insert(key, Arc::clone(&handle));
    handle
}

/// Drop indexed overlays past [`OVERLAY_LIMIT`], keeping every un-overlaid
/// entry (one per dialect profile, a closed set) and the overlay being built
/// right now.
///
/// This is the *reclaiming* half of the change described in the memory note
/// above: removing the entry drops the table's reference, so the generation is
/// freed as soon as the last consumer holding a handle to it finishes. It is
/// still also a table bound — lookups stay cheap — but it is no longer only
/// that.
///
/// Retaining `current` is load-bearing, not tidiness: a reload builds all ~17
/// dialect profiles for one pack key in a loop, so a sweep that dropped every
/// overlay could fire partway through and evict the profiles this very key had
/// already built. Those lookups then miss, [`registry_for_profile_if_built`]
/// returns `None`, and the analyser falls back to the plain registry — every
/// pack and EDA command unknown until something happened to rebuild that
/// profile.
fn prune_overlays(map: &mut RegistryTable, current: u64) {
    if map.len() >= OVERLAY_LIMIT {
        map.retain(|(_, overlay), _| *overlay == 0 || *overlay == current);
    }
}

/// How many `(profile, overlay)` entries the lookup table indexes before it
/// drops the overlaid ones. Generous: a workspace has one pack set at a time,
/// so reaching this at all means a long editing session on a pack.
const OVERLAY_LIMIT: usize = 64;

/// Every command a safe interpreter hides, sorted.
///
/// The generic query behind both engines' `interp create -safe` (ledger row
/// B2, issue #945 fault 7): the set is `Traits::SAFE_INTERP_HIDDEN`, and no
/// consumer spells a command name. C's own set is `CmdInfo` rows lacking
/// `CMD_IS_SAFE` plus the whole-command rows of `unsafeEnsembleCommands`
/// (`tclBasic.c`), which is exactly what the trait records.
///
/// Deliberately **release-agnostic**: it is the union over every modelled
/// release, and a caller narrows it by asking whether the command is
/// actually present in the interpreter being made safe. That is not a
/// shortcut — a release-gated command such as `unload` (8.5+) or `zipfs`
/// (9.0+) is not registered under an older pin, so filtering by registration
/// reproduces the measured per-release sets without a second availability
/// rule. Measured on the reference interpreters
/// (`interp create -safe s; lsort [interp hidden s]`, top-level command
/// names only):
///
/// | release | hidden |
/// |---|---|
/// | 8.4.20 | the twelve below, minus `unload` and `zipfs` |
/// | 8.5.19, 8.6.14 | + `unload` |
/// | 9.0.4 | + `zipfs` |
/// | 9.1b0 | + `clock` |
///
/// 9.1's `clock` is **not** in this set and must not be: 9.1 hides the C
/// `clock` and immediately re-provides a safe one, so `clock format 0 -gmt 1`
/// works inside a 9.1 safe child exactly as it does inside an 8.6 one
/// (measured). The trait means "not callable in a safe interpreter", and
/// `clock` is callable; putting it here would make the analyser's
/// safe-context walk report a false positive on working code. Its presence
/// in `interp hidden` is an artefact of the safe base's hide-then-alias, not
/// of unsafety.
#[must_use]
pub fn safe_interp_hidden_commands() -> &'static [&'static str] {
    static HIDDEN: OnceLock<Vec<&'static str>> = OnceLock::new();
    HIDDEN.get_or_init(|| {
        // The default build carries every plain-Tcl release's commands; the
        // trait itself is release-invariant, so no profile is needed.
        let registry = crate::CommandRegistry::build_default();
        let mut names: Vec<&'static str> = registry
            .commands_with_trait(crate::Traits::SAFE_INTERP_HIDDEN)
            .into_iter()
            // The registry's own names are `'static` string literals from the
            // compiled specs; `commands_with_trait` merely reborrows them
            // through `&self`, so look each one back up to recover the
            // lifetime rather than leaking a copy.
            .filter_map(|name| registry.get(name).map(|spec| spec.name))
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Crossing [`OVERLAY_LIMIT`] mid-reload must not evict the overlay being
    /// built.
    ///
    /// One reload installs a single pack key across every dialect profile in a
    /// loop, so the sweep can fire partway through that loop. Before this, it
    /// dropped *every* overlaid entry — including the profiles the in-flight
    /// key had already built — and those commands went silently unknown.
    ///
    /// Driven through [`prune_overlays`] against a table of synthetic keys
    /// rather than by building 65 real registries: the entries all point at
    /// one leaked registry, so this exercises the real threshold and the real
    /// predicate for the cost of a single build.
    #[test]
    fn pruning_keeps_the_overlay_being_built() {
        const CURRENT: u64 = 0x00C0_FFEE;
        const OLD: u64 = 0x00DE_CAF0;

        let one = Arc::new(CommandRegistry::build_default());
        let mut map = RegistryTable::default();

        // A closed set of un-overlaid entries, the profiles this reload has
        // already built for the current key, and a long tail of stale ones
        // from earlier edits — together past the cap.
        let profiles: Vec<&'static str> = DialectProfile::all()
            .iter()
            .map(|profile| profile.name)
            .collect();
        assert!(profiles.len() >= 2, "the catalogue must have real profiles");
        for name in &profiles {
            map.insert((name, 0), Arc::clone(&one));
            map.insert((name, CURRENT), Arc::clone(&one));
        }
        for stale in 0..OVERLAY_LIMIT as u64 {
            map.insert((profiles[0], OLD + stale), Arc::clone(&one));
        }
        assert!(map.len() >= OVERLAY_LIMIT, "the fill must cross the cap");

        prune_overlays(&mut map, CURRENT);

        for name in &profiles {
            assert!(
                map.contains_key(&(name, CURRENT)),
                "profile {name} lost the overlay being built"
            );
            assert!(
                map.contains_key(&(name, 0)),
                "profile {name} lost its un-overlaid entry"
            );
        }
        assert!(
            !map.contains_key(&(profiles[0], OLD)),
            "a stale overlay must not survive the sweep"
        );
    }

    /// A superseded overlay generation is **retired**, not leaked.
    ///
    /// This is the falsifiable core of the ownership half of the change. It
    /// asserts both halves of the retirement condition separately, because
    /// either one alone would be a bug:
    ///
    /// * while a holder is still walking with the generation, eviction from
    ///   the table must **not** free it (a live analysis reading freed specs
    ///   is the failure mode the `&'static` was protecting against); and
    /// * once the table has evicted it *and* the last holder drops, the
    ///   allocation must actually go away.
    ///
    /// Driven end to end through the public door rather than by poking
    /// [`prune_overlays`] with a synthetic map: the claim under test is about
    /// what the cache does to a real generation, so the sweep is provoked the
    /// way a long pack-editing session provokes it — distinct overlay keys
    /// until the table crosses [`OVERLAY_LIMIT`].
    #[test]
    fn a_superseded_overlay_generation_is_retired() {
        const WATCHED: u64 = 0x5EED_0001;
        const FILL_BASE: u64 = 0x6000_0000;

        let profile = crate::model::ingress::resolve_environment("tcl").analyser_profile();

        // The plain entry for this profile, captured before anything is
        // swept, so we can prove the sweep left it exactly where it was.
        let plain = registry_handle_for_profile(profile);

        let generation = registry_for_profile_with_overlay(profile, WATCHED, |_| {});
        let watch = Arc::downgrade(&generation);
        assert!(
            registry_for_profile_if_built(profile, WATCHED).is_some(),
            "the generation must be indexed before it can be superseded"
        );

        // Supersede it: fresh pack contents, one distinct key at a time,
        // until the sweep fires and lets go of the watched one.
        let mut built = 0_u64;
        while registry_for_profile_if_built(profile, WATCHED).is_some() {
            built += 1;
            assert!(
                built < 4 * OVERLAY_LIMIT as u64,
                "the sweep never dropped the watched generation after {built} builds"
            );
            let _ = registry_for_profile_with_overlay(profile, FILL_BASE + built, |_| {});
        }

        // The table has let go. This test still holds the handle, and that
        // alone must keep the generation alive and readable.
        let alive = watch.upgrade().expect("a live holder must keep it alive");
        assert!(
            alive.get("set").is_some(),
            "an evicted-but-held generation must still be usable"
        );
        drop(alive);

        // Last holder finishes.
        drop(generation);
        assert!(
            watch.upgrade().is_none(),
            "a superseded generation outlived its last holder — it leaked"
        );

        // The plain axis is a closed set and the sweep retains it
        // unconditionally: same allocation, not a rebuild.
        let map = REGISTRIES.get_or_init(|| Mutex::new(FxHashMap::default()));
        let guard = map.lock().expect("registry cache mutex");
        let still = guard
            .get(&(profile.name, 0))
            .expect("the sweep evicted an un-overlaid entry");
        assert!(
            Arc::ptr_eq(still, &plain),
            "the un-overlaid entry was replaced rather than retained"
        );
    }
}
