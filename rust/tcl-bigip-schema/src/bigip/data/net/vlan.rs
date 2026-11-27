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

//! BIG-IP object specs for `net vlan`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "net_vlan",
        table_name: None,
        resolver_name: None,
        module: Some("net"),
        object_types: &["vlan"],
    },
    header_types: &[("net", "vlan")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auto-lasthop",
            value_type: ValueKind::Enum,
            enum_values: &["default", "disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cmp-hash",
            value_type: ValueKind::Enum,
            enum_values: &["default", "dst-ip", "src-ip"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "customer-tag",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dag-adjustment",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["bit-roll", "nibble-roll", "none", "xor-5mid-xor-5low"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dag-round-robin",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dag-tunnel",
            value_type: ValueKind::Enum,
            enum_values: &["inner", "outer"],
            default: Some("outer"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "failsafe",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "failsafe-action",
            value_type: ValueKind::Enum,
            enum_values: &["failover", "failover-restart-tm", "reboot", "restart-all"],
            default: Some("failover-restart-tm"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "failsafe-timeout",
            value_type: ValueKind::Integer,
            default: Some("90 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fwd-mode",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["l3", "none", "passive", "virtual-wire"],
            usage_flags: &["read_only"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hardware-syncookie",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "interfaces",
            value_type: ValueKind::Unknown,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ipv6-prefix-len",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "learning",
            value_type: ValueKind::Enum,
            enum_values: &["disable-drop", "disable-forward", "enable-forward"],
            default: Some("enable-forward"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mtu",
            value_type: ValueKind::Integer,
            default: Some("1500"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "nti",
            value_type: ValueKind::Integer,
            default: Some("4096"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sflow",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "poll-interval",
                    value_type: ValueKind::Integer,
                    in_sections: &["sflow"],
                    default: Some("0"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "poll-interval-global",
                    value_type: ValueKind::Enum,
                    in_sections: &["sflow"],
                    enum_values: &["no", "yes"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("yes"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "sampling-rate",
                    value_type: ValueKind::Integer,
                    in_sections: &["sflow"],
                    default: Some("0"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "sampling-rate-global",
                    value_type: ValueKind::Enum,
                    in_sections: &["sflow"],
                    enum_values: &["no", "yes"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("yes"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "poll-interval",
            value_type: ValueKind::Integer,
            in_sections: &["sflow"],
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "poll-interval-global",
            value_type: ValueKind::Enum,
            in_sections: &["sflow"],
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("yes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sampling-rate",
            value_type: ValueKind::Integer,
            in_sections: &["sflow"],
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sampling-rate-global",
            value_type: ValueKind::Enum,
            in_sections: &["sflow"],
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("yes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source-checking",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "syn-flood-rate-limit",
            value_type: ValueKind::Integer,
            default: Some("set at 1000 packets per second"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "syncache-threshold",
            value_type: ValueKind::Integer,
            default: Some("set to 6000 packets"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tag",
            value_type: ValueKind::Enum,
            enum_values: &["4096"],
            default: Some(
                "to not use this option, and the system assigns a tag number between 1 to 4094",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tag-mode",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["customer", "double", "none", "service"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
