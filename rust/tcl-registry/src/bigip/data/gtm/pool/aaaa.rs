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

//! BIG-IP object specs for `gtm pool aaaa`. Hand-maintained: edit them directly.

// Large tmsh bounds appear as bare f64 literals.
#![allow(clippy::unreadable_literal)]
use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "gtm_pool_aaaa",
        table_name: None,
        resolver_name: None,
        module: Some("gtm"),
        object_types: &["pool aaaa"],
    },
    header_types: &[("gtm", "pool aaaa")],
    properties: &[
        BigipPropertySpec {
            name: "alternate-mode",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &[
                "drop-packet",
                "fallback-ip",
                "none",
                "packet-rate",
                "ratio",
                "return-to-dns",
                "round-robin",
                "static-persistence",
                "topology",
                "virtual-server-capacity",
                "virtual-server-score",
            ],
            default: Some("round-robin"),
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
            name: "dynamic",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fallback-ip",
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fallback-mode",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &[
                "completion-rate",
                "cpu",
                "drop-packet",
                "fallback-ip",
                "fewest-hops",
                "kilobytes-per-second",
                "least-connections",
                "lowest-round-trip-time",
                "none",
                "packet-rate",
                "quality-of-service",
                "ratio",
                "return-to-dns",
                "round-robin",
                "static-persistence",
                "topology",
                "virtual-server-capacity",
                "virtual-server-score",
            ],
            default: Some("return-to-dns"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-max-bps",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
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
            default: Some("0 (zero)"),
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
            default: Some("0 (zero)"),
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
            name: "load-balancing-mode",
            value_type: ValueKind::Enum,
            enum_values: &[
                "completion-rate",
                "cpu",
                "drop-packet",
                "fallback-ip",
                "fewest-hops",
                "kilobytes-per-second",
                "least-connections",
                "lowest-round-trip-time",
                "packet-rate",
                "quality-of-service",
                "ratio",
                "return-to-dns",
                "round-robin",
                "static-persistence",
                "topology",
                "virtual-server-capacity",
                "virtual-server-score",
            ],
            default: Some("round-robin"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "manual-resume",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-answers-returned",
            value_type: ValueKind::Integer,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "member-order",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "members",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("none"),
            address_grammar: Some(crate::bigip::AddressGrammar::GtmMemberKey),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "metadata",
            value_type: ValueKind::Unknown,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "monitor",
            value_type: ValueKind::Unknown,
            repeated: true,
            references: &[
                "analytics_system_monitor_report",
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
                "ltm_default_node_monitor",
                "ltm_monitor_diameter",
                "ltm_monitor_dns",
                "ltm_monitor_external",
                "ltm_monitor_firepass",
                "ltm_monitor_ftp",
                "ltm_monitor_gateway_icmp",
                "ltm_monitor_http",
                "ltm_monitor_http2",
                "ltm_monitor_https",
                "ltm_monitor_icmp",
                "ltm_monitor_imap",
                "ltm_monitor_inband",
                "ltm_monitor_ldap",
                "ltm_monitor_module_score",
                "ltm_monitor_mqtt",
                "ltm_monitor_mssql",
                "ltm_monitor_mysql",
                "ltm_monitor_nntp",
                "ltm_monitor_none",
                "ltm_monitor_oracle",
                "ltm_monitor_pop3",
                "ltm_monitor_postgresql",
                "ltm_monitor_radius",
                "ltm_monitor_radius_accounting",
                "ltm_monitor_real_server",
                "ltm_monitor_rpc",
                "ltm_monitor_sasp",
                "ltm_monitor_scripted",
                "ltm_monitor_sip",
                "ltm_monitor_smb",
                "ltm_monitor_smtp",
                "ltm_monitor_snmp_dca",
                "ltm_monitor_snmp_dca_base",
                "ltm_monitor_soap",
                "ltm_monitor_tcp",
                "ltm_monitor_tcp_echo",
                "ltm_monitor_tcp_half_open",
                "ltm_monitor_udp",
                "ltm_monitor_virtual_location",
                "ltm_monitor_wap",
                "ltm_monitor_wmi",
                "sys_file_external_monitor",
                "util_test_monitor",
            ],
            shape_kind: Some(ValueKind::Object),
            default: Some("none"),
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
            name: "qos-hit-ratio",
            value_type: ValueKind::Integer,
            default: Some("5"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-hops",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-kilobytes-second",
            value_type: ValueKind::Integer,
            default: Some("3"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-lcs",
            value_type: ValueKind::Integer,
            default: Some("30"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-packet-rate",
            value_type: ValueKind::Integer,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-rtt",
            value_type: ValueKind::Integer,
            default: Some("50"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-topology",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-vs-capacity",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-vs-score",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ratio",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ttl",
            value_type: ValueKind::Integer,
            min_value: Some(0f64),
            max_value: Some(4294967295f64),
            default: Some("30"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "verify-member-availability",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
