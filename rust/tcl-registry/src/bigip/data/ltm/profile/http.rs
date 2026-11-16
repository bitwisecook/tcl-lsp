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

//! BIG-IP object specs for `ltm profile http`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_http",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile http"],
    },
    header_types: &[("ltm", "profile http")],
    properties: &[
        BigipPropertySpec {
            name: "accept-xff",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
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
            name: "basic-auth-realm",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_profile_http"],
            default: Some("http"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "encrypt-cookie-secret",
            value_type: ValueKind::Unknown,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "encrypt-cookies",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforcement",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "allow-ws-header-name",
                    value_type: ValueKind::Enum,
                    in_sections: &["enforcement"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "excess-client-headers",
                    value_type: ValueKind::Enum,
                    in_sections: &["enforcement"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled which rejects the connection"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "excess-server-headers",
                    value_type: ValueKind::Enum,
                    in_sections: &["enforcement"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled which rejects the connection"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "known-methods",
                    value_type: ValueKind::Unknown,
                    in_sections: &["enforcement"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "max-header-count",
                    value_type: ValueKind::Integer,
                    in_sections: &["enforcement"],
                    default: Some("64"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "max-header-size",
                    value_type: ValueKind::Integer,
                    in_sections: &["enforcement"],
                    default: Some("32768"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "max-requests",
                    value_type: ValueKind::Integer,
                    in_sections: &["enforcement"],
                    default: Some(
                        "0 (zero), which means the system does not limit the number of requests per connection",
                    ),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "oversize-client-headers",
                    value_type: ValueKind::Enum,
                    in_sections: &["enforcement"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled which rejects the connection"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "oversize-server-headers",
                    value_type: ValueKind::Enum,
                    in_sections: &["enforcement"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled which rejects the connection"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "pipeline",
                    value_type: ValueKind::Enum,
                    in_sections: &["enforcement"],
                    enum_values: &["allow", "pass-through", "reject"],
                    default: Some(
                        "allow, which means that clients can make requests even when prior requests have not received a response",
                    ),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "rfc-compliance",
                    value_type: ValueKind::Enum,
                    in_sections: &["enforcement"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled which ignores rfc non-compliance"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "truncated-redirects",
                    value_type: ValueKind::Enum,
                    in_sections: &["enforcement"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled, which will silently drop the invalid HTTP"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "unknown-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["enforcement"],
                    enum_values: &["allow", "pass-through", "reject"],
                    default: Some("allow which allows all methods, (known or unknown)"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "allow-ws-header-name",
            value_type: ValueKind::Enum,
            in_sections: &["enforcement"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "excess-client-headers",
            value_type: ValueKind::Enum,
            in_sections: &["enforcement"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled which rejects the connection"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "excess-server-headers",
            value_type: ValueKind::Enum,
            in_sections: &["enforcement"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled which rejects the connection"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "known-methods",
            value_type: ValueKind::Unknown,
            in_sections: &["enforcement"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-header-count",
            value_type: ValueKind::Integer,
            in_sections: &["enforcement"],
            default: Some("64"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-header-size",
            value_type: ValueKind::Integer,
            in_sections: &["enforcement"],
            default: Some("32768"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-requests",
            value_type: ValueKind::Integer,
            in_sections: &["enforcement"],
            default: Some(
                "0 (zero), which means the system does not limit the number of requests per connection",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "oversize-client-headers",
            value_type: ValueKind::Enum,
            in_sections: &["enforcement"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled which rejects the connection"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "oversize-server-headers",
            value_type: ValueKind::Enum,
            in_sections: &["enforcement"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled which rejects the connection"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pipeline",
            value_type: ValueKind::Enum,
            in_sections: &["enforcement"],
            enum_values: &["allow", "pass-through", "reject"],
            default: Some(
                "allow, which means that clients can make requests even when prior requests have not received a response",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rfc-compliance",
            value_type: ValueKind::Enum,
            in_sections: &["enforcement"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled which ignores rfc non-compliance"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "truncated-redirects",
            value_type: ValueKind::Enum,
            in_sections: &["enforcement"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled, which will silently drop the invalid HTTP"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "unknown-method",
            value_type: ValueKind::Enum,
            in_sections: &["enforcement"],
            enum_values: &["allow", "pass-through", "reject"],
            default: Some("allow which allows all methods, (known or unknown)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "explicit-proxy",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "bad-request-message",
                    value_type: ValueKind::Unknown,
                    in_sections: &["explicit-proxy"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bad-response-message",
                    value_type: ValueKind::Unknown,
                    in_sections: &["explicit-proxy"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "connect-error-message",
                    value_type: ValueKind::Unknown,
                    in_sections: &["explicit-proxy"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "default-connect-handling",
                    value_type: ValueKind::Enum,
                    in_sections: &["explicit-proxy"],
                    enum_values: &["allow", "deny"],
                    default: Some("deny"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dns-error-message",
                    value_type: ValueKind::Unknown,
                    in_sections: &["explicit-proxy"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dns-resolver",
                    value_type: ValueKind::Unknown,
                    in_sections: &["explicit-proxy"],
                    default: Some("dns-resolver"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enabled",
                    value_type: ValueKind::Enum,
                    in_sections: &["explicit-proxy"],
                    enum_values: &["no", "yes"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ipv6",
                    value_type: ValueKind::Enum,
                    in_sections: &["explicit-proxy"],
                    enum_values: &["no", "yes"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("no, which will try a IPv4 lookup before a IPv6"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "route-domain",
                    value_type: ValueKind::Unknown,
                    in_sections: &["explicit-proxy"],
                    default: Some("0"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "tunnel-name",
                    value_type: ValueKind::Unknown,
                    in_sections: &["explicit-proxy"],
                    default: Some("http-tunnel"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "tunnel-on-any-request",
                    value_type: ValueKind::Enum,
                    in_sections: &["explicit-proxy"],
                    enum_values: &["no", "yes"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("no"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bad-request-message",
            value_type: ValueKind::Unknown,
            in_sections: &["explicit-proxy"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bad-response-message",
            value_type: ValueKind::Unknown,
            in_sections: &["explicit-proxy"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "connect-error-message",
            value_type: ValueKind::Unknown,
            in_sections: &["explicit-proxy"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "default-connect-handling",
            value_type: ValueKind::Enum,
            in_sections: &["explicit-proxy"],
            enum_values: &["allow", "deny"],
            default: Some("deny"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dns-error-message",
            value_type: ValueKind::Unknown,
            in_sections: &["explicit-proxy"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dns-resolver",
            value_type: ValueKind::Unknown,
            in_sections: &["explicit-proxy"],
            default: Some("dns-resolver"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Enum,
            in_sections: &["explicit-proxy"],
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ipv6",
            value_type: ValueKind::Enum,
            in_sections: &["explicit-proxy"],
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("no, which will try a IPv4 lookup before a IPv6"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-domain",
            value_type: ValueKind::Unknown,
            in_sections: &["explicit-proxy"],
            references: &["net_route_domain"],
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tunnel-name",
            value_type: ValueKind::Unknown,
            in_sections: &["explicit-proxy"],
            default: Some("http-tunnel"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tunnel-on-any-request",
            value_type: ValueKind::Enum,
            in_sections: &["explicit-proxy"],
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("no"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fallback-host",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fallback-status-codes",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "header-erase",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "header-insert",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hsts",
            value_type: ValueKind::List,
            block: &[
                BigipPropertySpec {
                    name: "include-subdomains",
                    value_type: ValueKind::Enum,
                    in_sections: &["hsts"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "maximum-age",
                    value_type: ValueKind::Integer,
                    in_sections: &["hsts"],
                    default: Some("16070400"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["hsts"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "preload",
                    value_type: ValueKind::Enum,
                    in_sections: &["hsts"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "include-subdomains",
            value_type: ValueKind::Enum,
            in_sections: &["hsts"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "maximum-age",
            value_type: ValueKind::Integer,
            in_sections: &["hsts"],
            default: Some("16070400"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mode",
            value_type: ValueKind::Enum,
            in_sections: &["hsts"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "preload",
            value_type: ValueKind::Enum,
            in_sections: &["hsts"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "insert-xforwarded-for",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "lws-separator",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["none"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "lws-width",
            value_type: ValueKind::Integer,
            default: Some("80"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "oneconnect-status-reuse",
            value_type: ValueKind::Unknown,
            default: Some("\"200 206\""),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "oneconnect-transformations",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "proxy-type",
            value_type: ValueKind::Enum,
            enum_values: &["explicit", "reverse", "transparent"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "redirect-rewrite",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["all", "matching", "nodes", "none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-chunking",
            value_type: ValueKind::Enum,
            enum_values: &["rechunk", "sustain"],
            default: Some("sustain"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "response-chunking",
            value_type: ValueKind::Enum,
            enum_values: &["rechunk", "sustain", "unchunk"],
            default: Some("sustain"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "response-headers-permitted",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "server-agent-name",
            value_type: ValueKind::String,
            default: Some("BigIP"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sflow",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "poll-interval",
                    value_type: ValueKind::Integer,
                    in_sections: &["sflow"],
                    default: Some("0"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "poll-interval-global",
                    value_type: ValueKind::Enum,
                    in_sections: &["sflow"],
                    enum_values: &["no", "yes"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("yes"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "sampling-rate",
                    value_type: ValueKind::Integer,
                    in_sections: &["sflow"],
                    default: Some("0"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "sampling-rate-global",
                    value_type: ValueKind::Enum,
                    in_sections: &["sflow"],
                    enum_values: &["no", "yes"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("yes"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "poll-interval",
            value_type: ValueKind::Integer,
            in_sections: &["sflow"],
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "poll-interval-global",
            value_type: ValueKind::Enum,
            in_sections: &["sflow"],
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("yes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sampling-rate",
            value_type: ValueKind::Integer,
            in_sections: &["sflow"],
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sampling-rate-global",
            value_type: ValueKind::Enum,
            in_sections: &["sflow"],
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("yes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "via-host-name",
            value_type: ValueKind::String,
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "via-request",
            value_type: ValueKind::Enum,
            enum_values: &["append", "preserve", "remove"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "via-response",
            value_type: ValueKind::Enum,
            enum_values: &["append", "preserve", "remove"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "xff-alternative-names",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
