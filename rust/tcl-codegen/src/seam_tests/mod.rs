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

//! Tests that drive `tcl-compiler` internals through the emitters. They lived
//! beside the code they test in `tcl-compiler`; that crate cannot reach the
//! emitters any more, so they run here.

#[allow(unused_imports)]
mod cfg_builder;
#[allow(unused_imports)]
mod native_control_carrier;
#[allow(unused_imports)]
mod ordered_locals;
#[allow(unused_imports)]
mod runtime_entry;
#[allow(unused_imports)]
mod semantic_optimisation;
#[allow(unused_imports)]
mod site_claims;
