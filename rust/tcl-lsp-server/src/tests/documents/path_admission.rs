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

//! Canonical file URIs and the path admission of constructed ones.

use super::*;

#[cfg(unix)]
#[test]
fn canonical_file_uri_declines_opaque_actual_path_without_replacement_alias() {
    use std::os::unix::{ffi::OsStringExt, fs::symlink};
    let workspace = TmpWs::new("uri-native-path");
    let opaque = workspace
        .0
        .join(std::ffi::OsString::from_vec(b"opaque-\xff.tcl".to_vec()));
    let replacement = workspace.0.join("opaque-\u{fffd}.tcl");
    let unicode = workspace.0.join("space é.tcl");
    for path in [&opaque, &replacement, &unicode] {
        std::fs::write(path, "original fixture").unwrap();
    }
    assert!(canonical_file_uri(&opaque).is_none());
    let replacement_uri =
        canonical_file_uri(&replacement).expect("literal replacement character is Unicode");
    assert!(replacement_uri.as_str().ends_with("/opaque-%EF%BF%BD.tcl"));
    assert!(
        canonical_file_uri(&unicode)
            .unwrap()
            .as_str()
            .ends_with("/space%20%C3%A9.tcl")
    );

    let link = workspace.0.join("ascii-link.tcl");
    symlink(&opaque, &link).unwrap();
    assert!(
        canonical_file_uri(&link).is_some(),
        "absolute symlinks keep their written path"
    );
    let cwd = std::env::current_dir().unwrap();
    let mut relative = PathBuf::new();
    for _ in cwd.components().skip(1) {
        relative.push("..");
    }
    relative.push(link.strip_prefix("/").unwrap());
    assert!(!relative.is_absolute());
    assert!(relative.to_str().is_some());
    assert_eq!(
        std::fs::canonicalize(&relative).unwrap(),
        std::fs::canonicalize(&opaque).unwrap()
    );
    assert!(
        canonical_file_uri(&relative).is_none(),
        "admission must inspect the actual canonical target, not the ASCII symlink spelling"
    );
}

#[cfg(unix)]
#[test]
fn canonical_file_uri_preserves_absolute_and_relative_existence_policy() {
    let workspace = TmpWs::new("uri-existence");
    let absent = workspace.0.join("absent é.tcl");
    assert!(!absent.exists());
    assert!(
        canonical_file_uri(&absent)
            .unwrap()
            .as_str()
            .ends_with("/absent%20%C3%A9.tcl")
    );
    let cwd = std::env::current_dir().unwrap();
    let mut relative = PathBuf::new();
    for _ in cwd.components().skip(1) {
        relative.push("..");
    }
    relative.push(absent.strip_prefix("/").unwrap());
    assert!(canonical_file_uri(&relative).is_none());
    std::fs::write(&absent, "original fixture").unwrap();
    assert_eq!(canonical_file_uri(&relative), canonical_file_uri(&absent));
}

#[cfg(windows)]
#[test]
fn canonical_file_uri_declines_a_verbatim_volume_instead_of_redirecting_it() {
    let original = Path::new(r"\\?\Volume{01234567-89ab-cdef-0123-456789abcdef}\project\file.tcl");
    assert!(original.is_absolute());
    let legacy = crate::uri_norm::legacy_provider_file_uri(original).unwrap();
    let repaired = uri_norm::repair_file_uri_from_path(legacy.as_str(), true).unwrap();
    let legacy_uri = Uri::from_str(&repaired).unwrap();
    let redirected = legacy_uri.to_file_path().unwrap();
    assert!(
        !redirected.is_absolute(),
        "{repaired} produced {redirected:?}"
    );
    assert_ne!(redirected.as_ref(), original);
    assert!(canonical_file_uri(original).is_none());
}

#[cfg(unix)]
#[test]
fn canonical_file_uri_preserves_a_posix_question_mark_segment() {
    assert_eq!(
        canonical_file_uri("/?/foo.tcl").unwrap().as_str(),
        "file:///%3F/foo.tcl",
    );
}

#[cfg(unix)]
#[test]
fn canonical_file_uri_matches_vscode_for_a_posix_double_slash_path() {
    assert_eq!(
        canonical_file_uri("//srv/share/a.tcl").unwrap().as_str(),
        "file://srv/share/a.tcl",
    );
}

#[cfg(unix)]
#[test]
fn constructed_product_alias_retains_the_original_counted_path_for_admission() {
    for spelling in [
        "//srv/share/a.tcl",
        "///local/path.tcl",
        "//server",
        "//?/é.tcl",
    ] {
        let actual = Path::new(spelling);
        let constructed = constructed_file_uri(actual).unwrap();
        assert_eq!(constructed.path.as_os_str(), actual.as_os_str());
        assert_eq!(Some(constructed.uri.clone()), canonical_file_uri(actual));
        let admitted = constructed.admission(&vfs::NativeStore).unwrap();
        assert_eq!(admitted.path().unwrap().as_os_str(), actual.as_os_str());
        admitted
            .check_for(&constructed.uri, &vfs::NativeStore)
            .unwrap();
        // A virtual registration is an independent stronger owner. The
        // forward alias cannot replace its different selected Path.
        let virtual_store = vfs::MemoryStore::new();
        virtual_store.upsert_uri(
            constructed.uri.as_str(),
            "/different/source.tcl",
            Vec::new(),
        );
        assert!(constructed.admission(&virtual_store).is_err());
    }
}
