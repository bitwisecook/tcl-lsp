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

//! BIG-IP object specs for `ltm dns`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_analytics_global_settings",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns analytics global-settings"],
        },
        header_types: &[("ltm", "dns analytics global-settings")],
        properties: &[
            BigipPropertySpec {
                name: "collect-client-ip",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "collect-query-name",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_cache_global_settings",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns cache global-settings"],
        },
        header_types: &[("ltm", "dns cache global-settings")],
        properties: &[
            BigipPropertySpec {
                name: "cache-maximum-ttl",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-minimum-ttl",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "resolver-edns-buffer-size",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serve-expired",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serve-expired-client-timeout",
                value_type: ValueKind::Integer,
                default: Some("1800 milliseconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serve-expired-reply-ttl",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serve-expired-ttl",
                value_type: ValueKind::Integer,
                default: Some("86400 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serve-expired-ttl-reset",
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
            kind: "ltm_dns_cache_records_all",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns cache records all"],
        },
        header_types: &[("ltm", "dns cache records all")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_cache_records_key",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns cache records key"],
        },
        header_types: &[("ltm", "dns cache records key")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_cache_records_msg",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns cache records msg"],
        },
        header_types: &[("ltm", "dns cache records msg")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_cache_records_nameserver",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns cache records nameserver"],
        },
        header_types: &[("ltm", "dns cache records nameserver")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_cache_records_rrset",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns cache records rrset"],
        },
        header_types: &[("ltm", "dns cache records rrset")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_cache_resolver",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns cache resolver"],
        },
        header_types: &[("ltm", "dns cache resolver")],
        properties: &[
            BigipPropertySpec {
                name: "allowed-query-time",
                value_type: ValueKind::Integer,
                default: Some("200 milliseconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "answer-default-zones",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
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
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "forward-zones",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "nameservers",
                    value_type: ValueKind::List,
                    in_sections: &["forward-zones"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nameservers",
                value_type: ValueKind::List,
                in_sections: &["forward-zones"],
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-zones",
                value_type: ValueKind::List,
                repeated: true,
                default: Some("empty"),
                list_operators: &["add"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-concurrent-queries",
                value_type: ValueKind::Integer,
                default: Some("1024"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-concurrent-tcp",
                value_type: ValueKind::Integer,
                default: Some("20"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-concurrent-udp",
                value_type: ValueKind::Integer,
                default: Some("8192"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "msg-cache-size",
                value_type: ValueKind::Integer,
                default: Some("1048576"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nameserver-cache-count",
                value_type: ValueKind::Integer,
                default: Some("16536 entries"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nameserver-min-rtt",
                value_type: ValueKind::Integer,
                default: Some("50 milliseconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nameserver-ttl",
                value_type: ValueKind::Integer,
                default: Some("900 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "outbound-msg-retry",
                value_type: ValueKind::Integer,
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prefetch",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "randomize-query-name-case",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "response-policy-zones",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify"],
                block: &[
                    BigipPropertySpec {
                        name: "action",
                        value_type: ValueKind::Enum,
                        in_sections: &["response-policy-zones"],
                        enum_values: &["nxdomain", "walled-garden"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "walled-garden",
                        value_type: ValueKind::Unknown,
                        in_sections: &["response-policy-zones"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["response-policy-zones"],
                enum_values: &["nxdomain", "walled-garden"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "walled-garden",
                value_type: ValueKind::Unknown,
                in_sections: &["response-policy-zones"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "root-hints",
                value_type: ValueKind::List,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Reference,
                references: &["net_route_domain"],
                default: Some("the default route domain"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rrset-cache-size",
                value_type: ValueKind::Integer,
                default: Some("10485760"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rrset-rotate",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "query-id"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "unwanted-query-reply-threshold",
                value_type: ValueKind::Integer,
                default: Some("0 (off)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-ipv4",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-ipv6",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-tcp",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-udp",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_cache_transparent",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns cache transparent"],
        },
        header_types: &[("ltm", "dns cache transparent")],
        properties: &[
            BigipPropertySpec {
                name: "answer-default-zones",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
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
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-zones",
                value_type: ValueKind::List,
                repeated: true,
                default: Some("empty"),
                list_operators: &["add"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "msg-cache-size",
                value_type: ValueKind::Integer,
                default: Some("1048576"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "response-policy-zones",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify"],
                block: &[
                    BigipPropertySpec {
                        name: "action",
                        value_type: ValueKind::Enum,
                        in_sections: &["response-policy-zones"],
                        enum_values: &["nxdomain", "walled-garden"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "walled-garden",
                        value_type: ValueKind::Unknown,
                        in_sections: &["response-policy-zones"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["response-policy-zones"],
                enum_values: &["nxdomain", "walled-garden"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "walled-garden",
                value_type: ValueKind::Unknown,
                in_sections: &["response-policy-zones"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rrset-cache-size",
                value_type: ValueKind::Integer,
                default: Some("10485760"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rrset-rotate",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "query-id"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_cache_validating_resolver",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns cache validating-resolver"],
        },
        header_types: &[("ltm", "dns cache validating-resolver")],
        properties: &[
            BigipPropertySpec {
                name: "allowed-query-time",
                value_type: ValueKind::Integer,
                default: Some("200 milliseconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "answer-default-zones",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
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
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dlv-anchors",
                value_type: ValueKind::List,
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "forward-zones",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "nameservers",
                    value_type: ValueKind::List,
                    in_sections: &["forward-zones"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nameservers",
                value_type: ValueKind::List,
                in_sections: &["forward-zones"],
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-cd",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "key-cache-size",
                value_type: ValueKind::Integer,
                default: Some("1048576"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-zones",
                value_type: ValueKind::List,
                repeated: true,
                default: Some("empty"),
                list_operators: &["add"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-concurrent-queries",
                value_type: ValueKind::Integer,
                default: Some("1024"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-concurrent-tcp",
                value_type: ValueKind::Integer,
                default: Some("20"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-concurrent-udp",
                value_type: ValueKind::Integer,
                default: Some("8192"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "msg-cache-size",
                value_type: ValueKind::Integer,
                default: Some("1048576"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nameserver-cache-count",
                value_type: ValueKind::Integer,
                default: Some("16536 entries"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nameserver-min-rtt",
                value_type: ValueKind::Integer,
                default: Some("50 milliseconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nameserver-ttl",
                value_type: ValueKind::Integer,
                default: Some("900 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "outbound-msg-retry",
                value_type: ValueKind::Integer,
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prefetch",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prefetch-key",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "randomize-query-name-case",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "response-policy-zones",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify"],
                block: &[
                    BigipPropertySpec {
                        name: "action",
                        value_type: ValueKind::Enum,
                        in_sections: &["response-policy-zones"],
                        enum_values: &["nxdomain", "walled-garden"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "walled-garden",
                        value_type: ValueKind::Unknown,
                        in_sections: &["response-policy-zones"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["response-policy-zones"],
                enum_values: &["nxdomain", "walled-garden"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "walled-garden",
                value_type: ValueKind::Unknown,
                in_sections: &["response-policy-zones"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "root-hints",
                value_type: ValueKind::List,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Reference,
                references: &["net_route_domain"],
                default: Some("the default route domain"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rrset-cache-size",
                value_type: ValueKind::Integer,
                default: Some("10485760"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rrset-rotate",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "query-id"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trust-anchors",
                value_type: ValueKind::List,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "unwanted-query-reply-threshold",
                value_type: ValueKind::Integer,
                default: Some("0 (off)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-ipv4",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-ipv6",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-tcp",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-udp",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_dnssec_key",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns dnssec key"],
        },
        header_types: &[("ltm", "dns dnssec key")],
        properties: &[
            BigipPropertySpec {
                name: "algorithm",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "ecdsap256sha256",
                    "ecdsap384sha384",
                    "rsasha1",
                    "rsasha256",
                    "rsasha512",
                ],
                default: Some("RSASHA1"),
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
                name: "bitwidth",
                value_type: ValueKind::Enum,
                enum_values: &["1024", "2048", "4096", "512"],
                default: Some("1024"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "certificate-file",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "expiration-period",
                value_type: ValueKind::Integer,
                default: Some("0 (zero), which indicates unset, and thus the key does not expire"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "generation",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "expiration",
                        value_type: ValueKind::Unknown,
                        in_sections: &["generation"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "rollover",
                        value_type: ValueKind::Unknown,
                        in_sections: &["generation"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "expiration",
                value_type: ValueKind::Unknown,
                in_sections: &["generation"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rollover",
                value_type: ValueKind::Unknown,
                in_sections: &["generation"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "key-file",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "key-type",
                value_type: ValueKind::Enum,
                enum_values: &["ksk", "zsk"],
                default: Some("zsk"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rollover-period",
                value_type: ValueKind::Integer,
                default: Some(
                    "0 (zero), which indicates unset, and thus the key does not roll over",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signature-pub-period",
                value_type: ValueKind::Integer,
                default: Some("403200 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signature-valid-period",
                value_type: ValueKind::Integer,
                default: Some("604800 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ttl",
                value_type: ValueKind::Integer,
                default: Some("86400"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-fips",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["external", "internal", "none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_dnssec_zone",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns dnssec zone"],
        },
        header_types: &[("ltm", "dns dnssec zone")],
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
                name: "ds-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &["sha1", "sha256", "sha384"],
                default: Some("sha1"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ds-algorithms",
                value_type: ValueKind::List,
                default: Some("sha1"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ds-records",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "external-delegations",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "indicate-authenticated",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "keys",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nsec3-algorithm",
                value_type: ValueKind::Unknown,
                default: Some("SHA1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nsec3-iterations",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "publish-cds-cdnskey",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secure",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_hpke_key",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns hpke key"],
        },
        header_types: &[("ltm", "dns hpke key")],
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
                name: "profile",
                value_type: ValueKind::String,
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
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_hpke_profile",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns hpke profile"],
        },
        header_types: &[("ltm", "dns hpke profile")],
        properties: &[
            BigipPropertySpec {
                name: "aead",
                value_type: ValueKind::String,
                default: Some("AES-128-GCM"),
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
                name: "expiration-period",
                value_type: ValueKind::Integer,
                default: Some(
                    "0 (zero), which indicates unset, and thus the hpke key does not expire",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "kdf",
                value_type: ValueKind::String,
                default: Some("HKDF-SHA256"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "kem",
                value_type: ValueKind::String,
                default: Some("X25519"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rollover-period",
                value_type: ValueKind::Integer,
                default: Some(
                    "0 (zero), which indicates unset, and thus the hpke key does not roll over",
                ),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_nameserver",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns nameserver"],
        },
        header_types: &[("ltm", "dns nameserver")],
        properties: &[
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Address),
                default: Some("127"),
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
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                default: Some("53"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "route-domain",
                value_type: ValueKind::Reference,
                allow_none: true,
                enum_values: &["none"],
                references: &["net_route_domain"],
                default: Some("the default route domain"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tsig-key",
                value_type: ValueKind::Reference,
                allow_none: true,
                enum_values: &["none"],
                references: &["ltm_dns_tsig_key"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_tsig_key",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns tsig-key"],
        },
        header_types: &[("ltm", "dns tsig-key")],
        properties: &[
            BigipPropertySpec {
                name: "algorithm",
                value_type: ValueKind::Enum,
                enum_values: &["hmacmd5", "hmacsha1", "hmacsha256"],
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
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secret",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_dns_zone",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["dns zone"],
        },
        header_types: &[("ltm", "dns zone")],
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
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-express-allow-notify",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-express-enabled",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-express-notify-action",
                value_type: ValueKind::Enum,
                enum_values: &["bypass", "consume", "repeat"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-express-notify-tsig-verify",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-express-server",
                value_type: ValueKind::Reference,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "response-policy",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server-tsig-key",
                value_type: ValueKind::Reference,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transfer-clients",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
