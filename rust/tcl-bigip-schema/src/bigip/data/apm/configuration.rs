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

//! BIG-IP object specs for `apm configuration`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_configuration_captcha",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["configuration captcha"],
    },
    header_types: &[("apm", "configuration captcha")],
    properties: &[
        BigipPropertySpec {
            name: "captcha-data-size",
            value_type: ValueKind::Enum,
            enum_values: &["data-size-compact", "data-size-normal"],
            default: Some("data-size-normal"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "captcha-data-theme",
            value_type: ValueKind::Enum,
            enum_values: &["data-theme-dark", "data-theme-light"],
            default: Some("data-theme-light"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "captcha-data-type",
            value_type: ValueKind::Enum,
            enum_values: &["data-type-audio", "data-type-image"],
            default: Some("data-type-image"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "captcha-theme",
            value_type: ValueKind::Enum,
            enum_values: &[
                "theme-blackglass",
                "theme-clean",
                "theme-custom",
                "theme-red",
                "theme-white",
            ],
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "challenge-url",
            value_type: ValueKind::String,
            default: Some("www"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "exposition-threshold",
            value_type: ValueKind::Integer,
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "noscript-url",
            value_type: ValueKind::String,
            default: Some("www"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "private-key",
            value_type: ValueKind::Unknown,
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "proceed-on-verification-error",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("true"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "public-key",
            value_type: ValueKind::Unknown,
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "secret",
            value_type: ValueKind::Unknown,
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "site-key",
            value_type: ValueKind::Unknown,
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "track-by-ip",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("true"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "track-by-username",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("true"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "verification-url",
            value_type: ValueKind::String,
            default: Some("www"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
