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

//! BIG-IP object specs for `wam object-type`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "wam_object_type",
        table_name: None,
        resolver_name: None,
        module: Some("wam"),
        object_types: &["object-type"],
    },
    header_types: &[("wam", "object-type")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "code",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "compression",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "policy-controlled"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "extensions",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mime-types",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "symmetric-compression",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
