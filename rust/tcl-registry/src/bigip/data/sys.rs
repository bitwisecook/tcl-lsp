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

//! BIG-IP object specs for the `sys` tmsh module.
//!
//! **Hand-maintained.** These files were originally produced by a
//! one-time port of the pre-rewrite Python registry
//! (`dialects/f5/bigip/registry/specs/`, still present on `main`) via a
//! `scripts/registry-audit/gen_bigip_rust.py` generator that no longer
//! exists — deleted along with the rest of the retired Python tooling on
//! this branch, and the commits that ran it were squashed away in the
//! `rust` branch's rebase-onto-main history (see issue #1404). Originally
//! organised by the first letter of `kind`; reorganised by tmsh module
//! name (this file's own module field, `Some("sys")`) per
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
            kind: "sys_appiq_config",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["appiq config"],
        },
        header_types: &[("sys", "appiq config")],
        properties: &[BigipPropertySpec {
            name: "host-ip",
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        }],
    },
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
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_autoscale_group",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["autoscale-group"],
        },
        header_types: &[("sys", "autoscale-group")],
        properties: &[
            BigipPropertySpec {
                name: "autoscale-group-id",
                value_type: ValueKind::String,
                allow_none: true,
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
            kind: "sys_clock",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["clock"],
        },
        header_types: &[("sys", "clock")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_cluster",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["cluster"],
        },
        header_types: &[("sys", "cluster")],
        properties: &[
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "alt-address",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "members",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "address",
                        value_type: ValueKind::Enum,
                        in_sections: &["members"],
                        allow_none: true,
                        enum_values: &["none"],
                        shape_kind: Some(ValueKind::IpAddress),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "alt-address",
                        value_type: ValueKind::Enum,
                        in_sections: &["members"],
                        allow_none: true,
                        enum_values: &["none"],
                        shape_kind: Some(ValueKind::IpAddress),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "priming",
                        value_type: ValueKind::Enum,
                        in_sections: &["members"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::Enum,
                in_sections: &["members"],
                allow_none: true,
                enum_values: &["none"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "alt-address",
                value_type: ValueKind::Enum,
                in_sections: &["members"],
                allow_none: true,
                enum_values: &["none"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "priming",
                value_type: ValueKind::Enum,
                in_sections: &["members"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-up-members",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-up-members-enabled",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_config",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["config"],
        },
        header_types: &[("sys", "config")],
        properties: &[
            BigipPropertySpec {
                name: "base",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "binary",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "current-partition",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "exclude-gtm",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "file",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "files-folder",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "from-terminal",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gtm-only",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "merge",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-passphrase",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "partitions",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "passphrase",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "replace",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tar-file",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-stamp",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-only",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "wait",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_connection",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["connection"],
        },
        header_types: &[("sys", "connection")],
        properties: &[
            BigipPropertySpec {
                name: "flow-accel-type",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idle-timeout",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_console",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["console"],
        },
        header_types: &[("sys", "console")],
        properties: &[BigipPropertySpec {
            name: "baud-rate",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_core",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["core"],
        },
        header_types: &[("sys", "core")],
        properties: &[
            BigipPropertySpec {
                name: "bigd-action",
                value_type: ValueKind::Enum,
                enum_values: &["rotate", "skip"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bigd-manage",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bigd-max",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mcpd-action",
                value_type: ValueKind::Enum,
                enum_values: &["rotate", "skip"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mcpd-manage",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mcpd-max",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "retention",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tmm-action",
                value_type: ValueKind::Enum,
                enum_values: &["rotate", "skip"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tmm-manage",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tmm-max",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_crypto_allow_key_export",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["crypto allow-key-export"],
        },
        header_types: &[("sys", "crypto allow-key-export")],
        properties: &[BigipPropertySpec {
            name: "value",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_crypto_ca_bundle_manager",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["crypto ca-bundle-manager"],
        },
        header_types: &[("sys", "crypto ca-bundle-manager")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-server",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-out",
                value_type: ValueKind::Unknown,
                default: Some("8 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trusted-ca-bundle",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "update-interval",
                value_type: ValueKind::Unknown,
                default: Some("0, which means the generated ca-bundle is not dynamically updated"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "update-now",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_crypto_cert",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["crypto cert"],
        },
        header_types: &[("sys", "crypto cert")],
        properties: &[
            BigipPropertySpec {
                name: "cert-validation-options",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "ocsp"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cert-validators",
                value_type: ValueKind::Reference,
                references: &[
                    "sys_crypto_cert_validator_crl",
                    "sys_crypto_cert_validator_ocsp",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "city",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "common-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "consumer",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "country",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-address",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "issuer-cert",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "key",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "organization",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ou",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subject-alternative-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_crypto_cert_order_manager",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["crypto cert-order-manager"],
        },
        header_types: &[("sys", "crypto cert-order-manager")],
        properties: &[
            BigipPropertySpec {
                name: "additional-headers",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "authority",
                value_type: ValueKind::Enum,
                enum_values: &["comodo", "digicert", "godaddy", "symantec"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-renew",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "base-url",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["URL", "none"],
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ca-cert",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-cert",
                value_type: ValueKind::Enum,
                required: true,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-key",
                value_type: ValueKind::Enum,
                required: true,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-key-passphrase",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "edit-order-info",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "internal-proxy",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "login-name",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "login-password",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "order-info",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "validity-days",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["days", "none"],
                default: Some("365 days"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_crypto_cert_validation_response_ocsp",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["crypto cert-validation-response ocsp"],
        },
        header_types: &[("sys", "crypto cert-validation-response ocsp")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_crypto_cert_validator_crl",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["crypto cert-validator crl"],
        },
        header_types: &[("sys", "crypto cert-validator crl")],
        properties: &[
            BigipPropertySpec {
                name: "internal-proxy",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "strict-revocation-check",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_crypto_cert_validator_ocsp",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["crypto cert-validator ocsp"],
        },
        header_types: &[("sys", "crypto cert-validator ocsp")],
        properties: &[
            BigipPropertySpec {
                name: "cache-error-timeout",
                value_type: ValueKind::Integer,
                default: Some("3600 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-timeout",
                value_type: ValueKind::Integer,
                default: Some(
                    "indefinite, indicating that the response validity period takes precedence",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "clock-skew",
                value_type: ValueKind::Integer,
                default: Some("300"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "concurrent-connections-limit",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-resolver",
                value_type: ValueKind::Reference,
                references: &["net_dns_resolver"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-server-pool",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "responder-url",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Reference,
                references: &["net_route_domain"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sign-hash",
                value_type: ValueKind::Enum,
                enum_values: &["sha1", "sha256"],
                default: Some("sha256"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signer-cert",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signer-key",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signer-key-passphrase",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "status-age",
                value_type: ValueKind::Integer,
                default: Some("86400 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "strict-resp-cert-check",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("8"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trusted-responders",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_crypto_client",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["crypto client"],
        },
        header_types: &[("sys", "crypto client")],
        properties: &[
            BigipPropertySpec {
                name: "addr",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "connection-reset",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "heartbeat",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-retries",
                value_type: ValueKind::Enum,
                enum_values: &["infinite"],
                default: Some("infinite"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "profiles",
                value_type: ValueKind::List,
                repeated: true,
                references: &[
                    "analytics_dns_profile_report",
                    "api_protection_profile_apiprotection",
                    "apm_profile_access",
                    "apm_profile_connectivity",
                    "apm_profile_exchange",
                    "apm_profile_oauth",
                    "apm_profile_remote_desktop",
                    "apm_profile_vdi",
                    "ltm_alg_log_profile",
                    "ltm_auth_profile",
                    "ltm_dns_hpke_profile",
                    "ltm_lsn_log_profile",
                    "ltm_message_routing_diameter_profile_router",
                    "ltm_message_routing_diameter_profile_session",
                    "ltm_message_routing_mqtt_profile_router",
                    "ltm_message_routing_mqtt_profile_session",
                    "ltm_message_routing_sip_profile_router",
                    "ltm_message_routing_sip_profile_session",
                    "ltm_profile_analytics",
                    "ltm_profile_certificate_authority",
                    "ltm_profile_classification",
                    "ltm_profile_client_ldap",
                    "ltm_profile_client_ssl",
                    "ltm_profile_connector",
                    "ltm_profile_dhcpv4",
                    "ltm_profile_dhcpv6",
                    "ltm_profile_diameter",
                    "ltm_profile_dns",
                    "ltm_profile_dns_logging",
                    "ltm_profile_doh_proxy",
                    "ltm_profile_doh_server",
                    "ltm_profile_fasthttp",
                    "ltm_profile_fastl4",
                    "ltm_profile_fix",
                    "ltm_profile_ftp",
                    "ltm_profile_georedundancy",
                    "ltm_profile_gtp",
                    "ltm_profile_html",
                    "ltm_profile_http",
                    "ltm_profile_http2",
                    "ltm_profile_http3",
                    "ltm_profile_http_compression",
                    "ltm_profile_httprouter",
                    "ltm_profile_icap",
                    "ltm_profile_iiop",
                    "ltm_profile_ilx",
                    "ltm_profile_imap",
                    "ltm_profile_ipother",
                    "ltm_profile_ipsecalg",
                    "ltm_profile_json",
                    "ltm_profile_mapt",
                    "ltm_profile_mblb",
                    "ltm_profile_mqtt",
                    "ltm_profile_mr_ratelimit",
                    "ltm_profile_mr_ratelimit_action",
                    "ltm_profile_mssql",
                    "ltm_profile_netflow",
                    "ltm_profile_ntlm",
                    "ltm_profile_ocsp",
                    "ltm_profile_ocsp_stapling_params",
                    "ltm_profile_one_connect",
                    "ltm_profile_pcp",
                    "ltm_profile_pop3",
                    "ltm_profile_pptp",
                    "ltm_profile_qoe",
                    "ltm_profile_quic",
                    "ltm_profile_radius",
                    "ltm_profile_ramcache",
                    "ltm_profile_request_adapt",
                    "ltm_profile_request_log",
                    "ltm_profile_response_adapt",
                    "ltm_profile_rewrite",
                    "ltm_profile_rtsp",
                    "ltm_profile_sctp",
                    "ltm_profile_server_ldap",
                    "ltm_profile_server_ssl",
                    "ltm_profile_sip",
                    "ltm_profile_smtp",
                    "ltm_profile_smtps",
                    "ltm_profile_socks",
                    "ltm_profile_splitsessionclient",
                    "ltm_profile_splitsessionserver",
                    "ltm_profile_sse",
                    "ltm_profile_statistics",
                    "ltm_profile_stream",
                    "ltm_profile_tcp",
                    "ltm_profile_tcp_analytics",
                    "ltm_profile_tdr",
                    "ltm_profile_tftp",
                    "ltm_profile_traffic_acceleration",
                    "ltm_profile_udp",
                    "ltm_profile_wa_cache",
                    "ltm_profile_web_acceleration",
                    "ltm_profile_web_security",
                    "ltm_profile_websocket",
                    "ltm_profile_xml",
                    "net_routing_profile_bgp",
                    "pem_profile_diameter_endpoint",
                    "pem_profile_radius_aaa",
                    "pem_profile_spm",
                    "pem_profile_subscriber_mgmt",
                    "pem_protocol_profile_gx",
                    "pem_protocol_profile_radius",
                    "saas_ap_ai_profile",
                    "saas_ati_profile",
                    "saas_bd_profile",
                    "saas_csd_profile",
                    "security_anti_fraud_profile",
                    "security_blacklist_publisher_profile",
                    "security_bot_defense_profile",
                    "security_datasync_global_profile",
                    "security_datasync_local_profile",
                    "security_dos_profile",
                    "security_flowspec_route_injector_profile",
                    "security_http_profile",
                    "security_log_profile",
                    "security_protocol_inspection_profile",
                    "security_protocol_inspection_profile_status",
                    "security_scrubber_profile",
                    "security_ssh_profile",
                    "sys_fpga_turboflex_profile",
                    "sys_turboflex_profile_all",
                    "sys_turboflex_profile_config",
                    "sys_turboflex_profile_feature",
                    "vcmp_traffic_profile",
                    "wom_profile_cifs",
                    "wom_profile_isession",
                    "wom_profile_mapi",
                ],
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "req-timeout",
                value_type: ValueKind::Integer,
                default: Some("5000 milliseconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "retry-interval",
                value_type: ValueKind::Integer,
                default: Some("10 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_crypto_csr",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["crypto csr"],
        },
        header_types: &[("sys", "crypto csr")],
        properties: &[
            BigipPropertySpec {
                name: "admin-email-address",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "basic-constraints",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "challenge-password",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "city",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "common-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "consumer",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "country",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-address",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "key",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "key-usage",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "organization",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ou",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subject-alternative-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_crypto_fips_external_hsm",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["crypto fips external-hsm"],
        },
        header_types: &[("sys", "crypto fips external-hsm")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_crypto_fips_key",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["crypto fips key"],
        },
        header_types: &[("sys", "crypto fips key")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_crypto_key",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["crypto key"],
        },
        header_types: &[("sys", "crypto key")],
        properties: &[
            BigipPropertySpec {
                name: "admin-email-address",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cert-order-manager",
                value_type: ValueKind::List,
                usage_flags: &["optional"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "check-status",
                        value_type: ValueKind::Enum,
                        in_sections: &["cert-order-manager"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "order-id",
                        value_type: ValueKind::Enum,
                        in_sections: &["cert-order-manager"],
                        required: true,
                        allow_none: true,
                        enum_values: &["none"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "order-passphrase",
                        value_type: ValueKind::Enum,
                        in_sections: &["cert-order-manager"],
                        allow_none: true,
                        enum_values: &["none"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "order-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["cert-order-manager"],
                        enum_values: &["cancel", "new", "renew", "revoke"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "revoke-reason",
                        value_type: ValueKind::Enum,
                        in_sections: &["cert-order-manager"],
                        enum_values: &[
                            "AACompromise",
                            "CACompromise",
                            "affiliationChanged",
                            "certificateHold",
                            "cessationOfOperation",
                            "keyCompromise",
                            "privilegeWithdrawn",
                            "removeFromCRL",
                            "superseded",
                            "unspecified",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "check-status",
                value_type: ValueKind::Enum,
                in_sections: &["cert-order-manager"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "order-id",
                value_type: ValueKind::Enum,
                in_sections: &["cert-order-manager"],
                required: true,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "order-passphrase",
                value_type: ValueKind::Enum,
                in_sections: &["cert-order-manager"],
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "order-type",
                value_type: ValueKind::Enum,
                in_sections: &["cert-order-manager"],
                enum_values: &["cancel", "new", "renew", "revoke"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "revoke-reason",
                value_type: ValueKind::Enum,
                in_sections: &["cert-order-manager"],
                enum_values: &[
                    "AACompromise",
                    "CACompromise",
                    "affiliationChanged",
                    "certificateHold",
                    "cessationOfOperation",
                    "keyCompromise",
                    "privilegeWithdrawn",
                    "removeFromCRL",
                    "superseded",
                    "unspecified",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "challenge-password",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "city",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "common-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "consumer",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "country",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "curve-name",
                value_type: ValueKind::Enum,
                enum_values: &["prime256v1", "secp384r1", "secp521r1"],
                default: Some("prime256v1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-address",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "key-size",
                value_type: ValueKind::Enum,
                enum_values: &["1024", "2048", "4096", "512"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "key-type",
                value_type: ValueKind::Enum,
                enum_values: &["dsa-private", "ec-private", "rsa-private"],
                default: Some("rsa-private"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "organization",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ou",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "passphrase",
                value_type: ValueKind::Unknown,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prompt-for-password",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "security-type",
                value_type: ValueKind::Enum,
                enum_values: &["fips", "nethsm", "normal", "password"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subject-alternative-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_crypto_master_key",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["crypto master-key"],
        },
        header_types: &[("sys", "crypto master-key")],
        properties: &[BigipPropertySpec {
            name: "prompt-for-password",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_crypto_server",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["crypto server"],
        },
        header_types: &[("sys", "crypto server")],
        properties: &[
            BigipPropertySpec {
                name: "addr",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "clients",
                value_type: ValueKind::List,
                repeated: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "profiles",
                value_type: ValueKind::List,
                repeated: true,
                references: &[
                    "analytics_dns_profile_report",
                    "api_protection_profile_apiprotection",
                    "apm_profile_access",
                    "apm_profile_connectivity",
                    "apm_profile_exchange",
                    "apm_profile_oauth",
                    "apm_profile_remote_desktop",
                    "apm_profile_vdi",
                    "ltm_alg_log_profile",
                    "ltm_auth_profile",
                    "ltm_dns_hpke_profile",
                    "ltm_lsn_log_profile",
                    "ltm_message_routing_diameter_profile_router",
                    "ltm_message_routing_diameter_profile_session",
                    "ltm_message_routing_mqtt_profile_router",
                    "ltm_message_routing_mqtt_profile_session",
                    "ltm_message_routing_sip_profile_router",
                    "ltm_message_routing_sip_profile_session",
                    "ltm_profile_analytics",
                    "ltm_profile_certificate_authority",
                    "ltm_profile_classification",
                    "ltm_profile_client_ldap",
                    "ltm_profile_client_ssl",
                    "ltm_profile_connector",
                    "ltm_profile_dhcpv4",
                    "ltm_profile_dhcpv6",
                    "ltm_profile_diameter",
                    "ltm_profile_dns",
                    "ltm_profile_dns_logging",
                    "ltm_profile_doh_proxy",
                    "ltm_profile_doh_server",
                    "ltm_profile_fasthttp",
                    "ltm_profile_fastl4",
                    "ltm_profile_fix",
                    "ltm_profile_ftp",
                    "ltm_profile_georedundancy",
                    "ltm_profile_gtp",
                    "ltm_profile_html",
                    "ltm_profile_http",
                    "ltm_profile_http2",
                    "ltm_profile_http3",
                    "ltm_profile_http_compression",
                    "ltm_profile_httprouter",
                    "ltm_profile_icap",
                    "ltm_profile_iiop",
                    "ltm_profile_ilx",
                    "ltm_profile_imap",
                    "ltm_profile_ipother",
                    "ltm_profile_ipsecalg",
                    "ltm_profile_json",
                    "ltm_profile_mapt",
                    "ltm_profile_mblb",
                    "ltm_profile_mqtt",
                    "ltm_profile_mr_ratelimit",
                    "ltm_profile_mr_ratelimit_action",
                    "ltm_profile_mssql",
                    "ltm_profile_netflow",
                    "ltm_profile_ntlm",
                    "ltm_profile_ocsp",
                    "ltm_profile_ocsp_stapling_params",
                    "ltm_profile_one_connect",
                    "ltm_profile_pcp",
                    "ltm_profile_pop3",
                    "ltm_profile_pptp",
                    "ltm_profile_qoe",
                    "ltm_profile_quic",
                    "ltm_profile_radius",
                    "ltm_profile_ramcache",
                    "ltm_profile_request_adapt",
                    "ltm_profile_request_log",
                    "ltm_profile_response_adapt",
                    "ltm_profile_rewrite",
                    "ltm_profile_rtsp",
                    "ltm_profile_sctp",
                    "ltm_profile_server_ldap",
                    "ltm_profile_server_ssl",
                    "ltm_profile_sip",
                    "ltm_profile_smtp",
                    "ltm_profile_smtps",
                    "ltm_profile_socks",
                    "ltm_profile_splitsessionclient",
                    "ltm_profile_splitsessionserver",
                    "ltm_profile_sse",
                    "ltm_profile_statistics",
                    "ltm_profile_stream",
                    "ltm_profile_tcp",
                    "ltm_profile_tcp_analytics",
                    "ltm_profile_tdr",
                    "ltm_profile_tftp",
                    "ltm_profile_traffic_acceleration",
                    "ltm_profile_udp",
                    "ltm_profile_wa_cache",
                    "ltm_profile_web_acceleration",
                    "ltm_profile_web_security",
                    "ltm_profile_websocket",
                    "ltm_profile_xml",
                    "net_routing_profile_bgp",
                    "pem_profile_diameter_endpoint",
                    "pem_profile_radius_aaa",
                    "pem_profile_spm",
                    "pem_profile_subscriber_mgmt",
                    "pem_protocol_profile_gx",
                    "pem_protocol_profile_radius",
                    "saas_ap_ai_profile",
                    "saas_ati_profile",
                    "saas_bd_profile",
                    "saas_csd_profile",
                    "security_anti_fraud_profile",
                    "security_blacklist_publisher_profile",
                    "security_bot_defense_profile",
                    "security_datasync_global_profile",
                    "security_datasync_local_profile",
                    "security_dos_profile",
                    "security_flowspec_route_injector_profile",
                    "security_http_profile",
                    "security_log_profile",
                    "security_protocol_inspection_profile",
                    "security_protocol_inspection_profile_status",
                    "security_scrubber_profile",
                    "security_ssh_profile",
                    "sys_fpga_turboflex_profile",
                    "sys_turboflex_profile_all",
                    "sys_turboflex_profile_config",
                    "sys_turboflex_profile_feature",
                    "vcmp_traffic_profile",
                    "wom_profile_cifs",
                    "wom_profile_isession",
                    "wom_profile_mapi",
                ],
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_daemon_ha",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["daemon-ha"],
        },
        header_types: &[("sys", "daemon-ha")],
        properties: &[
            BigipPropertySpec {
                name: "heartbeat",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some(
                    "enabled for all daemons, except the named daemon, which is disabled by default",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "heartbeat-action",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "go-offline",
                    "go-offline-downlinks-restart",
                    "go-offline-restart",
                    "reboot",
                    "restart",
                    "restart-all",
                ],
                default: Some(
                    "dependent on the specified daemon, the most common default value is restart",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "running",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some(
                    "dependent on the specified daemon, the most common default value is enabled",
                ),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_daemon_log_settings_clusterd",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["daemon-log-settings clusterd"],
        },
        header_types: &[("sys", "daemon-log-settings clusterd")],
        properties: &[BigipPropertySpec {
            name: "log-level",
            value_type: ValueKind::Enum,
            enum_values: &[
                "critical",
                "debug",
                "error",
                "informational",
                "notice",
                "warning",
            ],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_daemon_log_settings_csyncd",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["daemon-log-settings csyncd"],
        },
        header_types: &[("sys", "daemon-log-settings csyncd")],
        properties: &[BigipPropertySpec {
            name: "log-level",
            value_type: ValueKind::Enum,
            enum_values: &[
                "critical",
                "debug",
                "error",
                "informational",
                "notice",
                "warning",
            ],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_daemon_log_settings_icr_eventd",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["daemon-log-settings icr-eventd"],
        },
        header_types: &[("sys", "daemon-log-settings icr-eventd")],
        properties: &[BigipPropertySpec {
            name: "log-level",
            value_type: ValueKind::Enum,
            enum_values: &[
                "critical",
                "debug",
                "error",
                "informational",
                "notice",
                "warning",
            ],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_daemon_log_settings_icrd",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["daemon-log-settings icrd"],
        },
        header_types: &[("sys", "daemon-log-settings icrd")],
        properties: &[BigipPropertySpec {
            name: "audit",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["all", "modifications", "none"],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_daemon_log_settings_lind",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["daemon-log-settings lind"],
        },
        header_types: &[("sys", "daemon-log-settings lind")],
        properties: &[BigipPropertySpec {
            name: "log-level",
            value_type: ValueKind::Enum,
            enum_values: &[
                "critical",
                "debug",
                "error",
                "informational",
                "notice",
                "warning",
            ],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_daemon_log_settings_mcpd",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["daemon-log-settings mcpd"],
        },
        header_types: &[("sys", "daemon-log-settings mcpd")],
        properties: &[
            BigipPropertySpec {
                name: "audit",
                value_type: ValueKind::Enum,
                enum_values: &["all", "disabled", "enabled", "verbose"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert",
                    "critical",
                    "debug",
                    "emergency",
                    "error",
                    "informational",
                    "notice",
                    "panic",
                    "warning",
                ],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_daemon_log_settings_tmm",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["daemon-log-settings tmm"],
        },
        header_types: &[("sys", "daemon-log-settings tmm")],
        properties: &[
            BigipPropertySpec {
                name: "arp-log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "error", "informational", "notice", "warning"],
                default: Some("warning"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "http-compression-log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "error", "informational", "notice", "warning"],
                default: Some("error"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "http-log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "error", "informational", "notice", "warning"],
                default: Some("error"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "informational", "notice", "warning"],
                default: Some("warning"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "irule-log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "error", "informational", "notice", "warning"],
                default: Some("warning"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "layer4-log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "informational", "notice"],
                default: Some("notice"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "net-log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "critical",
                    "debug",
                    "error",
                    "informational",
                    "notice",
                    "warning",
                ],
                default: Some("warning"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "os-log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert",
                    "critical",
                    "debug",
                    "emergency",
                    "error",
                    "informational",
                    "notice",
                    "warning",
                ],
                default: Some("notice"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pva-log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "informational", "notice"],
                default: Some("informational"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert",
                    "critical",
                    "debug",
                    "emergency",
                    "error",
                    "informational",
                    "notice",
                    "warning",
                ],
                default: Some("warning"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_datastor",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["datastor"],
        },
        header_types: &[("sys", "datastor")],
        properties: &[
            BigipPropertySpec {
                name: "dedup-cache-weight",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "disk",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "high-water-mark",
                value_type: ValueKind::Integer,
                default: Some("92"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "low-water-mark",
                value_type: ValueKind::Integer,
                default: Some("80"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "web-cache-weight",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_db",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["db"],
        },
        header_types: &[("sys", "db")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_diags_ihealth",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["diags ihealth"],
        },
        header_types: &[("sys", "diags ihealth")],
        properties: &[
            BigipPropertySpec {
                name: "expiration",
                value_type: ValueKind::Unknown,
                default: Some("30, and the maximum is 365"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-ihealth",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_disk_application_volume",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["disk application-volume"],
        },
        header_types: &[("sys", "disk application-volume")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_disk_directory",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["disk directory"],
        },
        header_types: &[("sys", "disk directory")],
        properties: &[BigipPropertySpec {
            name: "new-size",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_disk_logical_disk",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["disk logical-disk"],
        },
        header_types: &[("sys", "disk logical-disk")],
        properties: &[
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vg-reserved",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_dns",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["dns"],
        },
        header_types: &[("sys", "dns")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "name-servers",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "number-of-dots",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "search",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_dynad_instrumentation",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["dynad instrumentation"],
        },
        header_types: &[("sys", "dynad instrumentation")],
        properties: &[BigipPropertySpec {
            name: "active",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_dynad_key",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["dynad key"],
        },
        header_types: &[("sys", "dynad key")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_dynad_rpm",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["dynad rpm"],
        },
        header_types: &[("sys", "dynad rpm")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_dynad_settings",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["dynad settings"],
        },
        header_types: &[("sys", "dynad settings")],
        properties: &[BigipPropertySpec {
            name: "development-mode",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_ecm_config",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["ecm config"],
        },
        header_types: &[("sys", "ecm config")],
        properties: &[
            BigipPropertySpec {
                name: "dns-resolver",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "seed-ip",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_feature_module",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["feature-module"],
        },
        header_types: &[("sys", "feature-module")],
        properties: &[BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_apache_ssl_cert",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file apache-ssl-cert"],
        },
        header_types: &[("sys", "file apache-ssl-cert")],
        properties: &[BigipPropertySpec {
            name: "source-path",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_browser_capabilities_db",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file browser-capabilities-db"],
        },
        header_types: &[("sys", "file browser-capabilities-db")],
        properties: &[BigipPropertySpec {
            name: "source-path",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_data_group",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file data-group"],
        },
        header_types: &[("sys", "file data-group")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "data-group-description",
                value_type: ValueKind::String,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "data-group-name",
                value_type: ValueKind::Reference,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "separator",
                value_type: ValueKind::String,
                default: Some(":="),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Integer,
                required: true,
                enum_values: &["ip"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_device_capabilities_db",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file device-capabilities-db"],
        },
        header_types: &[("sys", "file device-capabilities-db")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_external_monitor",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file external-monitor"],
        },
        header_types: &[("sys", "file external-monitor")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_ifile",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file ifile"],
        },
        header_types: &[("sys", "file ifile")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_lwtunneltbl",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file lwtunneltbl"],
        },
        header_types: &[("sys", "file lwtunneltbl")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_rewrite_rule",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file rewrite-rule"],
        },
        header_types: &[("sys", "file rewrite-rule")],
        properties: &[BigipPropertySpec {
            name: "local-path",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_ssl_cert",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file ssl-cert"],
        },
        header_types: &[("sys", "file ssl-cert")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cert-validation-options",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "ocsp"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cert-validators",
                value_type: ValueKind::Reference,
                references: &[
                    "sys_crypto_cert_validator_crl",
                    "sys_crypto_cert_validator_ocsp",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "issuer-cert",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_ssl_crl",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file ssl-crl"],
        },
        header_types: &[("sys", "file ssl-crl")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_ssl_key",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file ssl-key"],
        },
        header_types: &[("sys", "file ssl-key")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "passphrase",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_folder",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["folder"],
        },
        header_types: &[("sys", "folder")],
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
                name: "device-group",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-ref-check",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-group",
                value_type: ValueKind::String,
                allow_none: true,
                references: &["cm_traffic_group"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_fpga_firmware_config",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["fpga firmware-config"],
        },
        header_types: &[("sys", "fpga firmware-config")],
        properties: &[BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Enum,
            enum_values: &[
                "l4-performance-fpga",
                "l7-intelligent-fpga",
                "standard-balanced-fpga",
                "traffic-acceleration-fpga",
            ],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_global_settings",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["global-settings"],
        },
        header_types: &[("sys", "global-settings")],
        properties: &[
            BigipPropertySpec {
                name: "aws-access-key",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aws-api-max-concurrency",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aws-secret-key",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "console-inactivity-timeout",
                value_type: ValueKind::Integer,
                default: Some("0 (zero), which means that no timeout is set"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "custom-addr",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("::"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                default: Some("no description"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failsafe-action",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "failover-restart-tm",
                    "go-offline",
                    "go-offline-restart-tm",
                    "reboot",
                    "restart-all",
                ],
                default: Some("go-offline-restart-tm"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "file-blacklist-path-prefix",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "file-blacklist-read-only-path-prefix",
                value_type: ValueKind::String,
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "file-local-path-prefix",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "file-whitelist-path-prefix",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gui-audit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gui-expired-cert-alert",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gui-security-banner",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gui-security-banner-text",
                value_type: ValueKind::String,
                default: Some("Welcome to the BIG-IP Configuration Utility"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gui-setup",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host-addr-mode",
                value_type: ValueKind::Enum,
                enum_values: &["custom", "management", "state-mirror"],
                default: Some("management"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hostname",
                value_type: ValueKind::String,
                default: Some("bigip1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hosts-allow-include",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lcd-display",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mgmt-dhcp",
                value_type: ValueKind::Enum,
                enum_values: &["dhcpv4", "dhcpv6", "disabled", "enabled"],
                default: Some("enabled for VE and disabled for all other platforms"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "net-reboot",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-prompt",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "quiet-boot",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-host",
                value_type: ValueKind::List,
                allow_none: true,
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "addr",
                        value_type: ValueKind::String,
                        in_sections: &["remote-host"],
                        shape_kind: Some(ValueKind::IpAddress),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hostname",
                        value_type: ValueKind::String,
                        in_sections: &["remote-host"],
                        default: Some("bigip1"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "addr",
                value_type: ValueKind::String,
                in_sections: &["remote-host"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hostname",
                value_type: ValueKind::String,
                in_sections: &["remote-host"],
                default: Some("bigip1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssh-max-session-limit",
                value_type: ValueKind::Integer,
                default: Some("10 and the range is 1 to 65535"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssh-max-session-limit-per-user",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssh-root-session-limit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssh-session-limit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-prompt",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_ha_group",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["ha-group"],
        },
        header_types: &[("sys", "ha-group")],
        properties: &[
            BigipPropertySpec {
                name: "active-bonus",
                value_type: ValueKind::Integer,
                default: Some("10 (ten)"),
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
                name: "clusters",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["clusters"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "attribute",
                        value_type: ValueKind::Unknown,
                        in_sections: &["clusters"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "minimum-threshold",
                        value_type: ValueKind::Integer,
                        in_sections: &["clusters"],
                        default: Some("0 (zero), which indicates this option is disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "sufficient",
                        value_type: ValueKind::Enum,
                        in_sections: &["clusters"],
                        enum_values: &["all"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "threshold",
                        value_type: ValueKind::Integer,
                        in_sections: &["clusters"],
                        usage_flags: &["deprecated"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "weight",
                        value_type: ValueKind::Integer,
                        in_sections: &["clusters"],
                        default: Some("10"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["clusters"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attribute",
                value_type: ValueKind::Unknown,
                in_sections: &["clusters"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimum-threshold",
                value_type: ValueKind::Integer,
                in_sections: &["clusters"],
                default: Some("0 (zero), which indicates this option is disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sufficient",
                value_type: ValueKind::Enum,
                in_sections: &["clusters"],
                enum_values: &["all"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "threshold",
                value_type: ValueKind::Integer,
                in_sections: &["clusters"],
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "weight",
                value_type: ValueKind::Integer,
                in_sections: &["clusters"],
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["pools"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "attribute",
                        value_type: ValueKind::Unknown,
                        in_sections: &["pools"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "minimum-threshold",
                        value_type: ValueKind::Integer,
                        in_sections: &["pools"],
                        default: Some("0 (zero), which indicates this option is disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "sufficient",
                        value_type: ValueKind::Integer,
                        in_sections: &["pools"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "threshold",
                        value_type: ValueKind::Integer,
                        in_sections: &["pools"],
                        usage_flags: &["deprecated"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "weight",
                        value_type: ValueKind::Integer,
                        in_sections: &["pools"],
                        default: Some("10"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["pools"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attribute",
                value_type: ValueKind::Unknown,
                in_sections: &["pools"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimum-threshold",
                value_type: ValueKind::Integer,
                in_sections: &["pools"],
                default: Some("0 (zero), which indicates this option is disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sufficient",
                value_type: ValueKind::Integer,
                in_sections: &["pools"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "threshold",
                value_type: ValueKind::Integer,
                in_sections: &["pools"],
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "weight",
                value_type: ValueKind::Integer,
                in_sections: &["pools"],
                default: Some("10"),
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
                        name: "attribute",
                        value_type: ValueKind::Unknown,
                        in_sections: &["trunks"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "minimum-threshold",
                        value_type: ValueKind::Integer,
                        in_sections: &["trunks"],
                        default: Some("0 (zero), which indicates this option is disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "sufficient",
                        value_type: ValueKind::Integer,
                        in_sections: &["trunks"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "threshold",
                        value_type: ValueKind::Integer,
                        in_sections: &["trunks"],
                        usage_flags: &["deprecated"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "weight",
                        value_type: ValueKind::Integer,
                        in_sections: &["trunks"],
                        default: Some("10"),
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
                name: "attribute",
                value_type: ValueKind::Unknown,
                in_sections: &["trunks"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimum-threshold",
                value_type: ValueKind::Integer,
                in_sections: &["trunks"],
                default: Some("0 (zero), which indicates this option is disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sufficient",
                value_type: ValueKind::Integer,
                in_sections: &["trunks"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "threshold",
                value_type: ValueKind::Integer,
                in_sections: &["trunks"],
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "weight",
                value_type: ValueKind::Integer,
                in_sections: &["trunks"],
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_httpd",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["httpd"],
        },
        header_types: &[("sys", "httpd")],
        properties: &[
            BigipPropertySpec {
                name: "allow",
                value_type: ValueKind::List,
                allow_none: true,
                default: Some("All"),
                list_operators: &["add", "delete", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "hostname",
                    value_type: ValueKind::Unknown,
                    in_sections: &["allow"],
                    repeated: true,
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hostname",
                value_type: ValueKind::Unknown,
                in_sections: &["allow"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-name",
                value_type: ValueKind::String,
                default: Some("BIG-IP"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-pam-dashboard-timeout",
                value_type: ValueKind::Enum,
                enum_values: &["off", "on"],
                default: Some("off"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-pam-idle-timeout",
                value_type: ValueKind::Integer,
                default: Some("1200 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-pam-validate-ip",
                value_type: ValueKind::Enum,
                enum_values: &["off", "on"],
                default: Some("on"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fastcgi-timeout",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hostname-lookup",
                value_type: ValueKind::Enum,
                enum_values: &["double", "off", "on"],
                default: Some("off"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "error", "info", "notice", "warn",
                ],
                default: Some("warn"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirect-http-to-https",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-body-max-timeout",
                value_type: ValueKind::Integer,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-body-min-rate",
                value_type: ValueKind::Integer,
                default: Some("500"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-body-timeout",
                value_type: ValueKind::Integer,
                default: Some("60"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-header-max-timeout",
                value_type: ValueKind::Integer,
                default: Some("40"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-header-min-rate",
                value_type: ValueKind::Integer,
                default: Some("500"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-header-timeout",
                value_type: ValueKind::Integer,
                default: Some("20"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-ca-cert-file",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-certchainfile",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-certfile",
                value_type: ValueKind::String,
                default: Some("/etc/httpd/conf/ssl"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-certkeyfile",
                value_type: ValueKind::String,
                default: Some("/etc/httpd/conf/ssl"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-ciphersuite",
                value_type: ValueKind::String,
                default: Some(
                    "\"ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-RSA-AES128-SHA:ECDHE-RSA-AES256-SHA:ECDHE-RSA-AES128-SHA256:ECDHE-RSA-AES256-SHA384:ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-ECDSA-AES128-SHA:ECDHE-ECDSA-AES256-SHA:ECDHE-ECDSA-AES128-SHA256:ECDHE-ECDSA-AES256-SHA384:AES128-GCM-SHA256:AES256-GCM-SHA384:AES128-SHA:AES256-SHA:AES128-SHA256:AES256-SHA256\"",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-include",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-ocsp-default-responder",
                value_type: ValueKind::String,
                default: Some("http://localhost"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-ocsp-enable",
                value_type: ValueKind::Enum,
                enum_values: &["off", "on"],
                default: Some("off"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-ocsp-override-responder",
                value_type: ValueKind::Enum,
                enum_values: &["off", "on"],
                default: Some("off"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-ocsp-responder-timeout",
                value_type: ValueKind::Integer,
                default: Some("300 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-ocsp-response-max-age",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-ocsp-response-time-skew",
                value_type: ValueKind::Integer,
                default: Some("300 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-port",
                value_type: ValueKind::Integer,
                default: Some("443"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-protocol",
                value_type: ValueKind::String,
                default: Some("all -SSLv2 -SSLv3 -TLSv1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-verify-client",
                value_type: ValueKind::Enum,
                enum_values: &["no", "optional", "optional-no-ca", "require"],
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-verify-depth",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_iapp_restricted_key",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["iapp-restricted-key"],
        },
        header_types: &[("sys", "iapp-restricted-key")],
        properties: &[BigipPropertySpec {
            name: "restricted-key",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_iapprestricted_key",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["iapprestricted key"],
        },
        header_types: &[("sys", "iapprestricted key")],
        properties: &[BigipPropertySpec {
            name: "restricted-key",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        }],
    },
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
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_icontrol_soap",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["icontrol-soap"],
        },
        header_types: &[("sys", "icontrol-soap")],
        properties: &[BigipPropertySpec {
            name: "allow",
            value_type: ValueKind::List,
            allow_none: true,
            default: Some("All"),
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_internal_proxy",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["internal-proxy"],
        },
        header_types: &[("sys", "internal-proxy")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-resolver",
                value_type: ValueKind::Reference,
                references: &["net_dns_resolver"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-server-pool",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Reference,
                references: &["net_route_domain"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_ipfix_element",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["ipfix element"],
        },
        header_types: &[("sys", "ipfix element")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "data-type",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enterprise-id",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "id",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "size",
                value_type: ValueKind::Integer,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_alertd",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination alertd"],
        },
        header_types: &[("sys", "log-config destination alertd")],
        properties: &[BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_arcsight",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination arcsight"],
        },
        header_types: &[("sys", "log-config destination arcsight")],
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
                name: "forward-to",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_ipfix",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination ipfix"],
        },
        header_types: &[("sys", "log-config destination ipfix")],
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
                name: "pool-name",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol-version",
                value_type: ValueKind::Enum,
                enum_values: &["ipfix", "netflow-9"],
                default: Some("ipfix"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serverssl-profile",
                value_type: ValueKind::Reference,
                default: Some("not to use a server-side SSL profile"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "template-delete-delay",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "template-retransmit-interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transport-profile",
                value_type: ValueKind::Reference,
                default: Some("the default udp profile"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_local_database",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination local-database"],
        },
        header_types: &[("sys", "log-config destination local-database")],
        properties: &[BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_local_syslog",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination local-syslog"],
        },
        header_types: &[("sys", "log-config destination local-syslog")],
        properties: &[
            BigipPropertySpec {
                name: "default-facility",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "local0", "local1", "local2", "local3", "local4", "local5", "local6", "local7",
                ],
                default: Some("local0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-severity",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                default: Some("info"),
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
            kind: "sys_log_config_destination_management_port",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination management-port"],
        },
        header_types: &[("sys", "log-config destination management-port")],
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
                name: "ip-address",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Enum,
                enum_values: &["tcp", "udp"],
                default: Some("tcp"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_remote_high_speed_log",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination remote-high-speed-log"],
        },
        header_types: &[("sys", "log-config destination remote-high-speed-log")],
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
                name: "distribution",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["adaptive", "balanced", "replicated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-name",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Enum,
                enum_values: &["tcp", "udp"],
                default: Some("tcp"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_remote_syslog",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination remote-syslog"],
        },
        header_types: &[("sys", "log-config destination remote-syslog")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-facility",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "local0", "local1", "local2", "local3", "local4", "local5", "local6", "local7",
                ],
                default: Some("local0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-severity",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                default: Some("info"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "format",
                value_type: ValueKind::Enum,
                enum_values: &["legacy-bigip", "rfc3164", "rfc5424"],
                default: Some("rfc3164"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-high-speed-log",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_splunk",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination splunk"],
        },
        header_types: &[("sys", "log-config destination splunk")],
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
                name: "forward-to",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_filter",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config filter"],
        },
        header_types: &[("sys", "log-config filter")],
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
                name: "level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                default: Some("debug"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "message-id",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "publisher",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "accesscontrol",
                    "accessperrequest",
                    "adapt",
                    "adfs-proxy",
                    "alertd",
                    "all",
                    "api-protection",
                    "apmacl",
                    "arp",
                    "authz",
                    "autodiscd",
                    "autodosd",
                    "avr",
                    "based",
                    "bcm56xxd",
                    "bdosd",
                    "big3d",
                    "big3dshim",
                    "bigd",
                    "bigdb",
                    "bigdbd",
                    "bigpipe",
                    "bigstart",
                    "bp",
                    "checkcert",
                    "chmand",
                    "cifs",
                    "clusterd",
                    "coapi",
                    "common",
                    "common-f5logging",
                    "common-fpdd",
                    "config-db",
                    "connapi",
                    "cryptod",
                    "cs",
                    "cssd",
                    "csyncd",
                    "daemon",
                    "debugd",
                    "deflate",
                    "devmgmtd",
                    "diameter",
                    "dmon",
                    "dosprotect",
                    "dpi",
                    "dummy",
                    "dwbld",
                    "dynad",
                    "eca",
                    "em-admin",
                    "em-alert",
                    "em-clientlib",
                    "em-common",
                    "em-device",
                    "em-discovery",
                    "em-file",
                    "em-lib",
                    "em-stats",
                    "em-swim",
                    "errdefsd",
                    "eventd",
                    "evrouted",
                    "fflag",
                    "fips",
                    "firewall-FQDN",
                    "firewall-nat",
                    "fix",
                    "ftp",
                    "get-dossier",
                    "gpa",
                    "gtmd",
                    "gtp",
                    "guestagentd",
                    "ha",
                    "ha-table",
                    "halmsg",
                    "hclientd",
                    "hornet-lib",
                    "hornet-nest",
                    "hornet-nest-flow-manager",
                    "hornet-nest-updater",
                    "hornet-neuron-updater",
                    "hornet-server",
                    "hornet-text-client",
                    "hostagentd",
                    "htconnector",
                    "http",
                    "hwctl",
                    "hwpd",
                    "icr-eventd",
                    "icrd",
                    "imap",
                    "ip",
                    "ipfix",
                    "ipfix-proxy",
                    "ipfixirules",
                    "iprepd",
                    "ips",
                    "ipsec",
                    "isession",
                    "istatsd",
                    "ivs",
                    "keymgmtd",
                    "lacpd",
                    "layer4",
                    "libhal",
                    "lind",
                    "lldpd",
                    "localdb",
                    "lopd",
                    "lsn",
                    "lsnapi",
                    "mamidbridged",
                    "map",
                    "mapi",
                    "mcp",
                    "mcpd",
                    "mcpd-apm",
                    "mcpd-asm",
                    "mcpd-centmgmt",
                    "mcpd-clustering",
                    "mcpd-dev",
                    "mcpd-dpi",
                    "mcpd-firewall",
                    "mcpd-framework",
                    "mcpd-gtm",
                    "mcpd-ips",
                    "mcpd-ltm",
                    "mcpd-net",
                    "mcpd-pem",
                    "mcpd-sys",
                    "mcpd-wam",
                    "mcpd-woc",
                    "mdm",
                    "mgmt-acld",
                    "mr",
                    "mrsip",
                    "msgbusd",
                    "mysqlhad",
                    "natstatsd",
                    "net",
                    "network",
                    "no-source",
                    "packet-filter",
                    "pccd",
                    "pcp",
                    "pem",
                    "pfmand",
                    "pgadmind",
                    "pkcs11d",
                    "pktclass",
                    "plugin",
                    "policy",
                    "pop3",
                    "pptp",
                    "probe-plusplus",
                    "promptstatusd",
                    "provisioning",
                    "pva",
                    "pvad",
                    "qkcloud",
                    "radius",
                    "ramcache",
                    "rba",
                    "rewrite",
                    "rtsp",
                    "rules",
                    "saas",
                    "saspd",
                    "scim",
                    "scriptd",
                    "sctp",
                    "sdmd",
                    "sflow",
                    "shell",
                    "shmmapd",
                    "smtps",
                    "snmp",
                    "sod",
                    "spolicy",
                    "ssl",
                    "ssl-c3d",
                    "ssl-certificate",
                    "ssl-client-auth",
                    "ssl-forward-proxy",
                    "ssl-handshake",
                    "ssl-orchestrator",
                    "sso",
                    "stated",
                    "statsd",
                    "statusd",
                    "stmm",
                    "stpd",
                    "subagents",
                    "swg",
                    "syscall",
                    "system-check",
                    "tamd",
                    "tcl-checker",
                    "tcpdump",
                    "tftp",
                    "tmm",
                    "tmm-tcp",
                    "tmrouted",
                    "tmsh",
                    "ts",
                    "tunnel",
                    "urlc",
                    "urldb",
                    "urldbmgrd",
                    "vcmpd",
                    "vdi",
                    "vxland",
                    "webssh",
                    "websso",
                    "woc-plugin",
                    "wr-urldbd",
                    "xconfig",
                    "xdb",
                    "zfd",
                    "zxfrd",
                ],
                default: Some("all"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_publisher",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config publisher"],
        },
        header_types: &[("sys", "log-config publisher")],
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
                name: "destinations",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_rotate",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-rotate"],
        },
        header_types: &[("sys", "log-rotate")],
        properties: &[
            BigipPropertySpec {
                name: "common-backlogs",
                value_type: ValueKind::Integer,
                default: Some("24"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "common-include",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ilx-include",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ilx-rotations",
                value_type: ValueKind::String,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ilx-schedule",
                value_type: ValueKind::String,
                default: Some("daily"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ilx-size",
                value_type: ValueKind::String,
                default: Some("10240 kilobytes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-file-size",
                value_type: ValueKind::Integer,
                default: Some("1024000"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mysql-include",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "syslog-include",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tomcat-include",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "wa-include",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_management_dhcp",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["management-dhcp"],
        },
        header_types: &[("sys", "management-dhcp")],
        properties: &[
            BigipPropertySpec {
                name: "client-id",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hostname",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-options",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send-options",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "supersede-options",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_management_ip",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["management-ip"],
        },
        header_types: &[("sys", "management-ip")],
        properties: &[BigipPropertySpec {
            name: "description",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_management_ovsdb",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["management-ovsdb"],
        },
        header_types: &[("sys", "management-ovsdb")],
        properties: &[
            BigipPropertySpec {
                name: "bfd-disabled",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bfd-enabled",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bfd-route-domain",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ca-cert-file",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cert-file",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cert-key-file",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "controller-addresses",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "disabled",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enabled",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flooding-type",
                value_type: ValueKind::Enum,
                enum_values: &["multipoint", "replicator"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-level",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "logical-routing-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["backhaul", "none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tunnel-floating-addresses",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tunnel-local-address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tunnel-maintenance-mode",
                value_type: ValueKind::Enum,
                enum_values: &["active", "passive"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_management_proxy_config",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["management-proxy-config"],
        },
        header_types: &[("sys", "management-proxy-config")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-ip-addr",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-port",
                value_type: ValueKind::Unknown,
                default: Some("3128)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_management_route",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["management-route"],
        },
        header_types: &[("sys", "management-route")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gateway",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mtu",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "network",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &["blackhole", "interface"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_ntp",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["ntp"],
        },
        header_types: &[("sys", "ntp")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "restrict",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "address",
                        value_type: ValueKind::String,
                        in_sections: &["restrict"],
                        shape_kind: Some(ValueKind::IpAddress),
                        default: Some("0"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "default-entry",
                        value_type: ValueKind::Enum,
                        in_sections: &["restrict"],
                        enum_values: &["disable", "enabled"],
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["restrict"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ignore",
                        value_type: ValueKind::Enum,
                        in_sections: &["restrict"],
                        enum_values: &["disable", "enabled"],
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "kod",
                        value_type: ValueKind::Enum,
                        in_sections: &["restrict"],
                        enum_values: &["disable", "enabled"],
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limited",
                        value_type: ValueKind::Enum,
                        in_sections: &["restrict"],
                        enum_values: &["disable", "enabled"],
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "low-priority-trap",
                        value_type: ValueKind::Enum,
                        in_sections: &["restrict"],
                        enum_values: &["disable", "enabled"],
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "mask",
                        value_type: ValueKind::String,
                        in_sections: &["restrict"],
                        shape_kind: Some(ValueKind::IpAddress),
                        default: Some("0"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "no-modify",
                        value_type: ValueKind::Enum,
                        in_sections: &["restrict"],
                        enum_values: &["disable", "enabled"],
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "no-peer",
                        value_type: ValueKind::Enum,
                        in_sections: &["restrict"],
                        enum_values: &["disable", "enabled"],
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "no-query",
                        value_type: ValueKind::Enum,
                        in_sections: &["restrict"],
                        enum_values: &["disable", "enabled"],
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "no-serve-packets",
                        value_type: ValueKind::Enum,
                        in_sections: &["restrict"],
                        enum_values: &["disable", "enabled"],
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "no-trap",
                        value_type: ValueKind::Enum,
                        in_sections: &["restrict"],
                        enum_values: &["disable", "enabled"],
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "no-trust",
                        value_type: ValueKind::Enum,
                        in_sections: &["restrict"],
                        enum_values: &["disable", "enabled"],
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "non-ntp-port",
                        value_type: ValueKind::Enum,
                        in_sections: &["restrict"],
                        enum_values: &["disable", "enabled"],
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ntp-port",
                        value_type: ValueKind::Enum,
                        in_sections: &["restrict"],
                        enum_values: &["disable", "enabled"],
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "version",
                        value_type: ValueKind::Enum,
                        in_sections: &["restrict"],
                        enum_values: &["disable", "enabled"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::String,
                in_sections: &["restrict"],
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-entry",
                value_type: ValueKind::Enum,
                in_sections: &["restrict"],
                enum_values: &["disable", "enabled"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["restrict"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore",
                value_type: ValueKind::Enum,
                in_sections: &["restrict"],
                enum_values: &["disable", "enabled"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "kod",
                value_type: ValueKind::Enum,
                in_sections: &["restrict"],
                enum_values: &["disable", "enabled"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limited",
                value_type: ValueKind::Enum,
                in_sections: &["restrict"],
                enum_values: &["disable", "enabled"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "low-priority-trap",
                value_type: ValueKind::Enum,
                in_sections: &["restrict"],
                enum_values: &["disable", "enabled"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mask",
                value_type: ValueKind::String,
                in_sections: &["restrict"],
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-modify",
                value_type: ValueKind::Enum,
                in_sections: &["restrict"],
                enum_values: &["disable", "enabled"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-peer",
                value_type: ValueKind::Enum,
                in_sections: &["restrict"],
                enum_values: &["disable", "enabled"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-query",
                value_type: ValueKind::Enum,
                in_sections: &["restrict"],
                enum_values: &["disable", "enabled"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-serve-packets",
                value_type: ValueKind::Enum,
                in_sections: &["restrict"],
                enum_values: &["disable", "enabled"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-trap",
                value_type: ValueKind::Enum,
                in_sections: &["restrict"],
                enum_values: &["disable", "enabled"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-trust",
                value_type: ValueKind::Enum,
                in_sections: &["restrict"],
                enum_values: &["disable", "enabled"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "non-ntp-port",
                value_type: ValueKind::Enum,
                in_sections: &["restrict"],
                enum_values: &["disable", "enabled"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ntp-port",
                value_type: ValueKind::Enum,
                in_sections: &["restrict"],
                enum_values: &["disable", "enabled"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "version",
                value_type: ValueKind::Enum,
                in_sections: &["restrict"],
                enum_values: &["disable", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "servers",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timezone",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_outbound_smtp",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["outbound-smtp"],
        },
        header_types: &[("sys", "outbound-smtp")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "from-line-override",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mailhub",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rewrite-domain",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_pfman_consumer",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["pfman consumer"],
        },
        header_types: &[("sys", "pfman consumer")],
        properties: &[BigipPropertySpec {
            name: "state",
            value_type: ValueKind::Enum,
            enum_values: &["down", "reset", "up"],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_pfman_device",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["pfman device"],
        },
        header_types: &[("sys", "pfman device")],
        properties: &[BigipPropertySpec {
            name: "state",
            value_type: ValueKind::Enum,
            enum_values: &["down", "reset", "up"],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_provision",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["provision"],
        },
        header_types: &[("sys", "provision")],
        properties: &[
            BigipPropertySpec {
                name: "cpu-ratio",
                value_type: ValueKind::Integer,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "disk-ratio",
                value_type: ValueKind::Integer,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "level",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["custom", "dedicated", "minimum", "nominal", "none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "memory-ratio",
                value_type: ValueKind::Integer,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_raid_array",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["raid array"],
        },
        header_types: &[("sys", "raid array")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_raid_bay",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["raid bay"],
        },
        header_types: &[("sys", "raid bay")],
        properties: &[
            BigipPropertySpec {
                name: "flash-led",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-flash-led",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_scriptd",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["scriptd"],
        },
        header_types: &[("sys", "scriptd")],
        properties: &[
            BigipPropertySpec {
                name: "log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-script-run-time",
                value_type: ValueKind::Unknown,
                default: Some("300"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_service",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["service"],
        },
        header_types: &[("sys", "service")],
        properties: &[
            BigipPropertySpec {
                name: "force",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "restart",
                value_type: ValueKind::Reference,
                references: &["restart"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "start",
                value_type: ValueKind::Reference,
                references: &["start"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "stop",
                value_type: ValueKind::Reference,
                references: &["stop"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_sflow_global_settings_http",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["sflow global-settings http"],
        },
        header_types: &[("sys", "sflow global-settings http")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "poll-interval",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sampling-rate",
                value_type: ValueKind::Integer,
                default: Some("1024"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_sflow_global_settings_interface",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["sflow global-settings interface"],
        },
        header_types: &[("sys", "sflow global-settings interface")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "poll-interval",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_sflow_global_settings_system",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["sflow global-settings system"],
        },
        header_types: &[("sys", "sflow global-settings system")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "poll-interval",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_sflow_global_settings_vlan",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["sflow global-settings vlan"],
        },
        header_types: &[("sys", "sflow global-settings vlan")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "poll-interval",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sampling-rate",
                value_type: ValueKind::Integer,
                default: Some("2048"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_sflow_receiver",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["sflow receiver"],
        },
        header_types: &[("sys", "sflow receiver")],
        properties: &[
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-datagram-size",
                value_type: ValueKind::Integer,
                default: Some("1400"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Unknown,
                default: Some("the standard sFlow port, 6343"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_smtp_server",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["smtp-server"],
        },
        header_types: &[("sys", "smtp-server")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encrypted-connection",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "ssl", "tls"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "from-address",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-host-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-server-host-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-server-port",
                value_type: ValueKind::Integer,
                default: Some("25"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_snmp",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["snmp"],
        },
        header_types: &[("sys", "snmp")],
        properties: &[
            BigipPropertySpec {
                name: "agent-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "agent-trap",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allowed-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                default: Some("127"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-trap",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bigip-traps",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "communities",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "access",
                        value_type: ValueKind::Enum,
                        in_sections: &["communities"],
                        enum_values: &["ro", "rw"],
                        default: Some("ro"),
                        usage_flags: &["read_only"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "community-name",
                        value_type: ValueKind::String,
                        in_sections: &["communities"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["communities"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ipv6",
                        value_type: ValueKind::Enum,
                        in_sections: &["communities"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "oid-subset",
                        value_type: ValueKind::String,
                        in_sections: &["communities"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "source",
                        value_type: ValueKind::String,
                        in_sections: &["communities"],
                        shape_kind: Some(ValueKind::IpAddress),
                        default: Some(
                            "default, which means allow any source address to access the MIB",
                        ),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "access",
                value_type: ValueKind::Enum,
                in_sections: &["communities"],
                enum_values: &["ro", "rw"],
                default: Some("ro"),
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "community-name",
                value_type: ValueKind::String,
                in_sections: &["communities"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["communities"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv6",
                value_type: ValueKind::Enum,
                in_sections: &["communities"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "oid-subset",
                value_type: ValueKind::String,
                in_sections: &["communities"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source",
                value_type: ValueKind::String,
                in_sections: &["communities"],
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("default, which means allow any source address to access the MIB"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "disk-monitors",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["disk-monitors"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "minspace",
                        value_type: ValueKind::Integer,
                        in_sections: &["disk-monitors"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "minspace-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["disk-monitors"],
                        enum_values: &["percent", "size"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "path",
                        value_type: ValueKind::String,
                        in_sections: &["disk-monitors"],
                        required: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["disk-monitors"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minspace",
                value_type: ValueKind::Integer,
                in_sections: &["disk-monitors"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minspace-type",
                value_type: ValueKind::Enum,
                in_sections: &["disk-monitors"],
                enum_values: &["percent", "size"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "path",
                value_type: ValueKind::String,
                in_sections: &["disk-monitors"],
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "l2forward-vlan",
                value_type: ValueKind::List,
                required: true,
                allow_none: true,
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-max1",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-max15",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-max5",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "process-monitors",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["process-monitors"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "max-processes",
                        value_type: ValueKind::Integer,
                        in_sections: &["process-monitors"],
                        default: Some("1"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "min-processes",
                        value_type: ValueKind::Integer,
                        in_sections: &["process-monitors"],
                        default: Some("1"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "process",
                        value_type: ValueKind::String,
                        in_sections: &["process-monitors"],
                        required: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["process-monitors"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-processes",
                value_type: ValueKind::Integer,
                in_sections: &["process-monitors"],
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-processes",
                value_type: ValueKind::Integer,
                in_sections: &["process-monitors"],
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "process",
                value_type: ValueKind::String,
                in_sections: &["process-monitors"],
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "snmpv1",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "snmpv2",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sys-contact",
                value_type: ValueKind::String,
                default: Some("\"Customer Name<admin@customer"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sys-location",
                value_type: ValueKind::String,
                default: Some("Network Closet 1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sys-services",
                value_type: ValueKind::Integer,
                default: Some("78"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trap-community",
                value_type: ValueKind::String,
                default: Some("public"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trap-source",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traps",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "auth-password",
                        value_type: ValueKind::String,
                        in_sections: &["traps"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "auth-protocol",
                        value_type: ValueKind::Enum,
                        in_sections: &["traps"],
                        allow_none: true,
                        enum_values: &["md5", "none", "sha"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "community",
                        value_type: ValueKind::String,
                        in_sections: &["traps"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["traps"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "engine-id",
                        value_type: ValueKind::Unknown,
                        in_sections: &["traps"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "host",
                        value_type: ValueKind::String,
                        in_sections: &["traps"],
                        required: true,
                        shape_kind: Some(ValueKind::IpAddress),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "port",
                        value_type: ValueKind::Integer,
                        in_sections: &["traps"],
                        default: Some("162"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "privacy-password",
                        value_type: ValueKind::String,
                        in_sections: &["traps"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "privacy-protocol",
                        value_type: ValueKind::Enum,
                        in_sections: &["traps"],
                        allow_none: true,
                        enum_values: &["aes", "des", "none"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "security-level",
                        value_type: ValueKind::Enum,
                        in_sections: &["traps"],
                        enum_values: &["auth-no-privacy", "auth-privacy", "no-auth-no-privacy"],
                        default: Some("no-auth-no-privacy"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "security-name",
                        value_type: ValueKind::String,
                        in_sections: &["traps"],
                        required: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "version",
                        value_type: ValueKind::Enum,
                        in_sections: &["traps"],
                        enum_values: &["1", "2c", "3"],
                        default: Some("2c"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-password",
                value_type: ValueKind::String,
                in_sections: &["traps"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-protocol",
                value_type: ValueKind::Enum,
                in_sections: &["traps"],
                allow_none: true,
                enum_values: &["md5", "none", "sha"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "community",
                value_type: ValueKind::String,
                in_sections: &["traps"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["traps"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "engine-id",
                value_type: ValueKind::Unknown,
                in_sections: &["traps"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::String,
                in_sections: &["traps"],
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                in_sections: &["traps"],
                default: Some("162"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "privacy-password",
                value_type: ValueKind::String,
                in_sections: &["traps"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "privacy-protocol",
                value_type: ValueKind::Enum,
                in_sections: &["traps"],
                allow_none: true,
                enum_values: &["aes", "des", "none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "security-level",
                value_type: ValueKind::Enum,
                in_sections: &["traps"],
                enum_values: &["auth-no-privacy", "auth-privacy", "no-auth-no-privacy"],
                default: Some("no-auth-no-privacy"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "security-name",
                value_type: ValueKind::String,
                in_sections: &["traps"],
                required: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "version",
                value_type: ValueKind::Enum,
                in_sections: &["traps"],
                enum_values: &["1", "2c", "3"],
                default: Some("2c"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "users",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "access",
                        value_type: ValueKind::Enum,
                        in_sections: &["users"],
                        enum_values: &["ro", "rw"],
                        default: Some("ro"),
                        usage_flags: &["read_only"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "auth-password",
                        value_type: ValueKind::String,
                        in_sections: &["users"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "auth-protocol",
                        value_type: ValueKind::Enum,
                        in_sections: &["users"],
                        allow_none: true,
                        enum_values: &["md5", "none", "sha"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["users"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "oid-subset",
                        value_type: ValueKind::String,
                        in_sections: &["users"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "privacy-password",
                        value_type: ValueKind::String,
                        in_sections: &["users"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "privacy-protocol",
                        value_type: ValueKind::Enum,
                        in_sections: &["users"],
                        allow_none: true,
                        enum_values: &["aes", "des", "none"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "security-level",
                        value_type: ValueKind::Enum,
                        in_sections: &["users"],
                        enum_values: &["auth-no-privacy", "auth-privacy", "no-auth-no-privacy"],
                        default: Some("no-auth-no-privacy"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "username",
                        value_type: ValueKind::String,
                        in_sections: &["users"],
                        required: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "access",
                value_type: ValueKind::Enum,
                in_sections: &["users"],
                enum_values: &["ro", "rw"],
                default: Some("ro"),
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-password",
                value_type: ValueKind::String,
                in_sections: &["users"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-protocol",
                value_type: ValueKind::Enum,
                in_sections: &["users"],
                allow_none: true,
                enum_values: &["md5", "none", "sha"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["users"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "oid-subset",
                value_type: ValueKind::String,
                in_sections: &["users"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "privacy-password",
                value_type: ValueKind::String,
                in_sections: &["users"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "privacy-protocol",
                value_type: ValueKind::Enum,
                in_sections: &["users"],
                allow_none: true,
                enum_values: &["aes", "des", "none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "security-level",
                value_type: ValueKind::Enum,
                in_sections: &["users"],
                enum_values: &["auth-no-privacy", "auth-privacy", "no-auth-no-privacy"],
                default: Some("no-auth-no-privacy"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::String,
                in_sections: &["users"],
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "v1-traps",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "community",
                        value_type: ValueKind::String,
                        in_sections: &["v1-traps"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["v1-traps"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "host",
                        value_type: ValueKind::String,
                        in_sections: &["v1-traps"],
                        required: true,
                        shape_kind: Some(ValueKind::IpAddress),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "port",
                        value_type: ValueKind::Integer,
                        in_sections: &["v1-traps"],
                        default: Some("162"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "community",
                value_type: ValueKind::String,
                in_sections: &["v1-traps"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["v1-traps"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::String,
                in_sections: &["v1-traps"],
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                in_sections: &["v1-traps"],
                default: Some("162"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "v2-traps",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "community",
                        value_type: ValueKind::String,
                        in_sections: &["v2-traps"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["v2-traps"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "host",
                        value_type: ValueKind::String,
                        in_sections: &["v2-traps"],
                        required: true,
                        shape_kind: Some(ValueKind::IpAddress),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "port",
                        value_type: ValueKind::Integer,
                        in_sections: &["v2-traps"],
                        default: Some("162"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "community",
                value_type: ValueKind::String,
                in_sections: &["v2-traps"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["v2-traps"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::String,
                in_sections: &["v2-traps"],
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                in_sections: &["v2-traps"],
                default: Some("162"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_software_hotfix",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["software hotfix"],
        },
        header_types: &[("sys", "software hotfix")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_software_image",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["software image"],
        },
        header_types: &[("sys", "software image")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_software_signature",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["software signature"],
        },
        header_types: &[("sys", "software signature")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_software_update",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["software update"],
        },
        header_types: &[("sys", "software update")],
        properties: &[
            BigipPropertySpec {
                name: "auto-check",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-phonehome",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_software_volume",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["software volume"],
        },
        header_types: &[("sys", "software volume")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_sshd",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["sshd"],
        },
        header_types: &[("sys", "sshd")],
        properties: &[
            BigipPropertySpec {
                name: "allow",
                value_type: ValueKind::List,
                allow_none: true,
                default: Some("all"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "banner",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "banner-text",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "inactivity-timeout",
                value_type: ValueKind::Integer,
                default: Some(
                    "0 (zero) seconds, which indicates that inactivity timeout is disabled",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "debug", "debug1", "debug2", "debug3", "error", "fatal", "info", "quiet",
                    "verbose",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "login",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                default: Some("22"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_state_mirroring",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["state-mirroring"],
        },
        header_types: &[("sys", "state-mirroring")],
        properties: &[
            BigipPropertySpec {
                name: "addr",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("::"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peer-addr",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("::"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secondary-addr",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("::"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secondary-peer-addr",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("::"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_syslog",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["syslog"],
        },
        header_types: &[("sys", "syslog")],
        properties: &[
            BigipPropertySpec {
                name: "auth-priv-from",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("notice"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-priv-to",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("emerg"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "clustered-host-name",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "clustered-message-slot",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "console-log",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cron-from",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("warning"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cron-to",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("emerg"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "daemon-from",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("notice"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "daemon-to",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("emerg"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iso-date",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "kern-from",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("debug"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "kern-to",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("emerg"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local6-from",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("notice"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local6-to",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("emerg"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mail-from",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("notice"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mail-to",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("emerg"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "messages-from",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("notice"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "messages-to",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("warning"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-servers",
                value_type: ValueKind::List,
                allow_none: true,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "host",
                        value_type: ValueKind::Unknown,
                        in_sections: &["remote-servers"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "local-ip",
                        value_type: ValueKind::String,
                        in_sections: &["remote-servers"],
                        shape_kind: Some(ValueKind::IpAddress),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "remote-port",
                        value_type: ValueKind::Unknown,
                        in_sections: &["remote-servers"],
                        default: Some("514"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::Unknown,
                in_sections: &["remote-servers"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-ip",
                value_type: ValueKind::String,
                in_sections: &["remote-servers"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-port",
                value_type: ValueKind::Unknown,
                in_sections: &["remote-servers"],
                default: Some("514"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-log-from",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("notice"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-log-to",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
                ],
                default: Some("emerg"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_turboflex_profile_config",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["turboflex profile-config"],
        },
        header_types: &[("sys", "turboflex profile-config")],
        properties: &[BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Enum,
            enum_values: &[
                "turbofelx-asym-security",
                "turboflex-adc",
                "turboflex-base",
                "turboflex-dns",
                "turboflex-highspeed-layer4",
                "turboflex-low-latency",
                "turboflex-private-cloud",
                "turboflex-security",
            ],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_ucs",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["ucs"],
        },
        header_types: &[("sys", "ucs")],
        properties: &[
            BigipPropertySpec {
                name: "include-chassis-level-config",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load",
                value_type: ValueKind::Reference,
                references: &["gtm_global_settings_load_balancing", "load"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-license",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-platform-check",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-private-key",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "passphrase",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "platform-migrate",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reset-trust",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save",
                value_type: ValueKind::Reference,
                references: &["save"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_url_db_download_schedule",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["url-db download-schedule"],
        },
        header_types: &[("sys", "url-db download-schedule")],
        properties: &[
            BigipPropertySpec {
                name: "download-now",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "end-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "start-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "status",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-proxy",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_url_db_url_category",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["url-db url-category"],
        },
        header_types: &[("sys", "url-db url-category")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "display-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "initial-disposition",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "is-security-category",
                value_type: ValueKind::String,
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "parent-cat-number",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "severity-level",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "urls",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
