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

//! BIG-IP object specs for `gtm link`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "gtm_link",
        table_name: None,
        resolver_name: None,
        module: Some("gtm"),
        object_types: &["link"],
    },
    header_types: &[("gtm", "link")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cost-segments",
            value_type: ValueKind::List,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "datacenter",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "device-name",
            value_type: ValueKind::String,
            allow_none: true,
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "duplex-billing",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-max-inbound-bps",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-max-inbound-bps-status",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-max-outbound-bps",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-max-outbound-bps-status",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-max-total-bps",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-max-total-bps-status",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "link-ratio",
            value_type: ValueKind::Integer,
            default: Some("1"),
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
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "prepaid-segment",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "router-addresses",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "service-provider",
            value_type: ValueKind::Reference,
            usage_flags: &["optional"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "translation",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("any6"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "uplink-address",
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "weighting",
            value_type: ValueKind::Enum,
            enum_values: &["price", "ratio"],
            default: Some("ratio"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
