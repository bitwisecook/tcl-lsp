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

//! Where `.tclspec` files come from: the three discovery tiers.
//!
//! `docs/design/registry/spec-packs.md` fixes both the tiers and their order —
//! **nearest wins: workspace > user > bundled**:
//!
//! - **workspace** — the `tclLsp.specPacks` setting (mirrored in every editor
//!   integration), plus `*.tclspec` under a `.tcl-lsp/` directory or beside a
//!   `tclpkg.tcl` manifest. A manifest with a `spec` directive names its own
//!   packs, and those are the packs beside it; one without keeps every
//!   `*.tclspec` under its directory ([`crate::package_specs`]).
//! - **user** — packs dropped in the platform config directory
//!   (`$XDG_CONFIG_HOME/tcl-lsp/specs/` and the macOS / Windows equivalents),
//!   loaded for every workspace. The directory comes from [`tcl_userdirs`],
//!   which is the same machinery `tcl pkg` reads its own config layer from.
//! - **bundled** — the shipped loadables (the EDA vendor libraries), so the
//!   loader path is exercised in production rather than reserved for private
//!   packs.
//!
//! Discovery answers *which files*, in a deterministic order. It never reads
//! or parses the pack files — deciding which file belongs to which pack, and
//! which pack wins, is [`crate::pack`]'s job, because that needs each file's
//! `speclib` name and therefore a parse. The one thing it reads beside them
//! is the package metadata that says how far the package shipping a file sits
//! from the workspace root ([`PackFile::dependency_tier`]), and the `spec`
//! directive that says which files beside it are the package's packs.
//!
//! ## Determinism
//!
//! Every directory scan sorts its results by path before appending, so two
//! runs over the same tree produce the same list in the same order on every
//! platform. That matters beyond tidiness: the merge order of a multi-file
//! pack *is* this order, and so is the compiled-cache key.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use tcl_dialect::model::{DependencyTier, WorkspaceTrust};
use tcl_lsp_core::vfs::{NativeStore, SourceStore};
use tcl_pkg_model::lockfile::{LockFile, deserialise};
use tcl_pkg_model::manifest::{ManifestAst, SpecDirective, load_manifest_text};
use tcl_pkg_model::tier::{clamp_requested, dependency_tier};

use crate::PACK_EXTENSION;

/// Which discovery tier a file came from. `Ord` is the precedence order —
/// [`Tier::StudioOverride`] is the nearest and lowest, so a plain sort puts the
/// winner first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// Editor-hosted Studio metadata under `.tcl-lsp/.spec-studio/`.
    /// Actual session admission is separate from this precedence tier.
    StudioOverride,
    /// The `tclLsp.specPacks` setting, `.tcl-lsp/`, or beside a `tclpkg.tcl`.
    Workspace,
    /// The per-user platform config directory, loaded for every workspace.
    User,
    /// Shipped with the server.
    Bundled,
}

impl Tier {
    /// The tier's name as it appears in a notice or a log line.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Tier::StudioOverride => "Spec Studio override",
            Tier::Workspace => "workspace",
            Tier::User => "user",
            Tier::Bundled => "bundled",
        }
    }

    /// The trust a file of this tier loads under when the editor's state for
    /// the workspace is `workspace`: the workspace tier takes it, and every
    /// other tier is [`WorkspaceTrust::Trusted`] — the bundled and user tiers
    /// are not the workspace's to vouch for, and a Spec Studio override is
    /// untrusted by its own provenance whatever the editor says. One
    /// normalisation, so a snapshot key and a merged pack never carry a trust
    /// state their provenance does not read.
    #[must_use]
    pub fn trust_under(self, workspace: WorkspaceTrust) -> WorkspaceTrust {
        match self {
            Tier::Workspace => workspace,
            Tier::StudioOverride | Tier::User | Tier::Bundled => WorkspaceTrust::Trusted,
        }
    }
}

/// Why a file was discovered — kept so a notice can say *which* rule pulled a
/// pack in, which is the first question when an unexpected pack loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Origin {
    /// Materialised by an editor-hosted Spec Studio session.
    StudioOverride,
    /// Named by, or found under a directory named by, `tclLsp.specPacks`.
    Setting,
    /// Found under a `.tcl-lsp/` directory in a workspace folder.
    DotDir,
    /// Found beside a `tclpkg.tcl` package manifest.
    BesideManifest,
    /// Found in the per-user config directory.
    UserDir,
    /// Shipped with the server.
    Bundled,
    /// Supplied by the host under [`VIRTUAL_PACK_MOUNT`], because there is no
    /// executable to sit beside.
    HostMount,
}

impl Origin {
    /// The origin's name as it appears in a notice or a log line.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Origin::StudioOverride => ".tcl-lsp/.spec-studio/",
            Origin::Setting => "tclLsp.specPacks",
            Origin::DotDir => ".tcl-lsp/",
            Origin::BesideManifest => "beside tclpkg.tcl",
            Origin::UserDir => "user config dir",
            Origin::Bundled => "bundled",
            Origin::HostMount => "host spec-pack mount",
        }
    }
}

