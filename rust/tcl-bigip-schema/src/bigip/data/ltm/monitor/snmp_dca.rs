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

//! BIG-IP object specs for `ltm monitor snmp-dca`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_monitor_snmp_dca",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["monitor snmp-dca"],
    },
    header_types: &[("ltm", "monitor snmp-dca")],
    properties: &[
        BigipPropertySpec {
            name: "agent-type",
            value_type: ValueKind::Enum,
            enum_values: &["generic", "other", "ucd", "win2000"],
            default: Some("ucd"),
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
            name: "community",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["net_routing_community_list"],
            default: Some("public"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cpu-coefficient",
            value_type: ValueKind::Integer,
            allow_none: true,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cpu-threshold",
            value_type: ValueKind::Integer,
            default: Some("80 percent"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            references: &["ltm_monitor_snmp_dca"],
            default: Some("snmp_dca"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "disk-coefficient",
            value_type: ValueKind::Integer,
            allow_none: true,
            default: Some("2"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "disk-threshold",
            value_type: ValueKind::Integer,
            default: Some("90 percent"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "interval",
            value_type: ValueKind::Integer,
            default: Some("10 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "memory-coefficient",
            value_type: ValueKind::Integer,
            allow_none: true,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "memory-threshold",
            value_type: ValueKind::Integer,
            default: Some("70 percent"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "time-until-up",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "timeout",
            value_type: ValueKind::Integer,
            default: Some("30 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "user-defined",
            value_type: ValueKind::Unknown,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "version",
            value_type: ValueKind::Integer,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
