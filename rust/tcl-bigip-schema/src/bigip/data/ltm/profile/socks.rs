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

//! BIG-IP object specs for `ltm profile socks`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_socks",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile socks"],
    },
    header_types: &[("ltm", "profile socks")],
    properties: &[
        BigipPropertySpec {
            name: "default-connect-handling",
            value_type: ValueKind::Enum,
            enum_values: &["allow", "deny"],
            default: Some("deny"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dns-resolver",
            value_type: ValueKind::Unknown,
            default: Some("dns-resolver"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ipv6",
            value_type: ValueKind::Enum,
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("no, which will try a IPv4 lookup before a IPv6"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "protocol-versions",
            value_type: ValueKind::List,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-domain",
            value_type: ValueKind::Unknown,
            references: &["net_route_domain"],
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tunnel-name",
            value_type: ValueKind::Unknown,
            default: Some("socks-tunnel"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
