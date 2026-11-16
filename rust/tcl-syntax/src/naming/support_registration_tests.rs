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
fn support_registration_retains_counted_labels_and_last_nonempty_component() {
    for name in [b"".as_slice(), b"::", b"::::"] {
        assert!(support_command_registration_slot(name).is_none());
    }
    for (name, holder, tail) in [
        (b"::A:::b::".as_slice(), b"A".as_slice(), b"b".as_slice()),
        (b"\xff::k\0tail::", b"\xff", b"k\0tail"),
        (b"A:::b", b"A", b"b"),
        (b"$literal::member", b"$literal", b"member"),
    ] {
        let slot = support_command_registration_slot(name).unwrap();
        assert_eq!(slot.namespace.as_segments().len(), 1);
        assert_eq!(slot.namespace.as_segments()[0].as_bytes(), holder);
        assert_eq!(slot.simple.as_bytes(), tail);
    }
}
