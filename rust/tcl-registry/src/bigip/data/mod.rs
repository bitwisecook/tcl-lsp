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

//! Hand-maintained BIG-IP object-spec data modules (one per kind-name
//! initial) — see each module's doc comment for provenance and issue #1404.
//! Aggregated by [`all_specs`]; `cargo xtask bigip-data-schema --check`
//! guards their internal consistency.
use super::BigipObjectSpec;

mod a;
mod c;
mod g;
mod i;
mod l;
mod m;
mod n;
mod p;
mod s;
mod u;
mod v;
mod w;

/// All generated BIG-IP object specs, in kind-name order.
#[must_use]
pub fn all_specs() -> Vec<&'static BigipObjectSpec> {
    let mut v: Vec<&'static BigipObjectSpec> = Vec::new();
    v.extend(a::SPECS.iter());
    v.extend(c::SPECS.iter());
    v.extend(g::SPECS.iter());
    v.extend(i::SPECS.iter());
    v.extend(l::SPECS.iter());
    v.extend(m::SPECS.iter());
    v.extend(n::SPECS.iter());
    v.extend(p::SPECS.iter());
    v.extend(s::SPECS.iter());
    v.extend(u::SPECS.iter());
    v.extend(v::SPECS.iter());
    v.extend(w::SPECS.iter());
    v
}
