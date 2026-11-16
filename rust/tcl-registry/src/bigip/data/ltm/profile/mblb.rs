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

//! BIG-IP object specs for `ltm profile mblb`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_mblb",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile mblb"],
    },
    header_types: &[("ltm", "profile mblb")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_profile_mblb"],
            default: Some("mblb"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "egress-high",
            value_type: ValueKind::Unknown,
            default: Some("50"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "egress-low",
            value_type: ValueKind::Unknown,
            default: Some("5"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ingress-high",
            value_type: ValueKind::Unknown,
            default: Some("50"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ingress-low",
            value_type: ValueKind::Unknown,
            default: Some("5"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "isolate-abort",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "isolate-client",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "isolate-expire",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "isolate-server",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "min-conn",
            value_type: ValueKind::Unknown,
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "shutdown-timeout",
            value_type: ValueKind::Unknown,
            default: Some("5 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tag-ttl",
            value_type: ValueKind::Unknown,
            default: Some("60"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
