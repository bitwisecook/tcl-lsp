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

//! BIG-IP object specs for `gtm wideip`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_a",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip a"],
        },
        header_types: &[("gtm", "wideip a")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_a"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_a"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools-cname",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_aaaa",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip aaaa"],
        },
        header_types: &[("gtm", "wideip aaaa")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_aaaa"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_aaaa"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools-cname",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_cname",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip cname"],
        },
        header_types: &[("gtm", "wideip cname")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_cname"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_cname"],
                default: Some("none"),
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_https",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip https"],
        },
        header_types: &[("gtm", "wideip https")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_https"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_https"],
                default: Some("none"),
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_mx",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip mx"],
        },
        header_types: &[("gtm", "wideip mx")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_mx"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_mx"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools-cname",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_naptr",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip naptr"],
        },
        header_types: &[("gtm", "wideip naptr")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_naptr"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_naptr"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools-cname",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_srv",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip srv"],
        },
        header_types: &[("gtm", "wideip srv")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_srv"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_srv"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools-cname",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_wideip_svcb",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["wideip svcb"],
        },
        header_types: &[("gtm", "wideip svcb")],
        properties: &[
            BigipPropertySpec {
                name: "aliases",
                value_type: ValueKind::Reference,
                repeated: true,
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-resort-pool",
                value_type: ValueKind::Reference,
                references: &["gtm_pool_svcb"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load-balancing-decision-log-verbosity",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "pool-member-selection",
                    "pool-member-traversal",
                    "pool-selection",
                    "pool-traversal",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "minimal-response",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-lb-mode",
                value_type: ValueKind::Enum,
                enum_values: &["ratio", "round-robin", "topology"],
                default: Some("round-robin"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pools",
                value_type: ValueKind::Unknown,
                allow_none: true,
                references: &["gtm_pool_svcb"],
                default: Some("none"),
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
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl-persistence",
                value_type: ValueKind::Integer,
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
