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

//! BIG-IP object specs for `sys crypto`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
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
];
