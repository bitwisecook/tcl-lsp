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

//! BIG-IP object specs for `apm aaa active-directory`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_aaa_active_directory",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["aaa active-directory"],
    },
    header_types: &[("apm", "aaa active-directory")],
    properties: &[
        BigipPropertySpec {
            name: "admin-encrypted-password",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "admin-name",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cleanup-cache",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["group", "kerberos", "none", "pso"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "domain",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "domain-controller",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "domain-controllers",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[BigipPropertySpec {
                name: "ip",
                value_type: ValueKind::String,
                in_sections: &["domain-controllers"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip",
            value_type: ValueKind::String,
            in_sections: &["domain-controllers"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "group-cache-ttl",
            value_type: ValueKind::Integer,
            default: Some("30"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "location-specific",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "padata",
            value_type: ValueKind::Unknown,
            default: Some("rc4-hmac"),
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
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pso-cache-ttl",
            value_type: ValueKind::Integer,
            default: Some("30"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "timeout",
            value_type: ValueKind::Integer,
            default: Some("15"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
