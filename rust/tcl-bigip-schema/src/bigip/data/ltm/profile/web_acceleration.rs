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

//! BIG-IP object specs for `ltm profile web-acceleration`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_web_acceleration",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile web-acceleration"],
    },
    header_types: &[("ltm", "profile web-acceleration")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cache-aging-rate",
            value_type: ValueKind::Integer,
            default: Some("9"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cache-client-cache-control-mode",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["all", "max-age", "none"],
            default: Some("all"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cache-insert-age-header",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cache-max-age",
            value_type: ValueKind::Integer,
            default: Some("3600 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cache-max-entries",
            value_type: ValueKind::Integer,
            default: Some("10000"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cache-object-max-size",
            value_type: ValueKind::Integer,
            default: Some("50000 bytes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cache-object-min-size",
            value_type: ValueKind::Integer,
            default: Some("500 bytes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cache-size",
            value_type: ValueKind::Integer,
            default: Some("100 megabytes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cache-uri-exclude",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("none and specifies that no URI will be excluded"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cache-uri-include",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cache-uri-include-override",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cache-uri-pinned",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_profile_web_acceleration"],
            default: Some("webacceleration"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
