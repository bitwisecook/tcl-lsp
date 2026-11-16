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

//! BIG-IP object specs for `apm aaa oam`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_aaa_oam",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["aaa oam"],
    },
    header_types: &[("apm", "aaa oam")],
    properties: &[
        BigipPropertySpec {
            name: "access-server-hostname",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "access-server-name",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "access-server-port",
            value_type: ValueKind::Integer,
            allow_none: true,
            default: Some("6021"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "access-server-retries",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "accessgate-encrypted-password",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "accessgates",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "action",
            value_type: ValueKind::Enum,
            enum_values: &["config-accessgate", "noop"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "admin-id",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "admin-password",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            default: Some("none"),
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
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enable",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("true"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "global-access-protocol-passphrase",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "location-specific",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "transport-security-mode",
            value_type: ValueKind::Enum,
            enum_values: &["cert", "open", "simple"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
