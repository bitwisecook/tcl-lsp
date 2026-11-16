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

//! BIG-IP object specs for `ltm monitor soap`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_monitor_soap",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["monitor soap"],
    },
    header_types: &[("ltm", "monitor soap")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "debug",
            value_type: ValueKind::Enum,
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("no"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            references: &["ltm_monitor_soap"],
            default: Some("soap"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "destination",
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::Endpoint),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "expect-fault",
            value_type: ValueKind::Enum,
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("no"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "interval",
            value_type: ValueKind::Integer,
            default: Some("5 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "manual-resume",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "method",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "namespace",
            value_type: ValueKind::Reference,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "parameter-name",
            value_type: ValueKind::Reference,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "parameter-type",
            value_type: ValueKind::Enum,
            enum_values: &["bool", "int", "long"],
            default: Some("bool"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "parameter-value",
            value_type: ValueKind::Integer,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "password",
            value_type: ValueKind::Unknown,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "protocol",
            value_type: ValueKind::Enum,
            enum_values: &["http", "https"],
            default: Some("http"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "return-type",
            value_type: ValueKind::Enum,
            enum_values: &["bool", "char", "double", "int", "long", "short"],
            default: Some("bool"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "return-value",
            value_type: ValueKind::Integer,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "soap-action",
            value_type: ValueKind::String,
            default: Some("the empty string"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "time-until-up",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "timeout",
            value_type: ValueKind::Integer,
            default: Some("16 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "up-interval",
            value_type: ValueKind::Integer,
            default: Some(
                "0 (zero), which specifies that the system uses the value of the interval option whether the resource is up or down",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "url-path",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "username",
            value_type: ValueKind::Reference,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