/// One discovered `.tclspec` file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackFile {
    /// Tier first so a sort orders by precedence, then by path so a
    /// multi-file pack merges in sorted path order.
    pub tier: Tier,
    /// Absolute (or as-configured) path to the file.
    pub path: PathBuf,
    /// The rule that pulled the file in.
    pub origin: Origin,
    /// How far the package that ships this file sits from the workspace
    /// root, read from the lockfile of the project it belongs to.
    ///
    /// `Some` for every file found beside a `tclpkg.tcl` in a project that
    /// has a `tclpkg.lock`: the project's own package is the root, and any
    /// other is what the lockfile's graph gives it, or transitive when the
    /// graph does not place it (an unlisted package). A dependency whose
    /// manifest, or whose project's lockfile, does not read is not selected
    /// at all. `None` for every other file — a pack no package
    /// ships, and a file in a workspace with no project lockfile — and a pack
    /// with no tier is not narrowed by
    /// [`tcl_registry::model::CodegenCapability`]. Last, so a sort orders by
    /// precedence and path first.
    pub dependency_tier: Option<DependencyTier>,
}

/// The directory a workspace folder keeps its own packs in.
pub const WORKSPACE_PACK_DIR: &str = ".tcl-lsp";

/// Hidden session directory used by native editor-hosted Spec Studio panels.
pub const STUDIO_OVERRIDE_DIR: &str = ".spec-studio";

/// The package manifest whose directory is scanned for sibling packs.
pub const PACKAGE_MANIFEST: &str = "tclpkg.tcl";

/// The lockfile beside a project's root manifest: the resolved dependency
/// graph a package's [`DependencyTier`] is read from.
pub const PACKAGE_LOCKFILE: &str = "tclpkg.lock";

/// The subdirectory of the platform config directory holding user packs.
pub const USER_PACK_SUBDIR: &str = "specs";

/// Environment override for the bundled-pack directory, so a distribution
/// that installs the loadables somewhere unusual — and every test — can point
/// the bundled tier at a known place without a rebuild.
pub const BUNDLED_DIR_ENV: &str = "TCL_LSP_SPEC_PACK_DIR";

/// The directory the bundled tier is read from when there is no executable to
/// sit beside — the contract between a host that has `.tclspec` bytes and the
/// server that loads them.
///
/// [`bundled_dir`] answers "the `specs/` directory next to the running
/// binary", which is meaningless in a browser worker: there is no executable,
/// no `specs/`, and no filesystem to hold either. A host that wants its own
/// packs loaded instead upserts them into the server's
/// [`SourceStore`](tcl_lsp_core::vfs::SourceStore) under this prefix before
/// (or during) the session — `<mount>/vendor.tclspec`,
/// `<mount>/eda/xilinx.tclspec`, any depth — and [`discover_in`] walks it as
/// the bundled tier.
///
/// The leading `\0` is what makes it safe to consult unconditionally: no real
/// filesystem can name a path containing a NUL byte, so a native session's
/// [`NativeStore`](tcl_lsp_core::vfs::NativeStore) can only ever answer "not
/// found" here, and the mount cannot collide with, shadow, or be shadowed by
/// anything a user actually has on disk. The `.tcl-lsp` component keeps it
/// self-describing in the one place it *is* visible: a pack notice naming a
/// file the host supplied.
pub const VIRTUAL_PACK_MOUNT: &str = "/\0.tcl-lsp/specs";

/// Ceiling on directories visited while hunting for `tclpkg.tcl` manifests.
///
/// The manifest hunt is the only *unbounded* rule — the other four look in a
/// named directory — so it is the only one that can meet a monorepo. The cap
/// makes the worst case a bounded, silent partial scan rather than a startup
/// that never finishes; a project that large should name its packs in
/// `tclLsp.specPacks`.
const MANIFEST_SCAN_DIR_CAP: usize = 4_000;

/// What to look at. Every field is optional in the sense that an empty or
/// absent one simply contributes no files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiscoveryOptions {
    /// The editor's workspace folders, as filesystem paths.
    pub workspace_roots: Vec<PathBuf>,
    /// The `tclLsp.specPacks` value pulled at **session scope**: files or
    /// directories. A relative path is resolved against each workspace root,
    /// which is what an unscoped setting means — the user asked for
    /// `.tcl-lsp/vendor` in "the workspace", and a multi-root workspace has
    /// several.
    pub configured: Vec<PathBuf>,
    /// `tclLsp.specPacks` pulled **scoped to one workspace folder**, as
    /// `(folder, paths)`.
    ///
    /// A relative path here resolves against *that folder only*: the setting
    /// was answered for that `scopeUri`, so applying it to a sibling root
    /// would invent a pack the user never configured. Without this a
    /// multi-root workspace that configures packs on a secondary folder loads
    /// none of them — the session-scope pull sees only the primary folder's
    /// answer.
    pub folder_configured: Vec<(PathBuf, Vec<PathBuf>)>,
    /// The per-user pack directory. [`None`] uses the platform default;
    /// point it somewhere else in a test.
    pub user_dir: Option<PathBuf>,
    /// The bundled-pack directory. [`None`] uses [`bundled_dir`].
    pub bundled_dir: Option<PathBuf>,
    /// Skip the user tier entirely (`tclLsp.specPacks.includeUserPacks:
    /// false`), for a workspace that wants only what it declares.
    pub skip_user_tier: bool,
    /// The editor's Workspace Trust state for these folders — the one input
    /// the trust ruling plumbs from the LSP client. It decides nothing about
    /// *which* files exist, so the scan never reads it; every workspace-tier
    /// file the scan finds loads under it ([`Tier::trust_under`]), which is
    /// what the load a caller makes with these options is handed
    /// ([`crate::bundled::load_discovered_in`]). The default, a client that
    /// reports nothing, is [`WorkspaceTrust::Trusted`].
    pub workspace_trust: WorkspaceTrust,
}

