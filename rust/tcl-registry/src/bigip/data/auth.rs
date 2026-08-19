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

//! BIG-IP object specs for the `auth` tmsh module.
//!
//! **Hand-maintained.** These files were originally produced by a
//! one-time port of the pre-rewrite Python registry
//! (`dialects/f5/bigip/registry/specs/`, still present on `main`) via a
//! `scripts/registry-audit/gen_bigip_rust.py` generator that no longer
//! exists — deleted along with the rest of the retired Python tooling on
//! this branch, and the commits that ran it were squashed away in the
//! `rust` branch's rebase-onto-main history (see issue #1404). Originally
//! organised by the first letter of `kind`; reorganised by tmsh module
//! name (this file's own module field, `Some("auth")`) per
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
            kind: "auth_apm_auth",
            table_name: None,
            resolver_name: None,
            module: Some("auth"),
            object_types: &["apm-auth"],
        },
        header_types: &[("auth", "apm-auth")],
        properties: &[BigipPropertySpec {
            name: "profile-access",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "auth_cert_ldap",
            table_name: None,
            resolver_name: None,
            module: Some("auth"),
            object_types: &["cert-ldap"],
        },
        header_types: &[("auth", "cert-ldap")],
        properties: &[
            BigipPropertySpec {
                name: "bind-dn",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bind-pw",
                value_type: ValueKind::Unknown,
                required: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bind-timeout",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "check-host-attr",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "check-roles-group",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "filter",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "apm_url_filter",
                    "net_packet_filter",
                    "net_packet_filter_trusted",
                    "security_packet_filter_default_rules",
                    "security_packet_filter_policy",
                    "security_packet_filter_rule_stat",
                    "sys_air_filter_reset",
                    "sys_log_config_filter",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idle-timeout",
                value_type: ValueKind::Integer,
                default: Some("3600 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-auth-info-unavail",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-unknown-user",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "login-attribute",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "login-filter",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "login-name",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Reference,
                references: &[
                    "net_port_list",
                    "net_port_mirror",
                    "security_firewall_port_list",
                    "security_firewall_port_misuse_policy",
                    "sys_log_config_destination_management_port",
                ],
                default: Some("ldap"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "referrals",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scope",
                value_type: ValueKind::Enum,
                enum_values: &["base", "one", "sub"],
                default: Some("sub"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "search-base-dn",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "search-timeout",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "servers",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-ca-cert-file",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-check-peer",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-ciphers",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-client-cert",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-client-key",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-cname-field",
                value_type: ValueKind::Enum,
                enum_values: &["san-ipadd", "san-other", "san-rid", "san-x400"],
                default: Some("subjectname-cn"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-cname-otheroid",
                value_type: ValueKind::Unknown,
                required: true,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sso",
                value_type: ValueKind::Enum,
                enum_values: &["off", "on"],
                default: Some("off"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "version",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "warnings",
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
            kind: "auth_ldap",
            table_name: None,
            resolver_name: None,
            module: Some("auth"),
            object_types: &["ldap"],
        },
        header_types: &[("auth", "ldap")],
        properties: &[
            BigipPropertySpec {
                name: "bind-dn",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bind-pw",
                value_type: ValueKind::Unknown,
                required: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bind-timeout",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "check-host-attr",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "check-roles-group",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "filter",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "apm_url_filter",
                    "net_packet_filter",
                    "net_packet_filter_trusted",
                    "security_packet_filter_default_rules",
                    "security_packet_filter_policy",
                    "security_packet_filter_rule_stat",
                    "sys_air_filter_reset",
                    "sys_log_config_filter",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "group-dn",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "group-member-attr",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idle-timeout",
                value_type: ValueKind::Integer,
                default: Some("3600 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-auth-info-unavail",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-unknown-user",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "login-attribute",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Reference,
                references: &[
                    "net_port_list",
                    "net_port_mirror",
                    "security_firewall_port_list",
                    "security_firewall_port_misuse_policy",
                    "sys_log_config_destination_management_port",
                ],
                default: Some("ldap"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "referrals",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scope",
                value_type: ValueKind::Enum,
                enum_values: &["base", "one", "sub"],
                default: Some("sub"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "search-base-dn",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "search-timeout",
                value_type: ValueKind::Integer,
                default: Some("30"),
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
                name: "ssl",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-ca-cert-file",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-check-peer",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-ciphers",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-client-cert",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-client-key",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-template",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "version",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "warnings",
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
            kind: "auth_partition",
            table_name: None,
            resolver_name: None,
            module: Some("auth"),
            object_types: &["partition"],
        },
        header_types: &[("auth", "partition")],
        properties: &[
            BigipPropertySpec {
                name: "default-route-domain",
                value_type: ValueKind::Unknown,
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
            kind: "auth_password",
            table_name: None,
            resolver_name: None,
            module: Some("auth"),
            object_types: &["password"],
        },
        header_types: &[("auth", "password")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "auth_password_policy",
            table_name: None,
            resolver_name: None,
            module: Some("auth"),
            object_types: &["password-policy"],
        },
        header_types: &[("auth", "password-policy")],
        properties: &[
            BigipPropertySpec {
                name: "expiration-warning",
                value_type: ValueKind::Integer,
                default: Some("7 days"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lockout-duration",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-duration",
                value_type: ValueKind::Integer,
                default: Some("99999"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-login-failures",
                value_type: ValueKind::Integer,
                default: Some("0 (zero - disabled)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-duration",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimum-length",
                value_type: ValueKind::Integer,
                default: Some("6"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-memory",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "policy-enforcement",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "required-lowercase",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "required-numeric",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "required-special",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "required-uppercase",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "auth_radius",
            table_name: None,
            resolver_name: None,
            module: Some("auth"),
            object_types: &["radius"],
        },
        header_types: &[("auth", "radius")],
        properties: &[
            BigipPropertySpec {
                name: "accounting-bug",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
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
                name: "client-id",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "retries",
                value_type: ValueKind::Integer,
                default: Some("3"),
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
                name: "service-type",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "administrative",
                    "authenticate-only",
                    "call-check",
                    "callback-administrative",
                    "callback-framed",
                    "callback-login",
                    "callback-nas-promit",
                    "default",
                    "framed",
                    "login",
                    "nas-prompt",
                    "outbound",
                ],
                default: Some("default, which behaves as authenticate-only"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "auth_radius_server",
            table_name: None,
            resolver_name: None,
            module: Some("auth"),
            object_types: &["radius-server"],
        },
        header_types: &[("auth", "radius-server")],
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
                name: "port",
                value_type: ValueKind::Reference,
                references: &[
                    "net_port_list",
                    "net_port_mirror",
                    "security_firewall_port_list",
                    "security_firewall_port_misuse_policy",
                    "sys_log_config_destination_management_port",
                ],
                default: Some("1812"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secret",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "auth_remote_role",
            table_name: None,
            resolver_name: None,
            module: Some("auth"),
            object_types: &["remote-role"],
        },
        header_types: &[("auth", "remote-role")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "role-info",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "attribute",
                        value_type: ValueKind::String,
                        in_sections: &["role-info"],
                        required: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "console",
                        value_type: ValueKind::Enum,
                        in_sections: &["role-info"],
                        enum_values: &["disabled", "tmsh"],
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "deny",
                        value_type: ValueKind::Enum,
                        in_sections: &["role-info"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["role-info"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "line-order",
                        value_type: ValueKind::Integer,
                        in_sections: &["role-info"],
                        required: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "role",
                        value_type: ValueKind::Enum,
                        in_sections: &["role-info"],
                        enum_values: &[
                            "acceleration-policy-editor",
                            "admin",
                            "application-editor",
                            "auditor",
                            "certificate-manager",
                            "firewall-manager",
                            "fraud-protection-manager",
                            "guest",
                            "irule-manager",
                            "log-manager",
                            "manager",
                            "no-access",
                            "operator",
                            "resource-admin",
                            "user-manager",
                            "web-application-security-administrator",
                            "web-application-security-editor",
                            "web-application-security-operations-administrator",
                        ],
                        default: Some("no-access"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "user-partition",
                        value_type: ValueKind::Reference,
                        in_sections: &["role-info"],
                        required: true,
                        default: Some("Common"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attribute",
                value_type: ValueKind::String,
                in_sections: &["role-info"],
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "console",
                value_type: ValueKind::Enum,
                in_sections: &["role-info"],
                enum_values: &["disabled", "tmsh"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "deny",
                value_type: ValueKind::Enum,
                in_sections: &["role-info"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["role-info"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "line-order",
                value_type: ValueKind::Integer,
                in_sections: &["role-info"],
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "role",
                value_type: ValueKind::Enum,
                in_sections: &["role-info"],
                enum_values: &[
                    "acceleration-policy-editor",
                    "admin",
                    "application-editor",
                    "auditor",
                    "certificate-manager",
                    "firewall-manager",
                    "fraud-protection-manager",
                    "guest",
                    "irule-manager",
                    "log-manager",
                    "manager",
                    "no-access",
                    "operator",
                    "resource-admin",
                    "user-manager",
                    "web-application-security-administrator",
                    "web-application-security-editor",
                    "web-application-security-operations-administrator",
                ],
                default: Some("no-access"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-partition",
                value_type: ValueKind::Reference,
                in_sections: &["role-info"],
                required: true,
                default: Some("Common"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "auth_remote_user",
            table_name: None,
            resolver_name: None,
            module: Some("auth"),
            object_types: &["remote-user"],
        },
        header_types: &[("auth", "remote-user")],
        properties: &[
            BigipPropertySpec {
                name: "default-partition",
                value_type: ValueKind::Reference,
                default: Some("all"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-role",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "acceleration-policy-editor",
                    "admin",
                    "application-editor",
                    "auditor",
                    "firewall-manager",
                    "fraud-protection-manager",
                    "guest",
                    "irule-manager",
                    "log-manager",
                    "manager",
                    "no-access",
                    "operator",
                    "resource-admin",
                    "user-manager",
                    "web-application-security-administrator",
                    "web-application-security-editor",
                    "web-application-security-operations-administrator",
                ],
                default: Some("no-access"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-console-access",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "tmsh"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "auth_source",
            table_name: None,
            resolver_name: None,
            module: Some("auth"),
            object_types: &["source"],
        },
        header_types: &[("auth", "source")],
        properties: &[
            BigipPropertySpec {
                name: "fallback",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "active-directory",
                    "apm-auth",
                    "cert-ldap",
                    "ldap",
                    "local",
                    "radius",
                    "tacacs",
                ],
                default: Some("local"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "auth_tacacs",
            table_name: None,
            resolver_name: None,
            module: Some("auth"),
            object_types: &["tacacs"],
        },
        header_types: &[("auth", "tacacs")],
        properties: &[
            BigipPropertySpec {
                name: "accounting",
                value_type: ValueKind::Enum,
                enum_values: &["send-to-all-servers", "send-to-first-server"],
                default: Some("send-to-first-server"),
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
                name: "authentication",
                value_type: ValueKind::Enum,
                enum_values: &["use-all-servers", "use-first-server"],
                default: Some("use-first-server"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encryption",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secret",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "servers",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::Endpoint),
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service",
                value_type: ValueKind::Reference,
                required: true,
                allow_none: true,
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "auth_user",
            table_name: None,
            resolver_name: None,
            module: Some("auth"),
            object_types: &["user"],
        },
        header_types: &[("auth", "user")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::Unknown,
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "partition-access",
                value_type: ValueKind::List,
                references: &["auth_partition"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prompt-for-password",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "session-limit",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "shell",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
