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
//! Each canonical dialect gets one lazily-built, cached `&'static`
//! registry so the per-call build cost is paid once. Unparseable
//! dialect strings collapse to the plain-Tcl entry so a stream of typos
//! cannot leak one registry per typo.
//!
//! Lives in `tcl-registry` (not a consumer crate) so every downstream
//! tool — the CLI, the compiler explorer, future MCP/AI surfaces — shares
//! one cache rather than each rebuilding its own.

use std::sync::{Mutex, OnceLock};

use rustc_hash::FxHashMap;
use tcl_dialect::DialectProfile;

use crate::registry::CommandRegistry;

/// Every `(profile, overlay)` registry built so far.
///
/// Module-level rather than function-local because two entry points read it:
/// [`registry_for_profile_with_overlay`], which builds a missing entry, and
/// [`registry_for_profile_if_built`], which deliberately does not.
static REGISTRIES: OnceLock<Mutex<FxHashMap<(&'static str, u64), &'static CommandRegistry>>> =
    OnceLock::new();

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

/// Return the cached registry for `profile`, building it on first use.
///
/// The registry is the default build plus the profile's
/// [`base_layers`](DialectProfile::base_layers) command packs, keyed by the
/// profile's canonical name — so aliases share their canonical entry and
/// every unknown dialect string shares the one `PLAIN_TCL` entry.
#[must_use]
pub fn registry_for_profile(profile: &'static DialectProfile) -> &'static CommandRegistry {
    registry_for_profile_with_overlay(profile, 0, |_| {})
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
pub fn registry_for_profile_if_built(
    profile: &'static DialectProfile,
    overlay: u64,
) -> Option<&'static CommandRegistry> {
    if overlay == 0 {
        return Some(registry_for_profile(profile));
    }
    let map = REGISTRIES.get_or_init(|| Mutex::new(FxHashMap::default()));
    let guard = map.lock().expect("registry cache mutex");
    guard.get(&(profile.name, overlay)).copied()
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
/// Like every entry here, an overlaid registry is **leaked for the process
/// lifetime**: consumers hold `&'static CommandRegistry` across threads and
/// requests, so there is nothing to free it against. That is the design's
/// stated model ("interned and leaked once, keyed by content hash"), and it is
/// fine for the profile axis, which is bounded by the dialect catalogue. It is
/// worth naming for the overlay axis, which is bounded by *user edits*: each
/// distinct pack-set content the server sees costs one more resident registry
/// until restart. Saving the same pack twice is free; editing it a hundred
/// times is a hundred registries. [`OVERLAY_LIMIT`] caps how many the lookup
/// table indexes, so lookup stays fast and the growth is visible in one place,
/// but eviction from the table does not reclaim the registry.
pub fn registry_for_profile_with_overlay(
    profile: &'static DialectProfile,
    overlay: u64,
    extend: impl FnOnce(&mut CommandRegistry),
) -> &'static CommandRegistry {
    let map = REGISTRIES.get_or_init(|| Mutex::new(FxHashMap::default()));
    let mut guard = map.lock().expect("registry cache mutex");
    let key = (profile.name, overlay);
    if let Some(r) = guard.get(&key) {
        return r;
    }

    let mut registry = CommandRegistry::build_default();
    for &layer in profile.base_layers {
        registry.load_dialect(layer);
    }
    extend(&mut registry);
    registry.set_profile(profile);
    let leaked: &'static CommandRegistry = Box::leak(Box::new(registry));

    // Keep every un-overlaid entry (one per dialect profile, a closed set) and
    // drop the oldest-indexed overlays past the cap.  Purely a table bound —
    // see the memory note above.
    if guard.len() >= OVERLAY_LIMIT {
        guard.retain(|(_, overlay), _| *overlay == 0);
    }
    guard.insert(key, leaked);
    leaked
}

/// How many `(profile, overlay)` entries the lookup table indexes before it
/// drops the overlaid ones. Generous: a workspace has one pack set at a time,
/// so reaching this at all means a long editing session on a pack.
const OVERLAY_LIMIT: usize = 64;

/// Return the cached registry for `dialect`, building it on first use.
///
/// String-keyed convenience over [`registry_for_profile`]: the name is
/// resolved through [`DialectProfile::by_name`], so a stream of typos
/// cannot leak one registry per typo (they all share the plain-Tcl entry).
#[must_use]
pub fn registry_for_dialect(dialect: &str) -> &'static CommandRegistry {
    registry_for_profile(DialectProfile::by_name(dialect))
}
