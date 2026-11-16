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
fn retained_source_follows_object_lifetime_and_revokes_on_value_mutation() {
    let source = new_string_bytes(b"error BOOM");
    retain_script_location(source, Some(std::rc::Rc::from(b"source.tcl".as_slice())), 7);
    let copy = duplicate(source);
    assert_eq!(script_location(copy), script_location(source));
    string_append_inplace(copy, b" changed");
    assert!(script_location(copy).is_none());
    assert_eq!(script_location(source).unwrap().1, 7);
    // SAFETY: both fresh objects are retained once and released exactly once.
    unsafe {
        incr_ref_count(source);
        incr_ref_count(copy);
        decr_ref_count(source);
        decr_ref_count(copy);
    }
    assert!(script_location(source).is_none());
    assert!(script_location(copy).is_none());
}
