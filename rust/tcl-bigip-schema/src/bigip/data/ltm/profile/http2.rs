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

//! BIG-IP object specs for `ltm profile http2`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_http2",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile http2"],
    },
    header_types: &[("ltm", "profile http2")],
    properties: &[
        BigipPropertySpec {
            name: "activation-modes",
            value_type: ValueKind::List,
            repeated: true,
            default: Some("{ alpn }"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "concurrent-streams-per-connection",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "connection-idle-timeout",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_profile_http2"],
            default: Some("http2"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforce-tls-requirements",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "frame-size",
            value_type: ValueKind::Integer,
            default: Some("2048"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "header-table-size",
            value_type: ValueKind::Integer,
            default: Some("4096 bytes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "insert-header",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "insert-header-name",
            value_type: ValueKind::Unknown,
            default: Some("\"X-HTTP2\""),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "receive-window",
            value_type: ValueKind::Integer,
            default: Some("32"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "write-size",
            value_type: ValueKind::Integer,
            default: Some("16384"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
