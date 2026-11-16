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

//! BIG-IP object specs for `ltm profile server-ssl`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_server_ssl",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile server-ssl"],
    },
    header_types: &[("ltm", "profile server-ssl")],
    properties: &[
        BigipPropertySpec {
            name: "alert-timeout",
            value_type: ValueKind::Integer,
            default: Some("indefinite"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "allow-expired-crl",
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
            name: "authenticate",
            value_type: ValueKind::Enum,
            enum_values: &["always", "once"],
            default: Some("once"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "authenticate-depth",
            value_type: ValueKind::Integer,
            default: Some("9"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "authenticate-name",
            value_type: ValueKind::Reference,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bypass-on-client-cert-fail",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bypass-on-handshake-alert",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "c3d-ca-cert",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "c3d-ca-key",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "c3d-ca-passphrase",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "c3d-cert-extension-custom-oids",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "c3d-cert-extension-includes",
            value_type: ValueKind::List,
            default: Some(
                "{ basic-constraints extended-key-usage key-usage subject-alternative-name }",
            ),
            block: &[
                BigipPropertySpec {
                    name: "key-usage",
                    value_type: ValueKind::Unknown,
                    in_sections: &["c3d-cert-extension-includes"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "none",
                    value_type: ValueKind::Unknown,
                    in_sections: &["c3d-cert-extension-includes"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "key-usage",
            value_type: ValueKind::Unknown,
            in_sections: &["c3d-cert-extension-includes"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "none",
            value_type: ValueKind::Unknown,
            in_sections: &["c3d-cert-extension-includes"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "c3d-cert-lifespan",
            value_type: ValueKind::Integer,
            default: Some("24"),
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
            name: "cache-size",
            value_type: ValueKind::Integer,
            default: Some("262144"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cache-timeout",
            value_type: ValueKind::Integer,
            default: Some("3600 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cert",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &[
                "apm_policy_agent_aaa_client_cert",
                "apm_policy_agent_endpoint_check_machine_cert",
                "apm_policy_agent_server_cert_response_control",
                "apm_policy_agent_server_cert_status",
                "auth_cert_ldap",
                "cm_cert",
                "sys_crypto_cert",
                "sys_crypto_cert_order_manager",
                "sys_crypto_cert_validation_response_ocsp",
                "sys_crypto_cert_validator_crl",
                "sys_crypto_cert_validator_ocsp",
                "sys_crypto_check_cert",
                "sys_file_apache_ssl_cert",
                "sys_file_ssl_cert",
            ],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "chain",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["net_sfc_chain", "pem_service_chain_endpoint"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cipher-group",
            value_type: ValueKind::Reference,
            allow_none: true,
            enum_values: &["none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ciphers",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_cipher_group", "ltm_cipher_rule"],
            default: Some("DEFAULT"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "crl",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &[
                "sys_crypto_cert_validator_crl",
                "sys_crypto_crl",
                "sys_file_ssl_crl",
            ],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "crl-file",
            value_type: ValueKind::Unknown,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "data-0rtt",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_profile_server_ssl"],
            default: Some("serverssl"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "expire-cert-response-control",
            value_type: ValueKind::Enum,
            enum_values: &["drop", "ignore", "mask"],
            default: Some("drop, which causes the connection to be dropped"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "generic-alert",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "handshake-timeout",
            value_type: ValueKind::Integer,
            default: Some("10"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "key",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &[
                "cm_key",
                "ltm_dns_cache_records_key",
                "ltm_dns_dnssec_key",
                "ltm_dns_hpke_key",
                "ltm_dns_tsig_key",
                "sys_crypto_allow_key_export",
                "sys_crypto_fips_key",
                "sys_crypto_key",
                "sys_crypto_master_key",
                "sys_dynad_key",
                "sys_file_ssl_key",
                "sys_iapp_restricted_key",
                "sys_iapprestricted_key",
            ],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-publisher",
            value_type: ValueKind::Reference,
            allow_none: true,
            enum_values: &["none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-ssl-c3d-events",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-ssl-client-authentication-events",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-ssl-forward-proxy-events",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-ssl-handshake-events",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-active-handshakes",
            value_type: ValueKind::Integer,
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mod-ssl-methods",
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
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ocsp",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &[
                "apm_aaa_ocsp",
                "apm_policy_agent_aaa_ocsp",
                "ltm_auth_ocsp_responder",
                "ltm_auth_ssl_ocsp",
                "ltm_clientssl_ocsp_stapling_responses",
                "ltm_profile_ocsp",
                "ltm_profile_ocsp_stapling_params",
                "sys_crypto_cert_validation_response_ocsp",
                "sys_crypto_cert_validator_ocsp",
            ],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "options",
            value_type: ValueKind::List,
            default: Some("dont-insert-empty-fragments no-tlsv1"),
            block: &[
                BigipPropertySpec {
                    name: "no-session-resumption-on-renegotiation",
                    value_type: ValueKind::Unknown,
                    in_sections: &["options"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "no-ssl",
                    value_type: ValueKind::Unknown,
                    in_sections: &["options"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "none",
                    value_type: ValueKind::Unknown,
                    in_sections: &["options"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "single-dh-use",
                    value_type: ValueKind::Unknown,
                    in_sections: &["options"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "no-session-resumption-on-renegotiation",
            value_type: ValueKind::Unknown,
            in_sections: &["options"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "no-ssl",
            value_type: ValueKind::Unknown,
            in_sections: &["options"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "none",
            value_type: ValueKind::Unknown,
            in_sections: &["options"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "single-dh-use",
            value_type: ValueKind::Unknown,
            in_sections: &["options"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "passphrase",
            value_type: ValueKind::String,
            required: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "peer-cert-mode",
            value_type: ValueKind::Enum,
            enum_values: &["ignore", "require"],
            default: Some("ignore"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "proxy-ssl",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "proxy-ssl-passthrough",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "renegotiate-period",
            value_type: ValueKind::Integer,
            default: Some(
                "indefinite, which means that you do not want the system to renegotiate SSL sessions",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "renegotiate-size",
            value_type: ValueKind::Integer,
            default: Some("indefinite, which specifies that you do not want a throughput size"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "renegotiation",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "retain-certificate",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("true"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "revoked-cert-status-response-control",
            value_type: ValueKind::Enum,
            enum_values: &["drop", "ignore", "mask"],
            default: Some("drop, which causes the connection to be dropped"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "secure-renegotiation",
            value_type: ValueKind::Enum,
            enum_values: &["request", "require", "require-strict"],
            default: Some("require-strict"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "server-name",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "session-mirroring",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "session-ticket",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sni-default",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sni-require",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-c3d",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-forward-proxy",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-forward-proxy-bypass",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-forward-proxy-verified-handshake",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-sign-hash",
            value_type: ValueKind::Enum,
            enum_values: &["any", "sha1", "sha256", "sha384"],
            default: Some("sha1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "strict-resume",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some(
                "disabled, which indicates that the SSL profile refuses to resume SSL sessions after an unclean shutdown",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "unclean-shutdown",
            value_type: ValueKind::Enum,
            required: true,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "unknown-cert-status-response-control",
            value_type: ValueKind::Enum,
            enum_values: &["drop", "ignore", "mask"],
            default: Some(
                "ignore, which causes the connection to ignore the error and continue handshake",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "untrusted-cert-response-control",
            value_type: ValueKind::Enum,
            enum_values: &["drop", "ignore", "mask"],
            default: Some("drop, which causes the connection to be dropped"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
