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

//! BIG-IP object specs for `ltm profile http-compression`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_http_compression",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile http-compression"],
    },
    header_types: &[("ltm", "profile http-compression")],
    properties: &[
        BigipPropertySpec {
            name: "allow-http-10",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
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
            name: "browser-workarounds",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "buffer-size",
            value_type: ValueKind::Integer,
            default: Some("4096"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "content-type-exclude",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "content-type-include",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("{ text/ application/ (xml|x-javascript) }"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cpu-saver",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cpu-saver-high",
            value_type: ValueKind::Integer,
            default: Some("90"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cpu-saver-low",
            value_type: ValueKind::Integer,
            default: Some("75"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_profile_http_compression"],
            default: Some("httpcompression"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "gzip-level",
            value_type: ValueKind::Integer,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "gzip-memory-level",
            value_type: ValueKind::Unknown,
            default: Some("8"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "gzip-window-size",
            value_type: ValueKind::Integer,
            default: Some("16k"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "keep-accept-encoding",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "method-prefer",
            value_type: ValueKind::Enum,
            enum_values: &["deflate", "gzip"],
            default: Some("gzip"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "min-size",
            value_type: ValueKind::Integer,
            default: Some("1024"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "selective",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "uri-exclude",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "uri-include",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("{"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "vary-header",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
