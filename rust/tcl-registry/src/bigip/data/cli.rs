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

//! BIG-IP object specs for the `cli` tmsh module.
//!
//! **Hand-maintained.** These files were originally produced by a
//! one-time port of the pre-rewrite Python registry
//! (`dialects/f5/bigip/registry/specs/`, still present on `main`) via a
//! `scripts/registry-audit/gen_bigip_rust.py` generator that no longer
//! exists — deleted along with the rest of the retired Python tooling on
//! this branch, and the commits that ran it were squashed away in the
//! `rust` branch's rebase-onto-main history (see issue #1404). Originally
//! organised by the first letter of `kind`; reorganised by tmsh module
//! name (this file's own module field, `Some("cli")`) per
//! maintainer review on the PR that introduced the consistency gate below.
//! There is no live generator to regenerate these from; edit them by hand,
//! in the same shape the other module files already use.
//!
//! `cargo xtask bigip-data-schema --check` enforces the internal
//! invariants a generator would otherwise have guaranteed: every `kind`
//! is globally unique, filed under the module file matching its own
//! `module` field, and every `references` target either names a real
//! kind or is on the documented known-gap list.
// Some modules hold property-less kinds, so not every imported type
// is used in every file; large tmsh bounds appear as bare f64 literals.
#![allow(unused_imports, clippy::unreadable_literal)]
use super::super::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cli_admin_partitions",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["admin-partitions"],
        },
        header_types: &[("cli", "admin-partitions")],
        properties: &[BigipPropertySpec {
            name: "update-partition",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cli_alias_private",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["alias private"],
        },
        header_types: &[("cli", "alias private")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "command",
                value_type: ValueKind::Unknown,
                repeated: true,
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
            kind: "cli_alias_shared",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["alias shared"],
        },
        header_types: &[("cli", "alias shared")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "command",
                value_type: ValueKind::Unknown,
                repeated: true,
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
            kind: "cli_global_settings",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["global-settings"],
        },
        header_types: &[("cli", "global-settings")],
        properties: &[
            BigipPropertySpec {
                name: "audit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idle-timeout",
                value_type: ValueKind::Integer,
                enum_values: &["disabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scf-backup-number",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service",
                value_type: ValueKind::Enum,
                enum_values: &["number"],
                references: &[
                    "analytics_ssl_orchestrator_service_virtual_report",
                    "analytics_ssl_orchestrator_service_virtual_scheduled_report",
                    "apm_aaa_f5_service_connector",
                    "apm_saml_artifact_resolution_service",
                    "apm_saml_attribute_consuming_service",
                    "net_service_policy",
                    "pem_service_chain_endpoint",
                    "security_bot_defense_micro_service",
                    "security_protocol_inspection_service",
                    "sys_application_service",
                    "sys_service",
                ],
                default: Some("name"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cli_preference",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["preference"],
        },
        header_types: &[("cli", "preference")],
        properties: &[
            BigipPropertySpec {
                name: "alias-path",
                value_type: ValueKind::Unknown,
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
                name: "confirm-edit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "display-threshold",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "editor",
                value_type: ValueKind::Enum,
                enum_values: &["nano", "vi"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fully-qualified-host",
                value_type: ValueKind::Unknown,
                default: Some("to not display this information"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "history-date-time",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "history-file-size",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "history-size",
                value_type: ValueKind::Integer,
                default: Some("500"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "keymap",
                value_type: ValueKind::Enum,
                enum_values: &["default", "emacs", "vi"],
                default: Some("default"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "list-all-properties",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mcp-state",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("to not display this information"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pager",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prompt",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "show-aliases",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "stat-units",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "default", "exa", "gig", "kil", "meg", "peta", "raw", "tera", "yotta", "zetta",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "suppress-warnings",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["all", "config-version", "none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "table-indent-width",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tcl-syntax-highlighting",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "video",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "warn",
                value_type: ValueKind::Enum,
                enum_values: &["bell", "disabled", "visual-bell"],
                default: Some("bell"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cli_script",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["script"],
        },
        header_types: &[("cli", "script")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cli_transaction",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["transaction"],
        },
        header_types: &[("cli", "transaction")],
        properties: &[BigipPropertySpec {
            name: "submit",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "cli_version",
            table_name: None,
            resolver_name: None,
            module: Some("cli"),
            object_types: &["version"],
        },
        header_types: &[("cli", "version")],
        properties: &[BigipPropertySpec {
            name: "active",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
];
