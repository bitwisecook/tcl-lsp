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

//! BIG-IP object specs for `sys icall`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_icall_handler_periodic",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["icall handler periodic"],
        },
        header_types: &[("sys", "icall handler periodic")],
        properties: &[
            BigipPropertySpec {
                name: "arguments",
                value_type: ValueKind::List,
                usage_flags: &["optional"],
                block: &[BigipPropertySpec {
                    name: "value",
                    value_type: ValueKind::String,
                    in_sections: &["arguments"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                in_sections: &["arguments"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-occurrence",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-occurrence",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "script",
                value_type: ValueKind::Reference,
                references: &[
                    "cli_script",
                    "pem_reporting_format_script",
                    "sys_application_apl_script",
                    "sys_icall_script",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "status",
                value_type: ValueKind::Enum,
                enum_values: &["active", "inactive"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_icall_handler_perpetual",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["icall handler perpetual"],
        },
        header_types: &[("sys", "icall handler perpetual")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "restart",
                value_type: ValueKind::Reference,
                references: &["restart"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "script",
                value_type: ValueKind::Reference,
                references: &[
                    "cli_script",
                    "pem_reporting_format_script",
                    "sys_application_apl_script",
                    "sys_icall_script",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "start",
                value_type: ValueKind::Reference,
                references: &["start"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "status",
                value_type: ValueKind::Enum,
                enum_values: &["active", "inactive", "suspend"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "stop",
                value_type: ValueKind::Reference,
                references: &["stop"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subscriptions",
                value_type: ValueKind::List,
                usage_flags: &["optional"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "event-name",
                        value_type: ValueKind::Reference,
                        in_sections: &["subscriptions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "filters",
                        value_type: ValueKind::List,
                        in_sections: &["subscriptions"],
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "event-name",
                value_type: ValueKind::Reference,
                in_sections: &["subscriptions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "filters",
                value_type: ValueKind::List,
                in_sections: &["subscriptions"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "match-algorithm",
                        value_type: ValueKind::Enum,
                        in_sections: &["subscriptions", "filters"],
                        enum_values: &["accept-all", "exact", "subnet"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "value",
                        value_type: ValueKind::String,
                        in_sections: &["subscriptions", "filters"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-algorithm",
                value_type: ValueKind::Enum,
                in_sections: &["subscriptions", "filters"],
                enum_values: &["accept-all", "exact", "subnet"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                in_sections: &["subscriptions", "filters"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_icall_handler_triggered",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["icall handler triggered"],
        },
        header_types: &[("sys", "icall handler triggered")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "script",
                value_type: ValueKind::Reference,
                references: &[
                    "cli_script",
                    "pem_reporting_format_script",
                    "sys_application_apl_script",
                    "sys_icall_script",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "status",
                value_type: ValueKind::Enum,
                enum_values: &["active", "inactive"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subscriptions",
                value_type: ValueKind::List,
                usage_flags: &["optional"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "event-name",
                        value_type: ValueKind::Reference,
                        in_sections: &["subscriptions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "filters",
                        value_type: ValueKind::List,
                        in_sections: &["subscriptions"],
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "event-name",
                value_type: ValueKind::Reference,
                in_sections: &["subscriptions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "filters",
                value_type: ValueKind::List,
                in_sections: &["subscriptions"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "match-algorithm",
                        value_type: ValueKind::Enum,
                        in_sections: &["subscriptions", "filters"],
                        enum_values: &["accept-all", "exact", "subnet"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "value",
                        value_type: ValueKind::String,
                        in_sections: &["subscriptions", "filters"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-algorithm",
                value_type: ValueKind::Enum,
                in_sections: &["subscriptions", "filters"],
                enum_values: &["accept-all", "exact", "subnet"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                in_sections: &["subscriptions", "filters"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_icall_istats_trigger",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["icall istats-trigger"],
        },
        header_types: &[("sys", "icall istats-trigger")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "duration",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "event-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "istats-key",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "range-max",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "range-min",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "repeat",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_icall_script",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["icall script"],
        },
        header_types: &[("sys", "icall script")],
        properties: &[
            BigipPropertySpec {
                name: "definition",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "events",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "contexts",
                    value_type: ValueKind::List,
                    in_sections: &["events"],
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "contexts",
                value_type: ValueKind::List,
                in_sections: &["events"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
