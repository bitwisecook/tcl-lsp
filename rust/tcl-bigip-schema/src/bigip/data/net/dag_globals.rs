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

//! BIG-IP object specs for `net dag-globals`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "net_dag_globals",
        table_name: None,
        resolver_name: None,
        module: Some("net"),
        object_types: &["dag-globals"],
    },
    header_types: &[("net", "dag-globals")],
    properties: &[
        BigipPropertySpec {
            name: "dag-ipv6-prefix-len",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "icmp-hash",
            value_type: ValueKind::Enum,
            enum_values: &["icmp", "ipicmp"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "icmp-monitor-priority",
            value_type: ValueKind::Enum,
            enum_values: &["high", "normal"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "round-robin-mode",
            value_type: ValueKind::Enum,
            enum_values: &["local"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
