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

//! BIG-IP object specs for `gtm monitor scripted`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "gtm_monitor_scripted",
        table_name: None,
        resolver_name: None,
        module: Some("gtm"),
        object_types: &["monitor scripted"],
    },
    header_types: &[("gtm", "monitor scripted")],
    properties: &[
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
            references: &["gtm_monitor_scripted"],
            default: Some("scripted"),
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
            default: Some("*:*"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "filename",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ignore-down-response",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "interval",
            value_type: ValueKind::Integer,
            default: Some("10 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "probe-timeout",
            value_type: ValueKind::Integer,
            default: Some("5 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "timeout",
            value_type: ValueKind::Integer,
            default: Some("31 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
