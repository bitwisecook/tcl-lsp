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

//! Jim's `local` cleanup when the local command is renamed and replaced.

#[test]
fn available_local_cleanup_selects_the_actual_recreated_entry() {
    let profile = tcl_vm::test_support::profile("jim");
    let mut vm = crate::fixture::interpreter(profile);
    let source = "proc p {} {return BASE};proc driver {} {local proc p {} {return LOCAL};rename p {};proc p {} {return OTHER};return DONE};list [driver] [llength [info commands p]]";
    let result = crate::fixture::eval(&mut vm, source);
    assert!(result.code.is_ok());
    assert_eq!(result.result.string_bytes().as_ref(), b"DONE 0");
}
