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

//! The **bundled** tier on its own: the loadables tcl-lsp ships.
//!
//! `docs/design/spec-packs.md` puts the EDA vendor libraries here — "the EDA
//! vendor libraries ship as bundled `.tclspec` loadables … so the loader path
//! is exercised in production from day one rather than reserved for private
//! packs" — and since the migration that is literally true: `sdc_base` and the
//! five vendor packs have no Rust modules behind them at all, so a `get_cells`
//! or a `synth_design` reaches a registry only by way of [`crate::loader`].
//!
//! The LSP server does not need this module: it discovers all three tiers
//! together ([`crate::discover`]) and installs the merged set. This is for
//! every *other* consumer — the `tcl` CLI, `f5-query`, `tcl-mcp`, a test
//! harness — which has no workspace and no `tclLsp.specPacks`, and simply
//! wants the registry a dialect is supposed to have. Those callers used to get
//! the EDA packs for free from `CommandRegistry::load_eda_packs`; they get them
//! from here now, and the discovery, parse, merge and install they go through
//! is the same code the server runs.
//!
//! Loading is done **once per process** and the result is cached, so the cost
//! is one directory scan plus ~3,700 lines of Tcl parsed at first use.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use rustc_hash::FxHashMap;
use tcl_dialect::DialectProfile;
use tcl_registry::profile_queries::ProfileQueries;
use tcl_registry::registry::CommandRegistry;

use crate::discovery::{DiscoveryOptions, discover};
use crate::pack::PackSet;

/// Load the packs in `dir` as the bundled tier, ignoring every other tier.
///
/// Explicit rather than ambient so a caller that knows where the loadables are
/// — a test running from a source checkout, a distribution laying them out
/// itself — never has to mutate process environment to say so.
#[must_use]
pub fn load_from(dir: &Path) -> PackSet {
    let files = discover(&DiscoveryOptions {
        bundled_dir: Some(dir.to_path_buf()),
        skip_user_tier: true,
        ..DiscoveryOptions::default()
    });
    crate::pack::load(&files)
}

/// The shipped loadables, discovered and loaded once for the process.
///
/// Empty — not an error — when the bundled directory is absent, which is what
/// a stripped-down install or a bare `cargo run` from a checkout looks like.
#[must_use]
pub fn packs() -> &'static PackSet {
    static PACKS: OnceLock<PackSet> = OnceLock::new();
    PACKS.get_or_init(|| match crate::discovery::bundled_dir() {
        Some(dir) => load_from(&dir),
        None => PackSet::default(),
    })
}

/// The bundled directory this process will read, for a diagnostic or a log
/// line that needs to say *why* an EDA command did not resolve.
#[must_use]
pub fn dir() -> Option<PathBuf> {
    crate::discovery::bundled_dir()
}

/// The shipped loadables **this profile ships**, cached per profile.
///
/// Discovery is profile-blind — every `.tclspec` in the bundled directory is
/// found for every dialect — but the vendor libraries are not: an EDA shell
/// ships `sdc` plus its own tool packages ambient and no rival's
/// (`docs/design/eda-library-packages.md`). Without this filter the six packs
/// would all install into every profile, and since four of them declare a
/// `report_timing`, the Vivado profile's `report_timing` would be whichever
/// vendor's pack sorted first — a command that then fails
/// [`ProfileQueries::is_available`] and resolves to nothing at all.
///
/// Filtering is per **pack**, because a pack is one vendor's library: a pack
/// is kept when the profile ships any of its packages ambient. That is
/// precisely the set `CommandRegistry::load_eda_packs` used to insert for the
/// profile, so the registry a profile ends up with is the one it always had.
fn packs_for(profile: &'static DialectProfile) -> &'static PackSet {
    static BY_PROFILE: OnceLock<Mutex<FxHashMap<&'static str, &'static PackSet>>> = OnceLock::new();
    let map = BY_PROFILE.get_or_init(|| Mutex::new(FxHashMap::default()));
    let mut guard = map.lock().expect("bundled pack cache mutex");
    if let Some(set) = guard.get(profile.name) {
        return set;
    }
    let all = packs();
    let kept: Vec<crate::pack::MergedPack> = all
        .packs
        .iter()
        .filter(|pack| {
            pack.commands
                .iter()
                .any(|command| profile.package_available(command.spec.required_package))
        })
        .cloned()
        .collect();
    // `key` is left as the whole set's: the registry cache is keyed on
    // `(profile, key)` already, so two profiles holding different subsets
    // under the same key still get different entries.
    let set: &'static PackSet = Box::leak(Box::new(PackSet {
        packs: kept,
        notices: Vec::new(),
        key: all.key,
    }));
    guard.insert(profile.name, set);
    set
}

/// The cached registry for `profile` with the shipped loadables installed.
///
/// Drop-in for [`tcl_registry::registry_for_profile`] in any consumer that
/// wants the vendor libraries the shipped packs carry. With no bundled
/// directory present the pack set is empty and this *is*
/// `registry_for_profile`, down to the same `&'static`.
#[must_use]
pub fn registry_for_profile(profile: &'static DialectProfile) -> &'static CommandRegistry {
    crate::install::registry_with_packs(profile, packs_for(profile))
}

/// [`registry_for_profile`] by dialect name — the drop-in for
/// [`tcl_registry::registry_for_dialect`].
#[must_use]
pub fn registry_for_dialect(dialect: &str) -> &'static CommandRegistry {
    registry_for_profile(DialectProfile::by_name(dialect))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The repository's `specs/` directory, which is what a release lays down
    /// beside the executable.
    fn repo_specs() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../specs")
    }

    #[test]
    fn the_shipped_loadables_carry_the_eda_vendor_libraries() {
        let set = load_from(&repo_specs());
        let mut names: Vec<&str> = set.packs.iter().map(|p| p.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            vec![
                "eda_cadence",
                "eda_mentor",
                "eda_quartus",
                "eda_synopsys",
                "eda_xilinx",
                "sdc_base"
            ],
            "the six EDA loadables"
        );
        let warnings: Vec<String> = set
            .notices
            .iter()
            .filter(|n| n.severity == crate::pack::Severity::Warning)
            .map(|n| format!("{}:{} {}", n.path.display(), n.line, n.message))
            .collect();
        assert!(
            warnings.is_empty(),
            "the shipped packs must load clean:\n{}",
            warnings.join("\n")
        );
    }

    #[test]
    fn a_missing_bundled_directory_is_an_empty_set_not_an_error() {
        let set = load_from(Path::new("/no/such/bundled/dir"));
        assert!(set.is_empty());
        assert_eq!(set.key, 0);
    }
}
