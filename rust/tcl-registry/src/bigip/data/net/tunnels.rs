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

//! BIG-IP object specs for `net tunnels`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_etherip",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels etherip"],
        },
        header_types: &[("net", "tunnels etherip")],
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
                references: &["net_tunnels_etherip"],
                default: Some("etherip"),
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
            kind: "net_tunnels_fec",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels fec"],
        },
        header_types: &[("net", "tunnels fec")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "decode-idle-timeout",
                value_type: ValueKind::Integer,
                default: Some("1500 milliseconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "decode-max-packets",
                value_type: ValueKind::Integer,
                default: Some("512"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "decode-queues",
                value_type: ValueKind::Integer,
                default: Some("32"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["net_tunnels_fec"],
                default: Some("fec"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encode-max-delay",
                value_type: ValueKind::Integer,
                default: Some("500 microseconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "keepalive-interval",
                value_type: ValueKind::Integer,
                default: Some("5 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lzo",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "repair-adaptive",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "repair-packets",
                value_type: ValueKind::Integer,
                default: Some("15"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-adaptive",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-packets",
                value_type: ValueKind::Integer,
                default: Some("15"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "udp-port",
                value_type: ValueKind::Integer,
                default: Some("8288"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_geneve",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels geneve"],
        },
        header_types: &[("net", "tunnels geneve")],
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
                references: &["net_tunnels_geneve"],
                default: Some("geneve"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flooding-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["multicast", "multipoint", "none"],
                default: Some("multipoint"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                default: Some("6081"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_gre",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels gre"],
        },
        header_types: &[("net", "tunnels gre")],
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
                references: &["net_tunnels_gre"],
                default: Some("gre"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encapsulation",
                value_type: ValueKind::Enum,
                enum_values: &["nvgre", "standard"],
                default: Some("standard"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flooding-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["multipoint", "none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rx-csum",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tx-csum",
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
            kind: "net_tunnels_ipip",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels ipip"],
        },
        header_types: &[("net", "tunnels ipip")],
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
                references: &["net_tunnels_ipip"],
                default: Some("ipip"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ds-lite",
                value_type: ValueKind::Unknown,
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proto",
                value_type: ValueKind::Enum,
                enum_values: &["IPv4", "IPv6"],
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("IPv4"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_ipsec",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels ipsec"],
        },
        header_types: &[("net", "tunnels ipsec")],
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
                references: &["net_tunnels_ipsec"],
                default: Some("ipsec"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-selector",
                value_type: ValueKind::Reference,
                references: &["net_ipsec_traffic_selector"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_lw4o6",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels lw4o6"],
        },
        header_types: &[("net", "tunnels lw4o6")],
        properties: &[
            BigipPropertySpec {
                name: "all-protocols-pass",
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
                name: "defaults-from",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_tunnels_lw4o6"],
                default: Some("lw4o6"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lwtbl-file",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "psid-length",
                value_type: ValueKind::Integer,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_map",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels map"],
        },
        header_types: &[("net", "tunnels map")],
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
                references: &["net_tunnels_map"],
                default: Some("map"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ea-bits-length",
                value_type: ValueKind::Integer,
                default: Some("32 (IPv4 prefix 24 bits + PSID 8 bits)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip4-prefix",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip6-prefix",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port-offset",
                value_type: ValueKind::Integer,
                default: Some("6"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_ppp",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels ppp"],
        },
        header_types: &[("net", "tunnels ppp")],
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
                references: &["net_tunnels_ppp"],
                default: Some("ppp"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lcp-echo-failure",
                value_type: ValueKind::Integer,
                default: Some("4"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lcp-echo-interval",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vj",
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
            kind: "net_tunnels_tcp_forward",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels tcp-forward"],
        },
        header_types: &[("net", "tunnels tcp-forward")],
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
                references: &["net_tunnels_tcp_forward"],
                default: Some("tcp-forward"),
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
            kind: "net_tunnels_tunnel",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels tunnel"],
        },
        header_types: &[("net", "tunnels tunnel")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-lasthop",
                value_type: ValueKind::Enum,
                enum_values: &["default", "disabled", "enabled"],
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
                default: Some("300 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "key",
                value_type: ValueKind::Integer,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["bidirectional", "inbound", "outbound"],
                default: Some("bidirectional"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mtu",
                value_type: ValueKind::Integer,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "profile",
                value_type: ValueKind::Reference,
                required: true,
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secondary-address",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tos",
                value_type: ValueKind::Integer,
                default: Some("preserve"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-group",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["cm_traffic_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transparent",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-pmtu",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_v6rd",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels v6rd"],
        },
        header_types: &[("net", "tunnels v6rd")],
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
                references: &["net_tunnels_v6rd"],
                default: Some("v6rd"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv4prefix",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipv4prefixlen",
                value_type: ValueKind::Integer,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "v6rdprefix",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "v6rdprefixlen",
                value_type: ValueKind::Integer,
                default: Some("56"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_vxlan",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels vxlan"],
        },
        header_types: &[("net", "tunnels vxlan")],
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
                references: &["net_tunnels_vxlan"],
                default: Some("vxlan"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encapsulation-type",
                value_type: ValueKind::Enum,
                enum_values: &["vxlan", "vxlan-gpe"],
                default: Some("vxlan"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "flooding-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["multicast", "multipoint", "none", "replicator"],
                default: Some("multicast"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                default: Some("4789"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_tunnels_wccp",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["tunnels wccp"],
        },
        header_types: &[("net", "tunnels wccp")],
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
                references: &["net_tunnels_wccp"],
                default: Some("wccpgre"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rx-csum",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tx-csum",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "wccp-version",
                value_type: ValueKind::Enum,
                enum_values: &["1", "2"],
                default: Some("2"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
