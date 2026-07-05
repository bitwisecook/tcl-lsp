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

//! [incr Tcl] (`itcl`) command specifications — the `itcl::class` class
//! definer, external body definers (`itcl::body` / `itcl::configbody`), and the
//! runtime helpers. Folded into the base registry (like Tk) so `itcl::class`
//! bodies are recognised under every Tcl dialect once `package require Itcl`.

use crate::spec::CommandSpec;

mod itcl_body;
mod itcl_class;
mod itcl_code;
mod itcl_configbody;
mod itcl_delete;
mod itcl_ensemble;
mod itcl_find;
mod itcl_is;
mod itcl_local;
mod itcl_scope;

/// Every [incr Tcl] command spec.
#[must_use]
pub fn itcl_command_specs() -> Vec<CommandSpec> {
    vec![
        itcl_body::spec(),
        itcl_class::spec(),
        itcl_code::spec(),
        itcl_configbody::spec(),
        itcl_delete::spec(),
        itcl_ensemble::spec(),
        itcl_find::spec(),
        itcl_is::spec(),
        itcl_local::spec(),
        itcl_scope::spec(),
    ]
}
