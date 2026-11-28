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

use tcl_registry::prelude::ArgRole;

const WALKED_ROLES: &[ArgRole] = &[ArgRole::Body, ArgRole::Expr, ArgRole::LambdaLiteral];

#[test]
fn the_walker_descends_into_every_script_bearing_role() {
    for &role in ArgRole::ALL {
        assert_eq!(
            role.carries_script(),
            WALKED_ROLES.contains(&role),
            "ArgRole::{role:?} carries a script but the iRules object walker does \
             not descend into it (or vice versa). An object referenced only from \
             this position would be invisible to `f5 grep`, `f5 irule trace` and \
             `f5 query rename` — and `bigip-cleanup` would report it unreferenced \
             and generate a delete for it. Teach walker.rs to recurse it, then add \
             it to WALKED_ROLES."
        );
    }
}
