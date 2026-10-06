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

pub const CASES: &[&str] = &[
    "namespace current",
    "namespace current extra",
    "namespace origin $left",
    "namespace origin missing",
    "namespace code {puts X}",
    "namespace code $left",
    "namespace code {::namespace inscope }",
    "namespace code {::namespace inscope ::N X}",
    "info level",
    "info level 0",
    "info level $left",
    "info level 2147483648",
    "info level -2147483649",
    "info level invalid",
    "info level 0 extra",
    "array exists a",
    "array exists $left",
    "array exists a(k)",
    "array set a {}",
    "array set ::N::a {}",
    "array set a {k V j W}",
    "array set a {k}",
    "array set a $right",
    "array set $left {}",
    "array unset a",
    "array unset $left",
    "array unset a k",
    "set a scalar; array set a {}",
    "set a scalar; array unset a; set a",
    "array set a {k V}; array unset a; array exists a",
    "array set a {k V}; array unset a k; array names a",
];
