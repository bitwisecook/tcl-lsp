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

//! BIG-IP object specs for `ltm lsn-pool`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_lsn_pool",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["lsn-pool"],
    },
    header_types: &[("ltm", "lsn-pool")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "backup-members",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-connection-limit",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "egress-interfaces",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "egress-interfaces-disabled",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "egress-interfaces-enabled",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hairpin-mode",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "icmp-echo",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "inbound-connections",
            value_type: ValueKind::Enum,
            enum_values: &["automatic", "disabled", "explicit"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-profile",
            value_type: ValueKind::Reference,
            allow_none: true,
            enum_values: &["none"],
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
            name: "members",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mode",
            value_type: ValueKind::Enum,
            enum_values: &["deterministic", "napt", "pba"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pcp",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "profile",
                    value_type: ValueKind::Reference,
                    in_sections: &["pcp"],
                    allow_none: true,
                    enum_values: &["none"],
                    references: &[
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
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "selfip",
                    value_type: ValueKind::Reference,
                    in_sections: &["pcp"],
                    allow_none: true,
                    enum_values: &["none"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "profile",
            value_type: ValueKind::Reference,
            in_sections: &["pcp"],
            allow_none: true,
            enum_values: &["none"],
            references: &[
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
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "selfip",
            value_type: ValueKind::Reference,
            in_sections: &["pcp"],
            allow_none: true,
            enum_values: &["none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "persistence",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["persistence"],
                    allow_none: true,
                    enum_values: &["none"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "timeout",
                    value_type: ValueKind::Integer,
                    in_sections: &["persistence"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mode",
            value_type: ValueKind::Enum,
            in_sections: &["persistence"],
            allow_none: true,
            enum_values: &["none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "timeout",
            value_type: ValueKind::Integer,
            in_sections: &["persistence"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port-block-allocation",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "block-idle-timeout",
                    value_type: ValueKind::Integer,
                    in_sections: &["port-block-allocation"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "block-lifetime",
                    value_type: ValueKind::Integer,
                    in_sections: &["port-block-allocation"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "block-size",
                    value_type: ValueKind::Integer,
                    in_sections: &["port-block-allocation"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "client-block-limit",
                    value_type: ValueKind::Integer,
                    in_sections: &["port-block-allocation"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "zombie-timeout",
                    value_type: ValueKind::Integer,
                    in_sections: &["port-block-allocation"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "block-idle-timeout",
            value_type: ValueKind::Integer,
            in_sections: &["port-block-allocation"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "block-lifetime",
            value_type: ValueKind::Integer,
            in_sections: &["port-block-allocation"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "block-size",
            value_type: ValueKind::Integer,
            in_sections: &["port-block-allocation"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-block-limit",
            value_type: ValueKind::Integer,
            in_sections: &["port-block-allocation"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "zombie-timeout",
            value_type: ValueKind::Integer,
            in_sections: &["port-block-allocation"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-advertisement",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "translation-port-range",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