/// The platform default per-user pack directory:
/// `<config dir>/specs`, i.e. `$XDG_CONFIG_HOME/tcl-lsp/specs` on Linux.
#[must_use]
pub fn user_dir() -> PathBuf {
    tcl_userdirs::config_dir().join(USER_PACK_SUBDIR)
}

/// The bundled-pack directory: [`BUNDLED_DIR_ENV`] when set, else the
/// `specs/` directory beside the running executable, else — in a debug build
/// only — the `specs/` directory of the source checkout this binary was built
/// from, else nothing.
///
/// Deliberately *not* an `include_dir!` of the shipped loadables: the bundled
/// tier exists so the loader path is exercised in production, and a directory
/// on disk is what the production layout actually is.
///
/// # The debug fallback
///
/// A release lays `specs/` down beside the executable — the same Makefile
/// staging that puts a `tcl-lsp-server` binary in the `VSIX` and the
/// `JetBrains` plugin copies `$(SPEC_PACK_SRC)` next to it, and both packaging gates fail
/// if it is missing. A `cargo build` lays down nothing, so a
/// `target/debug/tcl` — or a test binary in `target/debug/deps/` — would find
/// no loadables and quietly lose the EDA vendor libraries, which exist only
/// as packs. Compiling the checkout's `specs/` path in
/// under `debug_assertions` makes a development build behave like an install
/// without any per-developer setup. It cannot fire in a shipped binary: the
/// path is not compiled in at all when `debug_assertions` is off, and a test
/// that wants a *different* bundled directory still sets
/// [`BUNDLED_DIR_ENV`] or passes `bundled_dir` explicitly, both of which win.
#[must_use]
pub fn bundled_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(BUNDLED_DIR_ENV)
        && !dir.is_empty()
    {
        return Some(PathBuf::from(dir));
    }
    if let Some(exe) = std::env::current_exe().ok()
        && let Some(parent) = exe.parent()
    {
        let beside = parent.join(USER_PACK_SUBDIR);
        if beside.is_dir() {
            return Some(beside);
        }
    }
    #[cfg(debug_assertions)]
    {
        let checkout = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs"));
        if checkout.is_dir() {
            return Some(checkout);
        }
    }
    None
}

/// `true` when `path` names a `.tclspec` file (case-insensitively, matching
/// how the rest of the server treats Tcl-family extensions).
#[must_use]
pub fn is_pack_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(PACK_EXTENSION))
}

/// Discover every pack file, ordered by tier then path.
///
/// The same file reached by two rules (named in `tclLsp.specPacks` *and*
/// sitting in `.tcl-lsp/`) is returned once, at its best tier and first
/// origin — the [`BTreeSet`] keyed on [`PackFile`]'s `(tier, path, origin)`
/// ordering does the deduplication and the sort in one step, and the final
/// pass keeps the first entry per path.
#[must_use]
pub fn discover(options: &DiscoveryOptions) -> Vec<PackFile> {
    discover_in(&NativeStore, options)
}

/// [`discover`] against `store` rather than `std::fs`, plus the
/// [`VIRTUAL_PACK_MOUNT`] bundled tier.
///
/// The store is the LSP server's closed-file seam
/// ([`tcl_lsp_core::vfs`]) — natively a literal `std::fs` delegation, so
/// `discover` is exactly this function; in a browser worker a byte map the host
/// filled in, so a page's `.tclspec` files are discovered from memory.
#[must_use]
pub fn discover_in(store: &dyn SourceStore, options: &DiscoveryOptions) -> Vec<PackFile> {
    discover_selected_in(store, options, None).files
}

/// Discover ordinary packs and only the explicitly retained active Studio
/// paths. This does not issue a session/source lease: the caller must retain
/// and revalidate its actual source, provider and publication owner.
/// Unregistered files in a workspace's reserved Studio directory cannot enter
/// through the workspace/configured scan. Standalone [`discover_in`] retains
/// its separate filesystem-only compatibility purpose.
#[must_use]
pub fn discover_with_active_studio_in(
    store: &dyn SourceStore,
    options: &DiscoveryOptions,
    active: &[PathBuf],
) -> Vec<PackFile> {
    discover_selected_in(store, options, Some(active)).files
}

/// Whether this original provider Path is a pack file strictly below one of
/// these workspace Studio directories. No canonicalisation, URI inverse,
/// file existence, source currency or active-session grant follows.
#[must_use]
pub fn is_workspace_studio_source(path: &Path, roots: &[PathBuf]) -> bool {
    is_pack_file(path)
        && roots.iter().any(|root| {
            path.strip_prefix(root.join(WORKSPACE_PACK_DIR).join(STUDIO_OVERRIDE_DIR))
                .is_ok_and(|relative| {
                    relative.components().next().is_some()
                        && relative
                            .components()
                            .all(|part| matches!(part, std::path::Component::Normal(_)))
                })
        })
}

fn is_reserved_workspace_studio_path(
    store: &dyn SourceStore,
    path: &Path,
    roots: &[PathBuf],
) -> bool {
    roots.iter().any(|root| {
        let reserved = root.join(WORKSPACE_PACK_DIR).join(STUDIO_OVERRIDE_DIR);
        path.starts_with(&reserved) || path.starts_with(normalise_in(store, &reserved))
    })
}

/// Actual configured-source contributions from one discovery pass.
/// This is source selection only, not publication or a live provider grant.
pub struct ConfiguredDiscovery {
    /// The exact final selected files passed to the loader.
    pub files: Vec<PackFile>,
    /// Original native files contributed by each session configured entry.
    /// Empty means no selected contribution, never a successful load.
    pub configured_sources: Vec<Vec<PathBuf>>,
    /// Actual manifest observations governing this file selection, not a census.
    pub(crate) manifest_selection: ManifestSelectionInputs,
}

