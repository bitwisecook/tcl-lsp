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

//! BIG-IP object specs for `ltm node`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_node",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["node"],
    },
    header_types: &[("ltm", "node")],
    properties: &[
        BigipPropertySpec {
            name: "address",
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            requires_route_domain: true,
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
            name: "connection-limit",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dynamic-ratio",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fqdn",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "address-family",
                    value_type: ValueKind::Enum,
                    in_sections: &["fqdn"],
                    enum_values: &["ipv4", "ipv6"],
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "autopopulate",
                    value_type: ValueKind::Enum,
                    in_sections: &["fqdn"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "down-interval",
                    value_type: ValueKind::Integer,
                    in_sections: &["fqdn"],
                    default: Some("5"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "interval",
                    value_type: ValueKind::Integer,
                    in_sections: &["fqdn"],
                    default: Some("3600"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "address-family",
            value_type: ValueKind::Enum,
            in_sections: &["fqdn"],
            enum_values: &["ipv4", "ipv6"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "autopopulate",
            value_type: ValueKind::Enum,
            in_sections: &["fqdn"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "down-interval",
            value_type: ValueKind::Integer,
            in_sections: &["fqdn"],
            default: Some("5"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "interval",
            value_type: ValueKind::Integer,
            in_sections: &["fqdn"],
            default: Some("3600"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "logging",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            usage_flags: &["not_synced", "runtime_only"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "metadata",
            value_type: ValueKind::Unknown,
            default: Some("persistent, which saves the data to the config file"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "monitor",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &[
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
            ],
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
            name: "rate-limit",
            value_type: ValueKind::Integer,
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ratio",
            value_type: ValueKind::Integer,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "session",
            value_type: ValueKind::Enum,
            enum_values: &["user-disabled", "user-enabled"],
            default: Some("user-enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "state",
            value_type: ValueKind::Enum,
            enum_values: &["user-down", "user-up"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
