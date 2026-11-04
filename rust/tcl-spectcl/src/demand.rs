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

//! **Which bundled packs this process needs** — the lazy half of the bundled
//! tier.
//!
//! A bundled pack whose every command requires a closed-world vendor package
//! (`vivado`, `sdc`, `upf`, …) can only ever answer in an environment that
//! places one of those packages ambient: in any other document its commands
//! are gated out at install. Evaluating it is the bulk of a cold load (the
//! eight EDA packs are 36k lines), so such a pack loads **manifest-only** —
//! its environments, extension routes, ambient rows and rosters, no
//! commands — until an environment that places one of its packages is seen.
//!
//! The demand set is the union of the ambient packages of every environment
//! [`note_dialect`] has been told about. It only grows: a pack, once
//! evaluated, stays in the set for the life of the process, which keeps the
//! pack-set key monotone and a document's answers stable.
//!
//! Which bundled packs are eligible, and which packages each needs, is
//! precompiled ([`crate::bundled_manifest`]) and checked against the source's
//! hash, so a distribution that replaces `specs/` loads its packs eagerly
//! rather than trusting a table built from other bytes.
//! Deferral is opt-in per process ([`enable`]);
//! `TCL_LSP_EAGER_SPEC_PACKS=1` turns it off again.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{OnceLock, PoisonError, RwLock};

/// The environment variable that loads every bundled pack eagerly.
pub const EAGER_ENV: &str = "TCL_LSP_EAGER_SPEC_PACKS";

static DEMAND: RwLock<BTreeSet<&'static str>> = RwLock::new(BTreeSet::new());
static GENERATION: AtomicU64 = AtomicU64::new(0);
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Turn deferral on for this process. A consumer that records what its inputs
/// need, such as the CLI, calls this before its first load. Other loaders keep
/// evaluating every bundled pack.
pub fn enable() {
    ENABLED.store(true, Ordering::Release);
}

/// Whether deferral is off for this process.
#[must_use]
pub fn eager() -> bool {
    static EAGER: OnceLock<bool> = OnceLock::new();
    *EAGER.get_or_init(|| std::env::var_os(EAGER_ENV).is_some_and(|value| value == "1"))
}

/// The packages `dialect`'s environment places ambient, plus its vendor
/// surface package.
fn ambient_packages(dialect: &str) -> Vec<&'static str> {
    let environment = tcl_registry::model::resolve_environment(dialect);
    let mut packages: Vec<&'static str> = environment
        .definition
        .expected_packages
        .iter()
        .filter(|placement| placement.ambient)
        .map(|placement| crate::loader::leak_str(&placement.package))
        .collect();
    if let Some(package) =
        tcl_registry::model::surface::vendor_surface_package(environment.definition.id.as_str())
    {
        packages.push(package);
    }
    packages
}

/// Record that a document in `dialect` is open. `true` when this added a
/// package some deferred bundled pack needs — the caller's cue to reload the
/// pack set, which will now evaluate that pack in full.
pub fn note_dialect(dialect: &str) -> bool {
    if eager() {
        return false;
    }
    let packages = ambient_packages(dialect);
    if packages.is_empty() {
        return false;
    }
    {
        let demand = DEMAND.read().unwrap_or_else(PoisonError::into_inner);
        if packages.iter().all(|package| demand.contains(package)) {
            return false;
        }
    }
    let wanted = deferrable_packages();
    let mut demand = DEMAND.write().unwrap_or_else(PoisonError::into_inner);
    let mut grew = false;
    for package in packages {
        if demand.insert(package) && wanted.contains(&package) {
            grew = true;
        }
    }
    if grew {
        GENERATION.fetch_add(1, Ordering::Release);
    }
    grew
}

/// Whether a pack needing any of `packages` is wanted now.
#[must_use]
pub fn wants_any(packages: &[&str]) -> bool {
    eager() || {
        let demand = DEMAND.read().unwrap_or_else(PoisonError::into_inner);
        packages.iter().any(|package| demand.contains(package))
    }
}

/// Moves whenever [`note_dialect`] widens what a load must evaluate.
#[must_use]
pub fn generation() -> u64 {
    GENERATION.load(Ordering::Acquire)
}

/// Forget every recorded demand, for tests that need a cold process.
#[doc(hidden)]
pub fn reset_for_tests() {
    DEMAND
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    GENERATION.fetch_add(1, Ordering::Release);
}

/// The hash a bundled manifest row records for its source, and the one a
/// load compares against: `xxh3` of the exact source bytes. Line endings can
/// occur inside literal values, so a translated checkout loads eagerly.
#[must_use]
pub fn source_hash(source: &str) -> u64 {
    xxhash_rust::xxh3::xxh3_64(source.as_bytes())
}

/// Every package some deferrable bundled pack needs.
fn deferrable_packages() -> &'static BTreeSet<&'static str> {
    static PACKAGES: OnceLock<BTreeSet<&'static str>> = OnceLock::new();
    PACKAGES.get_or_init(|| {
        crate::bundled_manifest::BUNDLED_MANIFESTS
            .iter()
            .flat_map(|row| row.packages.iter().copied())
            .collect()
    })
}

/// The precompiled manifest of the bundled pack shipped as `file_name` with
/// exactly these bytes, when no recorded demand needs it yet — the cue to load
/// it manifest-only.
#[must_use]
pub fn deferred(
    file_name: &str,
    source: &str,
) -> Option<&'static crate::bundled_manifest::BundledManifest> {
    if eager() || !ENABLED.load(Ordering::Acquire) {
        return None;
    }
    let row = crate::bundled_manifest::BUNDLED_MANIFESTS
        .iter()
        .find(|row| row.file == file_name)?;
    (row.source_hash == source_hash(source) && !wants_any(row.packages)).then_some(row)
}
