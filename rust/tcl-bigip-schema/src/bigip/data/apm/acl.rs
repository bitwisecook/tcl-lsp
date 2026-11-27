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

//! BIG-IP object specs for `apm acl`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_acl",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["acl"],
    },
    header_types: &[("apm", "acl")],
    properties: &[
        BigipPropertySpec {
            name: "acl-order",
            value_type: ValueKind::Integer,
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
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
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "entries",
            value_type: ValueKind::List,
            block: &[
                BigipPropertySpec {
                    name: "action",
                    value_type: ValueKind::Enum,
                    in_sections: &["entries"],
                    required: true,
                    enum_values: &["allow", "continue", "discard", "reject", "unspec"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dst-end-port",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries"],
                    allow_none: true,
                    default: Some("0"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dst-start-port",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dst-subnet",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "host",
                    value_type: ValueKind::String,
                    in_sections: &["entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "log",
                    value_type: ValueKind::Enum,
                    in_sections: &["entries"],
                    allow_none: true,
                    enum_values: &["config", "none", "packet", "summary", "verbose"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "paths",
                    value_type: ValueKind::String,
                    in_sections: &["entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "protocol",
                    value_type: ValueKind::Integer,
                    in_sections: &["entries"],
                    default: Some("0"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scheme",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "src-end-port",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "src-start-port",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "src-subnet",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "action",
            value_type: ValueKind::Enum,
            in_sections: &["entries"],
            required: true,
            enum_values: &["allow", "continue", "discard", "reject", "unspec"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dst-end-port",
            value_type: ValueKind::Unknown,
            in_sections: &["entries"],
            allow_none: true,
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dst-start-port",
            value_type: ValueKind::Unknown,
            in_sections: &["entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dst-subnet",
            value_type: ValueKind::Unknown,
            in_sections: &["entries"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "host",
            value_type: ValueKind::String,
            in_sections: &["entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log",
            value_type: ValueKind::Enum,
            in_sections: &["entries"],
            allow_none: true,
            enum_values: &["config", "none", "packet", "summary", "verbose"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "paths",
            value_type: ValueKind::String,
            in_sections: &["entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "protocol",
            value_type: ValueKind::Integer,
            in_sections: &["entries"],
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scheme",
            value_type: ValueKind::Unknown,
            in_sections: &["entries"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "src-end-port",
            value_type: ValueKind::Unknown,
            in_sections: &["entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "src-start-port",
            value_type: ValueKind::Unknown,
            in_sections: &["entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "src-subnet",
            value_type: ValueKind::Unknown,
            in_sections: &["entries"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "location-specific",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "path-match-case",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("true"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Enum,
            enum_values: &["dynamic", "static"],
            default: Some("static"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
