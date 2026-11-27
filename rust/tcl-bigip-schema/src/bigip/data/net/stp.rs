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

//! BIG-IP object specs for `net stp`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "net_stp",
        table_name: None,
        resolver_name: None,
        module: Some("net"),
        object_types: &["stp"],
    },
    header_types: &[("net", "stp")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "instance-id",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "interfaces",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["interfaces"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "external-path-cost",
                    value_type: ValueKind::Integer,
                    in_sections: &["interfaces"],
                    default: Some("20000"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "internal-path-cost",
                    value_type: ValueKind::Integer,
                    in_sections: &["interfaces"],
                    default: Some("20000"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "priority",
                    value_type: ValueKind::Integer,
                    in_sections: &["interfaces"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["interfaces"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "external-path-cost",
            value_type: ValueKind::Integer,
            in_sections: &["interfaces"],
            default: Some("20000"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "internal-path-cost",
            value_type: ValueKind::Integer,
            in_sections: &["interfaces"],
            default: Some("20000"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "priority",
            value_type: ValueKind::Integer,
            in_sections: &["interfaces"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "priority",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "trunks",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["trunks"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "external-path-cost",
                    value_type: ValueKind::Integer,
                    in_sections: &["trunks"],
                    default: Some("20000"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "internal-path-cost",
                    value_type: ValueKind::Integer,
                    in_sections: &["trunks"],
                    default: Some("20000"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "priority",
                    value_type: ValueKind::Integer,
                    in_sections: &["trunks"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["trunks"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "external-path-cost",
            value_type: ValueKind::Integer,
            in_sections: &["trunks"],
            default: Some("20000"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "internal-path-cost",
            value_type: ValueKind::Integer,
            in_sections: &["trunks"],
            default: Some("20000"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "priority",
            value_type: ValueKind::Integer,
            in_sections: &["trunks"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "vlans",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