/// Manifest reads used by this actual discovery operation. Unknown inputs
/// cannot certify the selected-file plan; ordinary successful packs remain data.
#[derive(Default)]
pub(crate) struct ManifestSelectionInputs {
    pub(crate) sources: Vec<(PathBuf, String)>,
    pub(crate) issues: Vec<(PathBuf, String)>,
    unavailable_subtrees: Vec<PathBuf>,
}

impl ManifestSelectionInputs {
    fn record(&mut self, path: &Path, source: String) {
        if let Some((_, previous)) = self.sources.iter().find(|(held, _)| held == path) {
            if previous != &source {
                self.issues.push((
                    path.to_path_buf(),
                    "manifest source changed during discovery".into(),
                ));
            }
        } else {
            self.sources.push((path.to_path_buf(), source));
        }
    }
}

/// Retain configured-entry correspondence from the same ordinary discovery.
/// The caller must join these files to its actual successfully installed set.
#[must_use]
pub fn discover_configuration_with_active_studio_in(
    store: &dyn SourceStore,
    options: &DiscoveryOptions,
    active: &[PathBuf],
) -> ConfiguredDiscovery {
    discover_selected_in(store, options, Some(active))
}

#[allow(clippy::too_many_lines)]
fn discover_selected_in(
    store: &dyn SourceStore,
    options: &DiscoveryOptions,
    active: Option<&[PathBuf]>,
) -> ConfiguredDiscovery {
    let mut found: BTreeSet<PackFile> = BTreeSet::new();
    let mut manifest_selection = ManifestSelectionInputs::default();
    let mut configured_sources = vec![BTreeSet::<PathBuf>::new(); options.configured.len()];

    // --- workspace tier -----------------------------------------------------
    for root in &options.workspace_roots {
        if active.is_none() {
            collect_dir(
                store,
                &root.join(WORKSPACE_PACK_DIR).join(STUDIO_OVERRIDE_DIR),
                Tier::StudioOverride,
                Origin::StudioOverride,
                &mut found,
            );
        }
        for (index, configured) in options.configured.iter().enumerate() {
            let path = if configured.is_absolute() {
                configured.clone()
            } else {
                root.join(configured)
            };
            let mut selected = BTreeSet::new();
            collect_path(
                store,
                &path,
                Tier::Workspace,
                Origin::Setting,
                &mut selected,
            );
            configured_sources[index].extend(selected.iter().map(|file| file.path.clone()));
            found.extend(selected);
        }
        collect_dir(
            store,
            &root.join(WORKSPACE_PACK_DIR),
            Tier::Workspace,
            Origin::DotDir,
            &mut found,
        );
        collect_beside_manifests_observed(store, root, &mut found, &mut manifest_selection);
    }
    // Folder-scoped `tclLsp.specPacks`. Resolved against its own folder and no
    // other, whether or not that folder is in `workspace_roots` — the client
    // answered this for that `scopeUri`, and a sibling root's relative
    // resolution would be a pack the user never asked for.
    for (folder, paths) in &options.folder_configured {
        for configured in paths {
            let path = if configured.is_absolute() {
                configured.clone()
            } else {
                folder.join(configured)
            };
            collect_path(store, &path, Tier::Workspace, Origin::Setting, &mut found);
        }
    }
    // An absolute `tclLsp.specPacks` entry is meaningful even with no folder
    // open (a single-file editor session), so honour it once more outside the
    // per-root loop rather than making it depend on a root existing.
    if options.workspace_roots.is_empty() {
        for (index, configured) in options.configured.iter().enumerate() {
            if configured.is_absolute() {
                let mut selected = BTreeSet::new();
                collect_path(
                    store,
                    configured,
                    Tier::Workspace,
                    Origin::Setting,
                    &mut selected,
                );
                configured_sources[index].extend(selected.iter().map(|file| file.path.clone()));
                found.extend(selected);
            }
        }
    }

    // --- user tier ----------------------------------------------------------
    if !options.skip_user_tier {
        let dir = options.user_dir.clone().unwrap_or_else(user_dir);
        collect_dir(store, &dir, Tier::User, Origin::UserDir, &mut found);
    }

    // --- bundled tier -------------------------------------------------------
    if let Some(dir) = options.bundled_dir.clone().or_else(bundled_dir) {
        collect_dir(store, &dir, Tier::Bundled, Origin::Bundled, &mut found);
    }
    // The store's virtual mount, consulted only when the directory beside the
    // executable produced nothing — the same "a real `specs/` stays
    // authoritative when it has anything in it" rule
    // `bundled::load_discovered` applies to its embedded fallback. Natively
    // the mount cannot exist (see [`VIRTUAL_PACK_MOUNT`]), so this is a
    // no-op there whether or not a `specs/` directory was found.
    if !found.iter().any(|file| file.tier == Tier::Bundled) {
        collect_dir(
            store,
            Path::new(VIRTUAL_PACK_MOUNT),
            Tier::Bundled,
            Origin::HostMount,
            &mut found,
        );
    }

    if let Some(active) = active {
        found.retain(|file| {
            !is_reserved_workspace_studio_path(store, &file.path, &options.workspace_roots)
        });
        for path in active {
            if is_workspace_studio_source(path, &options.workspace_roots) {
                found.insert(PackFile {
                    tier: Tier::StudioOverride,
                    path: path.clone(),
                    origin: Origin::StudioOverride,
                    dependency_tier: None,
                });
            }
        }
    }

    // One entry per path: the set is already ordered by (tier, path, origin),
    // so the first sighting of a path is its best tier.
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    let mut files: Vec<PackFile> = found
        .into_iter()
        .filter(|file| seen.insert(file.path.clone()))
        .collect();
    assign_dependency_tiers(
        store,
        &options.workspace_roots,
        &mut files,
        &mut manifest_selection,
    );
    let configured_sources = configured_sources
        .into_iter()
        .map(|sources| {
            sources
                .into_iter()
                .filter(|path| files.iter().any(|file| &file.path == path))
                .collect()
        })
        .collect();
    ConfiguredDiscovery {
        files,
        configured_sources,
        manifest_selection,
    }
}

