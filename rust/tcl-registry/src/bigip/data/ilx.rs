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

//! BIG-IP object specs for the `ilx` tmsh module.
//!
//! **Hand-maintained.** These files were originally produced by a
//! one-time port of the pre-rewrite Python registry
//! (`dialects/f5/bigip/registry/specs/`, still present on `main`) via a
//! `scripts/registry-audit/gen_bigip_rust.py` generator that no longer
//! exists — deleted along with the rest of the retired Python tooling on
//! this branch, and the commits that ran it were squashed away in the
//! `rust` branch's rebase-onto-main history (see issue #1404). Originally
//! organised by the first letter of `kind`; reorganised by tmsh module
//! name (this file's own module field, `Some("ilx")`) per
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
            kind: "ilx_global_settings",
            table_name: None,
            resolver_name: None,
            module: Some("ilx"),
            object_types: &["global-settings"],
        },
        header_types: &[("ilx", "global-settings")],
        properties: &[
            BigipPropertySpec {
                name: "debug-port-blacklist",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-publisher",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ilx_plugin",
            table_name: None,
            resolver_name: None,
            module: Some("ilx"),
            object_types: &["plugin"],
        },
        header_types: &[("ilx", "plugin")],
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
                name: "extensions",
                value_type: ValueKind::List,
                repeated: true,
                block: &[
                    BigipPropertySpec {
                        name: "command-arguments",
                        value_type: ValueKind::Unknown,
                        in_sections: &["extensions"],
                        usage_flags: &["optional"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "command-options",
                        value_type: ValueKind::Unknown,
                        in_sections: &["extensions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "concurrency-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["extensions"],
                        enum_values: &["dedicated", "single"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "data-groups",
                        value_type: ValueKind::Reference,
                        in_sections: &["extensions"],
                        repeated: true,
                        allow_none: true,
                        references: &[
                            "ltm_data_group_external",
                            "ltm_data_group_internal",
                            "sys_file_data_group",
                        ],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "debug-port-range-high",
                        value_type: ValueKind::Unknown,
                        in_sections: &["extensions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "debug-port-range-low",
                        value_type: ValueKind::Unknown,
                        in_sections: &["extensions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["extensions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "heartbeat-interval",
                        value_type: ValueKind::Unknown,
                        in_sections: &["extensions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ilx-logging",
                        value_type: ValueKind::Enum,
                        in_sections: &["extensions"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "log-publisher",
                        value_type: ValueKind::Reference,
                        in_sections: &["extensions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "max-restarts",
                        value_type: ValueKind::Integer,
                        in_sections: &["extensions"],
                        default: Some("5"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "restart-interval",
                        value_type: ValueKind::Integer,
                        in_sections: &["extensions"],
                        default: Some("60 seconds"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "trace-level",
                        value_type: ValueKind::Integer,
                        in_sections: &["extensions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "command-arguments",
                value_type: ValueKind::Unknown,
                in_sections: &["extensions"],
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "command-options",
                value_type: ValueKind::Unknown,
                in_sections: &["extensions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "concurrency-mode",
                value_type: ValueKind::Enum,
                in_sections: &["extensions"],
                enum_values: &["dedicated", "single"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "data-groups",
                value_type: ValueKind::Reference,
                in_sections: &["extensions"],
                repeated: true,
                allow_none: true,
                references: &[
                    "ltm_data_group_external",
                    "ltm_data_group_internal",
                    "sys_file_data_group",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug-port-range-high",
                value_type: ValueKind::Unknown,
                in_sections: &["extensions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug-port-range-low",
                value_type: ValueKind::Unknown,
                in_sections: &["extensions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["extensions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "heartbeat-interval",
                value_type: ValueKind::Unknown,
                in_sections: &["extensions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ilx-logging",
                value_type: ValueKind::Enum,
                in_sections: &["extensions"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-publisher",
                value_type: ValueKind::Reference,
                in_sections: &["extensions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-restarts",
                value_type: ValueKind::Integer,
                in_sections: &["extensions"],
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "restart-interval",
                value_type: ValueKind::Integer,
                in_sections: &["extensions"],
                default: Some("60 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trace-level",
                value_type: ValueKind::Integer,
                in_sections: &["extensions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "from-workspace",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-publisher",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ilx_workspace",
            table_name: None,
            resolver_name: None,
            module: Some("ilx"),
            object_types: &["workspace"],
        },
        header_types: &[("ilx", "workspace")],
        properties: &[
            BigipPropertySpec {
                name: "archive",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "extension",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "file",
                value_type: ValueKind::Reference,
                references: &[
                    "apm_aaa_kerberos_keytab_file",
                    "apm_policy_agent_endpoint_linux_check_file",
                    "apm_policy_agent_endpoint_mac_check_file",
                    "apm_policy_agent_endpoint_windows_check_file",
                    "apm_policy_image_file",
                    "apm_policy_windows_group_policy_file",
                    "apm_resource_remote_desktop_citrix_client_package_file",
                    "ltm_classification_urldb_file",
                    "ltm_tacdb_customdb_file",
                    "security_dos_autodos_file_object",
                    "security_dos_l4bdos_file_object",
                    "security_http_file_type",
                    "sys_file_apache_ssl_cert",
                    "sys_file_browser_capabilities_db",
                    "sys_file_data_group",
                    "sys_file_device_capabilities_db",
                    "sys_file_external_monitor",
                    "sys_file_ifile",
                    "sys_file_lwtunneltbl",
                    "sys_file_rewrite_rule",
                    "sys_file_ssl_cert",
                    "sys_file_ssl_crl",
                    "sys_file_ssl_key",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "from-archive",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "from-plugin",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "from-uri",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "from-workspace",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "node-version",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rule",
                value_type: ValueKind::Reference,
                references: &[
                    "gtm_rule",
                    "ltm_cipher_rule",
                    "ltm_global_settings_rule",
                    "ltm_rule",
                    "ltm_rule_profiler",
                    "security_firewall_matching_rule",
                    "security_firewall_on_demand_rule_deploy",
                    "security_firewall_rule_list",
                    "security_firewall_rule_stat",
                    "security_packet_filter_rule_stat",
                    "sys_file_rewrite_rule",
                ],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
