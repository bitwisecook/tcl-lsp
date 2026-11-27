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

//! BIG-IP object spec for `security anti-fraud profile`. Hand-maintained: edit it directly.
//!
//! The kind is too large for one file, so its property table is filed by
//! ranges of top-level properties and joined here.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, join_property_tables};

mod alert_client_side_caching_through_inject_javascript;
mod inject_javascript_removal_through_mobilesafe;
mod mobilesafe_encryption_through_suggested_username_header;
mod trigger_irule_through_whitelist_custom_alerts;

const PROPERTY_COUNT: usize = alert_client_side_caching_through_inject_javascript::PROPERTIES.len()
    + inject_javascript_removal_through_mobilesafe::PROPERTIES.len()
    + mobilesafe_encryption_through_suggested_username_header::PROPERTIES.len()
    + trigger_irule_through_whitelist_custom_alerts::PROPERTIES.len();

/// The kind's properties, in filing order.
static PROPERTIES: [BigipPropertySpec; PROPERTY_COUNT] = join_property_tables(&[
    alert_client_side_caching_through_inject_javascript::PROPERTIES,
    inject_javascript_removal_through_mobilesafe::PROPERTIES,
    mobilesafe_encryption_through_suggested_username_header::PROPERTIES,
    trigger_irule_through_whitelist_custom_alerts::PROPERTIES,
]);

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "security_anti_fraud_profile",
        table_name: None,
        resolver_name: None,
        module: Some("security"),
        object_types: &["anti-fraud profile"],
    },
    header_types: &[("security", "anti-fraud profile")],
    properties: &PROPERTIES,
}];
