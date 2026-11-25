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

//! `source` path resolution, entry points and constant folding.

use super::*;

#[test]
fn source_uri_resolution_matches_from_file_path() {
    // The resolver must produce the exact URI string the index keys the
    // child document by, or the graph edge won't connect.
    let app = crate::canonical_file_uri("/proj/app.tcl").unwrap();
    let child = resolve_source_uri(app.as_str(), "lib/util.tcl").unwrap();
    let expected = crate::canonical_file_uri("/proj/lib/util.tcl").unwrap();
    assert_eq!(child, expected.as_str());
}

#[test]
fn entry_point_paths_resolve_relative_and_absolute() {
    let root = PathBuf::from("/proj");
    let rel = entry_point_uri("src/main.tcl", Some(&root)).unwrap();
    assert_eq!(
        rel,
        crate::canonical_file_uri("/proj/src/main.tcl")
            .unwrap()
            .as_str()
    );
    let abs = entry_point_uri("/opt/app.tcl", Some(&root)).unwrap();
    assert_eq!(
        abs,
        crate::canonical_file_uri("/opt/app.tcl").unwrap().as_str()
    );
    // A relative entry with no folder root can't be resolved.
    assert!(entry_point_uri("main.tcl", None).is_none());
}

/// A statically-foldable computed source path
/// (`[file join [file dirname [info script]] b.tcl]`) resolves like a
/// literal; an unfoldable one abstains.
#[tokio::test]
async fn computed_source_paths_fold_statically_m9() {
    let backend = test_backend();
    let a = Uri::from_str("file:///proj/a.tcl").unwrap();
    let b = Uri::from_str("file:///proj/b.tcl").unwrap();
    register(&backend, &b, "proc helper {} {}\n").await;
    let a_src = "namespace eval ::x {\n    source [file join [file dirname [info script]] b.tcl]\n}\n::x::helper\n";
    register(&backend, &a, a_src).await;
    let analysis = {
        let mut an = Analyser::new();
        an.analyse(a_src, "tcl8.6").clone()
    };
    let refs = backend
        .cross_document_references(&a, a_src, &analysis, Position::new(3, 3), true)
        .await
        .unwrap();
    assert!(
        refs.iter().any(|l| l.uri == b && l.range.start.line == 0),
        "the folded path re-homes the sourced file: {refs:?}"
    );
    // Unfoldable: a `$var` path must abstain (no re-homing, no panic).
    let c = Uri::from_str("file:///proj/c.tcl").unwrap();
    register(&backend, &c, "namespace eval ::y { source $dynamic }\n").await;
    backend.refresh_source_rehoming().await.unwrap();
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .workspace_command_exists("::y::helper"),
        "an unfoldable path must not re-home anything"
    );
}
