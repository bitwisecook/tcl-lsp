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

//! BIG-IP object specs for `ltm virtual`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_virtual",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["virtual"],
    },
    header_types: &[("ltm", "virtual")],
    properties: &[
        BigipPropertySpec {
            name: "address-status",
            value_type: ValueKind::Enum,
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
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
            name: "auth",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auto-discovery",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auto-lasthop",
            value_type: ValueKind::Enum,
            enum_values: &["default", "disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "clone-pools",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            block: &[BigipPropertySpec {
                name: "context",
                value_type: ValueKind::Enum,
                in_sections: &["clone-pools"],
                enum_values: &["clientside", "serverside"],
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "context",
            value_type: ValueKind::Enum,
            in_sections: &["clone-pools"],
            enum_values: &["clientside", "serverside"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cmp-enabled",
            value_type: ValueKind::Enum,
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("yes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "connection-limit",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "destination",
            address_grammar: Some(crate::bigip::AddressGrammar::Destination),
            value_type: ValueKind::Reference,
            references: &["ltm_virtual_address"],
            default: Some("any:any"),
            address_validation: crate::bigip::AddressValidation::RouteDomain21_1,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dhcp-relay",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "eviction-protected",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fallback-persistence",
            value_type: ValueKind::Reference,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "flow-eviction-policy",
            value_type: ValueKind::Reference,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fw-context-stat",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fw-enforced-policy",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_policy", "ltm_policy_strategy"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fw-enforced-policy-rules",
            value_type: ValueKind::List,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fw-staged-policy",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_policy", "ltm_policy_strategy"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fw-staged-policy-rules",
            value_type: ValueKind::List,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "gtm-score",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "internal",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-forward",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-intelligence-categories",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-protocol",
            value_type: ValueKind::Unknown,
            default: Some("any"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "l2-forward",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "last-hop-pool",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_pool"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mask",
            value_type: ValueKind::Unknown,
            required: true,
            shape_kind: Some(ValueKind::Object),
            default: Some("255"),
            address_grammar: Some(crate::bigip::AddressGrammar::Mask),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "metadata",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify"],
            block: &[
                BigipPropertySpec {
                    name: "persist",
                    value_type: ValueKind::Enum,
                    in_sections: &["metadata"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "value",
                    value_type: ValueKind::String,
                    in_sections: &["metadata"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "persist",
            value_type: ValueKind::Enum,
            in_sections: &["metadata"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value",
            value_type: ValueKind::String,
            in_sections: &["metadata"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mirror",
            value_type: ValueKind::Unknown,
            allow_none: true,
            enum_values: &["disabled", "enabled", "none"],
            shape_kind: Some(ValueKind::Object),
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "nat64",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "per-flow-request-access-policy",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_policy", "ltm_policy_strategy"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "persist",
            value_type: ValueKind::List,
            allow_none: true,
            default: Some("none"),
            list_operators: &["replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "policies",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pool",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_pool"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port-misuse",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "profiles",
            value_type: ValueKind::List,
            allow_none: true,
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
            default: Some("fastL4"),
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
            name: "rate-class",
            value_type: ValueKind::Reference,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rate-limit",
            value_type: ValueKind::Integer,
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rate-limit-dst",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rate-limit-mode",
            value_type: ValueKind::Enum,
            enum_values: &[
                "destination",
                "object",
                "object-destination",
                "object-source",
                "object-source-destination",
                "source",
                "source-destination",
            ],
            default: Some(
                "object', which does not use the source or destination address as part of the key",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rate-limit-src",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "reject",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "related-rules",
            value_type: ValueKind::List,
            repeated: true,
            allow_none: true,
            references: &["ltm_rule", "ltm_rule_profiler"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rules",
            value_type: ValueKind::List,
            repeated: true,
            references: &["ltm_rule", "ltm_rule_profiler"],
            default: Some("none"),
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "security-nat-policy",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "policy",
                    value_type: ValueKind::Reference,
                    in_sections: &["security-nat-policy"],
                    allow_none: true,
                    references: &["ltm_policy", "ltm_policy_strategy"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "use-device-policy",
                    value_type: ValueKind::Enum,
                    in_sections: &["security-nat-policy"],
                    enum_values: &["no", "yes"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "use-route-domain-policy",
                    value_type: ValueKind::Enum,
                    in_sections: &["security-nat-policy"],
                    enum_values: &["no", "yes"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "policy",
            value_type: ValueKind::Reference,
            in_sections: &["security-nat-policy"],
            allow_none: true,
            references: &["security_nat_policy", "ltm_policy", "ltm_policy_strategy"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "use-device-policy",
            value_type: ValueKind::Enum,
            in_sections: &["security-nat-policy"],
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "use-route-domain-policy",
            value_type: ValueKind::Enum,
            in_sections: &["security-nat-policy"],
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "security-nat-rules",
            value_type: ValueKind::List,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "serverssl-use-sni",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "service-down-immediate-action",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["drop", "none", "reset"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "service-policy",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_policy", "ltm_policy_strategy"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "snat",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["automap", "none"],
            default: Some("none"),
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "snatpool",
            value_type: ValueKind::Reference,
            references: &["ltm_snatpool"],
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            address_grammar: Some(crate::bigip::AddressGrammar::Network),
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
                    references: &["ltm_pool"],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "type",
                    value_type: ValueKind::Enum,
                    in_sections: &["source-address-translation"],
                    allow_none: true,
                    enum_values: &["automap", "lsn", "none", "snat"],
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
            references: &["ltm_snatpool", "ltm_pool"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Enum,
            in_sections: &["source-address-translation"],
            allow_none: true,
            enum_values: &["automap", "lsn", "none", "snat"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source-port",
            value_type: ValueKind::Enum,
            enum_values: &["change", "preserve", "preserve-strict"],
            default: Some("preserve"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "traffic-classes",
            value_type: ValueKind::List,
            allow_none: true,
            default: Some("none"),
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "translate-address",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "translate-port",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "transparent-nexthop",
            value_type: ValueKind::Reference,
            references: &[
                "net_fdb_vlan",
                "net_vlan",
                "net_vlan_allowed",
                "net_vlan_group",
                "sys_sflow_data_source_vlan",
                "sys_sflow_global_settings_vlan",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "vlans",
            value_type: ValueKind::List,
            allow_none: true,
            default: Some("none"),
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "vlans-disabled",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "vlans-enabled",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
