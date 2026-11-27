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

//! BIG-IP object specs for `sys application`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_application_apl_script",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["application apl-script"],
        },
        header_types: &[("sys", "application apl-script")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_application_custom_stat",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["application custom-stat"],
        },
        header_types: &[("sys", "application custom-stat")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_application_service",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["application service"],
        },
        header_types: &[("sys", "application service")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "execute-action",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lists",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "encrypted",
                        value_type: ValueKind::Enum,
                        in_sections: &["lists"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "value",
                        value_type: ValueKind::Unknown,
                        in_sections: &["lists"],
                        repeated: true,
                        allow_none: true,
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encrypted",
                value_type: ValueKind::Enum,
                in_sections: &["lists"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::Unknown,
                in_sections: &["lists"],
                repeated: true,
                allow_none: true,
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                default: Some(
                    "persistent, which means the data will be saved into the config file",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "strict-updates",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tables",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "column-names",
                        value_type: ValueKind::List,
                        in_sections: &["tables"],
                        repeated: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "encrypted-columns",
                        value_type: ValueKind::List,
                        in_sections: &["tables"],
                        repeated: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "rows",
                        value_type: ValueKind::List,
                        in_sections: &["tables"],
                        repeated: true,
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "column-names",
                value_type: ValueKind::List,
                in_sections: &["tables"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encrypted-columns",
                value_type: ValueKind::List,
                in_sections: &["tables"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rows",
                value_type: ValueKind::List,
                in_sections: &["tables"],
                repeated: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "template",
                value_type: ValueKind::Reference,
                references: &[
                    "security_bot_defense_template",
                    "sys_application_template",
                    "vcmp_virtual_disk_template",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-group",
                value_type: ValueKind::String,
                allow_none: true,
                references: &["cm_traffic_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "variables",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "encrypted",
                        value_type: ValueKind::Enum,
                        in_sections: &["variables"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "value",
                        value_type: ValueKind::String,
                        in_sections: &["variables"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encrypted",
                value_type: ValueKind::Enum,
                in_sections: &["variables"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                in_sections: &["variables"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_application_template",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["application template"],
        },
        header_types: &[("sys", "application template")],
        properties: &[
            BigipPropertySpec {
                name: "actions",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "definition",
                    value_type: ValueKind::Unknown,
                    in_sections: &["actions"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "definition",
                value_type: ValueKind::Unknown,
                in_sections: &["actions"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "html-help",
                        value_type: ValueKind::String,
                        in_sections: &["actions", "definition"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "implementation",
                        value_type: ValueKind::Unknown,
                        in_sections: &["actions", "definition"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "presentation",
                        value_type: ValueKind::Unknown,
                        in_sections: &["actions", "definition"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "role-acl",
                        value_type: ValueKind::List,
                        in_sections: &["actions", "definition"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "run-as",
                        value_type: ValueKind::String,
                        in_sections: &["actions", "definition"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "html-help",
                value_type: ValueKind::String,
                in_sections: &["actions", "definition"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "implementation",
                value_type: ValueKind::Unknown,
                in_sections: &["actions", "definition"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "presentation",
                value_type: ValueKind::Unknown,
                in_sections: &["actions", "definition"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "role-acl",
                value_type: ValueKind::List,
                in_sections: &["actions", "definition"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "run-as",
                value_type: ValueKind::String,
                in_sections: &["actions", "definition"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                default: Some("persistent, which saves the data into the config file"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "requires-bigip-version-max",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "requires-bigip-version-min",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "requires-modules",
                value_type: ValueKind::List,
                required: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-verification",
                value_type: ValueKind::String,
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signing-key",
                value_type: ValueKind::String,
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tmpl-checksum",
                value_type: ValueKind::String,
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tmpl-signature",
                value_type: ValueKind::String,
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
