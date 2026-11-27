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

//! BIG-IP object specs for `ltm profile analytics`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_analytics",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile analytics"],
    },
    header_types: &[("ltm", "profile analytics")],
    properties: &[
        BigipPropertySpec {
            name: "alerts",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["alerts"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "granularity",
                    value_type: ValueKind::Enum,
                    in_sections: &["alerts"],
                    enum_values: &["application", "pool-member", "virtual-server"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "metric",
                    value_type: ValueKind::Enum,
                    in_sections: &["alerts"],
                    enum_values: &[
                        "average-page-load-time",
                        "average-request-throughput",
                        "average-response-throughput",
                        "average-server-latency",
                        "average-tps",
                        "max-page-load-time",
                        "max-request-throughput",
                        "max-response-throughput",
                        "max-server-latency",
                        "max-tps",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "sample-period",
                    value_type: ValueKind::Integer,
                    in_sections: &["alerts"],
                    default: Some("300"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "threshold",
                    value_type: ValueKind::Integer,
                    in_sections: &["alerts"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "threshold-relation",
                    value_type: ValueKind::Enum,
                    in_sections: &["alerts"],
                    enum_values: &["above", "below"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["alerts"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "granularity",
            value_type: ValueKind::Enum,
            in_sections: &["alerts"],
            enum_values: &["application", "pool-member", "virtual-server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "metric",
            value_type: ValueKind::Enum,
            in_sections: &["alerts"],
            enum_values: &[
                "average-page-load-time",
                "average-request-throughput",
                "average-response-throughput",
                "average-server-latency",
                "average-tps",
                "max-page-load-time",
                "max-request-throughput",
                "max-response-throughput",
                "max-server-latency",
                "max-tps",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sample-period",
            value_type: ValueKind::Integer,
            in_sections: &["alerts"],
            default: Some("300"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "threshold",
            value_type: ValueKind::Integer,
            in_sections: &["alerts"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "threshold-relation",
            value_type: ValueKind::Enum,
            in_sections: &["alerts"],
            enum_values: &["above", "below"],
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
            name: "captured-traffic-external-logging",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "captured-traffic-internal-logging",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-geo",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-http-throughput",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-http-timing-metrics",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-ip",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-max-tps-and-throughput",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-methods",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-page-load-time",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-response-codes",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-server-latency",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-subnets",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-url",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-user-agent",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-user-sessions",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collected-stats-external-logging",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collected-stats-internal-logging",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "countries-for-stat-collection",
            value_type: ValueKind::List,
            list_operators: &["add", "delete"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::String,
            allow_none: true,
            references: &["ltm_profile_analytics"],
            default: Some("analytics"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "external-logging-publisher",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ips-for-stat-collection",
            value_type: ValueKind::List,
            list_operators: &["add", "delete"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "notification-by-email",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "notification-by-snmp",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "notification-by-syslog",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "notification-email-addresses",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "publish-irule-statistics",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sampling",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "session-cookie-security",
            value_type: ValueKind::Enum,
            enum_values: &["always-secure", "never-secure", "ssl-only"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "session-timeout-minutes",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "smtp-config",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "subnet-masks",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[BigipPropertySpec {
                name: "subnet",
                value_type: ValueKind::String,
                in_sections: &["subnet-masks"],
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Network),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "subnet",
            value_type: ValueKind::String,
            in_sections: &["subnet-masks"],
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Network),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "subnets-for-stat-collection",
            value_type: ValueKind::List,
            list_operators: &["add", "delete"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "traffic-capture",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["traffic-capture"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "captured-protocols",
                    value_type: ValueKind::Enum,
                    in_sections: &["traffic-capture"],
                    enum_values: &["all", "http", "https"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "client-ips",
                    value_type: ValueKind::List,
                    in_sections: &["traffic-capture"],
                    allow_none: true,
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dos-activity",
                    value_type: ValueKind::Enum,
                    in_sections: &["traffic-capture"],
                    enum_values: &["any", "mitigated-by-dosl7"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "methods",
                    value_type: ValueKind::List,
                    in_sections: &["traffic-capture"],
                    allow_none: true,
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "node-addresses",
                    value_type: ValueKind::List,
                    in_sections: &["traffic-capture"],
                    allow_none: true,
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "request-captured-parts",
                    value_type: ValueKind::Enum,
                    in_sections: &["traffic-capture"],
                    allow_none: true,
                    enum_values: &["all", "body", "headers", "none"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "request-content-filter-search-part",
                    value_type: ValueKind::Enum,
                    in_sections: &["traffic-capture"],
                    allow_none: true,
                    enum_values: &["all", "body", "headers", "none", "uri"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "request-content-filter-search-string",
                    value_type: ValueKind::String,
                    in_sections: &["traffic-capture"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "response-captured-parts",
                    value_type: ValueKind::Enum,
                    in_sections: &["traffic-capture"],
                    allow_none: true,
                    enum_values: &["all", "body", "headers", "none"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "response-codes",
                    value_type: ValueKind::List,
                    in_sections: &["traffic-capture"],
                    allow_none: true,
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "response-content-filter-search-part",
                    value_type: ValueKind::Enum,
                    in_sections: &["traffic-capture"],
                    allow_none: true,
                    enum_values: &["all", "body", "headers", "none"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "response-content-filter-search-string",
                    value_type: ValueKind::String,
                    in_sections: &["traffic-capture"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "url-filter-type",
                    value_type: ValueKind::Enum,
                    in_sections: &["traffic-capture"],
                    enum_values: &["all", "black-list", "white-list"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "url-path-prefixes",
                    value_type: ValueKind::List,
                    in_sections: &["traffic-capture"],
                    allow_none: true,
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "user-agent-substrings",
                    value_type: ValueKind::List,
                    in_sections: &["traffic-capture"],
                    allow_none: true,
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "virtual-servers",
                    value_type: ValueKind::List,
                    in_sections: &["traffic-capture"],
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
            in_sections: &["traffic-capture"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "captured-protocols",
            value_type: ValueKind::Enum,
            in_sections: &["traffic-capture"],
            enum_values: &["all", "http", "https"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-ips",
            value_type: ValueKind::List,
            in_sections: &["traffic-capture"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dos-activity",
            value_type: ValueKind::Enum,
            in_sections: &["traffic-capture"],
            enum_values: &["any", "mitigated-by-dosl7"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "methods",
            value_type: ValueKind::List,
            in_sections: &["traffic-capture"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "node-addresses",
            value_type: ValueKind::List,
            in_sections: &["traffic-capture"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-captured-parts",
            value_type: ValueKind::Enum,
            in_sections: &["traffic-capture"],
            allow_none: true,
            enum_values: &["all", "body", "headers", "none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-content-filter-search-part",
            value_type: ValueKind::Enum,
            in_sections: &["traffic-capture"],
            allow_none: true,
            enum_values: &["all", "body", "headers", "none", "uri"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-content-filter-search-string",
            value_type: ValueKind::String,
            in_sections: &["traffic-capture"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "response-captured-parts",
            value_type: ValueKind::Enum,
            in_sections: &["traffic-capture"],
            allow_none: true,
            enum_values: &["all", "body", "headers", "none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "response-codes",
            value_type: ValueKind::List,
            in_sections: &["traffic-capture"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "response-content-filter-search-part",
            value_type: ValueKind::Enum,
            in_sections: &["traffic-capture"],
            allow_none: true,
            enum_values: &["all", "body", "headers", "none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "response-content-filter-search-string",
            value_type: ValueKind::String,
            in_sections: &["traffic-capture"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "url-filter-type",
            value_type: ValueKind::Enum,
            in_sections: &["traffic-capture"],
            enum_values: &["all", "black-list", "white-list"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "url-path-prefixes",
            value_type: ValueKind::List,
            in_sections: &["traffic-capture"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "user-agent-substrings",
            value_type: ValueKind::List,
            in_sections: &["traffic-capture"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "virtual-servers",
            value_type: ValueKind::List,
            in_sections: &["traffic-capture"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "urls-for-stat-collection",
            value_type: ValueKind::List,
            list_operators: &["add", "delete"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
