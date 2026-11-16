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

//! BIG-IP object specs for `ltm profile fix`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_fix",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile fix"],
    },
    header_types: &[("ltm", "profile fix")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_profile_fix"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "error-action",
            value_type: ValueKind::Enum,
            enum_values: &["dont_forward", "drop_connection"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "full-logon-parsing",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "message-log-publisher",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "quick-parsing",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "report-log-publisher",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "response-parsing",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sender-tag-class",
            value_type: ValueKind::List,
            repeated: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "statistics-sample-interval",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
