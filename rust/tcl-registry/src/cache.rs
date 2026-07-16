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

/// Return the cached registry for `profile`, building it on first use.
///
/// The registry is the default build plus the profile's
/// [`base_layers`](DialectProfile::base_layers) command packs, keyed by the
/// profile's canonical name — so aliases share their canonical entry and
/// every unknown dialect string shares the one `PLAIN_TCL` entry.
pub fn registry_for_profile(profile: &'static DialectProfile) -> &'static CommandRegistry {
    static REGISTRIES: OnceLock<Mutex<FxHashMap<&'static str, &'static CommandRegistry>>> =
        OnceLock::new();

    let map = REGISTRIES.get_or_init(|| Mutex::new(FxHashMap::default()));
    let mut guard = map.lock().expect("registry cache mutex");
    if let Some(r) = guard.get(profile.name) {
        return r;
    }

    let mut registry = CommandRegistry::build_default();
    for &layer in profile.base_layers {
        registry.load_dialect(layer);
    }
    let leaked: &'static CommandRegistry = Box::leak(Box::new(registry));
    guard.insert(profile.name, leaked);
    leaked
}

/// Return the cached registry for `dialect`, building it on first use.
///
/// String-keyed convenience over [`registry_for_profile`]: the name is
/// resolved through [`DialectProfile::by_name`], so a stream of typos
/// cannot leak one registry per typo (they all share the plain-Tcl entry).
#[must_use]
pub fn registry_for_dialect(dialect: &str) -> &'static CommandRegistry {
    registry_for_profile(DialectProfile::by_name(dialect))
}
