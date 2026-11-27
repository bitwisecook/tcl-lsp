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

//! BIG-IP object specs for `net routing debug`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "net_routing_debug",
        table_name: None,
        resolver_name: None,
        module: Some("net"),
        object_types: &["routing debug"],
    },
    header_types: &[("net", "routing debug")],
    properties: &[
        BigipPropertySpec {
            name: "bfd",
            value_type: ValueKind::Enum,
            enum_values: &[
                "all",
                "event",
                "ipc-error",
                "ipc-event",
                "nsm",
                "packet",
                "session",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp",
            value_type: ValueKind::Enum,
            enum_values: &[
                "all",
                "bfd",
                "dampening",
                "events",
                "filters",
                "fsm",
                "keepalives",
                "nht",
                "nsm",
                "updates",
                "updates-in",
                "updates-out",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "nsm",
            value_type: ValueKind::Enum,
            enum_values: &[
                "all",
                "bfd",
                "events",
                "ha",
                "ha-all",
                "kernel",
                "packet",
                "packet-detail",
                "packet-recv",
                "packet-send",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-domain",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["net_route_domain"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
