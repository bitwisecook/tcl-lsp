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

//! BIG-IP object specs for `ltm global-settings`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_global_settings_connection",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["global-settings connection"],
        },
        header_types: &[("ltm", "global-settings connection")],
        properties: &[
            BigipPropertySpec {
                name: "adaptive-reaper-hiwater",
                value_type: ValueKind::Integer,
                default: Some("95"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "adaptive-reaper-lowater",
                value_type: ValueKind::Integer,
                default: Some("85"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-last-hop",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-vs-syn-challenge-threshold",
                value_type: ValueKind::Integer,
                enum_values: &["infinite"],
                default: Some("infinite"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "global-flow-eviction-policy",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "global-syn-challenge-threshold",
                value_type: ValueKind::Integer,
                enum_values: &["infinite"],
                default: Some("64K"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "syncookies-threshold",
                value_type: ValueKind::Integer,
                default: Some("16384"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlan-keyed-conn",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlan-syn-cookie",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_global_settings_general",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["global-settings general"],
        },
        header_types: &[("ltm", "global-settings general")],
        properties: &[
            BigipPropertySpec {
                name: "gratuitous-arp-rate",
                value_type: ValueKind::Unknown,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "l2-cache-timeout",
                value_type: ValueKind::Integer,
                default: Some("300 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "maintenance-mode",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mgmt-auto-lasthop",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "share-single-mac",
                value_type: ValueKind::Enum,
                enum_values: &["unique", "vmw-compat"],
                default: Some(
                    "unique, which indicates that a VLAN uses a unique MAC address from the pool of mac addresses assigned to each hardware platform",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "snat-packet-forward",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_global_settings_rule",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["global-settings rule"],
        },
        header_types: &[("ltm", "global-settings rule")],
        properties: &[BigipPropertySpec {
            name: "rule-aborted-log-ratio",
            value_type: ValueKind::Unknown,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_global_settings_traffic_control",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["global-settings traffic-control"],
        },
        header_types: &[("ltm", "global-settings traffic-control")],
        properties: &[
            BigipPropertySpec {
                name: "accept-ip-options",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "accept-ip-source-route",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-ip-source-route",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "continue-matching",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-icmp-rate",
                value_type: ValueKind::Unknown,
                default: Some("100 errors per second"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-reject-rate",
                value_type: ValueKind::Integer,
                default: Some("250 per second"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-reject-rate-timeout",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-path-mtu",
                value_type: ValueKind::Integer,
                default: Some("296"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "path-mtu-discovery",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port-find-linear",
                value_type: ValueKind::Integer,
                default: Some("16"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port-find-random",
                value_type: ValueKind::Integer,
                default: Some("16"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port-find-threshold-timeout",
                value_type: ValueKind::Unknown,
                default: Some("30 (1/2 minute) with range from 0 - 300"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port-find-threshold-trigger",
                value_type: ValueKind::Unknown,
                default: Some("8"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port-find-threshold-warning",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reject-unmatched",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
