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

//! BIG-IP object specs for `ltm auth`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_auth_crldp_server",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["auth crldp-server"],
        },
        header_types: &[("ltm", "auth crldp-server")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "base-dn",
                value_type: ValueKind::Reference,
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
                name: "host",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                shape_kind: Some(ValueKind::IpAddress),
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
                default: Some("389"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reverse-dn",
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
            kind: "ltm_auth_kerberos_delegation",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["auth kerberos-delegation"],
        },
        header_types: &[("ltm", "auth kerberos-delegation")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-principal",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug-logging",
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
                name: "protocol-transition",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server-principal",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_auth_ldap",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["auth ldap"],
        },
        header_types: &[("ltm", "auth ldap")],
        properties: &[
            BigipPropertySpec {
                name: "bind-dn",
                value_type: ValueKind::Unknown,
                required: true,
                allow_none: true,
                default: Some("none"),
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bind-pw",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bind-timeout",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
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
                value_type: ValueKind::String,
                allow_none: true,
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
                value_type: ValueKind::String,
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
                value_type: ValueKind::Unknown,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "servers",
                value_type: ValueKind::Unknown,
                allow_none: true,
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
                value_type: ValueKind::Unknown,
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
            kind: "ltm_auth_ocsp_responder",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["auth ocsp-responder"],
        },
        header_types: &[("ltm", "auth ocsp-responder")],
        properties: &[
            BigipPropertySpec {
                name: "allow-certs",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
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
                name: "ca-file",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ca-path",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cert-id-digest",
                value_type: ValueKind::Enum,
                enum_values: &["md5", "sha1"],
                default: Some("sha1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chain",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "check-certs",
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
                name: "explicit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-aia",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "intern",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nonce",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sign-digest",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["md5", "sha1"],
                default: Some("sha1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sign-key",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sign-key-pass-phrase",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sign-other",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signer",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "status-age",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trust-other",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("value disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::Unknown,
                required: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "va-file",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "validity-period",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-cert",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-other",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-sig",
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
            kind: "ltm_auth_profile",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["auth profile"],
        },
        header_types: &[("ltm", "auth profile")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "configuration",
                value_type: ValueKind::Reference,
                required: true,
                allow_none: true,
                references: &["apm_aaa_f5_mfa_configuration", "apm_configuration_captcha"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cookie-key",
                value_type: ValueKind::String,
                default: Some("f5auth"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cookie-name",
                value_type: ValueKind::String,
                default: Some("abc123"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "credential-source",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                required: true,
                references: &["ltm_auth_profile"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enabled",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idle-timeout",
                value_type: ValueKind::Integer,
                default: Some("300 seconds"),
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
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_auth_radius",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["auth radius"],
        },
        header_types: &[("ltm", "auth radius")],
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
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["default", "none"],
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
                    "callback-nas-prompt",
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
            kind: "ltm_auth_radius_server",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["auth radius-server"],
        },
        header_types: &[("ltm", "auth radius-server")],
        properties: &[
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
                default: Some("3 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_auth_ssl_cc_ldap",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["auth ssl-cc-ldap"],
        },
        header_types: &[("ltm", "auth ssl-cc-ldap")],
        properties: &[
            BigipPropertySpec {
                name: "admin-dn",
                value_type: ValueKind::Reference,
                required: true,
                allow_none: true,
                default: Some("none"),
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "admin-password",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-size",
                value_type: ValueKind::Integer,
                default: Some("20000 bytes (20KB)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-timeout",
                value_type: ValueKind::Integer,
                default: Some("300 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "certmap-base",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "certmap-key",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "certmap-user-serial",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "group-base",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "group-key",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "group-member-key",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "role-key",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "search-type",
                value_type: ValueKind::Enum,
                enum_values: &["cert", "certmap", "user"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secure",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "servers",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-base",
                value_type: ValueKind::Unknown,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-class",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-key",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "valid-groups",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "valid-roles",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_auth_ssl_crldp",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["auth ssl-crldp"],
        },
        header_types: &[("ltm", "auth ssl-crldp")],
        properties: &[
            BigipPropertySpec {
                name: "cache-timeout",
                value_type: ValueKind::Integer,
                default: Some("86400 (24 hours)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "connection-timeout",
                value_type: ValueKind::Integer,
                default: Some("15"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "servers",
                value_type: ValueKind::Enum,
                required: true,
                allow_none: true,
                enum_values: &["default", "none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "update-interval",
                value_type: ValueKind::Integer,
                default: Some("0 (zero), which indicates an internal default value is active"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-issuer",
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
            kind: "ltm_auth_ssl_ocsp",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["auth ssl-ocsp"],
        },
        header_types: &[("ltm", "auth ssl-ocsp")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "responders",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["default", "none"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_auth_tacacs",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["auth tacacs"],
        },
        header_types: &[("ltm", "auth tacacs")],
        properties: &[
            BigipPropertySpec {
                name: "accounting",
                value_type: ValueKind::Enum,
                enum_values: &["send-to-all-servers", "send-to-first-server"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "authentication",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["use-all-servers", "use-first-server"],
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
        ],
    },
];
