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

//! The packs a package ships, named by its manifest's `spec` directive, and
//! the one hash the lockfile and a compiled unit's claims both identify them
//! by (`docs/design/tclpkg/architecture.md` § *Lockfile*).

use std::path::{Path, PathBuf};

use tcl_lsp_core::vfs::NativeStore;
use tcl_pkg_model::lockfile::parse_spec_integrity;
use tcl_pkg_model::manifest::load_manifest_text;
use tcl_spectcl::discovery::{DiscoveryOptions, Origin, PackFile, Tier, discover};
use tcl_spectcl::pack;
use tcl_spectcl::package_specs::{pack_file_hash, spec_integrity};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tcl-spectcl-package-specs-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir");
    }
    std::fs::write(path, text).expect("write");
}

const MANIFEST: &str =
    "package shipper\nversion 1.0.0\nspec {packs {alpha.tclspec sub/beta.tclspec}}\n";

const ALPHA: &str = "speclib alpha 1 {\n    command alpha::one { arity 0 }\n}\n";

/// `beta` brings its commands in from a fragment beside it.
const BETA: &str =
    "speclib beta 2.0 {\n    include more.frag\n    command beta::one { arity 0 }\n}\n";

fn fragment(command: &str) -> String {
    format!("command beta::{command} {{ arity 0 }}\n")
}

/// A package directory: a manifest naming two packs, one of which includes a
/// fragment.
fn package(name: &str) -> PathBuf {
    let dir = scratch(name);
    write(&dir.join("tclpkg.tcl"), MANIFEST);
    write(&dir.join("alpha.tclspec"), ALPHA);
    write(&dir.join("sub/beta.tclspec"), BETA);
    write(&dir.join("sub/more.frag"), &fragment("two"));
    dir
}

fn directive(dir: &Path) -> tcl_pkg_model::manifest::SpecDirective {
    let text = std::fs::read_to_string(dir.join("tclpkg.tcl")).expect("manifest");
    load_manifest_text(&text, None)
        .expect("the manifest reads")
        .spec
        .expect("the manifest has a directive")
}

fn recorded(dir: &Path) -> Vec<u64> {
    let text = spec_integrity(&NativeStore, dir, &directive(dir)).expect("the packs read");
    parse_spec_integrity(&text).expect("the value reads back")
}

/// The content hash each command of a pack carries into a unit's claims, by
/// the pack's name: the stamps a VM holds for the loaded set.
fn stamped(dir: &Path) -> Vec<(String, u64)> {
    let files = discover(&DiscoveryOptions {
        workspace_roots: vec![dir.to_path_buf()],
        user_dir: Some(dir.join("no-user-tier")),
        bundled_dir: Some(dir.join("no-bundled-tier")),
        ..DiscoveryOptions::default()
    });
    let set = pack::load(&files);
    assert!(
        set.notices.is_empty(),
        "the fixture packs load cleanly: {:#?}",
        set.notices
    );
    let mut rows: Vec<(String, u64)> = set
        .fact_stamps(0)
        .into_iter()
        .map(|stamp| (stamp.pack, stamp.content_hash))
        .collect();
    rows.sort();
    rows.dedup();
    rows
}

/// The lockfile's hash of a package's packs is the content hash its claims
/// carry — one value, so the package manager's record and an artefact's
/// stamp agree without a second hashing rule. A pack that includes a
/// fragment is hashed with it: editing the fragment moves both together, and
/// a pack that included nothing keeps its own.
#[test]
fn the_lockfile_hash_and_the_artefact_stamp_are_one_value() {
    let dir = package("one-value");
    let hashes = recorded(&dir);
    assert_eq!(hashes.len(), 2, "one hash per pack: {hashes:?}");
    let stamps = stamped(&dir);
    assert_eq!(
        stamps
            .iter()
            .map(|(pack, _)| pack.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "beta"]
    );
    // The manifest names `alpha` first, then `beta`.
    assert_eq!(
        hashes,
        [stamps[0].1, stamps[1].1],
        "the lockfile's entries are the stamps' content hashes, in the manifest's order"
    );
    assert_ne!(hashes[0], hashes[1]);

    // The pack that includes a fragment is not hashed by its own bytes.
    let beta_source = std::fs::read_to_string(dir.join("sub/beta.tclspec")).expect("beta");
    assert_ne!(
        hashes[1],
        pack_file_hash(Path::new("elsewhere/beta.tclspec"), "speclib beta 1 {}\n"),
    );
    assert_ne!(
        hashes[1],
        tcl_spectcl::loader::eval_snapshot_key(
            &beta_source,
            &tcl_spectcl::loader::EvalOptions::default()
        )
        .content_hash,
        "the fragment is folded in"
    );

    // Edit the fragment: the including pack's hash moves in both places and
    // the other pack's does not.
    write(&dir.join("sub/more.frag"), &fragment("three"));
    let edited = recorded(&dir);
    let restamped = stamped(&dir);
    assert_eq!(edited[0], hashes[0], "`alpha` included nothing");
    assert_ne!(edited[1], hashes[1], "`beta` moved with its fragment");
    assert_eq!(edited, [restamped[0].1, restamped[1].1]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The value follows the directive's order, not the paths': reversing the
/// manifest's list reverses the entries.
#[test]
fn the_hashes_follow_the_order_the_manifest_names_the_packs() {
    let dir = package("order");
    let forward = recorded(&dir);
    write(
        &dir.join("tclpkg.tcl"),
        "package shipper\nversion 1.0.0\nspec {packs {sub/beta.tclspec alpha.tclspec}}\n",
    );
    let reversed = recorded(&dir);
    assert_eq!(reversed, [forward[1], forward[0]]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A pack the directive names that cannot be read is an error that names the
/// file, never a hash of nothing.
#[test]
fn a_pack_that_cannot_be_read_is_named() {
    let dir = package("unreadable");
    std::fs::remove_file(dir.join("alpha.tclspec")).expect("remove");
    let error = spec_integrity(&NativeStore, &dir, &directive(&dir)).expect_err("alpha is gone");
    assert!(error.to_string().contains("alpha.tclspec"), "{error}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// What `discover` finds beside the manifest is what the directive lists, and
/// the load of those files is the set the stamps above are read from.
#[test]
fn discovery_finds_the_packs_the_hash_covers() {
    let dir = package("covers");
    write(&dir.join("draft.tclspec"), "speclib draft 1 {}\n");
    let found: Vec<(Tier, Origin, PathBuf)> = discover(&DiscoveryOptions {
        workspace_roots: vec![dir.clone()],
        user_dir: Some(dir.join("no-user-tier")),
        bundled_dir: Some(dir.join("no-bundled-tier")),
        ..DiscoveryOptions::default()
    })
    .into_iter()
    .map(|file: PackFile| (file.tier, file.origin, file.path))
    .collect();
    let names: Vec<String> = found
        .iter()
        .map(|(_, _, path)| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["alpha.tclspec", "beta.tclspec"], "{found:#?}");
    assert!(
        found
            .iter()
            .all(|(tier, origin, _)| *tier == Tier::Workspace && *origin == Origin::BesideManifest)
    );
    let _ = std::fs::remove_dir_all(&dir);
}
