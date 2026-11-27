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

//! BIG-IP object specs for `sys datastor`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "sys_datastor",
        table_name: None,
        resolver_name: None,
        module: Some("sys"),
        object_types: &["datastor"],
    },
    header_types: &[("sys", "datastor")],
    properties: &[
        BigipPropertySpec {
            name: "dedup-cache-weight",
            value_type: ValueKind::Integer,
            default: Some("10"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "disk",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "high-water-mark",
            value_type: ValueKind::Integer,
            default: Some("92"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "low-water-mark",
            value_type: ValueKind::Integer,
            default: Some("80"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "web-cache-weight",
            value_type: ValueKind::Integer,
            default: Some("10"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
