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

//! BIG-IP object specs for `ltm message-routing generic`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_message_routing_generic_peer",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["message-routing generic peer"],
        },
        header_types: &[("ltm", "message-routing generic peer")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-initialization",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-initialization-interval",
                value_type: ValueKind::Integer,
                default: Some("5000ms"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "connection-mode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "per-blade",
                    "per-client",
                    "per-client-alternate-tmm",
                    "per-client-per-blade",
                    "per-client-per-tmm",
                    "per-peer",
                    "per-peer-alternate-tmm",
                    "per-tmm",
                ],
                default: Some("per-peer"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "number-connections",
                value_type: ValueKind::Integer,
                default: Some("1"),
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
                name: "ratio",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transport-config",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_message_routing_generic_protocol",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["message-routing generic protocol"],
        },
        header_types: &[("ltm", "message-routing generic protocol")],
        properties: &[
            BigipPropertySpec {
                name: "cur-pending-req-sweeper-interval",
                value_type: ValueKind::Integer,
                default: Some("60000ms"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["ltm_message_routing_generic_protocol"],
                default: Some("genericmsg"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "disable-parser",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-egress-buffer",
                value_type: ValueKind::Integer,
                default: Some("32768"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-message-size",
                value_type: ValueKind::Integer,
                default: Some("32768"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "message-terminator",
                value_type: ValueKind::String,
                default: Some("en"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-response",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transaction-timeout",
                value_type: ValueKind::Integer,
                default: Some("10seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_message_routing_generic_route",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["message-routing generic route"],
        },
        header_types: &[("ltm", "message-routing generic route")],
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
                name: "destination-address",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peer-selection-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "sequential"],
                default: Some("sequential"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peers",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-address",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_message_routing_generic_router",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["message-routing generic router"],
        },
        header_types: &[("ltm", "message-routing generic router")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["ltm_message_routing_generic_router"],
                default: Some("messagerouter"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ha-message-sweeper-interval",
                value_type: ValueKind::Integer,
                default: Some("1000 milliseconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-client-port",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "irule-scope-message",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-pending-bytes",
                value_type: ValueKind::Integer,
                default: Some("32768"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-pending-messages",
                value_type: ValueKind::Integer,
                default: Some("64"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-retries",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mirrored",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "per-peer-stats",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "routes",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "replace-all-with"],
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
                name: "use-local-connection",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_message_routing_generic_transport_config",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["message-routing generic transport-config"],
        },
        header_types: &[("ltm", "message-routing generic transport-config")],
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
                name: "profiles",
                value_type: ValueKind::List,
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "context",
                    value_type: ValueKind::Enum,
                    in_sections: &["profiles"],
                    enum_values: &["all", "clientside", "serverside"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "context",
                value_type: ValueKind::Enum,
                in_sections: &["profiles"],
                enum_values: &["all", "clientside", "serverside"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::List,
                repeated: true,
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-address-translation",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "pool",
                        value_type: ValueKind::Reference,
                        in_sections: &["source-address-translation"],
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
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "type",
                        value_type: ValueKind::Enum,
                        in_sections: &["source-address-translation"],
                        allow_none: true,
                        enum_values: &["automap", "none", "snat"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                in_sections: &["source-address-translation"],
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["source-address-translation"],
                allow_none: true,
                enum_values: &["automap", "none", "snat"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-port",
                value_type: ValueKind::Integer,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-port-mode",
                value_type: ValueKind::Enum,
                enum_values: &["change", "preserve", "preserve-strict"],
                default: Some("change"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
