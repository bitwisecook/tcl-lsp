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
//! packs" — and since the migration that is literally true: `sdc_base`, `upf` and the
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
//! is one directory scan plus ~4,700 lines of Tcl parsed at first use.
//!
//! ## The embedded fallback
//!
//! [`discovery::bundled_dir`](crate::discovery::bundled_dir)'s directory scan
//! stays the primary path — its doc comment explains why (the loader path
//! gets exercised in production, not reserved for private packs) — and every
//! *packaged* distribution still stages `specs/` beside the executable to
//! take it: the VSIX, the `JetBrains` `.zip`, the Sublime package. A bare
//! release binary has nothing beside it, though — `tcl-lsp-server-<triple>`,
//! `tcl-mcp-<triple>`, `tcl-<triple>`, `f5-query-<triple>` are standalone
//! executables, which is what the Zed extension downloads at runtime and
//! what every "point an LSP client at a binary" editor
//! (`INSTALL-editors.md`: Neovim, Emacs, Helix, Vim, Kate, Kakoune, …) is
//! told to fetch — so without a fallback none of those carry the EDA vendor
//! libraries at all: every `synth_design` et al. reports unknown. [`packs`]
//! falls back to [`load_embedded`] only when
//! [`bundled_dir`](crate::discovery::bundled_dir) finds no directory at
//! all; a `specs/` directory that is merely empty still counts as present
//! and is not overridden. Anything that stages `specs/` — a distribution,
//! `TCL_LSP_SPEC_PACK_DIR`, a dev checkout — wins outright, exactly as
//! before.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

use tcl_dialect::DialectProfile;
use tcl_registry::registry::CommandRegistry;

use crate::discovery::{DiscoveryOptions, Origin, PackFile, Tier, discover};
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

/// The eight shipped EDA loadables' `.tclspec` sources, compiled into the
/// binary — see "The embedded fallback" above for why this exists.
///
/// The same eight files `specs/` ships, by `include_str!` on the identical
/// path the crate's own tests resolve `specs/` from — not a hand-copied
/// second source that could drift from the on-disk one.
const EMBEDDED_PACKS: &[(&str, &str)] = &[
    (
        "eda_cadence.tclspec",
        include_str!("../../../specs/eda_cadence.tclspec"),
    ),
    (
        "eda_mentor.tclspec",
        include_str!("../../../specs/eda_mentor.tclspec"),
    ),
    (
        "eda_microchip.tclspec",
        include_str!("../../../specs/eda_microchip.tclspec"),
    ),
    (
        "eda_quartus.tclspec",
        include_str!("../../../specs/eda_quartus.tclspec"),
    ),
    (
        "eda_synopsys.tclspec",
        include_str!("../../../specs/eda_synopsys.tclspec"),
    ),
    (
        "eda_xilinx.tclspec",
        include_str!("../../../specs/eda_xilinx.tclspec"),
    ),
    (
        "sdc_base.tclspec",
        include_str!("../../../specs/sdc_base.tclspec"),
    ),
    ("upf.tclspec", include_str!("../../../specs/upf.tclspec")),
];

/// Load [`EMBEDDED_PACKS`] as the bundled tier.
///
/// Goes through the exact same [`crate::pack::load_sources`] merge that a
/// disk-backed load does — cache lookup, `speclib` grouping, tier-shadowing,
/// the content key — so an embedded pack set behaves identically to a
/// directory-backed one everywhere downstream; only where the bytes come
/// from differs. The synthetic `<embedded>/…` paths stand in for the file
/// each source would otherwise be read from — they never resolve on disk,
/// but every notice still needs *a* path to report against.
#[must_use]
pub fn load_embedded() -> PackSet {
    crate::pack::load_sources(embedded_sources(), Vec::new())
}

/// [`EMBEDDED_PACKS`] as loader sources.
///
/// The synthetic `<embedded>/…` paths never resolve on disk; they exist
/// because every notice needs *a* path to report against.
fn embedded_sources() -> Vec<(PackFile, String)> {
    EMBEDDED_PACKS
        .iter()
        .map(|(name, text)| {
            let file = PackFile {
                tier: Tier::Bundled,
                path: PathBuf::from(format!("<embedded>/{name}")),
                origin: Origin::Bundled,
            };
            (file, (*text).to_owned())
        })
        .collect()
}

