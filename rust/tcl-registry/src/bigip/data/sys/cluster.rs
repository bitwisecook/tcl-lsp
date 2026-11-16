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

//! BIG-IP object specs for `sys cluster`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "sys_cluster",
        table_name: None,
        resolver_name: None,
        module: Some("sys"),
        object_types: &["cluster"],
    },
    header_types: &[("sys", "cluster")],
    properties: &[
        BigipPropertySpec {
            name: "address",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["none"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "alt-address",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["none"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "members",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "address",
                    value_type: ValueKind::Enum,
                    in_sections: &["members"],
                    allow_none: true,
                    enum_values: &["none"],
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "alt-address",
                    value_type: ValueKind::Enum,
                    in_sections: &["members"],
                    allow_none: true,
                    enum_values: &["none"],
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "priming",
                    value_type: ValueKind::Enum,
                    in_sections: &["members"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "address",
            value_type: ValueKind::Enum,
            in_sections: &["members"],
            allow_none: true,
            enum_values: &["none"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "alt-address",
            value_type: ValueKind::Enum,
            in_sections: &["members"],
            allow_none: true,
            enum_values: &["none"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "priming",
            value_type: ValueKind::Enum,
            in_sections: &["members"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "min-up-members",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "min-up-members-enabled",
            value_type: ValueKind::Enum,
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
