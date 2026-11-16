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

//! BIG-IP object specs for `sys sflow`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_sflow_global_settings_http",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["sflow global-settings http"],
        },
        header_types: &[("sys", "sflow global-settings http")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "poll-interval",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sampling-rate",
                value_type: ValueKind::Integer,
                default: Some("1024"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_sflow_global_settings_interface",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["sflow global-settings interface"],
        },
        header_types: &[("sys", "sflow global-settings interface")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "poll-interval",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_sflow_global_settings_system",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["sflow global-settings system"],
        },
        header_types: &[("sys", "sflow global-settings system")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "poll-interval",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_sflow_global_settings_vlan",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["sflow global-settings vlan"],
        },
        header_types: &[("sys", "sflow global-settings vlan")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "poll-interval",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sampling-rate",
                value_type: ValueKind::Integer,
                default: Some("2048"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_sflow_receiver",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["sflow receiver"],
        },
        header_types: &[("sys", "sflow receiver")],
        properties: &[
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
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
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-datagram-size",
                value_type: ValueKind::Integer,
                default: Some("1400"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Unknown,
                default: Some("the standard sFlow port, 6343"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
