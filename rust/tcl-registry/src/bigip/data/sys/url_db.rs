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

//! BIG-IP object specs for `sys url-db`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_url_db_download_schedule",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["url-db download-schedule"],
        },
        header_types: &[("sys", "url-db download-schedule")],
        properties: &[
            BigipPropertySpec {
                name: "download-now",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "end-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "start-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "status",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-proxy",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_url_db_url_category",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["url-db url-category"],
        },
        header_types: &[("sys", "url-db url-category")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "display-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "initial-disposition",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "is-security-category",
                value_type: ValueKind::String,
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "parent-cat-number",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "severity-level",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "urls",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
