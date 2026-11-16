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

//! Computed command names and method dispatch: W307, W308, the `TclOO` form of E001 and dispatch-table references.

use super::*;

#[path = "var_command/computed_command_names.rs"]
mod computed_command_names;
#[path = "var_command/dispatch_tables.rs"]
mod dispatch_tables;
#[path = "var_command/method_dispatch.rs"]
mod method_dispatch;
#[path = "var_command/self_dispatch.rs"]
mod self_dispatch;

/// Object-`of` constructor typing — the W307/W308 item.  Each
/// case asserts the exact set of expected diagnostic codes.
fn w30x_codes(src: &str) -> Vec<String> {
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let mut codes: Vec<String> = r
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .filter(|c| c.starts_with("W30"))
        .collect();
    codes.sort();
    codes.dedup();
    codes
}
