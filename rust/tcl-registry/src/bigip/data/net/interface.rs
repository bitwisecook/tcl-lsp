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

//! BIG-IP object specs for `net interface`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "net_interface",
        table_name: None,
        resolver_name: None,
        module: Some("net"),
        object_types: &["interface"],
    },
    header_types: &[("net", "interface")],
    properties: &[
        BigipPropertySpec {
            name: "bundle",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bundle-speed",
            value_type: ValueKind::Enum,
            enum_values: &["100G", "40G"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "flow-control",
            value_type: ValueKind::Unknown,
            default: Some("tx-rx"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "force-gigabit-fiber",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "forward-error-correction",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "lacp-port-priority",
            value_type: ValueKind::Integer,
            default: Some("33768"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "link-traps-enabled",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("true"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "lldp-admin",
            value_type: ValueKind::Enum,
            enum_values: &["disable", "rxonly", "txonly", "txrx"],
            default: Some("txonly"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "lldp-tlvmap",
            value_type: ValueKind::Integer,
            default: Some("130943"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "media",
            value_type: ValueKind::Enum,
            enum_values: &["auto", "no-phy"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "media-fixed",
            value_type: ValueKind::Enum,
            enum_values: &["auto", "no-phy"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "media-sfp",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["auto", "no-phy", "none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "no-mgmt",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port-fwd-mode",
            value_type: ValueKind::Enum,
            enum_values: &["l3", "passive", "virtual-wire"],
            default: Some("l3"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "prefer-port",
            value_type: ValueKind::Enum,
            enum_values: &["fixed", "sfp"],
            default: Some("sfp"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qinq-ethertype",
            value_type: ValueKind::String,
            default: Some("set to 0x8100"),
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
            name: "span-mode",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "stp",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "stp-auto-edge-port",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "stp-edge-port",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("true"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "stp-link-type",
            value_type: ValueKind::Enum,
            enum_values: &["auto", "p2p", "shared"],
            default: Some("auto"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "stp-reset",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
