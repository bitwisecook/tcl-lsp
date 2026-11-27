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

//! BIG-IP object specs for `ltm classification`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_classification_application",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["classification application"],
        },
        header_types: &[("ltm", "classification application")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "application-id",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "category",
                value_type: ValueKind::Reference,
                references: &[
                    "ltm_classification_category",
                    "ltm_classification_stats_url_category",
                    "ltm_classification_url_category",
                    "security_blacklist_publisher_by_category",
                    "security_blacklist_publisher_category",
                    "security_bot_defense_anomaly_category",
                    "security_bot_defense_signature_category",
                    "security_dos_bot_signature_category",
                    "security_firewall_ipi_category_info",
                    "security_ip_intelligence_blacklist_category",
                    "security_scrubber_dwbl_scrubber_category_stats",
                    "sys_url_db_url_category",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_classification_auto_update_settings",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["classification auto-update settings"],
        },
        header_types: &[("ltm", "classification auto-update settings")],
        properties: &[
            BigipPropertySpec {
                name: "auto-update-interval",
                value_type: ValueKind::Enum,
                enum_values: &["daily", "monthly", "weekly"],
                default: Some("weekly"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enabled",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_classification_category",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["classification category"],
        },
        header_types: &[("ltm", "classification category")],
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
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_classification_ce",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["classification ce"],
        },
        header_types: &[("ltm", "classification ce")],
        properties: &[
            BigipPropertySpec {
                name: "allow-reclassification",
                value_type: ValueKind::Enum,
                enum_values: &["off", "on"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "analyze-dns",
                value_type: ValueKind::Enum,
                enum_values: &["off", "on"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "analyze-ssl-serverside",
                value_type: ValueKind::Enum,
                enum_values: &["off", "on"],
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
                name: "cache-results",
                value_type: ValueKind::Enum,
                enum_values: &["off", "on"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flow-bundling",
                value_type: ValueKind::Enum,
                enum_values: &["off", "on"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "policies",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_classification_signature_update_schedule",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["classification signature-update-schedule"],
        },
        header_types: &[("ltm", "classification signature-update-schedule")],
        properties: &[BigipPropertySpec {
            name: "auto-update-interval",
            value_type: ValueKind::Enum,
            enum_values: &["daily", "monthly", "weekly"],
            default: Some("weekly"),
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_classification_url_cat_policy",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["classification url-cat-policy"],
        },
        header_types: &[("ltm", "classification url-cat-policy")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::Reference,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "feed-lists",
                value_type: ValueKind::List,
                references: &["security_ip_intelligence_feed_list"],
                list_operators: &["add", "delete"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_classification_url_category",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["classification url-category"],
        },
        header_types: &[("ltm", "classification url-category")],
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
                name: "irule-event",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-category-id",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_classification_urldb_feed_list",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["classification urldb-feed-list"],
        },
        header_types: &[("ltm", "classification urldb-feed-list")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::Reference,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-url-category",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load",
                value_type: ValueKind::Reference,
                references: &["gtm_global_settings_load_balancing", "load"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "poll-interval",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_classification_urldb_file",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["classification urldb-file"],
        },
        header_types: &[("ltm", "classification urldb-file")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::Reference,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
