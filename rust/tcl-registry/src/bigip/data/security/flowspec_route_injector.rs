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

//! BIG-IP object specs for `security flowspec-route-injector`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "security_flowspec_route_injector_profile",
        table_name: None,
        resolver_name: None,
        module: Some("security"),
        object_types: &["flowspec-route-injector profile"],
    },
    header_types: &[("security", "flowspec-route-injector profile")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-flowspec-routes-limit",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "neighbor",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "adj-out",
                    value_type: ValueKind::Enum,
                    in_sections: &["neighbor"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-multiple-instance",
                    value_type: ValueKind::Enum,
                    in_sections: &["neighbor"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "extended-asn-cap",
                    value_type: ValueKind::Enum,
                    in_sections: &["neighbor"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "graceful-restart",
                    value_type: ValueKind::Enum,
                    in_sections: &["neighbor"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "graceful-restart-time",
                    value_type: ValueKind::Integer,
                    in_sections: &["neighbor"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "hold-time",
                    value_type: ValueKind::Integer,
                    in_sections: &["neighbor"],
                    default: Some("90 seconds"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "local-address",
                    value_type: ValueKind::String,
                    in_sections: &["neighbor"],
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "local-as",
                    value_type: ValueKind::Integer,
                    in_sections: &["neighbor"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "remote-as",
                    value_type: ValueKind::Integer,
                    in_sections: &["neighbor"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "router-id",
                    value_type: ValueKind::String,
                    in_sections: &["neighbor"],
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "adj-out",
            value_type: ValueKind::Enum,
            in_sections: &["neighbor"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-multiple-instance",
            value_type: ValueKind::Enum,
            in_sections: &["neighbor"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "extended-asn-cap",
            value_type: ValueKind::Enum,
            in_sections: &["neighbor"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "graceful-restart",
            value_type: ValueKind::Enum,
            in_sections: &["neighbor"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "graceful-restart-time",
            value_type: ValueKind::Integer,
            in_sections: &["neighbor"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hold-time",
            value_type: ValueKind::Integer,
            in_sections: &["neighbor"],
            default: Some("90 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "local-address",
            value_type: ValueKind::String,
            in_sections: &["neighbor"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "local-as",
            value_type: ValueKind::Integer,
            in_sections: &["neighbor"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "remote-as",
            value_type: ValueKind::Integer,
            in_sections: &["neighbor"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "router-id",
            value_type: ValueKind::String,
            in_sections: &["neighbor"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "peer-group",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "adj-out",
                    value_type: ValueKind::Enum,
                    in_sections: &["peer-group"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-multiple-instance",
                    value_type: ValueKind::Enum,
                    in_sections: &["peer-group"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "extended-asn-cap",
                    value_type: ValueKind::Enum,
                    in_sections: &["peer-group"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "graceful-restart",
                    value_type: ValueKind::Enum,
                    in_sections: &["peer-group"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "graceful-restart-time",
                    value_type: ValueKind::Integer,
                    in_sections: &["peer-group"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "hold-time",
                    value_type: ValueKind::Integer,
                    in_sections: &["peer-group"],
                    default: Some("90 seconds"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "local-address",
                    value_type: ValueKind::String,
                    in_sections: &["peer-group"],
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "local-as",
                    value_type: ValueKind::Integer,
                    in_sections: &["peer-group"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "remote-as",
                    value_type: ValueKind::Integer,
                    in_sections: &["peer-group"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "router-id",
                    value_type: ValueKind::String,
                    in_sections: &["peer-group"],
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "adj-out",
            value_type: ValueKind::Enum,
            in_sections: &["peer-group"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-multiple-instance",
            value_type: ValueKind::Enum,
            in_sections: &["peer-group"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "extended-asn-cap",
            value_type: ValueKind::Enum,
            in_sections: &["peer-group"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "graceful-restart",
            value_type: ValueKind::Enum,
            in_sections: &["peer-group"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "graceful-restart-time",
            value_type: ValueKind::Integer,
            in_sections: &["peer-group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hold-time",
            value_type: ValueKind::Integer,
            in_sections: &["peer-group"],
            default: Some("90 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "local-address",
            value_type: ValueKind::String,
            in_sections: &["peer-group"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "local-as",
            value_type: ValueKind::Integer,
            in_sections: &["peer-group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "remote-as",
            value_type: ValueKind::Integer,
            in_sections: &["peer-group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "router-id",
            value_type: ValueKind::String,
            in_sections: &["peer-group"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-domain",
            value_type: ValueKind::Reference,
            required: true,
            references: &["net_route_domain"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rules",
            value_type: ValueKind::List,
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
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "action",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "advertisement-ttl-from-now",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules"],
                    default: Some("5 minutes"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "alias",
                    value_type: ValueKind::String,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::String,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "match",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "remove-config-upon-expiry",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    default: Some("true"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "action",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "asn-community",
                    value_type: ValueKind::String,
                    in_sections: &["rules", "action"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dscp-value",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "action"],
                    default: Some("0"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "next-hop",
                    value_type: ValueKind::String,
                    in_sections: &["rules", "action"],
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "rate-limit",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "action"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "type",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules", "action"],
                    enum_values: &["drop", "qos", "rate-limit", "redirect"],
                    default: Some("redirect"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "asn-community",
            value_type: ValueKind::String,
            in_sections: &["rules", "action"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dscp-value",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "action"],
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "next-hop",
            value_type: ValueKind::String,
            in_sections: &["rules", "action"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rate-limit",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "action"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Enum,
            in_sections: &["rules", "action"],
            enum_values: &["drop", "qos", "rate-limit", "redirect"],
            default: Some("redirect"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "advertisement-ttl-from-now",
            value_type: ValueKind::Integer,
            in_sections: &["rules"],
            default: Some("5 minutes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "alias",
            value_type: ValueKind::String,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "match",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "destination-address",
                    value_type: ValueKind::String,
                    in_sections: &["rules", "match"],
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "destination-ports",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "match"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dscp-values",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "match"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "icmp-codes",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "match"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "icmp-types",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "match"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ip-fragments",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "match"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ip-protocols",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "match"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "packet-lengths",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "match"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ports",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "match"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "source-address",
                    value_type: ValueKind::String,
                    in_sections: &["rules", "match"],
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "source-ports",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules", "match"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "tcp-flags",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "match"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "destination-address",
            value_type: ValueKind::String,
            in_sections: &["rules", "match"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "destination-ports",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "match"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dscp-values",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "match"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "icmp-codes",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "match"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "icmp-types",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "match"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-fragments",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "match"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-protocols",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "match"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "packet-lengths",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "match"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ports",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "match"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source-address",
            value_type: ValueKind::String,
            in_sections: &["rules", "match"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source-ports",
            value_type: ValueKind::Unknown,
            in_sections: &["rules", "match"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-flags",
            value_type: ValueKind::List,
            in_sections: &["rules", "match"],
            block: &[
                BigipPropertySpec {
                    name: "bitwise-and-fields",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "match", "tcp-flags"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bitwise-or-fields",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules", "match", "tcp-flags"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bitwise-and-fields",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "match", "tcp-flags"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bitwise-or-fields",
            value_type: ValueKind::Integer,
            in_sections: &["rules", "match", "tcp-flags"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "remove-config-upon-expiry",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            default: Some("true"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "security-log-profile",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