/// Load a discovered file set, standing the embedded packs in for the bundled
/// tier when discovery found none on disk.
///
/// The entry point for any consumer that discovers packs itself — the LSP
/// server's workspace reload — rather than taking the process-wide [`packs`].
/// Such a consumer needs the workspace and user tiers that [`packs`] does not
/// look at, but it needs the bundled tier just as much, and a bare release
/// binary has no `specs/` directory beside it for
/// [`bundled_dir`](crate::discovery::bundled_dir) to find. Without this the
/// server would load an empty bundled tier and report every EDA command
/// unknown, while a `tcl-mcp` built from the same tree resolved them fine
/// through [`packs`] — the compiled-in EDA modules that used to cover the
/// difference are gone.
///
/// Falling back on "discovery produced no bundled-tier file" rather than on
/// `bundled_dir().is_none()` keeps the two in agreement when a directory
/// exists but is empty, and leaves a real on-disk `specs/` — a dev checkout,
/// a VSIX bundle, `TCL_LSP_SPEC_PACK_DIR` — authoritative when it has
/// anything in it.
#[must_use]
pub fn load_discovered(files: &[PackFile]) -> PackSet {
    let (mut sources, notices) = crate::pack::read_sources(files);
    if !sources.iter().any(|(file, _)| file.tier == Tier::Bundled) {
        sources.extend(embedded_sources());
    }
    crate::pack::load_sources(sources, notices)
}

/// [`packs`]'s resolution, factored out so a test can drive it with an
/// explicit directory instead of through the process-wide cache and the
/// ambient [`discovery::bundled_dir`](crate::discovery::bundled_dir).
fn resolve_packs(bundled_dir: Option<PathBuf>) -> PackSet {
    match bundled_dir {
        Some(dir) => load_from(&dir),
        None => load_embedded(),
    }
}

/// The shipped loadables, discovered and loaded once for the process.
///
/// Prefers a `specs/` directory
/// ([`bundled_dir`](crate::discovery::bundled_dir)) exactly as before;
/// falls back to [`load_embedded`] only when there is none at all — see
/// "The embedded fallback" above.
#[must_use]
pub fn packs() -> &'static PackSet {
    static PACKS: OnceLock<PackSet> = OnceLock::new();
    PACKS.get_or_init(|| resolve_packs(crate::discovery::bundled_dir()))
}

/// The workspace's loaded pack set, when a consumer has published one.
///
/// [`packs`] is the *bundled* tier only. A server that has discovered and
/// loaded the workspace and user tiers as well knows strictly more, and every
/// other consumer in the process should see the same specs it does — otherwise
/// the compiler explorer reports a pack's command unknown while the diagnostic
/// on the very same line resolves it.
static ACTIVE: RwLock<Option<Arc<PackSet>>> = RwLock::new(None);

/// Publish the pack set every consumer in this process should resolve against.
///
/// Called by the LSP server on each reload. A `None` set (or never calling
/// this) leaves consumers on [`packs`], which is what a bare CLI wants.
pub fn set_active(packs: Option<Arc<PackSet>>) {
    let mut active = match ACTIVE.write() {
        Ok(active) => active,
        Err(poisoned) => poisoned.into_inner(),
    };
    *active = packs;
}

