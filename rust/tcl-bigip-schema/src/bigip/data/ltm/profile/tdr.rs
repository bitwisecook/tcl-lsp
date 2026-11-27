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

//! BIG-IP object specs for `ltm profile tdr`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_tdr",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile tdr"],
    },
    header_types: &[("ltm", "profile tdr")],
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
            allow_none: true,
            references: &["ltm_profile_tdr"],
            default: Some("http2"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "filters",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["filters"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "condition-pattern-1",
                    value_type: ValueKind::Unknown,
                    in_sections: &["filters"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "condition-pattern-2",
                    value_type: ValueKind::Unknown,
                    in_sections: &["filters"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "condition-pattern-3",
                    value_type: ValueKind::Unknown,
                    in_sections: &["filters"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "condition-pattern-4",
                    value_type: ValueKind::Unknown,
                    in_sections: &["filters"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::String,
                    in_sections: &["filters"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "message-type",
                    value_type: ValueKind::Enum,
                    in_sections: &["filters"],
                    enum_values: &["all", "answer", "request"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "tdr-format",
                    value_type: ValueKind::String,
                    in_sections: &["filters"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "traffic-direction",
                    value_type: ValueKind::Enum,
                    in_sections: &["filters"],
                    enum_values: &["all", "egress", "ingress"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["filters"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "condition-pattern-1",
            value_type: ValueKind::Unknown,
            in_sections: &["filters"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "cmp-operator",
                    value_type: ValueKind::Enum,
                    in_sections: &["filters", "condition-pattern-1"],
                    allow_none: true,
                    enum_values: &[
                        "contains",
                        "ends-with",
                        "equal",
                        "none",
                        "not-equal",
                        "starts-with",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "field-name",
                    value_type: ValueKind::String,
                    in_sections: &["filters", "condition-pattern-1"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "field-value",
                    value_type: ValueKind::String,
                    in_sections: &["filters", "condition-pattern-1"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cmp-operator",
            value_type: ValueKind::Enum,
            in_sections: &["filters", "condition-pattern-1"],
            allow_none: true,
            enum_values: &[
                "contains",
                "ends-with",
                "equal",
                "none",
                "not-equal",
                "starts-with",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "field-name",
            value_type: ValueKind::String,
            in_sections: &["filters", "condition-pattern-1"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "field-value",
            value_type: ValueKind::String,
            in_sections: &["filters", "condition-pattern-1"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "condition-pattern-2",
            value_type: ValueKind::Unknown,
            in_sections: &["filters"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "cmp-operator",
                    value_type: ValueKind::Enum,
                    in_sections: &["filters", "condition-pattern-2"],
                    allow_none: true,
                    enum_values: &[
                        "contains",
                        "ends-with",
                        "equal",
                        "none",
                        "not-equal",
                        "starts-with",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "field-name",
                    value_type: ValueKind::String,
                    in_sections: &["filters", "condition-pattern-2"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "field-value",
                    value_type: ValueKind::String,
                    in_sections: &["filters", "condition-pattern-2"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cmp-operator",
            value_type: ValueKind::Enum,
            in_sections: &["filters", "condition-pattern-2"],
            allow_none: true,
            enum_values: &[
                "contains",
                "ends-with",
                "equal",
                "none",
                "not-equal",
                "starts-with",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "field-name",
            value_type: ValueKind::String,
            in_sections: &["filters", "condition-pattern-2"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "field-value",
            value_type: ValueKind::String,
            in_sections: &["filters", "condition-pattern-2"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "condition-pattern-3",
            value_type: ValueKind::Unknown,
            in_sections: &["filters"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "cmp-operator",
                    value_type: ValueKind::Enum,
                    in_sections: &["filters", "condition-pattern-3"],
                    allow_none: true,
                    enum_values: &[
                        "contains",
                        "ends-with",
                        "equal",
                        "none",
                        "not-equal",
                        "starts-with",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "field-name",
                    value_type: ValueKind::String,
                    in_sections: &["filters", "condition-pattern-3"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "field-value",
                    value_type: ValueKind::String,
                    in_sections: &["filters", "condition-pattern-3"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cmp-operator",
            value_type: ValueKind::Enum,
            in_sections: &["filters", "condition-pattern-3"],
            allow_none: true,
            enum_values: &[
                "contains",
                "ends-with",
                "equal",
                "none",
                "not-equal",
                "starts-with",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "field-name",
            value_type: ValueKind::String,
            in_sections: &["filters", "condition-pattern-3"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "field-value",
            value_type: ValueKind::String,
            in_sections: &["filters", "condition-pattern-3"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "condition-pattern-4",
            value_type: ValueKind::Unknown,
            in_sections: &["filters"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "cmp-operator",
                    value_type: ValueKind::Enum,
                    in_sections: &["filters", "condition-pattern-4"],
                    allow_none: true,
                    enum_values: &[
                        "contains",
                        "ends-with",
                        "equal",
                        "none",
                        "not-equal",
                        "starts-with",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "field-name",
                    value_type: ValueKind::String,
                    in_sections: &["filters", "condition-pattern-4"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "field-value",
                    value_type: ValueKind::String,
                    in_sections: &["filters", "condition-pattern-4"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cmp-operator",
            value_type: ValueKind::Enum,
            in_sections: &["filters", "condition-pattern-4"],
            allow_none: true,
            enum_values: &[
                "contains",
                "ends-with",
                "equal",
                "none",
                "not-equal",
                "starts-with",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "field-name",
            value_type: ValueKind::String,
            in_sections: &["filters", "condition-pattern-4"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "field-value",
            value_type: ValueKind::String,
            in_sections: &["filters", "condition-pattern-4"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["filters"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "message-type",
            value_type: ValueKind::Enum,
            in_sections: &["filters"],
            enum_values: &["all", "answer", "request"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tdr-format",
            value_type: ValueKind::String,
            in_sections: &["filters"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "traffic-direction",
            value_type: ValueKind::Enum,
            in_sections: &["filters"],
            enum_values: &["all", "egress", "ingress"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-publisher",
            value_type: ValueKind::Reference,
            allow_none: true,
            enum_values: &["none"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
