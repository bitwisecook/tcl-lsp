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

//! BIG-IP object specs for `gtm region`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "gtm_region",
        table_name: None,
        resolver_name: None,
        module: Some("gtm"),
        object_types: &["region"],
    },
    header_types: &[("gtm", "region")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "continent",
            value_type: ValueKind::Enum,
            enum_values: &[
                "Africa",
                "Antarctica",
                "Asia",
                "Australia",
                "Europe",
                "unknown",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "country",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "datacenter",
            value_type: ValueKind::Reference,
            references: &["gtm_datacenter"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "geoip-isp",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "isp",
            value_type: ValueKind::Enum,
            enum_values: &[
                "AOL",
                "BeijingCNC",
                "CNC",
                "ChinaTelecom",
                "Comcast",
                "Earthlink",
                "ShanghaiCNC",
                "ShanghaiTelecom",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "not",
            value_type: ValueKind::Enum,
            enum_values: &[
                "continent",
                "country",
                "datacenter",
                "isp",
                "pool",
                "subnet",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pool",
            value_type: ValueKind::Reference,
            references: &[
                "gtm_pool_a",
                "gtm_pool_aaaa",
                "gtm_pool_cname",
                "gtm_pool_https",
                "gtm_pool_mx",
                "gtm_pool_naptr",
                "gtm_pool_srv",
                "gtm_pool_svcb",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "region-members",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "region-name",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "state",
            value_type: ValueKind::Reference,
            references: &[
                "security_firewall_current_state",
                "sys_mcp_state",
                "sys_state_mirroring",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "subnet",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
