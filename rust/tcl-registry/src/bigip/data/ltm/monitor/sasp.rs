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

//! BIG-IP object specs for `ltm monitor sasp`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_monitor_sasp",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["monitor sasp"],
    },
    header_types: &[("ltm", "monitor sasp")],
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
            references: &["ltm_monitor_sasp"],
            default: Some("sasp"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "interval",
            value_type: ValueKind::Integer,
            default: Some("auto"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mode",
            value_type: ValueKind::Enum,
            enum_values: &["pull", "push"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "primary-address",
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "protocol",
            value_type: ValueKind::Enum,
            enum_values: &["tcp", "udp"],
            default: Some("tcp"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "secondary-address",
            value_type: ValueKind::String,
            allow_none: true,
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "service",
            value_type: ValueKind::Unknown,
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
            default: Some("100 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
