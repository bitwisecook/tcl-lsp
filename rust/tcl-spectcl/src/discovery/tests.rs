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

#[test]
fn workspace_pack_discovery_prunes_the_shared_fixture() {
    let fixture = tcl_test_support::fixture::WorkspaceScan::new();
    let found = discover(&DiscoveryOptions {
        workspace_roots: vec![fixture.root().to_path_buf()],
        bundled_dir: Some(fixture.root().join("absent-bundled")),
        skip_user_tier: true,
        ..DiscoveryOptions::default()
    });
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].path, fixture.root().join("src/pack.tclspec"));
    assert_eq!(found[0].tier, Tier::Workspace);
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
fn configured_discovery_retains_exact_native_sources_and_missing_entry() {
    let root = tmpdir("configured-receipt");
    write(&root.join("packs/one.tclspec"), "speclib one 1 {}\n");
    let options = DiscoveryOptions {
        workspace_roots: vec![root.clone()],
        configured: vec![PathBuf::from("packs"), PathBuf::from("missing.tclspec")],
        skip_user_tier: true,
        bundled_dir: Some(root.join("no-bundled")),
        ..DiscoveryOptions::default()
    };
    let receipt = discover_configuration_with_active_studio_in(&NativeStore, &options, &[]);
    assert_eq!(
        receipt.files,
        discover_with_active_studio_in(&NativeStore, &options, &[])
    );
    assert_eq!(receipt.configured_sources.len(), 2);
    assert_eq!(
        receipt.configured_sources[0],
        vec![normalise(&root.join("packs/one.tclspec"))]
    );
    assert_eq!(receipt.configured_sources[1], [] as [std::path::PathBuf; 0]);
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
    assert_eq!(found, [] as [crate::discovery::PackFile; 0]);
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

/// `tcl pkg install` materialises a dependency as a symlink into the
/// package store; its pack is discovered and tiered like a copied one, and
/// reported at its workspace path.
#[cfg(unix)]
#[test]
fn an_installed_dependency_reached_through_a_symlink_is_discovered() {
    let root = tmpdir("tier-symlink");
    let store = tmpdir("tier-symlink-store");
    write(&root.join("tclpkg.tcl"), ROOT_MANIFEST);
    write(&root.join("app.tclspec"), "speclib app 1 {}\n");
    let stored = store.join("json-1.0.0");
    write(&stored.join("tclpkg.tcl"), "package json\nversion 1.0.0\n");
    write(&stored.join("json.tclspec"), "speclib json 1 {}\n");
    std::fs::create_dir_all(root.join("lib")).unwrap();
    std::os::unix::fs::symlink(&stored, root.join("lib/json-1.0.0")).unwrap();
    // A link elsewhere in the workspace is still not followed.
    std::os::unix::fs::symlink(&stored, root.join("vendored")).unwrap();
    write(
        &root.join("tclpkg.lock"),
        &lockfile_text(&[("json", &[], false)]),
    );

    let found = discover_workspace(&root);
    assert_eq!(
        tiers_by_pack(&found),
        vec![
            ("app".to_owned(), Some(DependencyTier::Root)),
            ("json".to_owned(), Some(DependencyTier::Direct)),
        ],
        "{found:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&store);
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

/// The manifest-selection issues one discovery of `root` records.
fn selection_issues(root: &Path) -> Vec<PathBuf> {
    discover_configuration_with_active_studio_in(
        &NativeStore,
        &DiscoveryOptions {
            workspace_roots: vec![root.to_path_buf()],
            skip_user_tier: true,
            bundled_dir: Some(root.join("no-such-bundled")),
            ..DiscoveryOptions::default()
        },
        &[],
    )
    .manifest_selection
    .issues
    .into_iter()
    .map(|(path, _)| path)
    .collect()
}

/// A project lockfile or a dependency manifest that does not read selects
/// nothing: the dependency's pack is dropped and the selection issue is
/// recorded, rather than the pack being placed on the strength of a file
/// that cannot be read. The workspace's own pack is still the root's,
/// which is a fact about where its manifest sits. A manifest that reads but
/// names a package the lockfile does not list is not unreadable input, and
/// stays at the transitive floor.
#[test]
fn an_unreadable_manifest_or_lockfile_selects_nothing() {
    // Selection fails closed on unreadable input (user decision 2026-10-28T1450Z).
    let root = tmpdir("tier-unreadable");
    installed_workspace(&root, &["json"]);
    write(&root.join("tclpkg.lock"), "{ not json");
    assert_eq!(
        tiers_by_pack(&discover_workspace(&root)),
        vec![("app".to_owned(), Some(DependencyTier::Root))]
    );
    assert!(
        selection_issues(&root)
            .iter()
            .any(|path| path.ends_with("tclpkg.lock")),
        "the unreadable lockfile is an issue"
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
        vec![("app".to_owned(), Some(DependencyTier::Root))]
    );
    assert!(
        selection_issues(&root)
            .iter()
            .any(|path| path.ends_with("lib/json-1.0.0/tclpkg.tcl")),
        "the unreadable dependency manifest is an issue"
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
    assert_eq!(sources, [] as [(crate::discovery::PackFile, String); 0]);
    assert_eq!(notices.len(), 1, "{notices:#?}");
    assert!(
        notices[0].message.contains("cannot read pack file"),
        "{:?}",
        notices[0]
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A directive that does not read — a pack outside the package — selects
/// nothing, exactly as for a manifest that does not read at all: no scan
/// stands in for it, and the selection issue is recorded.
#[test]
fn a_directive_that_does_not_read_selects_nothing() {
    // Selection fails closed on unreadable input (user decision 2026-10-28T1450Z).
    let root = tmpdir("spec-unreadable");
    write(
        &root.join("tclpkg.tcl"),
        "package app\nversion 1.0.0\nspec {packs {../elsewhere.tclspec}}\n",
    );
    write(&root.join("app.tclspec"), "speclib app 1 {}\n");
    write(&root.join("draft.tclspec"), "speclib draft 1 {}\n");
    assert_eq!(names(&discover_workspace(&root)), Vec::<&str>::new());
    assert!(
        selection_issues(&root)
            .iter()
            .any(|path| path.ends_with("tclpkg.tcl")),
        "the unreadable directive is an issue"
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
