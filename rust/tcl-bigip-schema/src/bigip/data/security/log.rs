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

//! BIG-IP object spec for `security log`. Hand-maintained: edit it directly.
//!
//! The kind is too large for one file, so its property table is filed by
//! ranges of top-level properties and joined here.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, join_property_tables};

mod antifraud_through_ip_intelligence;
mod nat;
mod network_through_traffic_statistics;

const PROPERTY_COUNT: usize = antifraud_through_ip_intelligence::PROPERTIES.len()
    + nat::PROPERTIES.len()
    + network_through_traffic_statistics::PROPERTIES.len();

/// The kind's properties, in filing order.
static PROPERTIES: [BigipPropertySpec; PROPERTY_COUNT] = join_property_tables(&[
    antifraud_through_ip_intelligence::PROPERTIES,
    nat::PROPERTIES,
    network_through_traffic_statistics::PROPERTIES,
]);

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "security_log_profile",
        table_name: None,
        resolver_name: None,
        module: Some("security"),
        object_types: &["log profile"],
    },
    header_types: &[("security", "log profile")],
    properties: &PROPERTIES,
}];