/// The published workspace pack set, if any.
#[must_use]
pub fn active() -> Option<Arc<PackSet>> {
    match ACTIVE.read() {
        Ok(active) => active.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

/// The registry for `dialect` against the **active** pack set — the workspace's
/// when one has been published, the bundled loadables otherwise.
///
/// The drop-in for any consumer that wants what the workspace actually has
/// loaded rather than only what shipped: the compiler explorer, `tcl explore`,
/// and anything else that renders a pipeline the user expects to match their
/// diagnostics.
#[must_use]
pub fn active_registry_for_dialect(dialect: &str) -> Arc<CommandRegistry> {
    match active() {
        Some(packs) => registry_for_dialect_from(dialect, &packs),
        None => registry_for_dialect(dialect),
    }
}

/// The cached registry for `profile` with the shipped loadables installed.
///
/// Drop-in for [`tcl_registry::registry_for_profile`] in any consumer that
/// wants the vendor libraries the shipped packs carry — [`packs`] always has
/// something to install, a directory-backed set or the embedded fallback,
/// down to the same cached instance.
///
/// Handed back as an owning handle, like everything on the pack-carrying
/// axis: hold it while you read it, drop it when done, and a later pack edit
/// can retire this generation.
///
/// The bundled tier carries *all eight* EDA libraries regardless of dialect —
/// discovery cannot know which shell a document belongs to — and
/// [`install`](crate::install) is what narrows them to the packages this
/// profile ships ambient, so a Vivado registry never takes a Cadence
/// `report_timing`.
#[must_use]
pub fn registry_for_profile(profile: &'static DialectProfile) -> Arc<CommandRegistry> {
    crate::install::registry_with_packs(profile, packs())
}

/// [`registry_for_profile`] by dialect name — the drop-in for
/// [`tcl_registry::registry_for_dialect`].
#[must_use]
pub fn registry_for_dialect(dialect: &str) -> Arc<CommandRegistry> {
    registry_for_profile(crate::environment::profile_for_dialect(dialect))
}

/// [`registry_for_dialect`] against a pack set the caller supplies rather than
/// the one [`bundled_dir`](crate::discovery::bundled_dir) finds.
///
/// For a harness that knows where the loadables are — a test running out of a
/// source checkout — so it never depends on the process's ambient environment
/// or on where its own binary happens to live.
#[must_use]
pub fn registry_for_dialect_from(dialect: &str, all: &PackSet) -> Arc<CommandRegistry> {
    crate::install::registry_with_packs(crate::environment::profile_for_dialect(dialect), all)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
                "eda_microchip",
                "eda_quartus",
                "eda_synopsys",
                "eda_xilinx",
                "sdc_base",
                "upf"
            ],
            "the eight EDA loadables"
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

    /// The standalone-binary scenario this fallback exists for: no
    /// `specs/` directory anywhere `bundled_dir` would find one — a bare
    /// `tcl-lsp-server-<triple>` release asset, or the Zed extension's
    /// runtime download of it.
    #[test]
    fn the_embedded_packs_carry_the_same_eight_eda_vendor_libraries() {
        let _cache = cache_guard();
        let set = load_embedded();
        let mut names: Vec<&str> = set.packs.iter().map(|p| p.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            vec![
                "eda_cadence",
                "eda_mentor",
                "eda_microchip",
                "eda_quartus",
                "eda_synopsys",
                "eda_xilinx",
                "sdc_base",
                "upf"
            ],
            "the eight EDA loadables, embedded"
        );
        let warnings: Vec<String> = set
            .notices
            .iter()
            .filter(|n| n.severity == crate::pack::Severity::Warning)
            .map(|n| format!("{}:{} {}", n.path.display(), n.line, n.message))
            .collect();
        assert!(
            warnings.is_empty(),
            "the embedded packs must load clean:\n{}",
            warnings.join("\n")
        );
    }

    /// The embedded copy is `include_str!`'d from the exact same `specs/`
    /// files the on-disk tier loads — this catches a wrong relative path or
    /// a stale embed before it ships silently degraded packs.
    #[test]
    fn the_embedded_packs_are_command_for_command_identical_to_the_on_disk_copy() {
        let _cache = cache_guard();
        let on_disk = load_from(&repo_specs());
        let embedded = load_embedded();
        // The content key folds in each source's path, and the embedded
        // fallback deliberately uses synthetic `<embedded>/…` paths (so a
        // notice always has *something* to report against) — so the keys
        // differ by design even though the parsed commands below are
        // identical. What must match is the command set each source
        // produces, not the key.

        let mut on_disk_packs = on_disk.packs.clone();
        let mut embedded_packs = embedded.packs.clone();
        on_disk_packs.sort_by(|a, b| a.name.cmp(&b.name));
        embedded_packs.sort_by(|a, b| a.name.cmp(&b.name));
        for (disk_pack, emb_pack) in on_disk_packs.iter().zip(embedded_packs.iter()) {
            assert_eq!(disk_pack.name, emb_pack.name);
            let disk_names: Vec<&str> = disk_pack.commands.iter().map(|c| c.spec.name).collect();
            let emb_names: Vec<&str> = emb_pack.commands.iter().map(|c| c.spec.name).collect();
            assert_eq!(
                disk_names, emb_names,
                "pack `{}` command set must match between on-disk and embedded",
                disk_pack.name
            );
        }
    }

    /// [`packs`]'s override contract: a `specs/` directory, when present,
    /// always wins over the embedded fallback — a distribution, a dev
    /// checkout, or `TCL_LSP_SPEC_PACK_DIR` must be able to override or
    /// replace the shipped loadables, not merely add to them.
    #[test]
    fn an_on_disk_bundled_dir_wins_over_the_embedded_fallback() {
        let _cache = cache_guard();
        let set = resolve_packs(Some(repo_specs()));
        assert!(!set.is_empty());
        assert!(
            set.packs
                .iter()
                .all(|p| p.files.iter().all(|f| !f.starts_with("<embedded>"))),
            "an on-disk bundled dir must never resolve to the embedded paths: {:#?}",
            set.packs
        );
    }

    /// The other half of the same contract: with no on-disk directory at
    /// all, [`packs`] falls back to the embedded copy rather than shipping
    /// an empty set — the bug this fallback exists to close.
    #[test]
    fn no_bundled_dir_falls_back_to_the_embedded_packs() {
        let _cache = cache_guard();
        let set = resolve_packs(None);
        assert!(!set.is_empty(), "the embedded fallback must not be empty");
        assert!(
            set.packs
                .iter()
                .all(|p| p.files.iter().all(|f| f.starts_with("<embedded>"))),
            "with no bundled dir, every pack must resolve from the embedded paths: {:#?}",
            set.packs
        );
    }

    /// A private scratch directory for the two `load_discovered` tests, in
    /// the crate's own temp-dir idiom (`discovery`'s `tmpdir`) rather than a
    /// new dev-dependency.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tcl-spectcl-bundled-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    /// A workspace load — the LSP server's path, which discovers its own
    /// files rather than taking [`packs`] — still gets the bundled tier when
    /// there is no `specs/` directory to discover it from. This is the
    /// standalone-binary case for the *server* specifically: it was loading
    /// exactly what discovery returned, so a bare `tcl-lsp-server` release
    /// (Zed, the standalone-editor instructions) had no EDA commands at all
    /// once the compiled-in modules were deleted.
    #[test]
    fn a_discovered_load_with_no_bundled_tier_falls_back_to_the_embedded_packs() {
        let _cache = cache_guard();
        let workspace = scratch("discovered").join("pack.tclspec");
        std::fs::write(&workspace, "speclib demo 1 {}\n").expect("write pack");

        let files = vec![PackFile {
            tier: Tier::Workspace,
            path: workspace.clone(),
            origin: Origin::Setting,
        }];
        let set = load_discovered(&files);

        assert!(
            set.packs
                .iter()
                .any(|p| p.files.iter().any(|f| f.starts_with("<embedded>"))),
            "a discovered set with no bundled tier must gain the embedded packs: {:#?}",
            set.packs
        );
        assert!(
            set.packs.iter().any(|p| p.files.contains(&workspace)),
            "the discovered workspace pack must survive the fallback: {:#?}",
            set.packs
        );
    }

    /// The converse: a real on-disk bundled tier stays authoritative, so a
    /// dev checkout or a VSIX bundle never loads each shipped pack twice.
    #[test]
    fn a_discovered_bundled_tier_suppresses_the_embedded_fallback() {
        let _cache = cache_guard();
        let bundled = scratch("on-disk").join("eda_demo.tclspec");
        std::fs::write(&bundled, "speclib demo 1 {}\n").expect("write pack");

        let set = load_discovered(&[PackFile {
            tier: Tier::Bundled,
            path: bundled,
            origin: Origin::Bundled,
        }]);

        assert!(
            set.packs
                .iter()
                .all(|p| p.files.iter().all(|f| !f.starts_with("<embedded>"))),
            "an on-disk bundled tier must suppress the embedded fallback: {:#?}",
            set.packs
        );
    }

    /// The end-to-end proof this whole module exists for: a **standalone
    /// install** — a bare `tcl-lsp-server-<triple>` / `tcl-mcp-<triple>` /
    /// `tcl-<triple>` / `f5-query-<triple>` binary with no `specs/` beside
    /// it anywhere, i.e. `resolve_packs(None)` — resolves an EDA vendor
    /// command through the normal registry path, not just as bytes parsed
    /// into a [`PackSet`]. Before this fallback existed, `registry.get(
    /// "synth_design")` was `None` for exactly this scenario: every
    /// `synth_design` (and the other 370 EDA commands) reported unknown for
    /// the Zed extension's runtime download and every "point an LSP client
    /// at a binary" editor in `INSTALL-editors.md`.
    #[test]
    fn a_standalone_install_with_no_specs_directory_resolves_an_eda_command() {
        let _cache = cache_guard();
        let standalone = resolve_packs(None);
        let profile = crate::environment::profile_for_dialect("xilinx-eda-tcl");
        let registry = crate::install::registry_with_packs(profile, &standalone);

        let command = registry
            .get("synth_design")
            .expect("a standalone install must resolve the Xilinx `synth_design` command");
        assert_eq!(command.name, "synth_design");
    }
}
