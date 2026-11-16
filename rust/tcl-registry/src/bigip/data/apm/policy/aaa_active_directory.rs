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

//! BIG-IP object specs for `apm policy aaa-active-directory`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_policy_agent_aaa_active_directory",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["policy agent aaa-active-directory"],
    },
    header_types: &[("apm", "policy agent aaa-active-directory")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auth-max-logon-attempt",
            value_type: ValueKind::Integer,
            default: Some("3"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fetch-nested-groups",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fetch-primary-groups",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hints",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "options",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "query-attrname",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "query-filter",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "server",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "show-extended-error",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "trusted-domains",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Enum,
            enum_values: &["auth", "last", "query"],
            default: Some("last"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "upn",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
