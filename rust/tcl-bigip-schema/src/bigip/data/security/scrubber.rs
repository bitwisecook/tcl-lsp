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

//! BIG-IP object specs for `security scrubber`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "security_scrubber_profile",
        table_name: None,
        resolver_name: None,
        module: Some("security"),
        object_types: &["scrubber profile"],
    },
    header_types: &[("security", "scrubber profile")],
    properties: &[
        BigipPropertySpec {
            name: "advertisement-ttl",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "recursive",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scrubber-categories",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "advertisement-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["scrubber-categories"],
                    allow_none: true,
                    enum_values: &[
                        "bgp-flowspec-method",
                        "bgp-method",
                        "none-method",
                        "silverline-method",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-categories"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-advertisement-action",
                    value_type: ValueKind::Enum,
                    in_sections: &["scrubber-categories"],
                    enum_values: &["drop", "qos", "rate-limit", "redirect"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-dscp-value",
                    value_type: ValueKind::Integer,
                    in_sections: &["scrubber-categories"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-rate-limit",
                    value_type: ValueKind::Integer,
                    in_sections: &["scrubber-categories"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-redirect-asn-community",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-categories"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "blacklist-category",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-categories"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "next-hop",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-categories"],
                    shape_kind: Some(ValueKind::IpAddress),
                    address_grammar: Some(crate::bigip::AddressGrammar::Address),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "next-hop-v6",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-categories"],
                    shape_kind: Some(ValueKind::IpAddress),
                    address_grammar: Some(crate::bigip::AddressGrammar::Address),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "route-domain-name",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-categories"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "advertisement-method",
            value_type: ValueKind::Enum,
            in_sections: &["scrubber-categories"],
            allow_none: true,
            enum_values: &[
                "bgp-flowspec-method",
                "bgp-method",
                "none-method",
                "silverline-method",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["scrubber-categories"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-advertisement-action",
            value_type: ValueKind::Enum,
            in_sections: &["scrubber-categories"],
            enum_values: &["drop", "qos", "rate-limit", "redirect"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-dscp-value",
            value_type: ValueKind::Integer,
            in_sections: &["scrubber-categories"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-rate-limit",
            value_type: ValueKind::Integer,
            in_sections: &["scrubber-categories"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-redirect-asn-community",
            value_type: ValueKind::String,
            in_sections: &["scrubber-categories"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "blacklist-category",
            value_type: ValueKind::String,
            in_sections: &["scrubber-categories"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "next-hop",
            value_type: ValueKind::String,
            in_sections: &["scrubber-categories"],
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "next-hop-v6",
            value_type: ValueKind::String,
            in_sections: &["scrubber-categories"],
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-domain-name",
            value_type: ValueKind::String,
            in_sections: &["scrubber-categories"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scrubber-netflow-protected-server",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "advertisement-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["scrubber-netflow-protected-server"],
                    allow_none: true,
                    enum_values: &[
                        "bgp-flowspec-method",
                        "bgp-method",
                        "none-method",
                        "silverline-method",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-netflow-protected-server"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-advertisement-action",
                    value_type: ValueKind::Enum,
                    in_sections: &["scrubber-netflow-protected-server"],
                    enum_values: &["drop", "qos", "rate-limit", "redirect"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-dscp-value",
                    value_type: ValueKind::Integer,
                    in_sections: &["scrubber-netflow-protected-server"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-rate-limit",
                    value_type: ValueKind::Integer,
                    in_sections: &["scrubber-netflow-protected-server"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-redirect-asn-community",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-netflow-protected-server"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "blacklist-category",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-netflow-protected-server"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "next-hop",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-netflow-protected-server"],
                    shape_kind: Some(ValueKind::IpAddress),
                    address_grammar: Some(crate::bigip::AddressGrammar::Address),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "next-hop-v6",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-netflow-protected-server"],
                    shape_kind: Some(ValueKind::IpAddress),
                    address_grammar: Some(crate::bigip::AddressGrammar::Address),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "route-domain-name",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-netflow-protected-server"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "advertisement-method",
            value_type: ValueKind::Enum,
            in_sections: &["scrubber-netflow-protected-server"],
            allow_none: true,
            enum_values: &[
                "bgp-flowspec-method",
                "bgp-method",
                "none-method",
                "silverline-method",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["scrubber-netflow-protected-server"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-advertisement-action",
            value_type: ValueKind::Enum,
            in_sections: &["scrubber-netflow-protected-server"],
            enum_values: &["drop", "qos", "rate-limit", "redirect"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-dscp-value",
            value_type: ValueKind::Integer,
            in_sections: &["scrubber-netflow-protected-server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-rate-limit",
            value_type: ValueKind::Integer,
            in_sections: &["scrubber-netflow-protected-server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-redirect-asn-community",
            value_type: ValueKind::String,
            in_sections: &["scrubber-netflow-protected-server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "blacklist-category",
            value_type: ValueKind::String,
            in_sections: &["scrubber-netflow-protected-server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "next-hop",
            value_type: ValueKind::String,
            in_sections: &["scrubber-netflow-protected-server"],
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "next-hop-v6",
            value_type: ValueKind::String,
            in_sections: &["scrubber-netflow-protected-server"],
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-domain-name",
            value_type: ValueKind::String,
            in_sections: &["scrubber-netflow-protected-server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scrubber-rt-domain",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "absolute-threshold",
                    value_type: ValueKind::Integer,
                    in_sections: &["scrubber-rt-domain"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "advertisement-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["scrubber-rt-domain"],
                    allow_none: true,
                    enum_values: &[
                        "bgp-flowspec-method",
                        "bgp-method",
                        "none-method",
                        "silverline-method",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-advertisement-action",
                    value_type: ValueKind::Enum,
                    in_sections: &["scrubber-rt-domain"],
                    enum_values: &["drop", "qos", "rate-limit", "redirect"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-dscp-value",
                    value_type: ValueKind::Integer,
                    in_sections: &["scrubber-rt-domain"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-rate-limit",
                    value_type: ValueKind::Integer,
                    in_sections: &["scrubber-rt-domain"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-redirect-asn-community",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-rt-domain"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "excluded-vlans",
                    value_type: ValueKind::List,
                    in_sections: &["scrubber-rt-domain"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "next-hop",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-rt-domain"],
                    shape_kind: Some(ValueKind::IpAddress),
                    address_grammar: Some(crate::bigip::AddressGrammar::Address),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "next-hop-v6",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-rt-domain"],
                    shape_kind: Some(ValueKind::IpAddress),
                    address_grammar: Some(crate::bigip::AddressGrammar::Address),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "percentage-threshold",
                    value_type: ValueKind::Integer,
                    in_sections: &["scrubber-rt-domain"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "route-domain",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-rt-domain"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scrubber-rd-network-prefix",
                    value_type: ValueKind::List,
                    in_sections: &["scrubber-rt-domain"],
                    allow_none: true,
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "absolute-threshold",
            value_type: ValueKind::Integer,
            in_sections: &["scrubber-rt-domain"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "advertisement-method",
            value_type: ValueKind::Enum,
            in_sections: &["scrubber-rt-domain"],
            allow_none: true,
            enum_values: &[
                "bgp-flowspec-method",
                "bgp-method",
                "none-method",
                "silverline-method",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-advertisement-action",
            value_type: ValueKind::Enum,
            in_sections: &["scrubber-rt-domain"],
            enum_values: &["drop", "qos", "rate-limit", "redirect"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-dscp-value",
            value_type: ValueKind::Integer,
            in_sections: &["scrubber-rt-domain"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-rate-limit",
            value_type: ValueKind::Integer,
            in_sections: &["scrubber-rt-domain"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-redirect-asn-community",
            value_type: ValueKind::String,
            in_sections: &["scrubber-rt-domain"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "excluded-vlans",
            value_type: ValueKind::List,
            in_sections: &["scrubber-rt-domain"],
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "next-hop",
            value_type: ValueKind::String,
            in_sections: &["scrubber-rt-domain"],
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "next-hop-v6",
            value_type: ValueKind::String,
            in_sections: &["scrubber-rt-domain"],
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "percentage-threshold",
            value_type: ValueKind::Integer,
            in_sections: &["scrubber-rt-domain"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-domain",
            value_type: ValueKind::String,
            in_sections: &["scrubber-rt-domain"],
            references: &["net_route_domain"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scrubber-rd-network-prefix",
            value_type: ValueKind::List,
            in_sections: &["scrubber-rt-domain"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-advertisement-action",
                    value_type: ValueKind::Enum,
                    in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
                    enum_values: &["drop", "qos", "rate-limit", "redirect"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-dscp-value",
                    value_type: ValueKind::Integer,
                    in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-rate-limit",
                    value_type: ValueKind::Integer,
                    in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-redirect-asn-community",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dst-ip",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
                    shape_kind: Some(ValueKind::IpAddress),
                    address_grammar: Some(crate::bigip::AddressGrammar::Network),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "mask",
                    value_type: ValueKind::Integer,
                    in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "next-hop",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
                    shape_kind: Some(ValueKind::IpAddress),
                    address_grammar: Some(crate::bigip::AddressGrammar::Address),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-advertisement-action",
            value_type: ValueKind::Enum,
            in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
            enum_values: &["drop", "qos", "rate-limit", "redirect"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-dscp-value",
            value_type: ValueKind::Integer,
            in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-rate-limit",
            value_type: ValueKind::Integer,
            in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-redirect-asn-community",
            value_type: ValueKind::String,
            in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dst-ip",
            value_type: ValueKind::String,
            in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Network),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mask",
            value_type: ValueKind::Integer,
            in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "next-hop",
            value_type: ValueKind::String,
            in_sections: &["scrubber-rt-domain", "scrubber-rd-network-prefix"],
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scrubber-virtual-server",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "absolute-threshold",
                    value_type: ValueKind::Integer,
                    in_sections: &["scrubber-virtual-server"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "advertisement-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["scrubber-virtual-server"],
                    allow_none: true,
                    enum_values: &[
                        "bgp-flowspec-method",
                        "bgp-method",
                        "none-method",
                        "silverline-method",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-virtual-server"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-advertisement-action",
                    value_type: ValueKind::Enum,
                    in_sections: &["scrubber-virtual-server"],
                    enum_values: &["drop", "qos", "rate-limit", "redirect"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-dscp-value",
                    value_type: ValueKind::Integer,
                    in_sections: &["scrubber-virtual-server"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-rate-limit",
                    value_type: ValueKind::Integer,
                    in_sections: &["scrubber-virtual-server"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bgp-flowspec-redirect-asn-community",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-virtual-server"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "next-hop",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-virtual-server"],
                    shape_kind: Some(ValueKind::IpAddress),
                    address_grammar: Some(crate::bigip::AddressGrammar::Address),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "next-hop-v6",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-virtual-server"],
                    shape_kind: Some(ValueKind::IpAddress),
                    address_grammar: Some(crate::bigip::AddressGrammar::Address),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "percentage-threshold",
                    value_type: ValueKind::Integer,
                    in_sections: &["scrubber-virtual-server"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "vs-name",
                    value_type: ValueKind::String,
                    in_sections: &["scrubber-virtual-server"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "absolute-threshold",
            value_type: ValueKind::Integer,
            in_sections: &["scrubber-virtual-server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "advertisement-method",
            value_type: ValueKind::Enum,
            in_sections: &["scrubber-virtual-server"],
            allow_none: true,
            enum_values: &[
                "bgp-flowspec-method",
                "bgp-method",
                "none-method",
                "silverline-method",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["scrubber-virtual-server"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-advertisement-action",
            value_type: ValueKind::Enum,
            in_sections: &["scrubber-virtual-server"],
            enum_values: &["drop", "qos", "rate-limit", "redirect"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-dscp-value",
            value_type: ValueKind::Integer,
            in_sections: &["scrubber-virtual-server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-rate-limit",
            value_type: ValueKind::Integer,
            in_sections: &["scrubber-virtual-server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bgp-flowspec-redirect-asn-community",
            value_type: ValueKind::String,
            in_sections: &["scrubber-virtual-server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "next-hop",
            value_type: ValueKind::String,
            in_sections: &["scrubber-virtual-server"],
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "next-hop-v6",
            value_type: ValueKind::String,
            in_sections: &["scrubber-virtual-server"],
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "percentage-threshold",
            value_type: ValueKind::Integer,
            in_sections: &["scrubber-virtual-server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "vs-name",
            value_type: ValueKind::String,
            in_sections: &["scrubber-virtual-server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "silverline",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
