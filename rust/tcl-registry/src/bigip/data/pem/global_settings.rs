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

//! BIG-IP object specs for `pem global-settings`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_analytics",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings analytics"],
        },
        header_types: &[("pem", "global-settings analytics")],
        properties: &[
            BigipPropertySpec {
                name: "logging",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[BigipPropertySpec {
                    name: "hsl",
                    value_type: ValueKind::Unknown,
                    in_sections: &["logging"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hsl",
                value_type: ValueKind::Unknown,
                in_sections: &["logging"],
                shape_kind: Some(ValueKind::Object),
                block: &[BigipPropertySpec {
                    name: "endpoint-id",
                    value_type: ValueKind::Unknown,
                    in_sections: &["logging", "hsl"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "endpoint-id",
                value_type: ValueKind::Unknown,
                in_sections: &["logging", "hsl"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subscriber-aware",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_gx",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings gx"],
        },
        header_types: &[("pem", "global-settings gx")],
        properties: &[
            BigipPropertySpec {
                name: "gx-immediate-reporting",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gx-usage-record-merge",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_hsl_flow",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings hsl-flow"],
        },
        header_types: &[("pem", "global-settings hsl-flow")],
        properties: &[
            BigipPropertySpec {
                name: "format-version",
                value_type: ValueKind::Unknown,
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "report-on-interim",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "report-on-start",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_hsl_report",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings hsl-report"],
        },
        header_types: &[("pem", "global-settings hsl-report")],
        properties: &[BigipPropertySpec {
            name: "format-version",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_insert_content",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings insert-content"],
        },
        header_types: &[("pem", "global-settings insert-content")],
        properties: &[BigipPropertySpec {
            name: "max-duration",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_policy",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings policy"],
        },
        header_types: &[("pem", "global-settings policy")],
        properties: &[
            BigipPropertySpec {
                name: "mandatory-action-list",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "report-flow",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "single-rule-match-mode",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_quota_mgmt",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings quota-mgmt"],
        },
        header_types: &[("pem", "global-settings quota-mgmt")],
        properties: &[
            BigipPropertySpec {
                name: "default-rating-group",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-context-id",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_session_mgmt_attributes",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings session-mgmt-attributes"],
        },
        header_types: &[("pem", "global-settings session-mgmt-attributes")],
        properties: &[
            BigipPropertySpec {
                name: "clear-on-nas-reboot",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "create-event",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "delete-event",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flow-term-on-sess-delete",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "update-event",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "pem_global_settings_subscriber_activity_log",
            table_name: None,
            resolver_name: None,
            module: Some("pem"),
            object_types: &["global-settings subscriber-activity-log"],
        },
        header_types: &[("pem", "global-settings subscriber-activity-log")],
        properties: &[
            BigipPropertySpec {
                name: "dynamic-subscriber-ids",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "publisher",
                value_type: ValueKind::Reference,
                references: &[
                    "security_blacklist_publisher_all_blacklist_publisher",
                    "security_blacklist_publisher_blacklist_publisher_stats",
                    "security_blacklist_publisher_by_addr",
                    "security_blacklist_publisher_by_category",
                    "security_blacklist_publisher_category",
                    "security_blacklist_publisher_profile",
                    "sys_icall_publisher",
                    "sys_log_config_publisher",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "static-subscriber-ids",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subscriber-ip-addresses",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
