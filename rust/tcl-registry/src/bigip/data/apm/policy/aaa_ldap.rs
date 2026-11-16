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

//! BIG-IP object specs for `apm policy aaa-ldap`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_policy_agent_aaa_ldap",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["policy agent aaa-ldap"],
    },
    header_types: &[("apm", "policy agent aaa-ldap")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "attr-name",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "filter",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "group-member-scope",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["all", "direct", "none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "group-membership-scope",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["all", "direct", "none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ldapmod-attributes",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-logon-attempt",
            value_type: ValueKind::Integer,
            default: Some("3"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "modify-type",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "search-dn",
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
            name: "type",
            value_type: ValueKind::List,
            required: true,
            default: Some("last"),
            list_operators: &["modify"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "user-dn",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
