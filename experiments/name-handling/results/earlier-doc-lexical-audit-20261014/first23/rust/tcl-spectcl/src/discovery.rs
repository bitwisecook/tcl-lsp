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
    /// A live editor-hosted Spec Studio session under `.tcl-lsp/.spec-studio/`.
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
    /// graph does not place it (an unlisted package, a manifest or lockfile
    /// that does not read). `None` for every other file — a pack no package
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
#[derive(Debug, Clone, Default)]
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
/// no loadables and quietly lose the EDA vendor libraries, which since their
/// migration exist *only* as packs. Compiling the checkout's `specs/` path in
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
    let mut found: BTreeSet<PackFile> = BTreeSet::new();

    // --- workspace tier -----------------------------------------------------
    for root in &options.workspace_roots {
        collect_dir(
            store,
            &root.join(WORKSPACE_PACK_DIR).join(STUDIO_OVERRIDE_DIR),
            Tier::StudioOverride,
            Origin::StudioOverride,
            &mut found,
        );
        for configured in &options.configured {
            let path = if configured.is_absolute() {
                configured.clone()
            } else {
                root.join(configured)
            };
            collect_path(store, &path, Tier::Workspace, Origin::Setting, &mut found);
        }
        collect_dir(
            store,
            &root.join(WORKSPACE_PACK_DIR),
            Tier::Workspace,
            Origin::DotDir,
            &mut found,
        );
        collect_beside_manifests(store, root, &mut found);
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
        for configured in &options.configured {
            if configured.is_absolute() {
                collect_path(
                    store,
                    configured,
                    Tier::Workspace,
                    Origin::Setting,
                    &mut found,
                );
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

    // One entry per path: the set is already ordered by (tier, path, origin),
    // so the first sighting of a path is its best tier.
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    let mut files: Vec<PackFile> = found
        .into_iter()
        .filter(|file| seen.insert(file.path.clone()))
        .collect();
    assign_dependency_tiers(store, &options.workspace_roots, &mut files);
    files
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
            path: normalise(path),
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
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = store.read_dir(&current) else {
            continue;
        };
        for entry in entries {
            if entry.is_dir {
                if !is_skipped_dir(&entry.path) && !skip(&entry.path) {
                    stack.push(entry.path);
                }
            } else if entry.is_file && is_pack_file(&entry.path) {
                out.insert(PackFile {
                    tier,
                    path: normalise(&entry.path),
                    origin,
                    dependency_tier: None,
                });
            }
        }
    }
}

/// Add the packs that sit beside each `tclpkg.tcl` under `root`.
///
/// A manifest with a `spec` directive names its packs, and exactly those are
/// its packs: no scan of its directory, so a draft or a fixture beside it
/// stays out. One without keeps the scan of every `.tclspec` under its
/// directory — except under a directory whose own manifest has a directive,
/// which decides its own packs and is read through its own entry.
fn collect_beside_manifests(store: &dyn SourceStore, root: &Path, out: &mut BTreeSet<PackFile>) {
    let dirs = manifest_dirs(store, root);
    let declared: HashMap<&Path, SpecDirective> = dirs
        .iter()
        .filter_map(|dir| {
            let directive = list_package_files(store, dir)
                .manifest
                .and_then(|manifest| read_manifest(store, &manifest))?
                .spec?;
            Some((dir.as_path(), directive))
        })
        .collect();
    for dir in &dirs {
        match declared.get(dir.as_path()) {
            Some(directive) => {
                for path in crate::package_specs::pack_paths(dir, directive) {
                    out.insert(PackFile {
                        tier: Tier::Workspace,
                        path: normalise(&path),
                        origin: Origin::BesideManifest,
                        dependency_tier: None,
                    });
                }
            }
            None => collect_dir_except(
                store,
                dir,
                Tier::Workspace,
                Origin::BesideManifest,
                &|nested| declared.contains_key(nested),
                out,
            ),
        }
    }
}

/// The directories under `root` that hold a `tclpkg.tcl` manifest.
fn manifest_dirs(store: &dyn SourceStore, root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut visited = 0usize;
    let mut stack = vec![root.to_path_buf()];
    while let Some(current) = stack.pop() {
        if visited >= MANIFEST_SCAN_DIR_CAP {
            break;
        }
        visited += 1;
        let Ok(entries) = store.read_dir(&current) else {
            continue;
        };
        let mut has_manifest = false;
        let mut children: Vec<PathBuf> = Vec::new();
        for entry in entries {
            if entry.is_dir {
                if !is_skipped_dir(&entry.path) {
                    children.push(entry.path);
                }
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
/// Mirrors the LSP server's own workspace-scan filter (vendor directories and
/// dot-directories), with the one exception that matters here: `.tcl-lsp` is a
/// dot-directory the design *requires* us to look in.
fn is_skipped_dir(path: &Path) -> bool {
    match path.file_name().and_then(|n| n.to_str()) {
        Some(WORKSPACE_PACK_DIR) => false,
        Some(name) => name.starts_with('.') || matches!(name, "node_modules" | "target" | "tmp"),
        // No representable file name (e.g. `..`): skip to be safe.
        None => true,
    }
}

/// Canonicalise when the filesystem allows it, so the same file reached by two
/// different spellings (a `tclLsp.specPacks` relative path and a `.tcl-lsp/`
/// scan) deduplicates. Falls back to the path as given — an unreadable path is
/// still worth reporting, and a notice naming a symlink the user wrote is
/// friendlier than one naming its target.
///
/// Deliberately **not** routed through the [`SourceStore`]: resolving `..`,
/// symlinks, and a relative path against the process's working directory is a
/// capability a byte map genuinely does not have, and the fallback is already
/// the right answer for a store-supplied path — its spelling is the only one
/// it has.
fn normalise(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
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
/// ([`dependency_tier`]). Below a project that has a lockfile a file is never
/// left without a tier: a manifest that does not read, a package the lockfile
/// does not list and a lockfile that does not read each leave it
/// [`DependencyTier::Transitive`], the least a package gets, because nothing
/// the package's own files say — or fail to say — may lift it. Only a
/// workspace with no project lockfile leaves a file with no tier.
fn assign_dependency_tiers(store: &dyn SourceStore, roots: &[PathBuf], files: &mut [PackFile]) {
    let mut reader = TierReader::new(store, roots);
    for file in files
        .iter_mut()
        .filter(|file| file.origin == Origin::BesideManifest)
    {
        file.dependency_tier = reader.tier_of(&file.path);
    }
}

/// The manifest and lockfile a directory holds, when it holds them.
#[derive(Clone, Default)]
struct PackageFiles {
    manifest: Option<PathBuf>,
    lockfile: Option<PathBuf>,
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
    projects: HashMap<PathBuf, Option<(ManifestAst, LockFile)>>,
    /// The tier of each package directory already worked out.
    packages: HashMap<PathBuf, Option<DependencyTier>>,
}

impl<'a> TierReader<'a> {
    fn new(store: &'a dyn SourceStore, roots: &[PathBuf]) -> Self {
        let mut spellings: Vec<PathBuf> = Vec::new();
        for root in roots {
            for spelling in [root.clone(), normalise(root)] {
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
    fn tier_of(&mut self, pack: &Path) -> Option<DependencyTier> {
        // The outermost workspace folder holding the pack, as the project
        // rule takes the outermost pair: a folder opened inside an installed
        // dependency would otherwise end the search at the dependency's own
        // manifest and let it stand as a project of its own.
        let root = self
            .roots
            .iter()
            .filter(|root| pack.starts_with(root))
            .min_by_key(|root| root.components().count())?
            .clone();
        // The directories from the pack's own up to the workspace folder,
        // innermost first.
        let chain: Vec<PathBuf> = pack
            .parent()?
            .ancestors()
            .take_while(|dir| dir.starts_with(&root))
            .map(Path::to_path_buf)
            .collect();
        let package = chain
            .iter()
            .find(|dir| self.listing(dir).manifest.is_some())?
            .clone();
        if let Some(known) = self.packages.get(&package) {
            return *known;
        }
        let tier = self.tier_of_package(&package, &chain);
        self.packages.insert(package, tier);
        tier
    }

    fn tier_of_package(&mut self, package: &Path, chain: &[PathBuf]) -> Option<DependencyTier> {
        let project = chain
            .iter()
            .rev()
            .find(|dir| {
                let files = self.listing(dir);
                files.manifest.is_some() && files.lockfile.is_some()
            })?
            .clone();
        if project == package {
            return Some(DependencyTier::Root);
        }
        let manifest = self
            .listing(package)
            .manifest
            .and_then(|manifest| read_manifest(self.store, &manifest));
        let graded = manifest
            .as_ref()
            .and_then(|manifest| {
                let (root, lock) = self.project(&project)?;
                dependency_tier(root, lock, &manifest.name)
            })
            .unwrap_or(DependencyTier::Transitive);
        // The package may ask for a tier further from the root than its
        // position gives it, and never a nearer one.
        let requested = manifest
            .and_then(|manifest| manifest.spec)
            .map(|spec| spec.requested_tier);
        Some(requested.map_or(graded, |requested| clamp_requested(requested, graded)))
    }

    fn listing(&mut self, dir: &Path) -> PackageFiles {
        let store = self.store;
        self.listings
            .entry(dir.to_path_buf())
            .or_insert_with(|| list_package_files(store, dir))
            .clone()
    }

    fn project(&mut self, dir: &Path) -> Option<&(ManifestAst, LockFile)> {
        if !self.projects.contains_key(dir) {
            let files = self.listing(dir);
            let read = files
                .manifest
                .zip(files.lockfile)
                .and_then(|(manifest, lockfile)| {
                    let manifest = read_manifest(self.store, &manifest)?;
                    let lock = deserialise(&self.store.read_to_string(&lockfile).ok()?).ok()?;
                    Some((manifest, lock))
                });
            self.projects.insert(dir.to_path_buf(), read);
        }
        self.projects.get(dir)?.as_ref()
    }
}

/// The manifest and the lockfile in `dir`, matched the way the manifest hunt
/// matches a manifest: by file name, ignoring ASCII case.
fn list_package_files(store: &dyn SourceStore, dir: &Path) -> PackageFiles {
    let mut files = PackageFiles::default();
    let Ok(entries) = store.read_dir(dir) else {
        return files;
    };
    for entry in entries.into_iter().filter(|entry| entry.is_file) {
        let name = entry.path.file_name().and_then(|name| name.to_str());
        if name.is_some_and(|name| name.eq_ignore_ascii_case(PACKAGE_MANIFEST)) {
            files.manifest = Some(entry.path);
        } else if name.is_some_and(|name| name.eq_ignore_ascii_case(PACKAGE_LOCKFILE)) {
            files.lockfile = Some(entry.path);
        }
    }
    files
}

fn read_manifest(store: &dyn SourceStore, path: &Path) -> Option<ManifestAst> {
    let text = store.read_to_string(path).ok()?;
    load_manifest_text(&text, path.to_str()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tcl-spectcl-discovery-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn write(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent dir");
        }
        std::fs::write(path, body).expect("write");
    }

    /// A multi-root workspace that configures `tclLsp.specPacks` on its
    /// *secondary* folder loads that folder's packs. The session-scope pull
    /// carries only the primary folder's answer, so before folder scoping
    /// existed these packs were discovered by nobody.
    #[test]
    fn a_folder_scoped_setting_loads_that_folders_packs() {
        let root = tmpdir("folder-scoped");
        let primary = root.join("primary");
        let secondary = root.join("secondary");
        write(&secondary.join("vendor/s.tclspec"), "speclib s 1 {}\n");
        write(&primary.join("vendor/p.tclspec"), "speclib p 1 {}\n");

        let found = discover(&DiscoveryOptions {
            workspace_roots: vec![primary.clone(), secondary.clone()],
            // The relative entry is scoped to the secondary folder only.
            folder_configured: vec![(secondary.clone(), vec![PathBuf::from("vendor")])],
            skip_user_tier: true,
            bundled_dir: Some(root.join("no-such-bundled")),
            ..DiscoveryOptions::default()
        });
        let names: Vec<String> = found
            .iter()
            .filter_map(|f| f.path.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        assert!(
            names.contains(&"s.tclspec".to_owned()),
            "the secondary folder's configured pack must load: {names:?}"
        );
        // And it must NOT be resolved against the primary root: that would
        // invent `primary/vendor` as a pack source the user never configured.
        assert!(
            !names.contains(&"p.tclspec".to_owned()),
            "a folder-scoped relative entry must not resolve against a sibling root: {names:?}"
        );
    }

    /// A session-scope relative entry keeps its documented meaning — every
    /// workspace root — so folder scoping adds a case rather than changing one.
    #[test]
    fn a_session_scoped_relative_setting_still_applies_to_every_root() {
        let root = tmpdir("session-scoped");
        let one = root.join("one");
        let two = root.join("two");
        write(&one.join("vendor/a.tclspec"), "speclib a 1 {}\n");
        write(&two.join("vendor/b.tclspec"), "speclib b 1 {}\n");

        let found = discover(&DiscoveryOptions {
            workspace_roots: vec![one, two],
            configured: vec![PathBuf::from("vendor")],
            skip_user_tier: true,
            bundled_dir: Some(root.join("no-such-bundled")),
            ..DiscoveryOptions::default()
        });
        let names: Vec<String> = found
            .iter()
            .filter_map(|f| f.path.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        assert!(
            names.contains(&"a.tclspec".to_owned()) && names.contains(&"b.tclspec".to_owned()),
            "an unscoped relative entry means 'in the workspace', all roots: {names:?}"
        );
    }

    #[test]
    fn workspace_tier_finds_dot_dir_and_manifest_siblings() {
        let root = tmpdir("ws");
        write(&root.join(".tcl-lsp/a.tclspec"), "speclib a 1 {}\n");
        write(&root.join("lib/tclpkg.tcl"), "package require x\n");
        write(&root.join("lib/b.tclspec"), "speclib b 1 {}\n");
        // Not beside a manifest and not under `.tcl-lsp/`: invisible.
        write(&root.join("other/c.tclspec"), "speclib c 1 {}\n");

        let found = discover(&DiscoveryOptions {
            workspace_roots: vec![root.clone()],
            skip_user_tier: true,
            bundled_dir: Some(root.join("no-such-bundled")),
            ..DiscoveryOptions::default()
        });
        let names: Vec<String> = found
            .iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["a.tclspec", "b.tclspec"], "{found:#?}");
        assert!(found.iter().all(|f| f.tier == Tier::Workspace));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn configured_paths_take_files_and_directories() {
        let root = tmpdir("configured");
        write(&root.join("packs/one.tclspec"), "speclib one 1 {}\n");
        write(&root.join("packs/nested/two.tclspec"), "speclib two 1 {}\n");
        write(&root.join("loose.tclspec"), "speclib loose 1 {}\n");

        let found = discover(&DiscoveryOptions {
            workspace_roots: vec![root.clone()],
            configured: vec![PathBuf::from("packs"), PathBuf::from("loose.tclspec")],
            skip_user_tier: true,
            bundled_dir: Some(root.join("no-such-bundled")),
            ..DiscoveryOptions::default()
        });
        let mut names: Vec<String> = found
            .iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, vec!["loose.tclspec", "one.tclspec", "two.tclspec"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn tiers_sort_studio_workspace_user_then_bundled() {
        let root = tmpdir("tiers");
        write(
            &root.join("ws/.tcl-lsp/.spec-studio/session/live.tclspec"),
            "speclib live 1 {}\n",
        );
        write(&root.join("ws/.tcl-lsp/w.tclspec"), "speclib w 1 {}\n");
        write(&root.join("user/u.tclspec"), "speclib u 1 {}\n");
        write(&root.join("bundled/b.tclspec"), "speclib b 1 {}\n");

        let found = discover(&DiscoveryOptions {
            workspace_roots: vec![root.join("ws")],
            user_dir: Some(root.join("user")),
            bundled_dir: Some(root.join("bundled")),
            ..DiscoveryOptions::default()
        });
        let tiers: Vec<Tier> = found.iter().map(|f| f.tier).collect();
        assert_eq!(
            tiers,
            vec![
                Tier::StudioOverride,
                Tier::Workspace,
                Tier::User,
                Tier::Bundled
            ]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_file_reached_twice_is_returned_once_at_its_best_tier() {
        let root = tmpdir("dedup");
        write(&root.join(".tcl-lsp/dup.tclspec"), "speclib dup 1 {}\n");

        let found = discover(&DiscoveryOptions {
            workspace_roots: vec![root.clone()],
            // Names the very same file the `.tcl-lsp/` rule finds.
            configured: vec![PathBuf::from(".tcl-lsp/dup.tclspec")],
            skip_user_tier: true,
            bundled_dir: Some(root.join("no-such-bundled")),
            ..DiscoveryOptions::default()
        });
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].tier, Tier::Workspace);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn vendor_directories_are_not_scanned() {
        let root = tmpdir("vendor");
        write(&root.join("tclpkg.tcl"), "");
        write(&root.join("node_modules/x.tclspec"), "speclib x 1 {}\n");
        write(&root.join("target/y.tclspec"), "speclib y 1 {}\n");
        write(&root.join("keep.tclspec"), "speclib keep 1 {}\n");

        let found = discover(&DiscoveryOptions {
            workspace_roots: vec![root.clone()],
            skip_user_tier: true,
            bundled_dir: Some(root.join("no-such-bundled")),
            ..DiscoveryOptions::default()
        });
        let names: Vec<String> = found
            .iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["keep.tclspec"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn discovery_of_nothing_is_empty_not_an_error() {
        let root = tmpdir("empty");
        let found = discover(&DiscoveryOptions {
            workspace_roots: vec![root.join("does-not-exist")],
            configured: vec![PathBuf::from("also-missing.tclspec")],
            folder_configured: vec![(
                root.join("no-such-folder"),
                vec![PathBuf::from("nor-this.tclspec")],
            )],
            user_dir: Some(root.join("no-user")),
            bundled_dir: Some(root.join("no-bundled")),
            skip_user_tier: false,
            workspace_trust: WorkspaceTrust::Trusted,
        });
        assert!(found.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_host_filled_store_supplies_the_bundled_tier_at_the_virtual_mount() {
        let store = tcl_lsp_core::vfs::MemoryStore::new();
        let mount = PathBuf::from(VIRTUAL_PACK_MOUNT);
        // Nested, to prove the mount is walked recursively like any other
        // bundled directory and that MemoryStore's implied ancestors list.
        store.upsert(
            mount.join("vendor.tclspec"),
            b"speclib vendor 1 {}\n".to_vec(),
        );
        store.upsert(
            mount.join("eda/xilinx.tclspec"),
            b"speclib xil 1 {}\n".to_vec(),
        );

        let found = discover_in(
            &store,
            &DiscoveryOptions {
                skip_user_tier: true,
                ..DiscoveryOptions::default()
            },
        );
        let names: Vec<String> = found
            .iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        // Sorted by full path, so the `eda/` subdirectory sorts before the
        // file sitting directly in the mount.
        assert_eq!(
            names,
            vec!["xilinx.tclspec", "vendor.tclspec"],
            "{found:#?}"
        );
        assert!(
            found
                .iter()
                .all(|f| f.tier == Tier::Bundled && f.origin == Origin::HostMount)
        );
    }

    #[test]
    fn an_on_disk_bundled_directory_still_wins_over_the_virtual_mount() {
        let root = tmpdir("mount-vs-disk");
        write(&root.join("bundled/b.tclspec"), "speclib b 1 {}\n");
        let store = tcl_lsp_core::vfs::MemoryStore::new();
        store.upsert(
            PathBuf::from(VIRTUAL_PACK_MOUNT).join("ignored.tclspec"),
            b"speclib ignored 1 {}\n".to_vec(),
        );
        // The store here answers for *both* halves, so the on-disk directory is
        // modelled as store content under its real path — the point under test
        // is the precedence rule, not which backend served it.
        store.upsert(root.join("bundled/b.tclspec"), b"speclib b 1 {}\n".to_vec());

        let found = discover_in(
            &store,
            &DiscoveryOptions {
                bundled_dir: Some(root.join("bundled")),
                skip_user_tier: true,
                ..DiscoveryOptions::default()
            },
        );
        let names: Vec<String> = found
            .iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["b.tclspec"], "{found:#?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_host_mount_pack_does_not_displace_the_shipped_loadables() {
        // `load_discovered_in`'s embedded fallback keys on a *shipped
        // directory*, not on the bundled tier being empty, so a host that
        // upserts one vendor pack still gets the EDA libraries.
        let store = tcl_lsp_core::vfs::MemoryStore::new();
        store.upsert(
            PathBuf::from(VIRTUAL_PACK_MOUNT).join("vendor.tclspec"),
            b"speclib hostvendor 1 {}\n".to_vec(),
        );
        let found = discover_in(
            &store,
            &DiscoveryOptions {
                skip_user_tier: true,
                ..DiscoveryOptions::default()
            },
        );
        let loaded = crate::bundled::load_discovered_in(&store, &found, WorkspaceTrust::Trusted);
        let names: Vec<&str> = loaded.packs.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"hostvendor"), "{names:?}");
        assert!(
            names.len() > 1,
            "the shipped loadables must survive: {names:?}"
        );
    }

    #[test]
    fn the_virtual_mount_cannot_name_a_real_file() {
        // The NUL is what makes the mount safe to consult on every native
        // session: `std::fs` refuses the path outright rather than reading
        // whatever happens to be there.
        assert!(VIRTUAL_PACK_MOUNT.contains('\0'));
        assert!(std::fs::read_dir(VIRTUAL_PACK_MOUNT).is_err());
        let found = discover(&DiscoveryOptions {
            skip_user_tier: true,
            bundled_dir: Some(PathBuf::from("/no-such-bundled-dir")),
            ..DiscoveryOptions::default()
        });
        assert!(found.is_empty(), "{found:#?}");
    }

    // ── how far the package shipping a pack sits from the workspace root ──

    /// The root manifest of the fixture workspace: `json` is required,
    /// `tcltest` is a development requirement, and `http` is `json`'s.
    const ROOT_MANIFEST: &str =
        "package myapp\nversion 1.0.0\nrequire json 1.0.0\ndev-require tcltest 2.5.0\n";

    /// `(name, requires, dev)` rows as the lockfile of the fixture workspace.
    fn lockfile_text(packages: &[(&str, &[&str], bool)]) -> String {
        use tcl_pkg_model::lockfile::{LockedPackage, SourceSpec};
        let mut lock = LockFile::new("myapp", ">=8.6");
        for (name, requires, dev) in packages {
            lock.packages.push(LockedPackage {
                name: (*name).to_owned(),
                version: "1.0.0".to_owned(),
                source: SourceSpec::new("tarball", ""),
                integrity: String::new(),
                size: 0,
                requires: requires.iter().map(|entry| (*entry).to_owned()).collect(),
                provides: Vec::new(),
                license: String::new(),
                dev: *dev,
                spec_integrity: None,
            });
        }
        tcl_pkg_model::lockfile::serialise(&lock)
    }

    /// The fixture workspace as `tcl pkg install` leaves one: the root
    /// package's manifest, lockfile and pack, and each dependency
    /// materialised at `lib/NAME-1.0.0` with a manifest and a pack of its own.
    fn installed_workspace(root: &Path, dependencies: &[&str]) {
        write(&root.join("tclpkg.tcl"), ROOT_MANIFEST);
        write(&root.join("app.tclspec"), "speclib app 1 {}\n");
        for name in dependencies {
            let dir = root.join(format!("lib/{name}-1.0.0"));
            write(
                &dir.join("tclpkg.tcl"),
                &format!("package {name}\nversion 1.0.0\n"),
            );
            write(
                &dir.join(format!("{name}.tclspec")),
                &format!("speclib {name} 1 {{}}\n"),
            );
        }
    }

    fn discover_workspace(root: &Path) -> Vec<PackFile> {
        discover(&DiscoveryOptions {
            workspace_roots: vec![root.to_path_buf()],
            skip_user_tier: true,
            bundled_dir: Some(root.join("no-such-bundled")),
            ..DiscoveryOptions::default()
        })
    }

    /// Each discovered pack's file stem with its tier, in name order.
    fn tiers_by_pack(found: &[PackFile]) -> Vec<(String, Option<DependencyTier>)> {
        let mut rows: Vec<(String, Option<DependencyTier>)> = found
            .iter()
            .map(|file| {
                (
                    file.path
                        .file_stem()
                        .expect("stem")
                        .to_string_lossy()
                        .into_owned(),
                    file.dependency_tier,
                )
            })
            .collect();
        rows.sort();
        rows
    }

    /// The lockfile's graph gives each package its tier: the workspace's own
    /// pack is the root, `json` (in `require`) is direct, `http` (`json`'s
    /// requirement) is transitive, and `tcltest` (in `dev-require`) is a
    /// development dependency.
    #[test]
    fn a_packs_package_is_placed_by_the_lockfiles_graph() {
        let root = tmpdir("tier-graph");
        installed_workspace(&root, &["json", "http", "tcltest"]);
        write(
            &root.join("tclpkg.lock"),
            &lockfile_text(&[
                ("json", &["http@1.0.0"], false),
                ("http", &[], false),
                ("tcltest", &[], true),
            ]),
        );

        let found = discover_workspace(&root);
        assert_eq!(
            tiers_by_pack(&found),
            vec![
                ("app".to_owned(), Some(DependencyTier::Root)),
                ("http".to_owned(), Some(DependencyTier::Transitive)),
                ("json".to_owned(), Some(DependencyTier::Direct)),
                ("tcltest".to_owned(), Some(DependencyTier::Development)),
            ],
            "{found:#?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// With no lockfile nothing is known of the graph, so no pack has a tier
    /// and none is narrowed. Once a lockfile is there, a package it does not
    /// list is no stranger to the gate: it is transitive, the least a package
    /// gets, and loses what a transitive package loses.
    #[test]
    fn no_lockfile_means_no_tier_and_an_unlisted_package_is_transitive() {
        let root = tmpdir("tier-none");
        installed_workspace(&root, &["json", "stranger"]);
        assert!(
            discover_workspace(&root)
                .iter()
                .all(|file| file.dependency_tier.is_none()),
            "no lockfile: no tier"
        );

        write(
            &root.join("tclpkg.lock"),
            &lockfile_text(&[("json", &[], false)]),
        );
        assert_eq!(
            tiers_by_pack(&discover_workspace(&root)),
            vec![
                ("app".to_owned(), Some(DependencyTier::Root)),
                ("json".to_owned(), Some(DependencyTier::Direct)),
                ("stranger".to_owned(), Some(DependencyTier::Transitive)),
            ]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A dependency's directory may carry a manifest and a lockfile of its
    /// own — its tarball can hold any file. Taking the nearest pair would let
    /// it name itself the root of its own graph, so the outermost pair, the
    /// one beside the workspace's own manifest, is the authority: the
    /// dependency stays what the workspace's lockfile says it is.
    #[test]
    fn a_dependency_shipping_its_own_lockfile_does_not_become_a_root() {
        let root = tmpdir("tier-spoof");
        installed_workspace(&root, &["json", "http"]);
        write(
            &root.join("tclpkg.lock"),
            &lockfile_text(&[("json", &["http@1.0.0"], false), ("http", &[], false)]),
        );
        // `http` claims to be a project of its own.
        let dir = root.join("lib/http-1.0.0");
        write(&dir.join("tclpkg.lock"), &lockfile_text(&[]));

        assert_eq!(
            tiers_by_pack(&discover_workspace(&root)),
            vec![
                ("app".to_owned(), Some(DependencyTier::Root)),
                ("http".to_owned(), Some(DependencyTier::Transitive)),
                ("json".to_owned(), Some(DependencyTier::Direct)),
            ]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Only a file found *beside a manifest* is placed by one. The same file
    /// named in `tclLsp.specPacks` is the user's own choice of it, and a
    /// pack under `.tcl-lsp/` is the workspace's.
    #[test]
    fn only_a_pack_beside_a_manifest_has_a_tier() {
        let root = tmpdir("tier-origin");
        installed_workspace(&root, &["json", "http"]);
        write(
            &root.join("tclpkg.lock"),
            &lockfile_text(&[("json", &["http@1.0.0"], false), ("http", &[], false)]),
        );
        write(&root.join(".tcl-lsp/local.tclspec"), "speclib local 1 {}\n");

        let found = discover(&DiscoveryOptions {
            workspace_roots: vec![root.clone()],
            configured: vec![PathBuf::from("lib/http-1.0.0/http.tclspec")],
            skip_user_tier: true,
            bundled_dir: Some(root.join("no-such-bundled")),
            ..DiscoveryOptions::default()
        });
        assert_eq!(
            tiers_by_pack(&found),
            vec![
                ("app".to_owned(), Some(DependencyTier::Root)),
                ("http".to_owned(), None),
                ("json".to_owned(), Some(DependencyTier::Direct)),
                ("local".to_owned(), None),
            ],
            "{found:#?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A manifest or a lockfile that does not read leaves the dependency's
    /// pack at the floor, transitive: nothing is granted on the strength of a
    /// file that cannot be read, and a dependency cannot lift itself by
    /// writing one that does not. A manifest that names a package the
    /// lockfile does not list is the same. The workspace's own pack is still
    /// the root's, which is a fact about where its manifest sits.
    #[test]
    fn an_unreadable_manifest_or_lockfile_is_transitive() {
        let root = tmpdir("tier-unreadable");
        installed_workspace(&root, &["json"]);
        write(&root.join("tclpkg.lock"), "{ not json");
        assert_eq!(
            tiers_by_pack(&discover_workspace(&root)),
            vec![
                ("app".to_owned(), Some(DependencyTier::Root)),
                ("json".to_owned(), Some(DependencyTier::Transitive))
            ]
        );

        write(
            &root.join("tclpkg.lock"),
            &lockfile_text(&[("json", &[], false)]),
        );
        write(
            &root.join("lib/json-1.0.0/tclpkg.tcl"),
            "not a manifest %%\n",
        );
        assert_eq!(
            tiers_by_pack(&discover_workspace(&root)),
            vec![
                ("app".to_owned(), Some(DependencyTier::Root)),
                ("json".to_owned(), Some(DependencyTier::Transitive))
            ]
        );

        write(
            &root.join("lib/json-1.0.0/tclpkg.tcl"),
            "package anything\nversion 1.0.0\n",
        );
        assert_eq!(
            tiers_by_pack(&discover_workspace(&root)),
            vec![
                ("app".to_owned(), Some(DependencyTier::Root)),
                ("json".to_owned(), Some(DependencyTier::Transitive))
            ],
            "a manifest that names an unlisted package places nothing"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A workspace folder opened inside an installed dependency is a second
    /// root holding the same pack. The outermost one decides, as the project
    /// rule takes the outermost pair, so that the dependency cannot stand as
    /// a project of its own by being the folder the search stops at, whichever
    /// order the client lists its folders in.
    #[test]
    fn nested_workspace_roots_take_the_outermost_in_either_order() {
        let root = tmpdir("tier-nested");
        installed_workspace(&root, &["json", "http"]);
        write(
            &root.join("tclpkg.lock"),
            &lockfile_text(&[("json", &["http@1.0.0"], false), ("http", &[], false)]),
        );
        let inner = root.join("lib/http-1.0.0");
        write(&inner.join("tclpkg.lock"), &lockfile_text(&[]));

        for roots in [
            vec![root.clone(), inner.clone()],
            vec![inner.clone(), root.clone()],
        ] {
            let found = discover(&DiscoveryOptions {
                workspace_roots: roots.clone(),
                skip_user_tier: true,
                bundled_dir: Some(root.join("no-such-bundled")),
                ..DiscoveryOptions::default()
            });
            let http = tiers_by_pack(&found)
                .into_iter()
                .find(|(name, _)| name == "http")
                .expect("the dependency's pack");
            assert_eq!(
                http.1,
                Some(DependencyTier::Transitive),
                "roots {roots:?}: the dependency is what the outer lockfile says"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The metadata is read through the source store, like the packs are, so
    /// a host with bytes and no filesystem places its packs the same way.
    #[test]
    fn a_host_filled_store_places_its_packs_too() {
        let store = tcl_lsp_core::vfs::MemoryStore::new();
        let root = PathBuf::from("/host/ws");
        store.upsert(root.join("tclpkg.tcl"), ROOT_MANIFEST.as_bytes().to_vec());
        store.upsert(
            root.join("tclpkg.lock"),
            lockfile_text(&[("json", &[], false)]).into_bytes(),
        );
        store.upsert(root.join("app.tclspec"), b"speclib app 1 {}\n".to_vec());
        store.upsert(
            root.join("lib/json-1.0.0/tclpkg.tcl"),
            b"package json\nversion 1.0.0\n".to_vec(),
        );
        store.upsert(
            root.join("lib/json-1.0.0/json.tclspec"),
            b"speclib json 1 {}\n".to_vec(),
        );

        let found = discover_in(
            &store,
            &DiscoveryOptions {
                workspace_roots: vec![root],
                skip_user_tier: true,
                bundled_dir: Some(PathBuf::from("/host/no-such-bundled")),
                ..DiscoveryOptions::default()
            },
        );
        assert_eq!(
            tiers_by_pack(&found),
            vec![
                ("app".to_owned(), Some(DependencyTier::Root)),
                ("json".to_owned(), Some(DependencyTier::Direct)),
            ],
            "{found:#?}"
        );
    }

    // ── a manifest's `spec` directive: which packs sit beside it, and at
    // what tier ──

    /// The file names found, in path order.
    fn names(found: &[PackFile]) -> Vec<String> {
        found
            .iter()
            .map(|file| {
                file.path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    /// A manifest that names its packs loads those and nothing else beside
    /// it — a draft, a fixture or a nested directory stays out — and the
    /// scan of a manifest that names none is as it was.
    #[test]
    fn a_manifest_that_names_its_packs_loads_those_and_no_others() {
        let root = tmpdir("spec-listed");
        write(
            &root.join("tclpkg.tcl"),
            "package app\nversion 1.0.0\nspec {packs {app.tclspec vendor/more.tclspec}}\n",
        );
        for name in ["app", "draft", "vendor/more", "vendor/scratch"] {
            write(
                &root.join(format!("{name}.tclspec")),
                "speclib listed 1 {}\n",
            );
        }
        assert_eq!(
            names(&discover_workspace(&root)),
            vec!["app.tclspec", "more.tclspec"]
        );

        // The same tree with no directive keeps every pack beside the
        // manifest.
        write(&root.join("tclpkg.tcl"), "package app\nversion 1.0.0\n");
        assert_eq!(
            names(&discover_workspace(&root)),
            vec![
                "app.tclspec",
                "draft.tclspec",
                "more.tclspec",
                "scratch.tclspec"
            ]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A pack the directive names that is not there is still discovered, so
    /// the load reports it on the file instead of the author's typo loading
    /// nothing in silence.
    #[test]
    fn a_listed_pack_that_is_missing_is_reported_by_the_load() {
        let root = tmpdir("spec-missing");
        write(
            &root.join("tclpkg.tcl"),
            "package app\nversion 1.0.0\nspec {packs {gone.tclspec}}\n",
        );
        let found = discover_workspace(&root);
        assert_eq!(names(&found), vec!["gone.tclspec"], "{found:#?}");
        let (sources, notices) = crate::pack::read_sources(&NativeStore, &found);
        assert!(sources.is_empty());
        assert_eq!(notices.len(), 1, "{notices:#?}");
        assert!(
            notices[0].message.contains("cannot read pack file"),
            "{:?}",
            notices[0]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A directive that does not read — a pack outside the package — names
    /// nothing, so the scan is the answer, exactly as for a manifest that
    /// does not read at all.
    #[test]
    fn a_directive_that_does_not_read_leaves_the_scan_in_place() {
        let root = tmpdir("spec-unreadable");
        write(
            &root.join("tclpkg.tcl"),
            "package app\nversion 1.0.0\nspec {packs {../elsewhere.tclspec}}\n",
        );
        write(&root.join("app.tclspec"), "speclib app 1 {}\n");
        write(&root.join("draft.tclspec"), "speclib draft 1 {}\n");
        assert_eq!(
            names(&discover_workspace(&root)),
            vec!["app.tclspec", "draft.tclspec"]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A dependency's own directive decides its packs, whatever the scan of
    /// the manifest above it would have found: the workspace's scan stops at
    /// the dependency's directory, and the dependency's list is read through
    /// its own manifest.
    #[test]
    fn a_dependencys_directive_decides_its_packs_whatever_the_manifest_above_scans() {
        let root = tmpdir("spec-nested");
        installed_workspace(&root, &["json"]);
        let dir = root.join("lib/json-1.0.0");
        write(
            &dir.join("tclpkg.tcl"),
            "package json\nversion 1.0.0\nspec {packs {json.tclspec}}\n",
        );
        write(&dir.join("draft.tclspec"), "speclib draft 1 {}\n");
        assert_eq!(
            names(&discover_workspace(&root)),
            vec!["app.tclspec", "json.tclspec"],
            "the dependency's draft is not its pack"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A package may ask for a tier further from the root than the graph
    /// puts it at, and is held at the graph's when it asks for a nearer one.
    /// The workspace's own package takes no request: it is the root.
    #[test]
    fn a_dependency_may_ask_for_a_further_tier_and_never_a_nearer_one() {
        let root = tmpdir("spec-tier");
        write(
            &root.join("tclpkg.tcl"),
            &format!("{ROOT_MANIFEST}spec {{packs {{app.tclspec}} tier development}}\n"),
        );
        write(&root.join("app.tclspec"), "speclib app 1 {}\n");
        write(
            &root.join("tclpkg.lock"),
            &lockfile_text(&[("json", &["http@1.0.0"], false), ("http", &[], false)]),
        );
        for (name, request) in [("json", "development"), ("http", "direct")] {
            let dir = root.join(format!("lib/{name}-1.0.0"));
            write(
                &dir.join("tclpkg.tcl"),
                &format!(
                    "package {name}\nversion 1.0.0\nspec {{packs {{{name}.tclspec}} tier {request}}}\n"
                ),
            );
            write(
                &dir.join(format!("{name}.tclspec")),
                &format!("speclib {name} 1 {{}}\n"),
            );
        }
        assert_eq!(
            tiers_by_pack(&discover_workspace(&root)),
            vec![
                ("app".to_owned(), Some(DependencyTier::Root)),
                ("http".to_owned(), Some(DependencyTier::Transitive)),
                ("json".to_owned(), Some(DependencyTier::Development)),
            ],
            "`json` is direct and asked for development; `http` is transitive and asked for direct"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