/// Add `path` — a `.tclspec` file, or a directory to scan for them.
fn collect_path(
    store: &dyn SourceStore,
    path: &Path,
    tier: Tier,
    origin: Origin,
    out: &mut BTreeSet<PackFile>,
) {
    if store.is_dir(path) {
        collect_dir(store, path, tier, origin, out);
    } else if store.is_file(path) && is_pack_file(path) {
        out.insert(PackFile {
            tier,
            path: normalise_in(store, path),
            origin,
            dependency_tier: None,
        });
    }
}

/// Add every `.tclspec` under `dir`, recursively.
///
/// Recursive because a pack is a logical unit an author groups however they
/// like (`docs/design/registry/spec-packs.md`, "Loading and tooling") — one file per
/// namespace in subdirectories is a shape the design explicitly invites.
fn collect_dir(
    store: &dyn SourceStore,
    dir: &Path,
    tier: Tier,
    origin: Origin,
    out: &mut BTreeSet<PackFile>,
) {
    collect_dir_except(store, dir, tier, origin, &|_| false, out);
}

/// [`collect_dir`], not descending into a directory `skip` names.
fn collect_dir_except(
    store: &dyn SourceStore,
    dir: &Path,
    tier: Tier,
    origin: Origin,
    skip: &dyn Fn(&Path) -> bool,
    out: &mut BTreeSet<PackFile>,
) {
    let _ = collect_dir_except_with_errors(store, dir, tier, origin, skip, out);
}

/// The existing recursive selector with explicit required-input failures.
/// Compatibility discovery still keeps its independently useful positive prefix.
fn collect_dir_except_with_errors(
    store: &dyn SourceStore,
    dir: &Path,
    tier: Tier,
    origin: Origin,
    skip: &dyn Fn(&Path) -> bool,
    out: &mut BTreeSet<PackFile>,
) -> Vec<(PathBuf, std::io::Error)> {
    let mut errors = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = match store.read_dir(&current) {
            Ok(entries) => entries,
            Err(error) => {
                errors.push((current, error));
                continue;
            }
        };
        for entry in entries {
            if entry.is_dir {
                if !is_skipped_dir(&entry.path) && !skip(&entry.path) {
                    stack.push(entry.path);
                }
            } else if entry.is_file && is_pack_file(&entry.path) {
                out.insert(PackFile {
                    tier,
                    path: if origin == Origin::BesideManifest {
                        normalise_beside_manifest_in(store, &entry.path)
                    } else {
                        normalise_in(store, &entry.path)
                    },
                    origin,
                    dependency_tier: None,
                });
            }
        }
    }
    errors
}

/// Required bundled generation uses the same native selector and skip policy;
/// a missing or unreadable directory cannot issue a complete selected roster.
pub(crate) fn required_bundled_files(
    store: &dyn SourceStore,
    dir: &Path,
) -> Result<Vec<PackFile>, String> {
    let mut files = BTreeSet::new();
    let errors = collect_dir_except_with_errors(
        store,
        dir,
        Tier::Bundled,
        Origin::Bundled,
        &|_| false,
        &mut files,
    );
    if let Some((path, error)) = errors.first() {
        return Err(format!(
            "required bundled directory {}: {error}",
            path.display()
        ));
    }
    Ok(files.into_iter().collect())
}

enum ManifestDirectiveObservation {
    Parsed(Option<SpecDirective>),
    Unavailable,
}

