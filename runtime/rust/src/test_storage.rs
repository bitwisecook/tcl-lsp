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

//! Readonly observers of actual owned storage, including after a Host refusal.
//! These observers grant no name resolution, getter or native object access.

use crate::{interp::Interp, namespace::GLOBAL, obj::TclObj};

/// Observe a known actual global scalar tail without executing variable lookup.
pub(crate) fn global_scalar(interpreter: &Interp, tail: &[u8]) -> Option<*mut TclObj> {
    interpreter.namespaces().var_table(GLOBAL).load_scalar(tail)
}
