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

//! BIG-IP object specs for `gtm server`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "gtm_server",
        table_name: None,
        resolver_name: None,
        module: Some("gtm"),
        object_types: &["server"],
    },
    header_types: &[("gtm", "server")],
    properties: &[
        BigipPropertySpec {
            name: "addresses",
            value_type: ValueKind::Unknown,
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
            name: "datacenter",
            value_type: ValueKind::Reference,
            required: true,
            references: &["gtm_datacenter"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "depends-on",
            value_type: ValueKind::Unknown,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "destination",
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "devices",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "explicit-link-name",
            value_type: ValueKind::Reference,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "expose-route-domains",
            value_type: ValueKind::Enum,
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("no"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "iq-allow-path",
            value_type: ValueKind::Enum,
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("yes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "iq-allow-service-check",
            value_type: ValueKind::Enum,
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("yes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "iq-allow-snmp",
            value_type: ValueKind::Enum,
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("yes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "iquery-cipher-list",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "iquery-minimum-tls-version",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-cpu-usage",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-cpu-usage-status",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-max-bps",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-max-bps-status",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-max-connections",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-max-connections-status",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-max-pps",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-max-pps-status",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-mem-avail",
            value_type: ValueKind::Integer,
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-mem-avail-status",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "link-discovery",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ltm-name",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "metadata",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "monitor",
            value_type: ValueKind::Unknown,
            repeated: true,
            references: &[
                "gtm_monitor_bigip",
                "gtm_monitor_bigip_link",
                "gtm_monitor_external",
                "gtm_monitor_firepass",
                "gtm_monitor_ftp",
                "gtm_monitor_gateway_icmp",
                "gtm_monitor_gtp",
                "gtm_monitor_http",
                "gtm_monitor_https",
                "gtm_monitor_imap",
                "gtm_monitor_ldap",
                "gtm_monitor_mssql",
                "gtm_monitor_mysql",
                "gtm_monitor_nntp",
                "gtm_monitor_none",
                "gtm_monitor_oracle",
                "gtm_monitor_pop3",
                "gtm_monitor_postgresql",
                "gtm_monitor_radius",
                "gtm_monitor_radius_accounting",
                "gtm_monitor_real_server",
                "gtm_monitor_scripted",
                "gtm_monitor_sip",
                "gtm_monitor_smtp",
                "gtm_monitor_snmp",
                "gtm_monitor_snmp_link",
                "gtm_monitor_soap",
                "gtm_monitor_tcp",
                "gtm_monitor_tcp_half_open",
                "gtm_monitor_udp",
                "gtm_monitor_wap",
                "gtm_monitor_wmi",
            ],
            shape_kind: Some(ValueKind::Object),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "persist",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "prober-fallback",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &[
                "any-available",
                "inherit",
                "inside-datacenter",
                "none",
                "outside-datacenter",
                "pool",
            ],
            default: Some("inherit"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "prober-pool",
            value_type: ValueKind::Reference,
            allow_none: true,
            enum_values: &["none"],
            references: &["gtm_prober_pool"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "prober-preference",
            value_type: ValueKind::Enum,
            enum_values: &["inherit", "inside-datacenter", "outside-datacenter", "pool"],
            default: Some("inherit"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "product",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "translation",
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::IpAddress),
            default: Some("::"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "translation-address",
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::IpAddress),
            default: Some("::"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "translation-port",
            value_type: ValueKind::Reference,
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "virtual-server-discovery",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "virtual-servers",
            value_type: ValueKind::Unknown,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
