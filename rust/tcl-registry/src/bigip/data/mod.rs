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

//! Hand-maintained BIG-IP object-spec data modules (one per tmsh module
//! word) — see each module's doc comment for provenance.
//! Aggregated by [`all_specs`]; `cargo xtask bigip-data-schema --check`
//! guards their internal consistency.
use super::BigipObjectSpec;

mod analytics;
mod api_protection;
mod apm;
mod asm;
mod auth;
mod cli;
mod cm;
mod gtm;
mod ilx;
mod ltm;
mod mgmt;
mod net;
mod pem;
mod saas;
mod security;
mod sys;
mod util;
mod vcmp;
mod wam;
mod wom;

/// All generated BIG-IP object specs, in kind-name order.
#[must_use]
pub fn all_specs() -> Vec<&'static BigipObjectSpec> {
    let mut v: Vec<&'static BigipObjectSpec> = Vec::new();
    v.extend(analytics::tables().into_iter().flatten());
    v.extend(api_protection::SPECS.iter());
    v.extend(apm::tables().into_iter().flatten());
    v.extend(asm::SPECS.iter());
    v.extend(auth::SPECS.iter());
    v.extend(cli::SPECS.iter());
    v.extend(cm::SPECS.iter());
    v.extend(gtm::tables().into_iter().flatten());
    v.extend(ilx::SPECS.iter());
    v.extend(ltm::tables().into_iter().flatten());
    v.extend(mgmt::SPECS.iter());
    v.extend(net::tables().into_iter().flatten());
    v.extend(pem::tables().into_iter().flatten());
    v.extend(saas::SPECS.iter());
    v.extend(security::tables().into_iter().flatten());
    v.extend(sys::tables().into_iter().flatten());
    v.extend(util::SPECS.iter());
    v.extend(vcmp::SPECS.iter());
    v.extend(wam::tables().into_iter().flatten());
    v.extend(wom::SPECS.iter());
    // A module filed by component lists its tables in filing order.
    v.sort_by_key(|spec| spec.kind_spec.kind);
    v
}
