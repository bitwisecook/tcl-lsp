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
//! `docs/design/spec-packs.md` fixes both the tiers and their order —
//! **nearest wins: workspace > user > bundled**:
//!
//! - **workspace** — the `tclLsp.specPacks` setting (mirrored in every editor
//!   integration), plus `*.tclspec` under a `.tcl-lsp/` directory or beside a
//!   `tclpkg.tcl` manifest.
//! - **user** — packs dropped in the platform config directory
//!   (`$XDG_CONFIG_HOME/tcl-lsp/specs/` and the macOS / Windows equivalents),
//!   loaded for every workspace. The directory comes from [`tcl_userdirs`],
//!   which is the same machinery `tcl pkg` reads its own config layer from.
//! - **bundled** — the shipped loadables (the EDA vendor libraries), so the
//!   loader path is exercised in production rather than reserved for private
//!   packs.
//!
//! Discovery answers *which files*, in a deterministic order. It never reads
//! or parses them — deciding which file belongs to which pack, and which pack
//! wins, is [`crate::pack`]'s job, because that needs each file's `speclib`
//! name and therefore a parse.
//!
//! ## Determinism
//!
//! Every directory scan sorts its results by path before appending, so two
//! runs over the same tree produce the same list in the same order on every
//! platform. That matters beyond tidiness: the merge order of a multi-file
//! pack *is* this order, and so is the compiled-cache key.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use tcl_lsp_core::vfs::{NativeStore, SourceStore};

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
    pub fn label(self) -> &'static str {
        match self {
            Tier::StudioOverride => "Spec Studio override",
            Tier::Workspace => "workspace",
            Tier::User => "user",
            Tier::Bundled => "bundled",
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
}

/// The directory a workspace folder keeps its own packs in.
pub const WORKSPACE_PACK_DIR: &str = ".tcl-lsp";

/// Hidden session directory used by native editor-hosted Spec Studio panels.
pub const STUDIO_OVERRIDE_DIR: &str = ".spec-studio";

/// The package manifest whose directory is scanned for sibling packs.
pub const PACKAGE_MANIFEST: &str = "tclpkg.tcl";

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
        for dir in manifest_dirs(store, root) {
            collect_dir(
                store,
                &dir,
                Tier::Workspace,
                Origin::BesideManifest,
                &mut found,
            );
        }
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
    found
        .into_iter()
        .filter(|file| seen.insert(file.path.clone()))
        .collect()
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
        });
    }
}

/// Add every `.tclspec` under `dir`, recursively.
///
/// Recursive because a pack is a logical unit an author groups however they
/// like (`docs/design/spec-packs.md`, "Loading and tooling") — one file per
/// namespace in subdirectories is a shape the design explicitly invites.
fn collect_dir(
    store: &dyn SourceStore,
    dir: &Path,
    tier: Tier,
    origin: Origin,
    out: &mut BTreeSet<PackFile>,
) {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = store.read_dir(&current) else {
            continue;
        };
        for entry in entries {
            if entry.is_dir {
                if !is_skipped_dir(&entry.path) {
                    stack.push(entry.path);
                }
            } else if entry.is_file && is_pack_file(&entry.path) {
                out.insert(PackFile {
                    tier,
                    path: normalise(&entry.path),
                    origin,
                });
            }
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
        let loaded = crate::bundled::load_discovered_in(&store, &found);
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
}