/// Add the packs that sit beside each `tclpkg.tcl` under `root`.
///
/// A manifest with a `spec` directive names its packs, and exactly those are
/// its packs: no scan of its directory, so a draft or a fixture beside it
/// stays out. One without keeps the scan of every `.tclspec` under its
/// directory — except under a directory whose own manifest has a directive,
/// which decides its own packs and is read through its own entry.
fn collect_beside_manifests_observed(
    store: &dyn SourceStore,
    root: &Path,
    out: &mut BTreeSet<PackFile>,
    selection: &mut ManifestSelectionInputs,
) {
    let dirs = manifest_dirs_observed(store, root, selection);
    let observed: HashMap<&Path, ManifestDirectiveObservation> = dirs
        .iter()
        .map(|dir| {
            let observation = if let Some(path) = list_package_files(store, dir).manifest {
                match store.read_to_string(&path) {
                    Ok(source) => {
                        selection.record(&path, source.clone());
                        match tcl_pkg_model::manifest::original_manifest_spec_selection(
                            &source,
                            path.to_str(),
                        ) {
                            Ok(selection) => {
                                ManifestDirectiveObservation::Parsed(selection.directive().cloned())
                            }
                            Err(error) => {
                                selection.issues.push((
                                    path,
                                    format!("cannot parse selected manifest: {error:?}"),
                                ));
                                ManifestDirectiveObservation::Unavailable
                            }
                        }
                    }
                    Err(error) => {
                        selection
                            .issues
                            .push((path, format!("cannot read selected manifest: {error}")));
                        ManifestDirectiveObservation::Unavailable
                    }
                }
            } else {
                selection.issues.push((
                    dir.clone(),
                    "selected manifest has no represented original path".into(),
                ));
                ManifestDirectiveObservation::Unavailable
            };
            (dir.as_path(), observation)
        })
        .collect();
    for dir in &dirs {
        match observed.get(dir.as_path()) {
            Some(ManifestDirectiveObservation::Parsed(Some(directive))) => {
                for path in crate::package_specs::pack_paths(dir, directive) {
                    out.insert(PackFile {
                        tier: Tier::Workspace,
                        path: normalise_beside_manifest_in(store, &path),
                        origin: Origin::BesideManifest,
                        dependency_tier: None,
                    });
                }
            }
            Some(ManifestDirectiveObservation::Parsed(None)) => {
                let errors = collect_dir_except_with_errors(
                    store,
                    dir,
                    Tier::Workspace,
                    Origin::BesideManifest,
                    &|nested| {
                        observed.contains_key(nested)
                            || selection
                                .unavailable_subtrees
                                .iter()
                                .any(|blocked| nested.starts_with(blocked))
                    },
                    out,
                );
                for (path, error) in errors {
                    selection.issues.push((
                        path,
                        format!("cannot inspect selected implicit pack directory: {error}"),
                    ));
                }
            }
            Some(ManifestDirectiveObservation::Unavailable) | None => {}
        }
    }
}

/// The directories under `root` that hold a `tclpkg.tcl` manifest.
fn manifest_dirs_observed(
    store: &dyn SourceStore,
    root: &Path,
    selection: &mut ManifestSelectionInputs,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut visited = 0usize;
    // Each frontier directory records whether it was reached through an
    // installed package's symlink, which is followed once and never again.
    let mut stack = vec![(root.to_path_buf(), false)];
    while let Some((current, linked)) = stack.pop() {
        if visited >= MANIFEST_SCAN_DIR_CAP {
            // The already observed positive prefix remains useful. Every
            // unvisited frontier subtree is unavailable, never proven no-spec.
            for (path, _) in std::iter::once((current, linked)).chain(stack.drain(..)) {
                selection.issues.push((
                    path.clone(),
                    "manifest source scan reached its directory bound".into(),
                ));
                selection.unavailable_subtrees.push(path);
            }
            break;
        }
        visited += 1;
        let entries = match store.read_dir(&current) {
            Ok(entries) => entries,
            Err(error) => {
                selection.issues.push((
                    current.clone(),
                    format!("cannot inspect manifest source directory: {error}"),
                ));
                selection.unavailable_subtrees.push(current);
                continue;
            }
        };
        let mut has_manifest = false;
        let mut children: Vec<(PathBuf, bool)> = Vec::new();
        for entry in entries {
            if entry.is_dir {
                if !is_skipped_dir(&entry.path) {
                    children.push((entry.path, linked));
                }
            } else if !entry.is_file
                && !linked
                && is_installed_package_dir(store, &current)
                && store.is_dir(&entry.path)
            {
                children.push((entry.path, true));
            } else if entry.is_file
                && entry
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case(PACKAGE_MANIFEST))
            {
                has_manifest = true;
            }
        }
        // Sort each directory's children, and push them reversed so the stack
        // pops in ascending path order.
        //
        // `read_dir` yields entries in filesystem order, which differs between
        // filesystems and even between runs on the same one. That is harmless
        // while every directory is visited, but [`MANIFEST_SCAN_DIR_CAP`] stops
        // the walk partway on a large tree — and *which* directories were
        // visited by then is then filesystem order too. Two runs over one
        // monorepo could load different packs, which is the worst shape a
        // truncation can take. Ordering the frontier makes the visited prefix a
        // function of the tree alone, so a capped scan is at least the *same*
        // partial scan every time. Sorting the result afterwards cannot do
        // this: by then the subset is already chosen.
        children.sort();
        stack.extend(children.into_iter().rev());
        if has_manifest {
            dirs.push(current);
        }
    }
    dirs.sort();
    dirs
}

/// Directories a scan never descends into.
///
/// Whether `dir` is the `lib` folder `tcl pkg install` materialises each
/// dependency into, as a symlink into the package store, beside a lockfile.
/// The walk follows those links, and only those, keeping the workspace path.
fn is_installed_package_dir(store: &dyn SourceStore, dir: &Path) -> bool {
    dir.file_name().and_then(|name| name.to_str()) == Some("lib")
        && dir
            .parent()
            .is_some_and(|project| store.exists(&project.join(PACKAGE_LOCKFILE)))
}

/// Apply the workspace scan policy while allowing the `.tcl-lsp` pack directory.
fn is_skipped_dir(path: &Path) -> bool {
    path.file_name()
        .is_none_or(|name| name != WORKSPACE_PACK_DIR)
        && tcl_registry::dialects::is_skipped_scan_dir(path)
}

/// Ask the actual source provider for its own canonical spelling. The native
/// provider preserves filesystem deduplication; virtual/foreign providers keep
/// their original Path unless they explicitly supply this namespace operation.
/// Unavailable canonicalisation retains the supplied spelling for reporting.
fn normalise_in(store: &dyn SourceStore, path: &Path) -> PathBuf {
    store
        .canonical_source_path(path)
        .unwrap_or_else(|| path.to_path_buf())
}

