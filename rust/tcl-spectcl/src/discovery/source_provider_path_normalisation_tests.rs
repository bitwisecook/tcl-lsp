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
use tcl_lsp_core::vfs::{MemoryStore, NativeStore};

#[test]
fn virtual_source_spelling_cannot_borrow_coincident_host_canonical_path() {
    let path = Path::new(".");
    let virtual_store = MemoryStore::new();
    assert_eq!(normalise_in(&virtual_store, path), path);
    let host = std::fs::canonicalize(path).unwrap();
    assert_ne!(host, path);
    assert_eq!(normalise_in(&NativeStore, path), host);
    let missing = path.join("actual-missing-provider-path-54c6c147.tclspec");
    assert_eq!(normalise_in(&virtual_store, &missing), missing);
}
