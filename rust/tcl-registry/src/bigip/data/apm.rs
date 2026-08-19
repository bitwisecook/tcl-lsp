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

//! BIG-IP object specs for the `apm` tmsh module.
//!
//! **Hand-maintained.** These files were originally produced by a
//! one-time port of the pre-rewrite Python registry
//! (`dialects/f5/bigip/registry/specs/`, still present on `main`) via a
//! `scripts/registry-audit/gen_bigip_rust.py` generator that no longer
//! exists — deleted along with the rest of the retired Python tooling on
//! this branch, and the commits that ran it were squashed away in the
//! `rust` branch's rebase-onto-main history (see issue #1404). Originally
//! organised by the first letter of `kind`; reorganised by tmsh module
//! name (this file's own module field, `Some("apm")`) per
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
            kind: "apm_aaa_active_directory",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa active-directory"],
        },
        header_types: &[("apm", "aaa active-directory")],
        properties: &[
            BigipPropertySpec {
                name: "admin-encrypted-password",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "admin-name",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
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
                name: "cleanup-cache",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["group", "kerberos", "none", "pso"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain-controller",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain-controllers",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "ip",
                    value_type: ValueKind::String,
                    in_sections: &["domain-controllers"],
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip",
                value_type: ValueKind::String,
                in_sections: &["domain-controllers"],
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "group-cache-ttl",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "padata",
                value_type: ValueKind::Unknown,
                default: Some("rc4-hmac"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                references: &[
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pso-cache-ttl",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("15"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_active_directory_trusted_domains",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa active-directory-trusted-domains"],
        },
        header_types: &[("apm", "aaa active-directory-trusted-domains")],
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
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "root-domain",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trusted-domains",
                value_type: ValueKind::List,
                required: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "active-directory",
                    value_type: ValueKind::Reference,
                    in_sections: &["trusted-domains"],
                    references: &["apm_aaa_active_directory"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "active-directory",
                value_type: ValueKind::Reference,
                in_sections: &["trusted-domains"],
                references: &["apm_aaa_active_directory"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_crldp",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa crldp"],
        },
        header_types: &[("apm", "aaa crldp")],
        properties: &[
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-nullcrl",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
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
                name: "base-dn",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-expire",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "connection-timeout",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("15"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                references: &[
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("389"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reverse-dn",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-issuer",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-pool",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-sig",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_endpoint_management_system",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa endpoint-management-system"],
        },
        header_types: &[("apm", "aaa endpoint-management-system")],
        properties: &[
            BigipPropertySpec {
                name: "access-key",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-version",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "application-id",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "billing-id",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-id",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-secret",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-resolver",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_dns_resolver"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fqdn",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mdm-token",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "platform",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Unknown,
                default: Some("443"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serverssl-profile",
                value_type: ValueKind::Reference,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sync-interval",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("240 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tenant-id",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["airwatch", "maas360", "ms-intune"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_f5_mfa_configuration",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa f5-mfa-configuration"],
        },
        header_types: &[("apm", "aaa f5-mfa-configuration")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "f5-service-connector",
                value_type: ValueKind::Reference,
                required: true,
                references: &["apm_aaa_f5_service_connector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-mobile-devices-per-user",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "permitted-devices-types",
                value_type: ValueKind::List,
                required: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "registration-sms-template",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "require-biometric",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_f5_service_connector",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa f5-service-connector"],
        },
        header_types: &[("apm", "aaa f5-service-connector")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customer-id",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customer-key",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-resolver",
                value_type: ValueKind::Reference,
                required: true,
                references: &["net_dns_resolver"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serverssl-profile",
                value_type: ValueKind::Reference,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-url",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_http",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa http"],
        },
        header_types: &[("apm", "aaa http")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-type",
                value_type: ValueKind::Enum,
                enum_values: &["basic-ntlm", "custom-post", "form-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "content-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "url-encoded-utf8", "xml-utf8"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "custom-body",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "follow-redirect",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-action",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-fields",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-method",
                value_type: ValueKind::Enum,
                enum_values: &["get", "post"],
                default: Some("POST"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-params",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-password",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-username",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "headers",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hname",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hvalue",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hname",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hvalue",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "start-uri",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "success-match-type",
                value_type: ValueKind::Enum,
                enum_values: &["cookie", "exact-cookie", "url"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "success-match-value",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_http_connector_request",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa http-connector-request"],
        },
        header_types: &[("apm", "aaa http-connector-request")],
        properties: &[
            BigipPropertySpec {
                name: "auth",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["basic", "bearer", "custom", "none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "method",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-body",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-headers",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "response-action",
                value_type: ValueKind::Enum,
                enum_values: &["ignore", "parse", "save"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "response-headers",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secure-variables",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "token",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transport",
                value_type: ValueKind::Reference,
                required: true,
                references: &[
                    "apm_aaa_http_connector_transport",
                    "ltm_message_routing_diameter_transport_config",
                    "ltm_message_routing_generic_transport_config",
                    "ltm_message_routing_mqtt_transport_config",
                    "ltm_message_routing_sip_transport_config",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_kerberos",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa kerberos"],
        },
        header_types: &[("apm", "aaa kerberos")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-realm",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "keytab-file-obj",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-name",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_kerberos_keytab_file",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa kerberos-keytab-file"],
        },
        header_types: &[("apm", "aaa kerberos-keytab-file")],
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
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_ldap",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa ldap"],
        },
        header_types: &[("apm", "aaa ldap")],
        properties: &[
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::Unknown,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "admin-dn",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "admin-encrypted-password",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
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
                name: "base-dn",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "is-ldaps",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                references: &[
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Unknown,
                required: true,
                allow_none: true,
                default: Some("ldap"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "schema-attr",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "group-member",
                        value_type: ValueKind::String,
                        in_sections: &["schema-attr"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "group-member-value",
                        value_type: ValueKind::String,
                        in_sections: &["schema-attr"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "group-memberof",
                        value_type: ValueKind::String,
                        in_sections: &["schema-attr"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "group-object-class",
                        value_type: ValueKind::String,
                        in_sections: &["schema-attr"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "user-memberof",
                        value_type: ValueKind::String,
                        in_sections: &["schema-attr"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "user-object-class",
                        value_type: ValueKind::String,
                        in_sections: &["schema-attr"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "group-member",
                value_type: ValueKind::String,
                in_sections: &["schema-attr"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "group-member-value",
                value_type: ValueKind::String,
                in_sections: &["schema-attr"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "group-memberof",
                value_type: ValueKind::String,
                in_sections: &["schema-attr"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "group-object-class",
                value_type: ValueKind::String,
                in_sections: &["schema-attr"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-memberof",
                value_type: ValueKind::String,
                in_sections: &["schema-attr"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-object-class",
                value_type: ValueKind::String,
                in_sections: &["schema-attr"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serverssl-profile",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "none",
                    "serverssl",
                    "serverssl-insecure-compatible",
                    "wom-default-serverssl",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("15"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-pool",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_oam",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa oam"],
        },
        header_types: &[("apm", "aaa oam")],
        properties: &[
            BigipPropertySpec {
                name: "access-server-hostname",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "access-server-name",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "access-server-port",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("6021"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "access-server-retries",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "accessgate-encrypted-password",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "accessgates",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                enum_values: &["config-accessgate", "noop"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "admin-id",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "admin-password",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                default: Some("none"),
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
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enable",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "global-access-protocol-passphrase",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transport-security-mode",
                value_type: ValueKind::Enum,
                enum_values: &["cert", "open", "simple"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_oauth_provider",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa oauth-provider"],
        },
        header_types: &[("apm", "aaa oauth-provider")],
        properties: &[
            BigipPropertySpec {
                name: "allow-self-signed-jwk-cert",
                value_type: ValueKind::Unknown,
                default: Some("yes"),
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
                name: "authentication-uri",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-jwt-config-name",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-expired-cert",
                value_type: ValueKind::Unknown,
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-discovery-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "manual-jwt-config-name",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-json-nesting-layers",
                value_type: ValueKind::Integer,
                default: Some("8"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-response-size",
                value_type: ValueKind::Integer,
                default: Some("128kb"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "openid-cfg-uri",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-json-payload",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "token-uri",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "token-validation-scope-uri",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trusted-ca-bundle",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &["custom", "f5", "facebook", "google", "ping"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-auto-jwt-config",
                value_type: ValueKind::Unknown,
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "userinfo-request-uri",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_oauth_request",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa oauth-request"],
        },
        header_types: &[("apm", "aaa oauth-request")],
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
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "headers",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "value",
                    value_type: ValueKind::Unknown,
                    in_sections: &["headers"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::Unknown,
                in_sections: &["headers"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "method",
                value_type: ValueKind::Enum,
                enum_values: &["get", "post"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "parameters",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "type",
                        value_type: ValueKind::Unknown,
                        in_sections: &["parameters"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "value",
                        value_type: ValueKind::String,
                        in_sections: &["parameters"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Unknown,
                in_sections: &["parameters"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                in_sections: &["parameters"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "uri",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_oauth_server",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa oauth-server"],
        },
        header_types: &[("apm", "aaa oauth-server")],
        properties: &[
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
                name: "client-jwe-key",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-secret",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-serverssl-profile-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-resolver-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["client", "client-rs", "rs"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "provider-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "resource-server-id",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "resource-server-secret",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "resource-serverssl-profile-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::String,
                allow_none: true,
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
            BigipPropertySpec {
                name: "token-validation-interval",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_ocsp",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa ocsp"],
        },
        header_types: &[("apm", "aaa ocsp")],
        properties: &[
            BigipPropertySpec {
                name: "allow-certs",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
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
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ca-path",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cert-id-digest",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chain",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "check-certs",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "explicit-ocsp",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-aia",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "intern",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nonce",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sign-digest",
                value_type: ValueKind::Unknown,
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
                name: "sign-key-passphrase",
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
                value_type: ValueKind::Unknown,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trust-other",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::Unknown,
                required: true,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "va-file",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "validity-period",
                value_type: ValueKind::Unknown,
                required: true,
                default: Some("300"),
                usage_flags: &["not_synced"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-cert",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-other",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-sig",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_okta_connector",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa okta-connector"],
        },
        header_types: &[("apm", "aaa okta-connector")],
        properties: &[
            BigipPropertySpec {
                name: "domain",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "token",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transport",
                value_type: ValueKind::Reference,
                required: true,
                references: &[
                    "apm_aaa_http_connector_transport",
                    "ltm_message_routing_diameter_transport_config",
                    "ltm_message_routing_generic_transport_config",
                    "ltm_message_routing_mqtt_transport_config",
                    "ltm_message_routing_sip_transport_config",
                ],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_radius",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa radius"],
        },
        header_types: &[("apm", "aaa radius")],
        properties: &[
            BigipPropertySpec {
                name: "acct-port",
                value_type: ValueKind::Integer,
                default: Some("radius-acct"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::Unknown,
                required: true,
                allow_none: true,
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
                name: "auth-port",
                value_type: ValueKind::Integer,
                required: true,
                default: Some("radius"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["acct", "auth", "both"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nas-ip-address",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nas-ipv6-address",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::String,
                allow_none: true,
                references: &[
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "retries",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secret",
                value_type: ValueKind::String,
                required: true,
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
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-pool",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_saml",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa saml"],
        },
        header_types: &[("apm", "aaa saml")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assertion-consumer-binding",
                value_type: ValueKind::Enum,
                enum_values: &["http-artifact", "http-post"],
                default: Some("http-post"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attribute-consuming-services",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "attribute-consuming-service-index",
                    value_type: ValueKind::Integer,
                    in_sections: &["attribute-consuming-services"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attribute-consuming-service-index",
                value_type: ValueKind::Integer,
                in_sections: &["attribute-consuming-services"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-context-class-list",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-context-comparison-method",
                value_type: ValueKind::Enum,
                enum_values: &["better", "exact", "maximum", "minimum"],
                default: Some("exact"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-context-methods",
                value_type: ValueKind::List,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-attribute-consuming-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "entity-id",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "export-metadata",
                value_type: ValueKind::Enum,
                enum_values: &["no-signing", "with-signing"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "force-authn",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-connectors",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "idp-matching-source",
                        value_type: ValueKind::String,
                        in_sections: &["idp-connectors"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "idp-matching-value",
                        value_type: ValueKind::String,
                        in_sections: &["idp-connectors"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-matching-source",
                value_type: ValueKind::String,
                in_sections: &["idp-connectors"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-matching-value",
                value_type: ValueKind::String,
                in_sections: &["idp-connectors"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "is-authn-request-signed",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata-cert",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata-file",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata-signkey",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "name-id-policy-allow-create",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "name-id-policy-format",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "name-id-policy-sp-name-qualifier",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "provider-name",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "relay-state",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sp-certificate",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sp-host",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sp-scheme",
                value_type: ValueKind::Enum,
                enum_values: &["http", "https"],
                default: Some("https"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sp-signkey",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "want-assertion-encrypted",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "want-assertion-signed",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_saml_idp_automation",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa saml-idp-automation"],
        },
        header_types: &[("apm", "aaa saml-idp-automation")],
        properties: &[
            BigipPropertySpec {
                name: "aaa-saml-server",
                value_type: ValueKind::String,
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
                name: "connection-properties",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["connection-properties"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "dns-resolver-name",
                        value_type: ValueKind::String,
                        in_sections: &["connection-properties"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "serverssl-profile-name",
                        value_type: ValueKind::String,
                        in_sections: &["connection-properties"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["connection-properties"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-resolver-name",
                value_type: ValueKind::String,
                in_sections: &["connection-properties"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serverssl-profile-name",
                value_type: ValueKind::String,
                in_sections: &["connection-properties"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Integer,
                default: Some("60"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-matching-source",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-obj-name-tag",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata-matching-tag",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata-urls",
                value_type: ValueKind::List,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_saml_idp_connector",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa saml-idp-connector"],
        },
        header_types: &[("apm", "aaa saml-idp-connector")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "artifact-resolution-service-addr",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "artifact-resolution-service-port",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "artifact-resolution-service-url",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "basic-auth-password",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "basic-auth-username",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "entity-id",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "identity-location",
                value_type: ValueKind::Enum,
                enum_values: &["attribute", "subject"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "identity-location-attribute",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-certificate",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "import-metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata-cert",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "name-qualifier",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serverssl-profile-name",
                value_type: ValueKind::Reference,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sign-artifact-resolution-rq",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "single-logout-binding",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "single-logout-response-uri",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "single-logout-uri",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sso-binding",
                value_type: ValueKind::Enum,
                enum_values: &["http-post", "http-redirect"],
                default: Some("http-post"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sso-uri",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "want-authn-request-signed",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "want-detached-signature",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_securid",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa securid"],
        },
        header_types: &[("apm", "aaa securid")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "config-files",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-ip",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_aaa_tacacsplus",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["aaa tacacsplus"],
        },
        header_types: &[("apm", "aaa tacacsplus")],
        properties: &[
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::Unknown,
                required: true,
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
                name: "auth-service",
                value_type: ValueKind::Enum,
                required: true,
                allow_none: true,
                enum_values: &[
                    "arap", "enable", "fwproxy", "login", "nasi", "none", "ppp", "pt", "rcmd",
                    "x25",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-type",
                value_type: ValueKind::Enum,
                enum_values: &["arap", "ascii", "chap", "mschap", "pap"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encrypt",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::String,
                allow_none: true,
                references: &[
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("49"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "priv-lvl",
                value_type: ValueKind::Enum,
                enum_values: &["max", "min", "user"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "atalk", "deccp", "ftp", "http", "ip", "ipx", "lat", "lcp", "osicp", "pad",
                    "rlogin", "telnet", "tn3270", "unknown", "vines", "vpdn", "xremote",
                ],
                default: Some("unknown"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secret",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "arap",
                    "connection",
                    "firewall",
                    "none",
                    "ppp",
                    "shell",
                    "slip",
                    "system",
                    "tty-daemon",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-pool",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_acl",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["acl"],
        },
        header_types: &[("apm", "acl")],
        properties: &[
            BigipPropertySpec {
                name: "acl-order",
                value_type: ValueKind::Integer,
                required: true,
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
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "entries",
                value_type: ValueKind::List,
                block: &[
                    BigipPropertySpec {
                        name: "action",
                        value_type: ValueKind::Enum,
                        in_sections: &["entries"],
                        required: true,
                        enum_values: &["allow", "continue", "discard", "reject", "unspec"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "dst-end-port",
                        value_type: ValueKind::Unknown,
                        in_sections: &["entries"],
                        allow_none: true,
                        default: Some("0"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "dst-start-port",
                        value_type: ValueKind::Unknown,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "dst-subnet",
                        value_type: ValueKind::Unknown,
                        in_sections: &["entries"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "host",
                        value_type: ValueKind::String,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "log",
                        value_type: ValueKind::Enum,
                        in_sections: &["entries"],
                        allow_none: true,
                        enum_values: &["config", "none", "packet", "summary", "verbose"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "paths",
                        value_type: ValueKind::String,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "protocol",
                        value_type: ValueKind::Integer,
                        in_sections: &["entries"],
                        default: Some("0"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "scheme",
                        value_type: ValueKind::Unknown,
                        in_sections: &["entries"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "src-end-port",
                        value_type: ValueKind::Unknown,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "src-start-port",
                        value_type: ValueKind::Unknown,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "src-subnet",
                        value_type: ValueKind::Unknown,
                        in_sections: &["entries"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["entries"],
                required: true,
                enum_values: &["allow", "continue", "discard", "reject", "unspec"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-end-port",
                value_type: ValueKind::Unknown,
                in_sections: &["entries"],
                allow_none: true,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-start-port",
                value_type: ValueKind::Unknown,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-subnet",
                value_type: ValueKind::Unknown,
                in_sections: &["entries"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::String,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log",
                value_type: ValueKind::Enum,
                in_sections: &["entries"],
                allow_none: true,
                enum_values: &["config", "none", "packet", "summary", "verbose"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "paths",
                value_type: ValueKind::String,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Integer,
                in_sections: &["entries"],
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scheme",
                value_type: ValueKind::Unknown,
                in_sections: &["entries"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-end-port",
                value_type: ValueKind::Unknown,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-start-port",
                value_type: ValueKind::Unknown,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-subnet",
                value_type: ValueKind::Unknown,
                in_sections: &["entries"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "path-match-case",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &["dynamic", "static"],
                default: Some("static"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_apm_avr_config",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["apm-avr-config"],
        },
        header_types: &[("apm", "apm-avr-config")],
        properties: &[
            BigipPropertySpec {
                name: "avr-collect-data",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "avr-sampling",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_client_image",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["client image"],
        },
        header_types: &[("apm", "client image")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_configuration_captcha",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["configuration captcha"],
        },
        header_types: &[("apm", "configuration captcha")],
        properties: &[
            BigipPropertySpec {
                name: "captcha-data-size",
                value_type: ValueKind::Enum,
                enum_values: &["data-size-compact", "data-size-normal"],
                default: Some("data-size-normal"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "captcha-data-theme",
                value_type: ValueKind::Enum,
                enum_values: &["data-theme-dark", "data-theme-light"],
                default: Some("data-theme-light"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "captcha-data-type",
                value_type: ValueKind::Enum,
                enum_values: &["data-type-audio", "data-type-image"],
                default: Some("data-type-image"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "captcha-theme",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "theme-blackglass",
                    "theme-clean",
                    "theme-custom",
                    "theme-red",
                    "theme-white",
                ],
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "challenge-url",
                value_type: ValueKind::String,
                default: Some("www"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "exposition-threshold",
                value_type: ValueKind::Integer,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "noscript-url",
                value_type: ValueKind::String,
                default: Some("www"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "private-key",
                value_type: ValueKind::Unknown,
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proceed-on-verification-error",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "public-key",
                value_type: ValueKind::Unknown,
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secret",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-key",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "track-by-ip",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "track-by-username",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verification-url",
                value_type: ValueKind::String,
                default: Some("www"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_epsec_epsec_package",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["epsec epsec-package"],
        },
        header_types: &[("apm", "epsec epsec-package")],
        properties: &[
            BigipPropertySpec {
                name: "local-path",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_log_setting",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["log-setting"],
        },
        header_types: &[("apm", "log-setting")],
        properties: &[
            BigipPropertySpec {
                name: "access",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "log-level",
                        value_type: ValueKind::Unknown,
                        in_sections: &["access"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "publisher",
                        value_type: ValueKind::String,
                        in_sections: &["access"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-level",
                value_type: ValueKind::Unknown,
                in_sections: &["access"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "access-control",
                        value_type: ValueKind::Enum,
                        in_sections: &["access", "log-level"],
                        enum_values: &[
                            "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "access-per-request",
                        value_type: ValueKind::Enum,
                        in_sections: &["access", "log-level"],
                        enum_values: &[
                            "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "apm-acl",
                        value_type: ValueKind::Enum,
                        in_sections: &["access", "log-level"],
                        enum_values: &[
                            "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "eca",
                        value_type: ValueKind::Enum,
                        in_sections: &["access", "log-level"],
                        enum_values: &[
                            "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "paa",
                        value_type: ValueKind::Enum,
                        in_sections: &["access", "log-level"],
                        enum_values: &[
                            "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "sso",
                        value_type: ValueKind::Enum,
                        in_sections: &["access", "log-level"],
                        enum_values: &[
                            "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "swg",
                        value_type: ValueKind::Enum,
                        in_sections: &["access", "log-level"],
                        enum_values: &[
                            "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "access-control",
                value_type: ValueKind::Enum,
                in_sections: &["access", "log-level"],
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "access-per-request",
                value_type: ValueKind::Enum,
                in_sections: &["access", "log-level"],
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "apm-acl",
                value_type: ValueKind::Enum,
                in_sections: &["access", "log-level"],
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "eca",
                value_type: ValueKind::Enum,
                in_sections: &["access", "log-level"],
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "paa",
                value_type: ValueKind::Enum,
                in_sections: &["access", "log-level"],
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sso",
                value_type: ValueKind::Enum,
                in_sections: &["access", "log-level"],
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "swg",
                value_type: ValueKind::Enum,
                in_sections: &["access", "log-level"],
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "publisher",
                value_type: ValueKind::String,
                in_sections: &["access"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-filters",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "filter",
                        value_type: ValueKind::Unknown,
                        in_sections: &["url-filters"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "publisher",
                        value_type: ValueKind::String,
                        in_sections: &["url-filters"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "filter",
                value_type: ValueKind::Unknown,
                in_sections: &["url-filters"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "publisher",
                value_type: ValueKind::String,
                in_sections: &["url-filters"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_ntlm_machine_account",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["ntlm machine-account"],
        },
        header_types: &[("apm", "ntlm machine-account")],
        properties: &[
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                enum_values: &["change-password", "noop"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "administrator-name",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "administrator-password",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
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
                name: "domain-controller-fqdn",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain-fqdn",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "machine-account-name",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_ntlm_ntlm_auth",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["ntlm ntlm-auth"],
        },
        header_types: &[("apm", "ntlm ntlm-auth")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dc-fqdn-list",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "machine-account-name",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_db_instance",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth db-instance"],
        },
        header_types: &[("apm", "oauth db-instance")],
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
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "purge-frequency",
                value_type: ValueKind::Enum,
                enum_values: &["daily", "hourly", "monthly", "never", "weekly"],
                default: Some("daily"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "purge-now",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "purge-time",
                value_type: ValueKind::String,
                default: Some("02:00"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_jwk_config",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth jwk-config"],
        },
        header_types: &[("apm", "oauth jwk-config")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_jwt_config",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth jwt-config"],
        },
        header_types: &[("apm", "oauth jwt-config")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_jwt_provider_list",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth jwt-provider-list"],
        },
        header_types: &[("apm", "oauth jwt-provider-list")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_oauth_claim",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth oauth-claim"],
        },
        header_types: &[("apm", "oauth oauth-claim")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "claim-description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "claim-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "claim-type",
                value_type: ValueKind::Enum,
                enum_values: &["boolean", "custom", "number"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "claim-value",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_oauth_client_app",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth oauth-client-app"],
        },
        header_types: &[("apm", "oauth oauth-client-app")],
        properties: &[
            BigipPropertySpec {
                name: "access-token-lifetime",
                value_type: ValueKind::Integer,
                default: Some("5 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-plain-code-challenge",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-name",
                value_type: ValueKind::String,
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
                name: "audience",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-code-lifetime",
                value_type: ValueKind::Integer,
                default: Some("5 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["certificate", "none", "secret"],
                default: Some("secret and other possible values are none and certificate"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-cert-dn",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "contact",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "generate-jwt-refresh-token",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "generate-refresh-token",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "grant-code",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "grant-password",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "grant-token",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "id-token-claims",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "id-token-lifetime",
                value_type: ValueKind::Integer,
                default: Some("5 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jwt-access-token-claims",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jwt-access-token-lifetime",
                value_type: ValueKind::Integer,
                default: Some("5 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jwt-refresh-token-lifetime",
                value_type: ValueKind::Integer,
                default: Some("60 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "logo-url",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "openid-connect",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirect-uris",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "refresh-token-lifetime",
                value_type: ValueKind::Integer,
                default: Some("480 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "refresh-token-usage-limit",
                value_type: ValueKind::Integer,
                default: Some("64"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "regenerate-client-secret",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "require-pkce",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reuse-access-token",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reuse-refresh-token",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scopes",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-profile-token-mgmt-settings",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "userinfo-claims",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "website-url",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_oauth_resource_server",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth oauth-resource-server"],
        },
        header_types: &[("apm", "oauth oauth-resource-server")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["certificate", "none", "secret"],
                default: Some("certificate and other possible values are none and secret"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "regenerate-resource-server-secret",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "resource-server-cert-dn",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_oauth_scope",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth oauth-scope"],
        },
        header_types: &[("apm", "oauth oauth-scope")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scope-description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scope-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scope-value",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_aaa_active_directory",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent aaa-active-directory"],
        },
        header_types: &[("apm", "policy agent aaa-active-directory")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-max-logon-attempt",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fetch-nested-groups",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fetch-primary-groups",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hints",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "query-attrname",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "query-filter",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "show-extended-error",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trusted-domains",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &["auth", "last", "query"],
                default: Some("last"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "upn",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_aaa_client_cert",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent aaa-client-cert"],
        },
        header_types: &[("apm", "policy agent aaa-client-cert")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["request", "require"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_aaa_crldp",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent aaa-crldp"],
        },
        header_types: &[("apm", "policy agent aaa-crldp")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::Unknown,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_aaa_http",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent aaa-http"],
        },
        header_types: &[("apm", "policy agent aaa-http")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-logon-attempt",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_aaa_ldap",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent aaa-ldap"],
        },
        header_types: &[("apm", "policy agent aaa-ldap")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attr-name",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "filter",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "group-member-scope",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["all", "direct", "none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "group-membership-scope",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["all", "direct", "none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ldapmod-attributes",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-logon-attempt",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "modify-type",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "search-dn",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "show-extended-error",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::List,
                required: true,
                default: Some("last"),
                list_operators: &["modify"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-dn",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_aaa_oauth",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent aaa-oauth"],
        },
        header_types: &[("apm", "policy agent aaa-oauth")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-redirect-request",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "grant-type",
                value_type: ValueKind::Enum,
                enum_values: &["authorization-code", "password"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirection-uri",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "response",
                value_type: ValueKind::Reference,
                references: &[
                    "api_protection_response",
                    "apm_policy_agent_response_selection",
                    "apm_policy_agent_server_cert_response_control",
                    "asm_response_code",
                    "ltm_profile_response_adapt",
                    "sys_crypto_cert_validation_response_ocsp",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scope",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scope-data-request",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::Reference,
                references: &[
                    "api_protection_server",
                    "apm_aaa_oauth_server",
                    "apm_oauth_oauth_resource_server",
                    "apm_policy_agent_api_server_selection",
                    "apm_policy_agent_server_cert_response_control",
                    "apm_policy_agent_server_cert_status",
                    "auth_radius_server",
                    "gtm_listener_doh_server",
                    "gtm_monitor_real_server",
                    "gtm_server",
                    "ltm_auth_crldp_server",
                    "ltm_auth_radius_server",
                    "ltm_monitor_real_server",
                    "ltm_profile_doh_server",
                    "ltm_profile_server_ldap",
                    "ltm_profile_server_ssl",
                    "sys_crypto_server",
                    "sys_smtp_server",
                    "wom_server_discovery",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "token-refresh-request",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "token-request",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &["client", "scope"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "validation-scopes-request",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_aaa_radius",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent aaa-radius"],
        },
        header_types: &[("apm", "policy agent aaa-radius")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-logon-attempt",
                value_type: ValueKind::Unknown,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-source",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("%{session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "show-extended-error",
                value_type: ValueKind::Unknown,
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("%{session"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_aaa_saml",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent aaa-saml"],
        },
        header_types: &[("apm", "policy agent aaa-saml")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attr-consuming-service-session-var",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attribute-consuming-service",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_aaa_securid",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent aaa-securid"],
        },
        header_types: &[("apm", "policy agent aaa-securid")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-logon-attempt",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-source",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("%{session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "show-extended-error",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("%{session"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_acct_radius",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent acct-radius"],
        },
        header_types: &[("apm", "policy agent acct-radius")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("%{session"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_acct_tacacsplus",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent acct-tacacsplus"],
        },
        header_types: &[("apm", "policy agent acct-tacacsplus")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_api_authentication",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent api-authentication"],
        },
        header_types: &[("apm", "policy agent api-authentication")],
        properties: &[BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_api_server_selection",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent api-server-selection"],
        },
        header_types: &[("apm", "policy agent api-server-selection")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_decision_box",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent decision-box"],
        },
        header_types: &[("apm", "policy agent decision-box")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::Reference,
                references: &["apm_policy_customization_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_dynamic_acl",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent dynamic-acl"],
        },
        header_types: &[("apm", "policy agent dynamic-acl")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "entries",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_ending_allow",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent ending-allow"],
        },
        header_types: &[("apm", "policy agent ending-allow")],
        properties: &[BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_ending_deny",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent ending-deny"],
        },
        header_types: &[("apm", "policy agent ending-deny")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::Reference,
                required: true,
                references: &["apm_policy_customization_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_ending_redirect",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent ending-redirect"],
        },
        header_types: &[("apm", "policy agent ending-redirect")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "close-session",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_endpoint_check_machine_cert",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent endpoint-check-machine-cert"],
        },
        header_types: &[("apm", "policy agent endpoint-check-machine-cert")],
        properties: &[
            BigipPropertySpec {
                name: "allow-elevation",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
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
                name: "ca-profile-name",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "issuer",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-cert",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serial-number",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "store-location",
                value_type: ValueKind::Enum,
                enum_values: &["machine", "user"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "store-name",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subject-alt-name",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subject-match-fqdn",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_endpoint_check_software",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent endpoint-check-software"],
        },
        header_types: &[("apm", "policy agent endpoint-check-software")],
        properties: &[
            BigipPropertySpec {
                name: "check-list-type",
                value_type: ValueKind::Enum,
                enum_values: &["allow", "deny", "required"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "collect",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "continuous-check",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "items",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "db-age",
                    "db-version",
                    "last-scan",
                    "missing-updates",
                    "platform",
                    "product_id",
                    "state",
                    "vendor_id",
                    "version",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "antispyware",
                    "antivirus",
                    "firewall",
                    "hard-disk-encryption",
                    "health-agent",
                    "patch-management",
                    "peer-to-peer",
                ],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_endpoint_linux_check_file",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent endpoint-linux-check-file"],
        },
        header_types: &[("apm", "policy agent endpoint-linux-check-file")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "continuous-check",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "files",
                value_type: ValueKind::Enum,
                enum_values: &["md5", "modified", "size"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_endpoint_linux_check_process",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent endpoint-linux-check-process"],
        },
        header_types: &[("apm", "policy agent endpoint-linux-check-process")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "continuous-check",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "expression",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_endpoint_mac_check_file",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent endpoint-mac-check-file"],
        },
        header_types: &[("apm", "policy agent endpoint-mac-check-file")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "continuous-check",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "files",
                value_type: ValueKind::Enum,
                enum_values: &["md5", "modified", "size"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_endpoint_mac_check_process",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent endpoint-mac-check-process"],
        },
        header_types: &[("apm", "policy agent endpoint-mac-check-process")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "continuous-check",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "expression",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_endpoint_machine_info",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent endpoint-machine-info"],
        },
        header_types: &[("apm", "policy agent endpoint-machine-info")],
        properties: &[BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_endpoint_windows_browser_cache_cleaner",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent endpoint-windows-browser-cache-cleaner"],
        },
        header_types: &[("apm", "policy agent endpoint-windows-browser-cache-cleaner")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-clean-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["all", "all-except-css-js", "all-except-img-css-js", "none"],
                default: Some("all"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "clean-passwords",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "empty-recycle-bin",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idle-timeout",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["immediate", "indefinite"],
                default: Some("0, which enforces no timeout"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idle-timeout-screen-lock",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "monitor-webtop",
                value_type: ValueKind::Enum,
                enum_values: &["disable", "enable"],
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remove-connection-entry",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_endpoint_windows_check_file",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent endpoint-windows-check-file"],
        },
        header_types: &[("apm", "policy agent endpoint-windows-check-file")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "continuous-check",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "files",
                value_type: ValueKind::Enum,
                enum_values: &["md5", "modified", "operation", "signer", "size", "version"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_endpoint_windows_check_process",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent endpoint-windows-check-process"],
        },
        header_types: &[("apm", "policy agent endpoint-windows-check-process")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "continuous-check",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "expression",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_endpoint_windows_check_registry",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent endpoint-windows-check-registry"],
        },
        header_types: &[("apm", "policy agent endpoint-windows-check-registry")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "continuous-check",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "expression",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_endpoint_windows_group_policy",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent endpoint-windows-group-policy"],
        },
        header_types: &[("apm", "policy agent endpoint-windows-group-policy")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "policy-file",
                value_type: ValueKind::Unknown,
                required: true,
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_endpoint_windows_info_os",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent endpoint-windows-info-os"],
        },
        header_types: &[("apm", "policy agent endpoint-windows-info-os")],
        properties: &[BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_endpoint_windows_protected_workspace",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent endpoint-windows-protected-workspace"],
        },
        header_types: &[("apm", "policy agent endpoint-windows-protected-workspace")],
        properties: &[
            BigipPropertySpec {
                name: "allow-burn-cid",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-printer-use",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-user-switch",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allowed-network-shares",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
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
                name: "close-google-desktop-search",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "usb-flash-access",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["all", "ironkey", "none"],
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_external_logon_page",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent external-logon-page"],
        },
        header_types: &[("apm", "policy agent external-logon-page")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "split-username",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "uri",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_http_header_modify",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent http-header-modify"],
        },
        header_types: &[("apm", "policy agent http-header-modify")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cookie-entries",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["cookie-entries"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "cookie-name",
                        value_type: ValueKind::String,
                        in_sections: &["cookie-entries"],
                        required: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "cookie-operation",
                        value_type: ValueKind::Enum,
                        in_sections: &["cookie-entries"],
                        enum_values: &["cookie-delete", "cookie-update"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "cookie-value",
                        value_type: ValueKind::String,
                        in_sections: &["cookie-entries"],
                        required: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["cookie-entries"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cookie-name",
                value_type: ValueKind::String,
                in_sections: &["cookie-entries"],
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cookie-operation",
                value_type: ValueKind::Enum,
                in_sections: &["cookie-entries"],
                enum_values: &["cookie-delete", "cookie-update"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cookie-value",
                value_type: ValueKind::String,
                in_sections: &["cookie-entries"],
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "header-entries",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["header-entries"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "header-delimiter",
                        value_type: ValueKind::String,
                        in_sections: &["header-entries"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "header-name",
                        value_type: ValueKind::String,
                        in_sections: &["header-entries"],
                        required: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "header-operation",
                        value_type: ValueKind::Enum,
                        in_sections: &["header-entries"],
                        enum_values: &[
                            "header-append",
                            "header-insert",
                            "header-remove",
                            "header-replace",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "header-value",
                        value_type: ValueKind::String,
                        in_sections: &["header-entries"],
                        required: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["header-entries"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "header-delimiter",
                value_type: ValueKind::String,
                in_sections: &["header-entries"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "header-name",
                value_type: ValueKind::String,
                in_sections: &["header-entries"],
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "header-operation",
                value_type: ValueKind::Enum,
                in_sections: &["header-entries"],
                enum_values: &[
                    "header-append",
                    "header-insert",
                    "header-remove",
                    "header-replace",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "header-value",
                value_type: ValueKind::String,
                in_sections: &["header-entries"],
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_ip_geolocation_lookup",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent ip-geolocation-lookup"],
        },
        header_types: &[("apm", "policy agent ip-geolocation-lookup")],
        properties: &[BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_ip_reputation_lookup",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent ip-reputation-lookup"],
        },
        header_types: &[("apm", "policy agent ip-reputation-lookup")],
        properties: &[BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_irule_event",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent irule-event"],
        },
        header_types: &[("apm", "policy agent irule-event")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "expect-data",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "id",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_kerberos",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent kerberos"],
        },
        header_types: &[("apm", "policy agent kerberos")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-logon-attempt",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_l7_protocol_lookup",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent l7-protocol-lookup"],
        },
        header_types: &[("apm", "policy agent l7-protocol-lookup")],
        properties: &[BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_logging",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent logging"],
        },
        header_types: &[("apm", "policy agent logging")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-message",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "variables",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_logon_page",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent logon-page"],
        },
        header_types: &[("apm", "policy agent logon-page")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "basic-auth-realm",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "clean-sess-var1",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "clean-sess-var2",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "clean-sess-var3",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "clean-sess-var4",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "clean-sess-var5",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-modifiable1",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-modifiable2",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-modifiable3",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-modifiable4",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-modifiable5",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-type1",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["checkbox", "none", "password", "text"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-type2",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["checkbox", "none", "password", "text"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-type3",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["checkbox", "none", "password", "text"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-type4",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["checkbox", "none", "password", "text"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-type5",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["checkbox", "none", "password", "text"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "http-401-auth-level",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["basic", "basic-negotiate", "negotiate", "none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "post-var-name1",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "post-var-name2",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "post-var-name3",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "post-var-name4",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "post-var-name5",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "session-var-name1",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "session-var-name2",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "session-var-name3",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "session-var-name4",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "session-var-name5",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "split-username",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &["401", "form-based"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_message_box",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent message-box"],
        },
        header_types: &[("apm", "policy agent message-box")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_oam",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent oam"],
        },
        header_types: &[("apm", "policy agent oam")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-logon-attempt",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "show-extended-error",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_oauth_authz",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent oauth-authz"],
        },
        header_types: &[("apm", "policy agent oauth-authz")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "audience",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "entries",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["entries"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "expression",
                        value_type: ValueKind::String,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "id-token-claim-entries",
                        value_type: ValueKind::List,
                        in_sections: &["entries"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "jwt-access-token-claim-entries",
                        value_type: ValueKind::List,
                        in_sections: &["entries"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "scope-entries",
                        value_type: ValueKind::List,
                        in_sections: &["entries"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "userinfo-claim-entries",
                        value_type: ValueKind::List,
                        in_sections: &["entries"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["entries"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "expression",
                value_type: ValueKind::String,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "id-token-claim-entries",
                value_type: ValueKind::List,
                in_sections: &["entries"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["entries", "id-token-claim-entries"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "claim-name",
                        value_type: ValueKind::Unknown,
                        in_sections: &["entries", "id-token-claim-entries"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "claim-value",
                        value_type: ValueKind::String,
                        in_sections: &["entries", "id-token-claim-entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["entries", "id-token-claim-entries"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "claim-name",
                value_type: ValueKind::Unknown,
                in_sections: &["entries", "id-token-claim-entries"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "claim-value",
                value_type: ValueKind::String,
                in_sections: &["entries", "id-token-claim-entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jwt-access-token-claim-entries",
                value_type: ValueKind::List,
                in_sections: &["entries"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["entries", "jwt-access-token-claim-entries"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "claim-name",
                        value_type: ValueKind::Unknown,
                        in_sections: &["entries", "jwt-access-token-claim-entries"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "claim-value",
                        value_type: ValueKind::String,
                        in_sections: &["entries", "jwt-access-token-claim-entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["entries", "jwt-access-token-claim-entries"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "claim-name",
                value_type: ValueKind::Unknown,
                in_sections: &["entries", "jwt-access-token-claim-entries"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "claim-value",
                value_type: ValueKind::String,
                in_sections: &["entries", "jwt-access-token-claim-entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scope-entries",
                value_type: ValueKind::List,
                in_sections: &["entries"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["entries", "scope-entries"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "scope-name",
                        value_type: ValueKind::Unknown,
                        in_sections: &["entries", "scope-entries"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "scope-value",
                        value_type: ValueKind::String,
                        in_sections: &["entries", "scope-entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["entries", "scope-entries"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scope-name",
                value_type: ValueKind::Unknown,
                in_sections: &["entries", "scope-entries"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scope-value",
                value_type: ValueKind::String,
                in_sections: &["entries", "scope-entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "userinfo-claim-entries",
                value_type: ValueKind::List,
                in_sections: &["entries"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["entries", "userinfo-claim-entries"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "claim-name",
                        value_type: ValueKind::Unknown,
                        in_sections: &["entries", "userinfo-claim-entries"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "claim-value",
                        value_type: ValueKind::String,
                        in_sections: &["entries", "userinfo-claim-entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["entries", "userinfo-claim-entries"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "claim-name",
                value_type: ValueKind::Unknown,
                in_sections: &["entries", "userinfo-claim-entries"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "claim-value",
                value_type: ValueKind::String,
                in_sections: &["entries", "userinfo-claim-entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prompt-for-authorization",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subject",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_request_classification",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent request-classification"],
        },
        header_types: &[("apm", "policy agent request-classification")],
        properties: &[BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_resource_assign",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent resource-assign"],
        },
        header_types: &[("apm", "policy agent resource-assign")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::Unknown,
                allow_none: true,
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
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &["acls", "general", "resources", "webtop-and-webtop-links"],
                default: Some("general"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_response_selection",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent response-selection"],
        },
        header_types: &[("apm", "policy agent response-selection")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "response",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_route_domain_selection",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent route-domain-selection"],
        },
        header_types: &[("apm", "policy agent route-domain-selection")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Integer,
                allow_none: true,
                references: &["net_route_domain"],
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "snat",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["automap", "none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "snatpool",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_server_cert_response_control",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent server-cert-response-control"],
        },
        header_types: &[("apm", "policy agent server-cert-response-control")],
        properties: &[
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Integer,
                default: Some(
                    "ignore which specifies that the system ignores untrusted/expired certificate and may allow the connection",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_server_cert_status",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent server-cert-status"],
        },
        header_types: &[("apm", "policy agent server-cert-status")],
        properties: &[BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_session_check",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent session-check"],
        },
        header_types: &[("apm", "policy agent session-check")],
        properties: &[BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_ssl_check",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent ssl-check"],
        },
        header_types: &[("apm", "policy agent ssl-check")],
        properties: &[BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_tacacsplus",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent tacacsplus"],
        },
        header_types: &[("apm", "policy agent tacacsplus")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-logon-attempt",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_policy_agent_variable_assign",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["policy agent variable-assign"],
        },
        header_types: &[("apm", "policy agent variable-assign")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "citrix-smart-access",
                    "general",
                    "intranet-webtop",
                    "sso-cred-mapping",
                    "virtual-keyboard",
                ],
                default: Some("general"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "variables",
                value_type: ValueKind::List,
                references: &["apm_policy_agent_variable_assign"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_profile_access",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["profile access"],
        },
        header_types: &[("apm", "profile access")],
        properties: &[
            BigipPropertySpec {
                name: "accept-languages",
                value_type: ValueKind::List,
                required: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "access-policy",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "access-policy-timeout",
                value_type: ValueKind::Integer,
                default: Some("300 seconds"),
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
                name: "cache-generation",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-language",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                default: Some("en (English)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                references: &["apm_profile_access"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain-cookie",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain-groups",
                value_type: ValueKind::List,
                required: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain-mode",
                value_type: ValueKind::Enum,
                enum_values: &["multi-domain", "single-domain"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce-policy",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true which means the access-policy is always enforced"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "eps-group",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "errormap-group",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "framework-installation-group",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "general-ui-group",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "generation-action",
                value_type: ValueKind::Enum,
                enum_values: &["increment", "noop"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "httponly-cookie",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "inactivity-timeout",
                value_type: ValueKind::Integer,
                default: Some("900 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-settings",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "logout-uri-include",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "logout-uri-timeout",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-concurrent-sessions",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("0 (zero), which represents unlimited sessions"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-concurrent-users",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("0 (zero), which represents unlimited sessions"),
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-failure-delay",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-in-progress-sessions",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("0, which represents an unlimited number of such sessions"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-session-timeout",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-failure-delay",
                value_type: ValueKind::Integer,
                default: Some("2 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "named-scope",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "oauth-profile",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistent-cookie",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "primary-auth-service",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "restrict-to-single-client-ip",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sandboxes",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scope",
                value_type: ValueKind::Enum,
                enum_values: &["profile", "public", "virtual-server"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secure-cookie",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sso-name",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "all",
                    "identity-service",
                    "ltm-apm",
                    "oauth-resource-server",
                    "rdg-rap",
                    "ssl-vpn",
                    "sso",
                    "swg-explicit",
                    "swg-transparent",
                    "system-authentication",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-http-503-on-error",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-identity-method",
                value_type: ValueKind::Enum,
                enum_values: &["http"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_profile_connectivity",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["profile connectivity"],
        },
        header_types: &[("apm", "profile connectivity")],
        properties: &[
            BigipPropertySpec {
                name: "adaptive-compression",
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
                name: "citrix-client-bundle",
                value_type: ValueKind::Reference,
                references: &["apm_resource_remote_desktop_citrix_client_bundle"],
                default: Some("default-citrix-client-bundle"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-policy",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "android-ec",
                        value_type: ValueKind::Unknown,
                        in_sections: &["client-policy"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "android-ep",
                        value_type: ValueKind::Unknown,
                        in_sections: &["client-policy"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "chromeos-ec",
                        value_type: ValueKind::Unknown,
                        in_sections: &["client-policy"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ec",
                        value_type: ValueKind::Unknown,
                        in_sections: &["client-policy"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ios-ec",
                        value_type: ValueKind::Unknown,
                        in_sections: &["client-policy"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ios-ep",
                        value_type: ValueKind::Unknown,
                        in_sections: &["client-policy"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "macos-ec",
                        value_type: ValueKind::Unknown,
                        in_sections: &["client-policy"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "oauth",
                        value_type: ValueKind::Unknown,
                        in_sections: &["client-policy"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "servers",
                        value_type: ValueKind::List,
                        in_sections: &["client-policy"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "android-ec",
                value_type: ValueKind::Unknown,
                in_sections: &["client-policy"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "device-lock-complexity",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ec"],
                        allow_none: true,
                        enum_values: &["high", "low", "medium", "none"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "device-lock-method",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ec"],
                        enum_values: &["alphabetic", "alphanumeric", "any", "numeric"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enable-mobilesafe",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enforce-device-lock",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("true"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enforce-logon-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "logon-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ec"],
                        enum_values: &["native", "web"],
                        default: Some("native"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "max-inactivity-time",
                        value_type: ValueKind::Integer,
                        in_sections: &["client-policy", "android-ec"],
                        default: Some("5"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "min-passcode-length",
                        value_type: ValueKind::Integer,
                        in_sections: &["client-policy", "android-ec"],
                        required: true,
                        default: Some("4"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "require-device-auth",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password-method",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ec"],
                        enum_values: &["disk", "memory"],
                        default: Some("disk"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password-timeout",
                        value_type: ValueKind::Integer,
                        in_sections: &["client-policy", "android-ec"],
                        default: Some("240"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-lock-complexity",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ec"],
                allow_none: true,
                enum_values: &["high", "low", "medium", "none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-lock-method",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ec"],
                enum_values: &["alphabetic", "alphanumeric", "any", "numeric"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enable-mobilesafe",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce-device-lock",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce-logon-mode",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "logon-mode",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ec"],
                enum_values: &["native", "web"],
                default: Some("native"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-inactivity-time",
                value_type: ValueKind::Integer,
                in_sections: &["client-policy", "android-ec"],
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-passcode-length",
                value_type: ValueKind::Integer,
                in_sections: &["client-policy", "android-ec"],
                required: true,
                default: Some("4"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "require-device-auth",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password-method",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ec"],
                enum_values: &["disk", "memory"],
                default: Some("disk"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password-timeout",
                value_type: ValueKind::Integer,
                in_sections: &["client-policy", "android-ec"],
                default: Some("240"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "android-ep",
                value_type: ValueKind::Unknown,
                in_sections: &["client-policy"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "device-lock-method",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ep"],
                        enum_values: &["alphabetic", "alphanumeric", "any", "numeric"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enable-mobilesafe",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ep"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enforce-device-lock",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ep"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("true"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enforce-logon-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ep"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "logon-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ep"],
                        enum_values: &["native", "web"],
                        default: Some("native"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "max-inactivity-time",
                        value_type: ValueKind::Integer,
                        in_sections: &["client-policy", "android-ep"],
                        default: Some("5"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "min-passcode-length",
                        value_type: ValueKind::Integer,
                        in_sections: &["client-policy", "android-ep"],
                        required: true,
                        default: Some("4"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ep"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password-method",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "android-ep"],
                        enum_values: &["disk", "memory"],
                        default: Some("disk"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password-timeout",
                        value_type: ValueKind::Integer,
                        in_sections: &["client-policy", "android-ep"],
                        default: Some("240"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-lock-method",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ep"],
                enum_values: &["alphabetic", "alphanumeric", "any", "numeric"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enable-mobilesafe",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ep"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce-device-lock",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ep"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce-logon-mode",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ep"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "logon-mode",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ep"],
                enum_values: &["native", "web"],
                default: Some("native"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-inactivity-time",
                value_type: ValueKind::Integer,
                in_sections: &["client-policy", "android-ep"],
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-passcode-length",
                value_type: ValueKind::Integer,
                in_sections: &["client-policy", "android-ep"],
                required: true,
                default: Some("4"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ep"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password-method",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "android-ep"],
                enum_values: &["disk", "memory"],
                default: Some("disk"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password-timeout",
                value_type: ValueKind::Integer,
                in_sections: &["client-policy", "android-ep"],
                default: Some("240"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chromeos-ec",
                value_type: ValueKind::Unknown,
                in_sections: &["client-policy"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "enforce-logon-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "chromeos-ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "logon-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "chromeos-ec"],
                        enum_values: &["native", "web"],
                        default: Some("native"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "chromeos-ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password-method",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "chromeos-ec"],
                        enum_values: &["disk", "memory"],
                        default: Some("disk"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password-timeout",
                        value_type: ValueKind::Integer,
                        in_sections: &["client-policy", "chromeos-ec"],
                        default: Some("240"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce-logon-mode",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "chromeos-ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "logon-mode",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "chromeos-ec"],
                enum_values: &["native", "web"],
                default: Some("native"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "chromeos-ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password-method",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "chromeos-ec"],
                enum_values: &["disk", "memory"],
                default: Some("disk"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password-timeout",
                value_type: ValueKind::Integer,
                in_sections: &["client-policy", "chromeos-ec"],
                default: Some("240"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ec",
                value_type: ValueKind::Unknown,
                in_sections: &["client-policy"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "component-update",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ec"],
                        enum_values: &["no", "prompt", "yes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "location-dns",
                        value_type: ValueKind::List,
                        in_sections: &["client-policy", "ec"],
                        default: Some("none"),
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "reuse-winlogon-creds",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "reuse-winlogon-session",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password-method",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ec"],
                        enum_values: &["disk", "memory"],
                        default: Some("disk"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password-timeout",
                        value_type: ValueKind::Integer,
                        in_sections: &["client-policy", "ec"],
                        default: Some("240"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-servers-on-exit",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("true"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "component-update",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ec"],
                enum_values: &["no", "prompt", "yes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-dns",
                value_type: ValueKind::List,
                in_sections: &["client-policy", "ec"],
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reuse-winlogon-creds",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reuse-winlogon-session",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password-method",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ec"],
                enum_values: &["disk", "memory"],
                default: Some("disk"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password-timeout",
                value_type: ValueKind::Integer,
                in_sections: &["client-policy", "ec"],
                default: Some("240"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-servers-on-exit",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ios-ec",
                value_type: ValueKind::Unknown,
                in_sections: &["client-policy"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "enable-mobilesafe",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ios-ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enforce-logon-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ios-ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "logon-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ios-ec"],
                        enum_values: &["native", "web"],
                        default: Some("native"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "require-device-auth",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ios-ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ios-ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password-method",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ios-ec"],
                        enum_values: &["disk", "memory"],
                        default: Some("disk"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password-timeout",
                        value_type: ValueKind::Integer,
                        in_sections: &["client-policy", "ios-ec"],
                        default: Some("240"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "vod-disconnect-timeout",
                        value_type: ValueKind::Integer,
                        in_sections: &["client-policy", "ios-ec"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enable-mobilesafe",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ios-ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce-logon-mode",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ios-ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "logon-mode",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ios-ec"],
                enum_values: &["native", "web"],
                default: Some("native"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "require-device-auth",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ios-ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ios-ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password-method",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ios-ec"],
                enum_values: &["disk", "memory"],
                default: Some("disk"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password-timeout",
                value_type: ValueKind::Integer,
                in_sections: &["client-policy", "ios-ec"],
                default: Some("240"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vod-disconnect-timeout",
                value_type: ValueKind::Integer,
                in_sections: &["client-policy", "ios-ec"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ios-ep",
                value_type: ValueKind::Unknown,
                in_sections: &["client-policy"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "enable-mobilesafe",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ios-ep"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enforce-logon-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ios-ep"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enforce-pin-lock",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ios-ep"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("true"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "logon-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ios-ep"],
                        enum_values: &["native", "web"],
                        default: Some("native"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "max-grace-period",
                        value_type: ValueKind::Integer,
                        in_sections: &["client-policy", "ios-ep"],
                        default: Some("2"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ios-ep"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password-method",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "ios-ep"],
                        enum_values: &["disk", "memory"],
                        default: Some("disk"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password-timeout",
                        value_type: ValueKind::Integer,
                        in_sections: &["client-policy", "ios-ep"],
                        default: Some("240"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enable-mobilesafe",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ios-ep"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce-logon-mode",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ios-ep"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce-pin-lock",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ios-ep"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "logon-mode",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ios-ep"],
                enum_values: &["native", "web"],
                default: Some("native"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-grace-period",
                value_type: ValueKind::Integer,
                in_sections: &["client-policy", "ios-ep"],
                default: Some("2"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ios-ep"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password-method",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "ios-ep"],
                enum_values: &["disk", "memory"],
                default: Some("disk"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password-timeout",
                value_type: ValueKind::Integer,
                in_sections: &["client-policy", "ios-ep"],
                default: Some("240"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "macos-ec",
                value_type: ValueKind::Unknown,
                in_sections: &["client-policy"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "component-update",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "macos-ec"],
                        enum_values: &["no", "prompt", "yes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enforce-logon-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "macos-ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "location-dns",
                        value_type: ValueKind::List,
                        in_sections: &["client-policy", "macos-ec"],
                        default: Some("none"),
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "logon-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "macos-ec"],
                        enum_values: &["native", "web"],
                        default: Some("native"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "macos-ec"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password-method",
                        value_type: ValueKind::Enum,
                        in_sections: &["client-policy", "macos-ec"],
                        enum_values: &["disk", "memory"],
                        default: Some("disk"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "save-password-timeout",
                        value_type: ValueKind::Integer,
                        in_sections: &["client-policy", "macos-ec"],
                        default: Some("240"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "component-update",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "macos-ec"],
                enum_values: &["no", "prompt", "yes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce-logon-mode",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "macos-ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-dns",
                value_type: ValueKind::List,
                in_sections: &["client-policy", "macos-ec"],
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "logon-mode",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "macos-ec"],
                enum_values: &["native", "web"],
                default: Some("native"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "macos-ec"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password-method",
                value_type: ValueKind::Enum,
                in_sections: &["client-policy", "macos-ec"],
                enum_values: &["disk", "memory"],
                default: Some("disk"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "save-password-timeout",
                value_type: ValueKind::Integer,
                in_sections: &["client-policy", "macos-ec"],
                default: Some("240"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "oauth",
                value_type: ValueKind::Unknown,
                in_sections: &["client-policy"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "client-id",
                        value_type: ValueKind::String,
                        in_sections: &["client-policy", "oauth"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "client-secret",
                        value_type: ValueKind::String,
                        in_sections: &["client-policy", "oauth"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "done-uri",
                        value_type: ValueKind::String,
                        in_sections: &["client-policy", "oauth"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "provider-name",
                        value_type: ValueKind::Reference,
                        in_sections: &["client-policy", "oauth"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "scopes",
                        value_type: ValueKind::String,
                        in_sections: &["client-policy", "oauth"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-id",
                value_type: ValueKind::String,
                in_sections: &["client-policy", "oauth"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-secret",
                value_type: ValueKind::String,
                in_sections: &["client-policy", "oauth"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "done-uri",
                value_type: ValueKind::String,
                in_sections: &["client-policy", "oauth"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "provider-name",
                value_type: ValueKind::Reference,
                in_sections: &["client-policy", "oauth"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scopes",
                value_type: ValueKind::String,
                in_sections: &["client-policy", "oauth"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "servers",
                value_type: ValueKind::List,
                in_sections: &["client-policy"],
                block: &[
                    BigipPropertySpec {
                        name: "alias",
                        value_type: ValueKind::String,
                        in_sections: &["client-policy", "servers"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "host",
                        value_type: ValueKind::String,
                        in_sections: &["client-policy", "servers"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "alias",
                value_type: ValueKind::String,
                in_sections: &["client-policy", "servers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::String,
                in_sections: &["client-policy", "servers"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compress-buffer-size",
                value_type: ValueKind::Integer,
                default: Some("4096"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compress-cpu-saver",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compress-cpu-saver-high",
                value_type: ValueKind::Integer,
                default: Some("90 percent"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compress-cpu-saver-low",
                value_type: ValueKind::Integer,
                default: Some("75 percent"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compress-gzip-level",
                value_type: ValueKind::Integer,
                default: Some(
                    "6, which provides a higher amount of compression at the expense of more CPU processing time",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compress-gzip-memlevel",
                value_type: ValueKind::Integer,
                default: Some("8192"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compress-gzip-window-size",
                value_type: ValueKind::Integer,
                default: Some("16384"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compress-ingress",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compress-preferred-method",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("zlib"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compression",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compression-codecs",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "ltm_default_node_monitor",
                    "security_firewall_uuid_default_autogenerate",
                    "security_packet_filter_default_rules",
                    "sys_default_config",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "deflate-compression-level",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tunnel-name",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_profile_exchange",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["profile exchange"],
        },
        header_types: &[("apm", "profile exchange")],
        properties: &[
            BigipPropertySpec {
                name: "active-sync-auth-type",
                value_type: ValueKind::Enum,
                enum_values: &["basic", "basic-ntlm", "ntlm"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "active-sync-sso-config",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "active-sync-url",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-discover-auth-type",
                value_type: ValueKind::Enum,
                enum_values: &["basic", "basic-ntlm", "ntlm"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-discover-sso-config",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-discover-url",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ntlm-auth-name",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "offline-address-book-auth-type",
                value_type: ValueKind::Enum,
                enum_values: &["basic", "basic-ntlm", "ntlm"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "offline-address-book-sso-config",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "offline-address-book-url",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rpc-over-http-auth-type",
                value_type: ValueKind::Enum,
                enum_values: &["basic", "basic-ntlm", "ntlm"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rpc-over-http-sso-config",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rpc-over-http-url",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-agent-pattern-for-utf8",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "web-service-auth-type",
                value_type: ValueKind::Enum,
                enum_values: &["basic", "basic-ntlm", "ntlm"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "web-service-sso-config",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "web-service-url",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_profile_oauth",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["profile oauth"],
        },
        header_types: &[("apm", "profile oauth")],
        properties: &[
            BigipPropertySpec {
                name: "access-token-lifetime",
                value_type: ValueKind::Integer,
                default: Some("5 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-plain-code-challenge",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enalbed"],
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
                name: "audience",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-code-lifetime",
                value_type: ValueKind::Integer,
                default: Some("5 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-url",
                value_type: ValueKind::String,
                default: Some("/f5-oauth2/v1/authorize"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-apps",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "db-instance",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::String,
                allow_none: true,
                references: &["apm_profile_oauth"],
                default: Some("oauth"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "generate-jwt-refresh-token",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "generate-refresh-token",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "id-token-claims",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "id-token-lifetime",
                value_type: ValueKind::Integer,
                default: Some("5 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "id-token-primary-key",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-expired-cert",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "issuer",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jwks-url",
                value_type: ValueKind::String,
                default: Some("/f5-oauth2/v1/jwks"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jwt-access-token-claims",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jwt-access-token-lifetime",
                value_type: ValueKind::Integer,
                default: Some("5 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jwt-ec-signature-format",
                value_type: ValueKind::Enum,
                enum_values: &["binary", "der"],
                default: Some("binary format"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jwt-refresh-token-enc-secret",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jwt-refresh-token-lifetime",
                value_type: ValueKind::Integer,
                default: Some("60 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jwt-token",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "opaque-token",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "openid-cfg-url",
                value_type: ValueKind::String,
                default: Some("/f5-oauth2/v1/"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "openid-connect",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "per-user-token-limit",
                value_type: ValueKind::Integer,
                default: Some("255"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "primary-key",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "refresh-token-lifetime",
                value_type: ValueKind::Integer,
                default: Some("480 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "refresh-token-usage-limit",
                value_type: ValueKind::Integer,
                default: Some("0, which represents unlimited number of times"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "require-pkce",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "resource-servers",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reuse-access-token",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reuse-refresh-token",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rotation-keys",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subject",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("%{session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "token-introspection-url",
                value_type: ValueKind::String,
                default: Some("/f5-oauth2/v1/introspect"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "token-issuance-url",
                value_type: ValueKind::String,
                default: Some("/f5-oauth2/v1/token"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "token-revocation-url",
                value_type: ValueKind::String,
                default: Some("/f5-oauth2/v1/revoke"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trusted-ca-bundle",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "userinfo-claims",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "userinfo-primary-key",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "userinfo-url",
                value_type: ValueKind::String,
                default: Some("/f5-oauth2/v1/userinfo"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_profile_vdi",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["profile vdi"],
        },
        header_types: &[("apm", "profile vdi")],
        properties: &[
            BigipPropertySpec {
                name: "citrix-storefront-replacement",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "msrdp-ntlm-auth-name",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_report_custom_report_field",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["report custom-report-field"],
        },
        header_types: &[("apm", "report custom-report-field")],
        properties: &[
            BigipPropertySpec {
                name: "alias",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-position",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "report-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sort-direction",
                value_type: ValueKind::Enum,
                enum_values: &["asc", "desc", "unsorted"],
                default: Some("asc"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sort-order",
                value_type: ValueKind::Integer,
                default: Some("100000"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_address_space",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource address-space"],
        },
        header_types: &[("apm", "resource address-space")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cfg-uri",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "discovery-interval",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns",
                value_type: ValueKind::List,
                repeated: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv4",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv6",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-discovery-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-response-size",
                value_type: ValueKind::Unknown,
                default: Some("128 kB"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trusted-ca-bundle",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_app_tunnel",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource app-tunnel"],
        },
        header_types: &[("apm", "resource app-tunnel")],
        properties: &[
            BigipPropertySpec {
                name: "acl-order",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
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
                name: "application-launch-warning",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "apps",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::List,
                required: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "app-tunnel",
                    "last",
                    "network-access",
                    "remote-desktop",
                    "web-application",
                ],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_client_rate_class",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource client-rate-class"],
        },
        header_types: &[("apm", "resource client-rate-class")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "burst",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ceiling",
                value_type: ValueKind::Integer,
                default: Some("the value of the rate option"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dscp",
                value_type: ValueKind::Integer,
                default: Some("-1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["borrow", "discard", "shape"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &["best-effort", "controlled-load", "guaranteed"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_client_traffic_classifier",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource client-traffic-classifier"],
        },
        header_types: &[("apm", "resource client-traffic-classifier")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "entries",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["entries"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "client-rate-class",
                        value_type: ValueKind::String,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "dst-ip",
                        value_type: ValueKind::String,
                        in_sections: &["entries"],
                        allow_none: true,
                        shape_kind: Some(ValueKind::IpAddress),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "dst-mask",
                        value_type: ValueKind::Integer,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "dst-port",
                        value_type: ValueKind::Integer,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "protocol",
                        value_type: ValueKind::Integer,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "src-ip",
                        value_type: ValueKind::String,
                        in_sections: &["entries"],
                        allow_none: true,
                        shape_kind: Some(ValueKind::IpAddress),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "src-mask",
                        value_type: ValueKind::Integer,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "src-port",
                        value_type: ValueKind::Integer,
                        in_sections: &["entries"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["entries"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-rate-class",
                value_type: ValueKind::String,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-ip",
                value_type: ValueKind::String,
                in_sections: &["entries"],
                allow_none: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-mask",
                value_type: ValueKind::Integer,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-port",
                value_type: ValueKind::Integer,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Integer,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-ip",
                value_type: ValueKind::String,
                in_sections: &["entries"],
                allow_none: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-mask",
                value_type: ValueKind::Integer,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-port",
                value_type: ValueKind::Integer,
                in_sections: &["entries"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_ipv6_leasepool",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource ipv6-leasepool"],
        },
        header_types: &[("apm", "resource ipv6-leasepool")],
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
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "members",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_leasepool",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource leasepool"],
        },
        header_types: &[("apm", "resource leasepool")],
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
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "members",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_network_access",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource network-access"],
        },
        header_types: &[("apm", "resource network-access")],
        properties: &[
            BigipPropertySpec {
                name: "address-space-dhcp-requests-excluded",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address-space-exclude",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address-space-exclude-dns-name",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address-space-exclude-subnet",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address-space-include",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address-space-include-dns-name",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address-space-include-subnet",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address-space-loc-dns-servers-excluded",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address-space-local-subnets-excluded",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address-space-protect",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
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
                name: "application-launch",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "application-launch-warning",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-launch",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-interface-speed",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("100000000"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-ip-filter-engine",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("<false>"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-power-management",
                value_type: ValueKind::Enum,
                enum_values: &["ignore", "prevent", "terminate"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-proxy",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-proxy-address",
                value_type: ValueKind::Unknown,
                default: Some("any6"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-proxy-enforce-subnets",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-proxy-exclusion-list",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-proxy-ignore-auto-config-error",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-proxy-local-bypass",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-proxy-port",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-proxy-script",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-proxy-use-http-pac",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-proxy-use-local-proxy",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-traffic-classifier",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compression",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["gzip", "none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-primary",
                value_type: ValueKind::Unknown,
                default: Some("any6"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-secondary",
                value_type: ValueKind::Unknown,
                default: Some("any6"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-suffix",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "drive-mapping",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dtls",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dtls-port",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("4433"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "execute-logoff-scripts",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idle-timeout-threshold",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idle-timeout-window",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv6-address-space-exclude-subnet",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv6-address-space-include-subnet",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv6-dns-primary",
                value_type: ValueKind::Unknown,
                default: Some("any6"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv6-dns-secondary",
                value_type: ValueKind::Unknown,
                default: Some("any6"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv6-leasepool-name",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "leasepool-name",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "microsoft-network-client",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "microsoft-network-server",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "network-tunnel",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "optimized-app",
                value_type: ValueKind::List,
                allow_none: true,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "provide-client-cert",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-arp",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "split-tunneling",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "static-host",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "supported-ip-version",
                value_type: ValueKind::Enum,
                enum_values: &["ipv4", "ipv4-ipv6"],
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("ipv4"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sync-with-active-directory",
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
                    "app-tunnel",
                    "last",
                    "network-access",
                    "remote-desktop",
                    "web-application",
                ],
                default: Some("network-access"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "wins-primary",
                value_type: ValueKind::Unknown,
                default: Some("any6"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "wins-secondary",
                value_type: ValueKind::Unknown,
                default: Some("any6"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_portal_access",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource portal-access"],
        },
        header_types: &[("apm", "resource portal-access")],
        properties: &[
            BigipPropertySpec {
                name: "acl-order",
                value_type: ValueKind::Integer,
                required: true,
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
                name: "application-uri",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "css-patching",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flash-patching",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host-replace-string",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host-search-strings",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "html-patching",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "items",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "javascript-patching",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "patching-type",
                value_type: ValueKind::Enum,
                enum_values: &["full-patch", "min-patch"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "path-match-case",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-host",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-port",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "publish-on-webtop",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scheme-patching",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_remote_desktop_citrix",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource remote-desktop citrix"],
        },
        header_types: &[("apm", "resource remote-desktop citrix")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-logon",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "caption",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "detailed-description",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "caption",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "detailed-description",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.domain"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enable-serverside-ssl",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.password"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                references: &[
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("80"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_remote_desktop_citrix_client_bundle",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource remote-desktop citrix-client-bundle"],
        },
        header_types: &[("apm", "resource remote-desktop citrix-client-bundle")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "download-url",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "packages",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sb-windows-package",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "windows-download-url",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "windows-min-version",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "windows-package",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_remote_desktop_citrix_client_package_file",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource remote-desktop citrix-client-package-file"],
        },
        header_types: &[("apm", "resource remote-desktop citrix-client-package-file")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "original-file-name",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_remote_desktop_quest",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource remote-desktop quest"],
        },
        header_types: &[("apm", "resource remote-desktop quest")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-logon",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "caption",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "detailed-description",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "caption",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "detailed-description",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.domain"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enable-serverside-ssl",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.password"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                references: &[
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("8080"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_remote_desktop_rdp",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource remote-desktop rdp"],
        },
        header_types: &[("apm", "resource remote-desktop rdp")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-logon",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "caption",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "detailed-description",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "caption",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "detailed-description",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.domain"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.password"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_remote_desktop_vmware_view",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource remote-desktop vmware-view"],
        },
        header_types: &[("apm", "resource remote-desktop vmware-view")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-logon",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "caption",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "detailed-description",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "caption",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "detailed-description",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.domain"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enable-serverside-ssl",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.password"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                references: &[
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("80"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_sandbox",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource sandbox"],
        },
        header_types: &[("apm", "resource sandbox")],
        properties: &[
            BigipPropertySpec {
                name: "base-uri",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "files",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "content-type",
                        value_type: ValueKind::String,
                        in_sections: &["files"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "file-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["files"],
                        required: true,
                        enum_values: &["citrix-bundle", "customization", "unknown"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "filename",
                        value_type: ValueKind::String,
                        in_sections: &["files"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "folder",
                        value_type: ValueKind::String,
                        in_sections: &["files"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "local-path",
                        value_type: ValueKind::String,
                        in_sections: &["files"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "content-type",
                value_type: ValueKind::String,
                in_sections: &["files"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "file-type",
                value_type: ValueKind::Enum,
                in_sections: &["files"],
                required: true,
                enum_values: &["citrix-bundle", "customization", "unknown"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "filename",
                value_type: ValueKind::String,
                in_sections: &["files"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "folder",
                value_type: ValueKind::String,
                in_sections: &["files"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-path",
                value_type: ValueKind::String,
                in_sections: &["files"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_webtop",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource webtop"],
        },
        header_types: &[("apm", "resource webtop")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimize-to-tray",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "portal-access-start-uri",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "warn-when-closed",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "webtop-type",
                value_type: ValueKind::Enum,
                enum_values: &["full", "last", "network-access"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_webtop_link",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource webtop-link"],
        },
        header_types: &[("apm", "resource webtop-link")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "application-uri",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_saml_artifact_resolution_service",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["saml artifact-resolution-service"],
        },
        header_types: &[("apm", "saml artifact-resolution-service")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "artifact-resolution-service-host",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "artifact-resolution-service-port",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "artifact-send-method",
                value_type: ValueKind::Enum,
                enum_values: &["http-post", "http-redirect"],
                default: Some("http-redirect"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "artifact-validity",
                value_type: ValueKind::Integer,
                default: Some("60 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "basic-auth-password",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "basic-auth-username",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "virtual-server-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "want-artifact-resolution-rq-signed",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_saml_attribute_consuming_service",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["saml attribute-consuming-service"],
        },
        header_types: &[("apm", "saml attribute-consuming-service")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attributes",
                value_type: ValueKind::List,
                required: true,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["attributes"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "attribute-name",
                        value_type: ValueKind::String,
                        in_sections: &["attributes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "friendly-name",
                        value_type: ValueKind::String,
                        in_sections: &["attributes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "is-required",
                        value_type: ValueKind::Unknown,
                        in_sections: &["attributes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "name-format",
                        value_type: ValueKind::String,
                        in_sections: &["attributes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["attributes"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attribute-name",
                value_type: ValueKind::String,
                in_sections: &["attributes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "friendly-name",
                value_type: ValueKind::String,
                in_sections: &["attributes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "is-required",
                value_type: ValueKind::Unknown,
                in_sections: &["attributes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "name-format",
                value_type: ValueKind::String,
                in_sections: &["attributes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-name",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_saml_auth_context_class_list",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["saml auth-context-class-list"],
        },
        header_types: &[("apm", "saml auth-context-class-list")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "classes",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "order",
                        value_type: ValueKind::Integer,
                        in_sections: &["classes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "value",
                        value_type: ValueKind::String,
                        in_sections: &["classes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "order",
                value_type: ValueKind::Integer,
                in_sections: &["classes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                in_sections: &["classes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_session",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["session"],
        },
        header_types: &[("apm", "session")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_basic",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso basic"],
        },
        header_types: &[("apm", "sso basic")],
        properties: &[
            BigipPropertySpec {
                name: "apm-log-config",
                value_type: ValueKind::String,
                allow_none: true,
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
                name: "headers",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hname",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hvalue",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.sso.token.last.password"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-conversion",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_form_based",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso form-based"],
        },
        header_types: &[("apm", "sso form-based")],
        properties: &[
            BigipPropertySpec {
                name: "apm-log-config",
                value_type: ValueKind::String,
                allow_none: true,
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
                name: "external-access-management",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "oam"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-action",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-field",
                value_type: ValueKind::String,
                required: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-method",
                value_type: ValueKind::Enum,
                enum_values: &["get", "post"],
                default: Some("post"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-password",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-username",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "headers",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hname",
                        value_type: ValueKind::Unknown,
                        in_sections: &["headers"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hvalue",
                        value_type: ValueKind::Integer,
                        in_sections: &["headers"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hname",
                value_type: ValueKind::Unknown,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hvalue",
                value_type: ValueKind::Integer,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.sso.token.last.password"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "start-uri",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "success-match-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["cookie", "none", "url"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "success-match-value",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_form_basedv2",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso form-basedv2"],
        },
        header_types: &[("apm", "sso form-basedv2")],
        properties: &[
            BigipPropertySpec {
                name: "apm-log-config",
                value_type: ValueKind::String,
                allow_none: true,
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
                name: "forms",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "attribute-value",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "controls",
                        value_type: ValueKind::List,
                        in_sections: &["forms"],
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "form-order",
                        value_type: ValueKind::Integer,
                        in_sections: &["forms"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "id-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["action", "id", "inputs", "order"],
                        default: Some("inputs"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-method",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["get", "post"],
                        default: Some("get"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-name",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-negative",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-prefix",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("true and specifies a partial match"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["cookie", "header", "uri"],
                        default: Some("uri"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-value",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-autodetect",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("true"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-javascript",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-javascript-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["auto", "custom", "extra"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-method",
                        value_type: ValueKind::Unknown,
                        in_sections: &["forms"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-name",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-negative",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-prefix",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("true and specifies partial match"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["cookie", "header", "uri"],
                        default: Some("uri"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-value",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "success-match-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        allow_none: true,
                        enum_values: &["cookie", "none", "url"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "success-match-value",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attribute-value",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "controls",
                value_type: ValueKind::List,
                in_sections: &["forms"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["forms", "controls"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "secure",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms", "controls"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "value",
                        value_type: ValueKind::String,
                        in_sections: &["forms", "controls"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["forms", "controls"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secure",
                value_type: ValueKind::Enum,
                in_sections: &["forms", "controls"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                in_sections: &["forms", "controls"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-order",
                value_type: ValueKind::Integer,
                in_sections: &["forms"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "id-type",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["action", "id", "inputs", "order"],
                default: Some("inputs"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-method",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["get", "post"],
                default: Some("get"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-name",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-negative",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-prefix",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true and specifies a partial match"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-type",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["cookie", "header", "uri"],
                default: Some("uri"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-value",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-autodetect",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-javascript",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-javascript-type",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["auto", "custom", "extra"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-method",
                value_type: ValueKind::Unknown,
                in_sections: &["forms"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-name",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-negative",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-prefix",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true and specifies partial match"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-type",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["cookie", "header", "uri"],
                default: Some("uri"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-value",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "success-match-type",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                allow_none: true,
                enum_values: &["cookie", "none", "url"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "success-match-value",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "headers",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "value",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_kerberos",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso kerberos"],
        },
        header_types: &[("apm", "sso kerberos")],
        properties: &[
            BigipPropertySpec {
                name: "account-name",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "account-password",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "apm-log-config",
                value_type: ValueKind::String,
                allow_none: true,
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
                name: "headers",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hname",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hvalue",
                        value_type: ValueKind::Integer,
                        in_sections: &["headers"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hname",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hvalue",
                value_type: ValueKind::Integer,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "kdc",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "realm",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send-authorization",
                value_type: ValueKind::Enum,
                enum_values: &["401", "always"],
                default: Some("always"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "spn-pattern",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ticket-lifetime",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("600 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "upn-support",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-realm-source",
                value_type: ValueKind::String,
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::String,
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_ntlmv1",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso ntlmv1"],
        },
        header_types: &[("apm", "sso ntlmv1")],
        properties: &[
            BigipPropertySpec {
                name: "apm-log-config",
                value_type: ValueKind::String,
                allow_none: true,
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
                name: "domain-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.domain"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "headers",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hname",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hvalue",
                        value_type: ValueKind::Integer,
                        in_sections: &["headers"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hname",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hvalue",
                value_type: ValueKind::Integer,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ntlm-domain",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.sso.token.last.password"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-conversion",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_ntlmv2",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso ntlmv2"],
        },
        header_types: &[("apm", "sso ntlmv2")],
        properties: &[
            BigipPropertySpec {
                name: "apm-log-config",
                value_type: ValueKind::String,
                allow_none: true,
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
                name: "domain-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.domain"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "headers",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hname",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hvalue",
                        value_type: ValueKind::Integer,
                        in_sections: &["headers"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hname",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hvalue",
                value_type: ValueKind::Integer,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ntlm-domain",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.sso.token.last.password"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-conversion",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_oauth_bearer",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso oauth-bearer"],
        },
        header_types: &[("apm", "sso oauth-bearer")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "headers",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hname",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hvalue",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "oauth-server",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_saml",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso saml"],
        },
        header_types: &[("apm", "sso saml")],
        properties: &[
            BigipPropertySpec {
                name: "apm-log-config",
                value_type: ValueKind::String,
                allow_none: true,
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
                name: "artifact-resolution-service-name",
                value_type: ValueKind::Reference,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assertion-validity",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attributes",
                value_type: ValueKind::List,
                allow_none: true,
                usage_flags: &["deprecated", "optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-context-method",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encrypt",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encrypt-subject",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encryption-type",
                value_type: ValueKind::Enum,
                enum_values: &["aes128", "aes192", "aes256"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encryption-type-subject",
                value_type: ValueKind::Enum,
                enum_values: &["aes128", "aes192", "aes256"],
                default: Some("aes128"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "entity-id",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "export-metadata",
                value_type: ValueKind::Enum,
                enum_values: &["no-signing", "with-signing"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-certificate",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-certificate-session-var",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-host",
                value_type: ValueKind::Enum,
                required: true,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-scheme",
                value_type: ValueKind::Enum,
                enum_values: &["http", "https"],
                default: Some("https"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-signkey",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-signkey-session-var",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "key-transport-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &["rsa-oaep", "rsa-v1.5"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata-cert",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata-file",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata-signkey",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-values",
                value_type: ValueKind::List,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "name-qualifier",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "saml-profiles",
                value_type: ValueKind::List,
                allow_none: true,
                default: Some("web-browser-sso"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sp-connectors",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subject-type",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "entity",
                    "kerberos",
                    "persistent",
                    "transient",
                    "unspecified",
                    "x509-subject",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subject-value",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_saml_resource",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso saml-resource"],
        },
        header_types: &[("apm", "sso saml-resource")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "publish-on-webtop",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sso-config-saml",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_saml_sp_automation",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso saml-sp-automation"],
        },
        header_types: &[("apm", "sso saml-sp-automation")],
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
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-resolver-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Integer,
                default: Some("60"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata-urls",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "url-value",
                    value_type: ValueKind::String,
                    in_sections: &["metadata-urls"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-value",
                value_type: ValueKind::String,
                in_sections: &["metadata-urls"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serverssl-profile-name",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sp-obj-name-tag",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sso-config-saml",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_saml_sp_connector",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso saml-sp-connector"],
        },
        header_types: &[("apm", "sso saml-sp-connector")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assertion-consumer-services",
                value_type: ValueKind::List,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encryption-type",
                value_type: ValueKind::Enum,
                enum_values: &["aes128", "aes192", "aes256"],
                default: Some("aes128"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "entity-id",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "import-metadata",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "index",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "is-authn-request-signed",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "is-default",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata-cert",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-domain-location",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "relay-state",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signature-type",
                value_type: ValueKind::Enum,
                enum_values: &["rsa-sha1", "rsa-sha256", "rsa-sha384", "rsa-sha512"],
                default: Some("rsa-sha1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "single-logout-binding",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "single-logout-response-uri",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "single-logout-uri",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sp-certificate",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sp-location",
                value_type: ValueKind::Enum,
                enum_values: &["external", "internal", "internal-multi-domain"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sp-name-qualifier",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "uri",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "want-assertion-encrypted",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "want-assertion-signed",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "want-response-signed",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_swg_scheme",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["swg-scheme"],
        },
        header_types: &[("apm", "swg-scheme")],
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
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_url_filter",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["url-filter"],
        },
        header_types: &[("apm", "url-filter")],
        properties: &[
            BigipPropertySpec {
                name: "allowed-categories",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blocked-categories",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