/// [`normalise_in`] for a pack found beside a manifest. Inside an installed
/// dependency (`lib/NAME-VERSION` beside a lockfile, usually a symlink into the
/// package store) only the `lib` folder is canonicalised: the pack keeps its
/// workspace path, which places it in its project and gives it its tier.
fn normalise_beside_manifest_in(store: &dyn SourceStore, path: &Path) -> PathBuf {
    for package in path.ancestors().skip(1) {
        let Some(lib) = package.parent() else {
            break;
        };
        if is_installed_package_dir(store, lib)
            && let (Some(name), Ok(rest)) = (package.file_name(), path.strip_prefix(package))
        {
            return normalise_in(store, lib).join(name).join(rest);
        }
    }
    normalise_in(store, path)
}

// Original test compatibility probe uses the same actual native provider;
// production discovery always supplies its retained SourceStore explicitly.
#[cfg(test)]
fn normalise(path: &Path) -> PathBuf {
    normalise_in(&tcl_lsp_core::vfs::NativeStore, path)
}

/// Give every file found beside a manifest the tier of the package that
/// ships it.
///
/// A package's tier is not something its own manifest can state, so the
/// authority is found outside the package: the *project root* is the
/// outermost directory, at or above the package and inside the workspace
/// folder, that holds both a `tclpkg.tcl` and a `tclpkg.lock`. The outermost,
/// not the nearest: an installed dependency's directory may hold a manifest
/// and a lockfile of its own (its tarball can carry any file), and taking the
/// nearest pair would let a dependency name itself the root of its own graph.
/// The lockfile beside the workspace's own manifest lists the dependency, and
/// nothing inside the dependency can outrank it.
///
/// The package a file belongs to is the one whose manifest is nearest above
/// it. It is the *root* package when that manifest sits in the project root
/// itself, and otherwise the tier is what the project's lockfile and root
/// manifest give the package the file's own manifest names
/// ([`dependency_tier`]). A package the lockfile does not list is
/// [`DependencyTier::Transitive`], the least a package gets, because nothing
/// the package's own files say may lift it. Selection fails closed on input
/// it cannot read: when the project's lockfile or root manifest, or the
/// package's own manifest, does not read or parse, the dependency's packs are
/// dropped and the selection issue is recorded. Only a workspace with no
/// project lockfile leaves a file with no tier.
fn assign_dependency_tiers(
    store: &dyn SourceStore,
    roots: &[PathBuf],
    files: &mut Vec<PackFile>,
    selection: &mut ManifestSelectionInputs,
) {
    let mut reader = TierReader::new(store, roots);
    files.retain_mut(|file| {
        if file.origin != Origin::BesideManifest {
            return true;
        }
        match reader.tier_of(&file.path) {
            Ok(tier) => {
                file.dependency_tier = tier;
                true
            }
            Err((path, issue)) => {
                if !selection.issues.iter().any(|(held, _)| held == &path) {
                    selection.issues.push((path, issue));
                }
                false
            }
        }
    });
}

/// A selected input that does not read, with the issue it records.
type SelectionIssue = (PathBuf, String);

/// The manifest and lockfile a directory holds, when it holds them.
#[derive(Clone, Default)]
struct PackageFiles {
    manifest: Option<PathBuf>,
    lockfile: Option<PathBuf>,
    /// An incomplete listing or distinct case-equivalent provider paths cannot
    /// choose a manifest/lockfile pair or establish a package trust floor.
    unavailable: bool,
}

/// The package metadata one discovery reads, kept so that the packs of one
/// package share a listing and a parse.
struct TierReader<'a> {
    store: &'a dyn SourceStore,
    /// Each workspace folder as given and as canonicalised: a discovered
    /// path is canonical, and a folder handed in need not be.
    roots: Vec<PathBuf>,
    listings: HashMap<PathBuf, PackageFiles>,
    /// A project root's manifest and lockfile, once read.
    projects: HashMap<PathBuf, Result<(ManifestAst, LockFile), SelectionIssue>>,
    /// The tier of each package directory already worked out.
    packages: HashMap<PathBuf, Result<Option<DependencyTier>, SelectionIssue>>,
}

impl<'a> TierReader<'a> {
    fn new(store: &'a dyn SourceStore, roots: &[PathBuf]) -> Self {
        let mut spellings: Vec<PathBuf> = Vec::new();
        for root in roots {
            for spelling in [root.clone(), normalise_in(store, root)] {
                if !spellings.contains(&spelling) {
                    spellings.push(spelling);
                }
            }
        }
        Self {
            store,
            roots: spellings,
            listings: HashMap::new(),
            projects: HashMap::new(),
            packages: HashMap::new(),
        }
    }

    /// The tier of the package shipping the pack file at `pack`.
    fn tier_of(&mut self, pack: &Path) -> Result<Option<DependencyTier>, SelectionIssue> {
        // The outermost workspace folder holding the pack, as the project
        // rule takes the outermost pair: a folder opened inside an installed
        // dependency would otherwise end the search at the dependency's own
        // manifest and let it stand as a project of its own.
        let Some(root) = self
            .roots
            .iter()
            .filter(|root| pack.starts_with(root))
            .min_by_key(|root| root.components().count())
            .cloned()
        else {
            return Ok(None);
        };
        let Some(parent) = pack.parent() else {
            return Ok(None);
        };
        // The directories from the pack's own up to the workspace folder,
        // innermost first.
        let chain: Vec<PathBuf> = parent
            .ancestors()
            .take_while(|dir| dir.starts_with(&root))
            .map(Path::to_path_buf)
            .collect();
        if chain.iter().any(|dir| self.listing(dir).unavailable) {
            // Do not walk past an unknown local provider to a stronger outer
            // package. Keep the actual pack but give it no promoted trust tier.
            return Ok(Some(DependencyTier::Transitive));
        }
        let Some(package) = chain
            .iter()
            .find(|dir| self.listing(dir).manifest.is_some())
            .cloned()
        else {
            return Ok(None);
        };
        if let Some(known) = self.packages.get(&package) {
            return known.clone();
        }
        let tier = self.tier_of_package(&package, &chain);
        self.packages.insert(package, tier.clone());
        tier
    }

