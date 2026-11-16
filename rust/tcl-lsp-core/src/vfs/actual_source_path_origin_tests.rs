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

#[test]
fn virtual_original_path_is_provider_issued_and_never_a_native_path_grant() {
    let actual = MemoryStore::new();
    let path = Path::new("/virtual//MiXeD/source.tcl");
    let origin = actual.source_path_origin(path).unwrap();
    assert_eq!(origin.path().as_os_str(), path.as_os_str());
    assert_eq!(origin.kind(), SourceUriPathKind::Virtual);
    actual.validate_source_path_origin(&origin).unwrap();
    assert_eq!(
        MemoryStore::new().validate_source_path_origin(&origin),
        Err(SourceUriPathUnavailable::ForeignProvider)
    );
    assert!(NativeStore.validate_source_path_origin(&origin).is_err());
}

#[cfg(all(not(target_os = "wasi"), any(unix, windows)))]
#[test]
fn actual_native_source_path_keeps_original_spelling_without_uri_reconstruction() {
    let path = std::env::temp_dir().join("MiXeD-source-path.tcl");
    let origin = NativeStore.source_path_origin(&path).unwrap();
    assert_eq!(origin.path().as_os_str(), path.as_os_str());
    assert_eq!(origin.kind(), SourceUriPathKind::Native);
    NativeStore.validate_source_path_origin(&origin).unwrap();
    assert_eq!(
        MemoryStore::new().validate_source_path_origin(&origin),
        Err(SourceUriPathUnavailable::ForeignProvider)
    );
    assert!(
        NativeStore
            .source_path_origin(Path::new("relative-source.tcl"))
            .is_err()
    );
}

#[cfg(target_os = "wasi")]
#[test]
fn actual_wasi_source_path_is_a_guest_path_origin() {
    let path = Path::new("/workspace/MiXeD-source.tcl");
    let origin = NativeStore.source_path_origin(path).unwrap();
    assert_eq!(origin.kind(), SourceUriPathKind::WasiGuest);
    assert_eq!(origin.path().as_os_str(), path.as_os_str());
    NativeStore.validate_source_path_origin(&origin).unwrap();
}
