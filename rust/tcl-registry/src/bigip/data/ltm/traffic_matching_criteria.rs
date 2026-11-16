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

//! BIG-IP object specs for `ltm traffic-matching-criteria`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_traffic_matching_criteria",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["traffic-matching-criteria"],
    },
    header_types: &[("ltm", "traffic-matching-criteria")],
    properties: &[
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "destination-address-inline",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "destination-address-list",
            value_type: ValueKind::Reference,
            allow_none: true,
            default: Some("any"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "destination-port-inline",
            value_type: ValueKind::Unknown,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "destination-port-list",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["net_port_list", "security_firewall_port_list"],
            default: Some("any"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source-address-inline",
            value_type: ValueKind::Unknown,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source-address-list",
            value_type: ValueKind::Reference,
            allow_none: true,
            default: Some("any"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source-port-inline",
            value_type: ValueKind::Unknown,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
