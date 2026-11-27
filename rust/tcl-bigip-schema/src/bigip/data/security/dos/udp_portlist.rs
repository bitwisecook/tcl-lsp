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

//! BIG-IP object specs for `security dos udp-portlist`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "security_dos_udp_portlist",
        table_name: None,
        resolver_name: None,
        module: Some("security"),
        object_types: &["dos udp-portlist"],
    },
    header_types: &[("security", "dos udp-portlist")],
    properties: &[
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "entries",
            value_type: ValueKind::List,
            list_operators: &["modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::String,
                    in_sections: &["entries"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "match-direction",
                    value_type: ValueKind::Enum,
                    in_sections: &["entries"],
                    allow_none: true,
                    enum_values: &["both", "dst", "none", "src"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "port-number",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["entries"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "match-direction",
            value_type: ValueKind::Enum,
            in_sections: &["entries"],
            allow_none: true,
            enum_values: &["both", "dst", "none", "src"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port-number",
            value_type: ValueKind::Unknown,
            in_sections: &["entries"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "list-type",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
