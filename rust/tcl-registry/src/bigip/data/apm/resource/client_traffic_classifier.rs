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

//! BIG-IP object specs for `apm resource client-traffic-classifier`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_resource_client_traffic_classifier",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["resource client-traffic-classifier"],
    },
    header_types: &[("apm", "resource client-traffic-classifier")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "entries",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["entries"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "client-rate-class",
                    value_type: ValueKind::String,
                    in_sections: &["entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dst-ip",
                    value_type: ValueKind::String,
                    in_sections: &["entries"],
                    allow_none: true,
                    shape_kind: Some(ValueKind::IpAddress),
                    address_grammar: Some(crate::bigip::AddressGrammar::Network),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dst-mask",
                    value_type: ValueKind::Integer,
                    in_sections: &["entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dst-port",
                    value_type: ValueKind::Integer,
                    in_sections: &["entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "protocol",
                    value_type: ValueKind::Integer,
                    in_sections: &["entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "src-ip",
                    value_type: ValueKind::String,
                    in_sections: &["entries"],
                    allow_none: true,
                    shape_kind: Some(ValueKind::IpAddress),
                    address_grammar: Some(crate::bigip::AddressGrammar::Network),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "src-mask",
                    value_type: ValueKind::Integer,
                    in_sections: &["entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "src-port",
                    value_type: ValueKind::Integer,
                    in_sections: &["entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["entries"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-rate-class",
            value_type: ValueKind::String,
            in_sections: &["entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dst-ip",
            value_type: ValueKind::String,
            in_sections: &["entries"],
            allow_none: true,
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Network),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dst-mask",
            value_type: ValueKind::Integer,
            in_sections: &["entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dst-port",
            value_type: ValueKind::Integer,
            in_sections: &["entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "protocol",
            value_type: ValueKind::Integer,
            in_sections: &["entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "src-ip",
            value_type: ValueKind::String,
            in_sections: &["entries"],
            allow_none: true,
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Network),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "src-mask",
            value_type: ValueKind::Integer,
            in_sections: &["entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "src-port",
            value_type: ValueKind::Integer,
            in_sections: &["entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "location-specific",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
