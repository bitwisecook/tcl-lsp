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

//! BIG-IP object specs for `net route`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "net_route",
        table_name: None,
        resolver_name: None,
        module: Some("net"),
        object_types: &["route"],
    },
    header_types: &[("net", "route")],
    properties: &[
        BigipPropertySpec {
            name: "blackhole",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "gw",
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            value_type: ValueKind::String,
            references: &["net_self"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "interface",
            value_type: ValueKind::Reference,
            references: &["net_interface", "net_interface_cos", "net_interface_ddm"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mtu",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "network",
            address_grammar: Some(crate::bigip::AddressGrammar::Network),
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::IpAddress),
            address_validation: crate::bigip::AddressValidation::RouteDomain21_1,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pool",
            value_type: ValueKind::Reference,
            references: &[
                "analytics_lsn_pool_report",
                "analytics_lsn_pool_scheduled_report",
                "analytics_pool_traffic_report",
                "analytics_pool_traffic_scheduled_report",
                "gtm_pool_a",
                "gtm_pool_aaaa",
                "gtm_pool_cname",
                "gtm_pool_https",
                "gtm_pool_mx",
                "gtm_pool_naptr",
                "gtm_pool_srv",
                "gtm_pool_svcb",
                "gtm_prober_pool",
                "ltm_lsn_pool",
                "ltm_pool",
            ],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
