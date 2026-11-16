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

//! BIG-IP object specs for `ltm profile one-connect`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_one_connect",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile one-connect"],
    },
    header_types: &[("ltm", "profile one-connect")],
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
            references: &["ltm_profile_one_connect"],
            default: Some("oneconnect"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "idle-timeout-override",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limit-type",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["idle", "none", "strict"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-age",
            value_type: ValueKind::Integer,
            default: Some("86400"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-reuse",
            value_type: ValueKind::Integer,
            default: Some("1000"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-size",
            value_type: ValueKind::Integer,
            default: Some("10000"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "share-pools",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source-mask",
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::IpAddress),
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
