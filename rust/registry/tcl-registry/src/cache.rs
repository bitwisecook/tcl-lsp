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

use crate::dialects::DialectSet;
use crate::registry::CommandRegistry;

/// Return the cached registry for `dialect`, building it on first use.
pub fn registry_for_dialect(dialect: &str) -> &'static CommandRegistry {
    static REGISTRIES: OnceLock<Mutex<FxHashMap<String, &'static CommandRegistry>>> =
        OnceLock::new();

    let parsed = DialectSet::parse(dialect);
    // Canonicalise the cache key: parseable dialects keep their string;
    // unparseable ones share the plain-Tcl entry.
    let key = if parsed.is_some() { dialect } else { "" };

    let map = REGISTRIES.get_or_init(|| Mutex::new(FxHashMap::default()));
    let mut guard = map.lock().expect("registry cache mutex");
    if let Some(r) = guard.get(key) {
        return r;
    }

    let mut registry = CommandRegistry::build_default();
    if let Some(d) = parsed {
        registry.load_dialect(d);
    }
    let leaked: &'static CommandRegistry = Box::leak(Box::new(registry));
    guard.insert(key.to_owned(), leaked);
    leaked
}
