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

//! The routes a pack set publishes for the extensions its packs claim.

use std::path::{Path, PathBuf};

use tcl_spectcl::discovery::{Origin, PackFile, Tier};
use tcl_spectcl::pack::{self, PackSet};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tcl-spectcl-extension-routes-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn write_pack(root: &Path, name: &str, body: &str) -> PathBuf {
    let dir = root.join(".tcl-lsp");
    std::fs::create_dir_all(&dir).expect("pack dir");
    let path = dir.join(name);
    std::fs::write(&path, body).expect("write pack");
    path
}

fn load_files(paths: &[PathBuf]) -> PackSet {
    let files: Vec<PackFile> = paths
        .iter()
        .map(|path| PackFile {
            path: path.clone(),
            tier: Tier::Workspace,
            origin: Origin::DotDir,
            dependency_tier: None,
        })
        .collect();
    pack::load(&files)
}

/// A `file_extension` with no `-dialect` is still a source extension the pack
/// claims: published with no dialect so the workspace indexes it and content
/// decides the dialect, and never in place of another pack's `-dialect` row.
#[test]
fn a_dialect_less_extension_claim_is_published_with_no_dialect() {
    let root = scratch("extension-plain");
    let a = write_pack(
        &root,
        "alpha.tclspec",
        "speclib alpha 1.1 {\n\
        \x20 file_extension plainx -name {Plain}\n\
        \x20 file_extension sharedx -name {Shared}\n\
        \x20 command alpha::cmd { arity 1 }\n\
        }\n",
    );
    let b = write_pack(
        &root,
        "beta.tclspec",
        "speclib beta 1.1 {\n\
        \x20 file_extension sharedx -name {Shared} -dialect tcl8.6\n\
        \x20 command beta::cmd { arity 1 }\n\
        }\n",
    );
    let set = load_files(&[a, b]);
    let routes = tcl_spectcl::registration::extension_routes(&set);
    assert!(
        routes.contains(&("plainx".to_owned(), None)),
        "a dialect-less claim is published as a plain source extension: {routes:?}"
    );
    assert!(
        routes.contains(&("sharedx".to_owned(), Some("tcl8.6"))),
        "a `-dialect` row beats another pack's dialect-less claim: {routes:?}"
    );
    assert_eq!(
        routes.iter().filter(|(ext, _)| ext == "sharedx").count(),
        1,
        "{routes:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}
