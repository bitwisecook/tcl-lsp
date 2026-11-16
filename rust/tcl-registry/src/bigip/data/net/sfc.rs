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

//! BIG-IP object specs for `net sfc`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_sfc_chain",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["sfc chain"],
        },
        header_types: &[("net", "sfc chain")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hopkey",
                value_type: ValueKind::Enum,
                enum_values: &["interface", "service-index"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hops",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "path-id",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-index",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-interface",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_sfc_sf",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["sfc sf"],
        },
        header_types: &[("net", "sfc sf")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "egress-interface",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ingress-interface",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-address",
                value_type: ValueKind::String,
                allow_none: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nsh-aware",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-name",
                value_type: ValueKind::Reference,
                allow_none: true,
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
            BigipPropertySpec {
                name: "virtual-name",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "analytics_ssl_orchestrator_service_virtual_report",
                    "analytics_ssl_orchestrator_service_virtual_scheduled_report",
                    "analytics_virtual_report",
                    "analytics_virtual_scheduled_report",
                    "ltm_monitor_virtual_location",
                    "ltm_virtual",
                    "ltm_virtual_address",
                    "security_dos_virtual",
                    "security_protocol_inspection_virtual_servers",
                    "vcmp_virtual_disk",
                    "vcmp_virtual_disk_template",
                ],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
