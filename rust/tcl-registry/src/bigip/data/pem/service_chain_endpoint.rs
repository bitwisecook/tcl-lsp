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

//! BIG-IP object specs for `pem service-chain-endpoint`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "pem_service_chain_endpoint",
        table_name: None,
        resolver_name: None,
        module: Some("pem"),
        object_types: &["service-chain-endpoint"],
    },
    header_types: &[("pem", "service-chain-endpoint")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "service-endpoints",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["service-endpoints"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "forwarding-endpoint",
                    value_type: ValueKind::Unknown,
                    in_sections: &["service-endpoints"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "from-vlan",
                    value_type: ValueKind::Reference,
                    in_sections: &["service-endpoints"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "http-adapt-service",
                    value_type: ValueKind::Unknown,
                    in_sections: &["service-endpoints"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "icap-type",
                    value_type: ValueKind::Enum,
                    in_sections: &["service-endpoints"],
                    allow_none: true,
                    enum_values: &["both", "none", "request", "response"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "internal-virtual",
                    value_type: ValueKind::Enum,
                    in_sections: &["service-endpoints"],
                    allow_none: true,
                    enum_values: &["none"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "order",
                    value_type: ValueKind::Integer,
                    in_sections: &["service-endpoints"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "service-option",
                    value_type: ValueKind::Enum,
                    in_sections: &["service-endpoints"],
                    enum_values: &["mandatory", "optional"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "steering-policy",
                    value_type: ValueKind::Reference,
                    in_sections: &["service-endpoints"],
                    allow_none: true,
                    enum_values: &["none"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "to-endpoint",
                    value_type: ValueKind::Reference,
                    in_sections: &["service-endpoints"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["service-endpoints"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "forwarding-endpoint",
            value_type: ValueKind::Unknown,
            in_sections: &["service-endpoints"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "from-vlan",
            value_type: ValueKind::Reference,
            in_sections: &["service-endpoints"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "http-adapt-service",
            value_type: ValueKind::Unknown,
            in_sections: &["service-endpoints"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "icap-type",
            value_type: ValueKind::Enum,
            in_sections: &["service-endpoints"],
            allow_none: true,
            enum_values: &["both", "none", "request", "response"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "internal-virtual",
            value_type: ValueKind::Enum,
            in_sections: &["service-endpoints"],
            allow_none: true,
            enum_values: &["none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "order",
            value_type: ValueKind::Integer,
            in_sections: &["service-endpoints"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "service-option",
            value_type: ValueKind::Enum,
            in_sections: &["service-endpoints"],
            enum_values: &["mandatory", "optional"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "steering-policy",
            value_type: ValueKind::Reference,
            in_sections: &["service-endpoints"],
            allow_none: true,
            enum_values: &["none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "to-endpoint",
            value_type: ValueKind::Reference,
            in_sections: &["service-endpoints"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
