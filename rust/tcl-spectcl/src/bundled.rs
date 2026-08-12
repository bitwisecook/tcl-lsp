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
use std::sync::OnceLock;

use tcl_dialect::DialectProfile;
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

/// The cached registry for `profile` with the shipped loadables installed.
///
/// Drop-in for [`tcl_registry::registry_for_profile`] in any consumer that
/// wants the vendor libraries the shipped packs carry. With no bundled
/// directory present the pack set is empty and this *is*
/// `registry_for_profile`, down to the same `&'static`.
///
/// The bundled tier carries *all six* EDA libraries regardless of dialect —
/// discovery cannot know which shell a document belongs to — and
/// [`install`](crate::install) is what narrows them to the packages this
/// profile ships ambient, so a Vivado registry never takes a Cadence
/// `report_timing`.
#[must_use]
pub fn registry_for_profile(profile: &'static DialectProfile) -> &'static CommandRegistry {
    crate::install::registry_with_packs(profile, packs())
}

/// [`registry_for_profile`] by dialect name — the drop-in for
/// [`tcl_registry::registry_for_dialect`].
#[must_use]
pub fn registry_for_dialect(dialect: &str) -> &'static CommandRegistry {
    registry_for_profile(DialectProfile::by_name(dialect))
}

/// [`registry_for_dialect`] against a pack set the caller supplies rather than
/// the one [`bundled_dir`](crate::discovery::bundled_dir) finds.
///
/// For a harness that knows where the loadables are — a test running out of a
/// source checkout — so it never depends on the process's ambient environment
/// or on where its own binary happens to live.
#[must_use]
pub fn registry_for_dialect_from(dialect: &str, all: &PackSet) -> &'static CommandRegistry {
    crate::install::registry_with_packs(DialectProfile::by_name(dialect), all)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The repository's `specs/` directory, which is what a release lays down
    /// beside the executable.
    /// Loading a pack writes a compiled-cache entry, and `cache`'s own tests
    /// count the entries under a redirected directory — so every test in this
    /// crate that loads a pack holds the same lock they do.
    fn cache_guard() -> std::sync::MutexGuard<'static, ()> {
        crate::cache::REDIRECT_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn repo_specs() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../specs")
    }

    #[test]
    fn the_shipped_loadables_carry_the_eda_vendor_libraries() {
        let _cache = cache_guard();
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
        let _cache = cache_guard();
        let set = load_from(Path::new("/no/such/bundled/dir"));
        assert!(set.is_empty());
        assert_eq!(set.key, 0);
    }
}
