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

//! The edit list between two encoded token arrays.

use super::*;

#[test]
fn diff_returns_none_for_identical_streams() {
    let a = vec![0, 0, 4, 0, 0, 0, 5, 3, 1, 0];
    assert_eq!(diff(&a, &a), None);
}

#[test]
fn diff_isolates_a_single_changed_token() {
    // Three tokens; only the middle one's type changes.
    let old = vec![
        0, 0, 4, 0, 0, /**/ 0, 5, 3, 1, 0, /**/ 1, 0, 2, 2, 0,
    ];
    let new = vec![
        0, 0, 4, 0, 0, /**/ 0, 5, 3, 4, 0, /**/ 1, 0, 2, 2, 0,
    ];
    let edit = diff(&old, &new).expect("an edit");
    // Skip the first token (5 ints), replace exactly one token.
    assert_eq!(edit.start, 5);
    assert_eq!(edit.delete_count, 5);
    assert_eq!(edit.data, vec![0, 5, 3, 4, 0]);
}

#[test]
fn diff_handles_appended_token() {
    let old = vec![0, 0, 4, 0, 0];
    let new = vec![0, 0, 4, 0, 0, 0, 5, 3, 1, 0];
    let edit = diff(&old, &new).expect("an edit");
    // Nothing deleted; one token appended after the prefix.
    assert_eq!(edit.start, 5);
    assert_eq!(edit.delete_count, 0);
    assert_eq!(edit.data, vec![0, 5, 3, 1, 0]);
}

#[test]
fn diff_handles_removed_token() {
    let old = vec![0, 0, 4, 0, 0, 0, 5, 3, 1, 0];
    let new = vec![0, 0, 4, 0, 0];
    let edit = diff(&old, &new).expect("an edit");
    // One trailing token removed, nothing spliced in.
    assert_eq!(edit.start, 5);
    assert_eq!(edit.delete_count, 5);
    assert_eq!(edit.data, [] as [u32; 0]);
}
