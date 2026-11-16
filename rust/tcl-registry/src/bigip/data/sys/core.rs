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

//! BIG-IP object specs for `sys core`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "sys_core",
        table_name: None,
        resolver_name: None,
        module: Some("sys"),
        object_types: &["core"],
    },
    header_types: &[("sys", "core")],
    properties: &[
        BigipPropertySpec {
            name: "bigd-action",
            value_type: ValueKind::Enum,
            enum_values: &["rotate", "skip"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bigd-manage",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bigd-max",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mcpd-action",
            value_type: ValueKind::Enum,
            enum_values: &["rotate", "skip"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mcpd-manage",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mcpd-max",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "retention",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tmm-action",
            value_type: ValueKind::Enum,
            enum_values: &["rotate", "skip"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tmm-manage",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tmm-max",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