    fn tier_of_package(
        &mut self,
        package: &Path,
        chain: &[PathBuf],
    ) -> Result<Option<DependencyTier>, SelectionIssue> {
        let Some(project) = chain
            .iter()
            .rev()
            .find(|dir| {
                let files = self.listing(dir);
                files.manifest.is_some() && files.lockfile.is_some()
            })
            .cloned()
        else {
            return Ok(None);
        };
        if project == package {
            return Ok(Some(DependencyTier::Root));
        }
        let Some(path) = self.listing(package).manifest else {
            return Ok(None);
        };
        let manifest = read_manifest(self.store, &path)?;
        let (root, lock) = self.project(&project)?;
        let graded =
            dependency_tier(root, lock, &manifest.name).unwrap_or(DependencyTier::Transitive);
        // The package may ask for a tier further from the root than its
        // position gives it, and never a nearer one.
        let requested = manifest.spec.map(|spec| spec.requested_tier);
        Ok(Some(requested.map_or(graded, |requested| {
            clamp_requested(requested, graded)
        })))
    }

    fn listing(&mut self, dir: &Path) -> PackageFiles {
        let store = self.store;
        self.listings
            .entry(dir.to_path_buf())
            .or_insert_with(|| list_package_files(store, dir))
            .clone()
    }

    /// The project root's manifest and lockfile; the root is chosen because
    /// its listing holds both.
    fn project(&mut self, dir: &Path) -> Result<&(ManifestAst, LockFile), SelectionIssue> {
        if !self.projects.contains_key(dir) {
            let files = self.listing(dir);
            let read = match files.manifest.zip(files.lockfile) {
                Some((manifest, lockfile)) => read_manifest(self.store, &manifest)
                    .and_then(|manifest| Ok((manifest, read_lockfile(self.store, &lockfile)?))),
                None => Err((
                    dir.to_path_buf(),
                    "selected project files are unavailable".into(),
                )),
            };
            self.projects.insert(dir.to_path_buf(), read);
        }
        match &self.projects[dir] {
            Ok(project) => Ok(project),
            Err(issue) => Err(issue.clone()),
        }
    }
}

/// The manifest and the lockfile in `dir`, matched the way the manifest hunt
/// matches a manifest: by file name, ignoring ASCII case.
fn list_package_files(store: &dyn SourceStore, dir: &Path) -> PackageFiles {
    let Ok(entries) = store.read_dir(dir) else {
        return PackageFiles {
            unavailable: true,
            ..PackageFiles::default()
        };
    };
    let mut files = PackageFiles::default();
    for entry in entries.into_iter().filter(|entry| entry.is_file) {
        files.observe_path(entry.path);
    }
    if files.unavailable {
        files.manifest = None;
        files.lockfile = None;
    }
    files
}

impl PackageFiles {
    /// The original case-insensitive admission keeps actual provider paths;
    /// distinct matches are unavailable regardless of enumeration order.
    fn observe_path(&mut self, path: PathBuf) {
        let name = path.file_name().and_then(|name| name.to_str());
        let slot = if name.is_some_and(|name| name.eq_ignore_ascii_case(PACKAGE_MANIFEST)) {
            &mut self.manifest
        } else if name.is_some_and(|name| name.eq_ignore_ascii_case(PACKAGE_LOCKFILE)) {
            &mut self.lockfile
        } else {
            return;
        };
        if slot.as_ref().is_some_and(|previous| previous != &path) {
            self.unavailable = true;
        } else {
            *slot = Some(path);
        }
    }
}

fn read_manifest(store: &dyn SourceStore, path: &Path) -> Result<ManifestAst, SelectionIssue> {
    let text = store.read_to_string(path).map_err(|error| {
        (
            path.to_path_buf(),
            format!("cannot read selected manifest: {error}"),
        )
    })?;
    load_manifest_text(&text, path.to_str()).map_err(|error| {
        (
            path.to_path_buf(),
            format!("cannot parse selected manifest: {error:?}"),
        )
    })
}

fn read_lockfile(store: &dyn SourceStore, path: &Path) -> Result<LockFile, SelectionIssue> {
    let text = store.read_to_string(path).map_err(|error| {
        (
            path.to_path_buf(),
            format!("cannot read selected lockfile: {error}"),
        )
    })?;
    deserialise(&text).map_err(|error| {
        (
            path.to_path_buf(),
            format!("cannot parse selected lockfile: {error:?}"),
        )
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod active_studio_discovery_tests;

#[cfg(test)]
mod reserved_studio_path_tests;

#[cfg(test)]
mod source_provider_path_normalisation_tests;

#[cfg(test)]
mod manifest_selection_source_controls;

#[cfg(test)]
mod manifest_frontier_completeness_controls;

#[cfg(test)]
mod actual_manifest_case_controls;
