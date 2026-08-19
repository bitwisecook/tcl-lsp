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

//! BIG-IP object specs for the `wom` tmsh module.
//!
//! **Hand-maintained.** These files were originally produced by a
//! one-time port of the pre-rewrite Python registry
//! (`dialects/f5/bigip/registry/specs/`, still present on `main`) via a
//! `scripts/registry-audit/gen_bigip_rust.py` generator that no longer
//! exists — deleted along with the rest of the retired Python tooling on
//! this branch, and the commits that ran it were squashed away in the
//! `rust` branch's rebase-onto-main history (see issue #1404). Originally
//! organised by the first letter of `kind`; reorganised by tmsh module
//! name (this file's own module field, `Some("wom")`) per
//! maintainer review on the PR that introduced the consistency gate below.
//! There is no live generator to regenerate these from; edit them by hand,
//! in the same shape the other module files already use.
//!
//! `cargo xtask bigip-data-schema --check` enforces the internal
//! invariants a generator would otherwise have guaranteed: every `kind`
//! is globally unique, filed under the module file matching its own
//! `module` field, and every `references` target either names a real
//! kind or is on the documented known-gap list.
// Some modules hold property-less kinds, so not every imported type
// is used in every file; large tmsh bounds appear as bare f64 literals.
#![allow(unused_imports, clippy::unreadable_literal)]
use super::super::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "wom_advertised_route",
            table_name: None,
            resolver_name: None,
            module: Some("wom"),
            object_types: &["advertised-route"],
        },
        header_types: &[("wom", "advertised-route")],
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
                name: "dest",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "label",
                value_type: ValueKind::Unknown,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metric",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "origin",
                value_type: ValueKind::Enum,
                enum_values: &["configured", "discovered", "manually-saved", "persistable"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "wom_deduplication",
            table_name: None,
            resolver_name: None,
            module: Some("wom"),
            object_types: &["deduplication"],
        },
        header_types: &[("wom", "deduplication")],
        properties: &[
            BigipPropertySpec {
                name: "codec",
                value_type: ValueKind::Enum,
                enum_values: &["sdd-v2", "sdd-v3"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-endpoint-count",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "wom_endpoint_discovery",
            table_name: None,
            resolver_name: None,
            module: Some("wom"),
            object_types: &["endpoint-discovery"],
        },
        header_types: &[("wom", "endpoint-discovery")],
        properties: &[
            BigipPropertySpec {
                name: "auto-save",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "discoverable",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "discovered-endpoint",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "icmp-max-requests",
                value_type: ValueKind::Integer,
                default: Some("1024"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "icmp-min-backoff",
                value_type: ValueKind::Integer,
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "icmp-num-retries",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-endpoint-count",
                value_type: ValueKind::Integer,
                default: Some("0, which indicates no limit"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["disable", "enable-all", "enable-icmp", "enable-tcp"],
                default: Some("enable-all"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "wom_local_endpoint",
            table_name: None,
            resolver_name: None,
            module: Some("wom"),
            object_types: &["local-endpoint"],
        },
        header_types: &[("wom", "local-endpoint")],
        properties: &[
            BigipPropertySpec {
                name: "addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-nat",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "endpoint",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "internal-forwarding",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-encap-mtu",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-encap-profile",
                value_type: ValueKind::Reference,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-encap-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["gre", "ipip", "ipsec", "none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "no-route",
                value_type: ValueKind::Enum,
                enum_values: &["drop", "passthru"],
                default: Some("passthru"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server-ssl",
                value_type: ValueKind::Reference,
                allow_none: true,
                enum_values: &["none"],
                references: &["ltm_profile_server_ssl"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "snat",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["local", "none", "remote"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tunnel-port",
                value_type: ValueKind::Integer,
                default: Some("443"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "wom_profile_cifs",
            table_name: None,
            resolver_name: None,
            module: Some("wom"),
            object_types: &["profile cifs"],
        },
        header_types: &[("wom", "profile cifs")],
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
                references: &["wom_profile_cifs"],
                default: Some("cifs"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fast-close",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fast-set-file-info",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "office-2003-extended",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "read-ahead",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "record-replay",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "write-behind",
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
            kind: "wom_profile_isession",
            table_name: None,
            resolver_name: None,
            module: Some("wom"),
            object_types: &["profile isession"],
        },
        header_types: &[("wom", "profile isession")],
        properties: &[
            BigipPropertySpec {
                name: "adaptive-compression",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
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
                name: "compression",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compression-codecs",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "bzip2",
                        value_type: ValueKind::Unknown,
                        in_sections: &["compression-codecs"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "deflate",
                        value_type: ValueKind::Unknown,
                        in_sections: &["compression-codecs"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "lzo",
                        value_type: ValueKind::Unknown,
                        in_sections: &["compression-codecs"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bzip2",
                value_type: ValueKind::Unknown,
                in_sections: &["compression-codecs"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "deflate",
                value_type: ValueKind::Unknown,
                in_sections: &["compression-codecs"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lzo",
                value_type: ValueKind::Unknown,
                in_sections: &["compression-codecs"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "data-encryption",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "deduplication",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["wom_profile_isession"],
                default: Some("isession"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "deflate-compression-level",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port-transparency",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reuse-connection",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "target-virtual",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &[
                    "host-match-all",
                    "host-match-no-isession",
                    "none",
                    "virtual-match-all",
                ],
                default: Some("virtual-match-all"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "wom_profile_mapi",
            table_name: None,
            resolver_name: None,
            module: Some("wom"),
            object_types: &["profile mapi"],
        },
        header_types: &[("wom", "profile mapi")],
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
                references: &["wom_profile_mapi"],
                default: Some("mapi"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "discover-exchange-servers",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "native-compression",
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
            kind: "wom_remote_endpoint",
            table_name: None,
            resolver_name: None,
            module: Some("wom"),
            object_types: &["remote-endpoint"],
        },
        header_types: &[("wom", "remote-endpoint")],
        properties: &[
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-routing",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
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
                name: "dedup-action",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["cache-refresh", "none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "endpoint",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "internal-forwarding",
                value_type: ValueKind::Enum,
                enum_values: &["default", "disabled", "enabled"],
                default: Some("default"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-encap-mtu",
                value_type: ValueKind::Integer,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-encap-profile",
                value_type: ValueKind::Reference,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-encap-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["default", "gre", "ipip", "ipsec", "none"],
                default: Some("default"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "origin",
                value_type: ValueKind::Enum,
                enum_values: &["configured", "discovered", "manually-saved", "persistable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server-ssl",
                value_type: ValueKind::Reference,
                allow_none: true,
                enum_values: &["none"],
                references: &["ltm_profile_server_ssl"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "snat",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["default", "local", "none", "remote"],
                default: Some("default"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tunnel-encrypt",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tunnel-port",
                value_type: ValueKind::Integer,
                default: Some("443"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "wom_server_discovery",
            table_name: None,
            resolver_name: None,
            module: Some("wom"),
            object_types: &["server-discovery"],
        },
        header_types: &[("wom", "server-discovery")],
        properties: &[
            BigipPropertySpec {
                name: "auto-save",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "filter-mode",
                value_type: ValueKind::Enum,
                enum_values: &["exclude", "include"],
                default: Some(
                    "exclude with no IP addresses specified, which means that all advertised routes that conform to the specified attributes are discovered",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idle-time-limit",
                value_type: ValueKind::Integer,
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-ttl-limit",
                value_type: ValueKind::Integer,
                default: Some("5"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-server-count",
                value_type: ValueKind::Integer,
                default: Some("50"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-idle-time",
                value_type: ValueKind::Integer,
                default: Some("0, which indicates that idle time is not considered in discovery"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-prefix-length-ipv4",
                value_type: ValueKind::Integer,
                default: Some("32"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-prefix-length-ipv6",
                value_type: ValueKind::Integer,
                default: Some("128"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rtt-threshold",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subnet-filter",
                value_type: ValueKind::List,
                allow_none: true,
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-unit",
                value_type: ValueKind::Enum,
                enum_values: &["days", "hours", "minutes"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
