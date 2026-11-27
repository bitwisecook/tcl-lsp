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

//! BIG-IP object specs for `net routing bgp`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "net_routing_bgp",
        table_name: None,
        resolver_name: None,
        module: Some("net"),
        object_types: &["routing bgp"],
    },
    header_types: &[("net", "routing bgp")],
    properties: &[
        BigipPropertySpec {
            name: "address-family",
            value_type: ValueKind::Reference,
            shape_kind: Some(ValueKind::List),
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "activate",
                    value_type: ValueKind::Enum,
                    in_sections: &["address-family"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "aggregate-address",
                    value_type: ValueKind::Reference,
                    in_sections: &["address-family"],
                    shape_kind: Some(ValueKind::List),
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "allow-as-in",
                    value_type: ValueKind::Boolean,
                    in_sections: &["address-family"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "as-override",
                    value_type: ValueKind::Enum,
                    in_sections: &["address-family"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "attribute-unchanged",
                    value_type: ValueKind::String,
                    in_sections: &["address-family"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "auto-summary",
                    value_type: ValueKind::Enum,
                    in_sections: &["address-family"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "capability",
                    value_type: ValueKind::String,
                    in_sections: &["address-family"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "default-originate",
                    value_type: ValueKind::String,
                    in_sections: &["address-family"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "distance",
                    value_type: ValueKind::String,
                    in_sections: &["address-family"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "distribute-list",
                    value_type: ValueKind::String,
                    in_sections: &["address-family"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "filter-list",
                    value_type: ValueKind::String,
                    in_sections: &["address-family"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "maximum-prefix",
                    value_type: ValueKind::String,
                    in_sections: &["address-family"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "network-synchronization",
                    value_type: ValueKind::Enum,
                    in_sections: &["address-family"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "next-hop-self",
                    value_type: ValueKind::Enum,
                    in_sections: &["address-family"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "prefix-list",
                    value_type: ValueKind::String,
                    in_sections: &["address-family"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "remove-private-as",
                    value_type: ValueKind::Enum,
                    in_sections: &["address-family"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "route-map",
                    value_type: ValueKind::String,
                    in_sections: &["address-family"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "route-reflector-client",
                    value_type: ValueKind::Enum,
                    in_sections: &["address-family"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "route-server-client",
                    value_type: ValueKind::Enum,
                    in_sections: &["address-family"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "send-community",
                    value_type: ValueKind::Boolean,
                    in_sections: &["address-family"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "soft-reconfiguration-inbound",
                    value_type: ValueKind::Enum,
                    in_sections: &["address-family"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "unsuppress-map",
                    value_type: ValueKind::Boolean,
                    in_sections: &["address-family"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "weight",
                    value_type: ValueKind::Boolean,
                    in_sections: &["address-family"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "activate",
            value_type: ValueKind::Enum,
            in_sections: &["address-family"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "aggregate-address",
            value_type: ValueKind::Reference,
            in_sections: &["address-family"],
            shape_kind: Some(ValueKind::List),
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "allow-as-in",
            value_type: ValueKind::Boolean,
            in_sections: &["address-family"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "as-override",
            value_type: ValueKind::Enum,
            in_sections: &["address-family"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "attribute-unchanged",
            value_type: ValueKind::String,
            in_sections: &["address-family"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auto-summary",
            value_type: ValueKind::Enum,
            in_sections: &["address-family"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "capability",
            value_type: ValueKind::String,
            in_sections: &["address-family"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "default-originate",
            value_type: ValueKind::String,
            in_sections: &["address-family"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "distance",
            value_type: ValueKind::String,
            in_sections: &["address-family"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "distribute-list",
            value_type: ValueKind::String,
            in_sections: &["address-family"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "filter-list",
            value_type: ValueKind::String,
            in_sections: &["address-family"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "maximum-prefix",
            value_type: ValueKind::String,
            in_sections: &["address-family"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "network-synchronization",
            value_type: ValueKind::Enum,
            in_sections: &["address-family"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "next-hop-self",
            value_type: ValueKind::Enum,
            in_sections: &["address-family"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "prefix-list",
            value_type: ValueKind::String,
            in_sections: &["address-family"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "remove-private-as",
            value_type: ValueKind::Enum,
            in_sections: &["address-family"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-map",
            value_type: ValueKind::String,
            in_sections: &["address-family"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-reflector-client",
            value_type: ValueKind::Enum,
            in_sections: &["address-family"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-server-client",
            value_type: ValueKind::Enum,
            in_sections: &["address-family"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "send-community",
            value_type: ValueKind::Boolean,
            in_sections: &["address-family"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "soft-reconfiguration-inbound",
            value_type: ValueKind::Enum,
            in_sections: &["address-family"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "unsuppress-map",
            value_type: ValueKind::Boolean,
            in_sections: &["address-family"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "weight",
            value_type: ValueKind::Boolean,
            in_sections: &["address-family"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "as-set",
            value_type: ValueKind::Enum,
            in_sections: &["aggregate-address"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "summary-only",
            value_type: ValueKind::Enum,
            in_sections: &["aggregate-address"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "allow-infinite-hold-time",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "always-compare-med",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "as-path",
            value_type: ValueKind::Enum,
            in_sections: &["attribute-unchanged"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "med",
            value_type: ValueKind::Enum,
            in_sections: &["attribute-unchanged"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "next-hop",
            value_type: ValueKind::Enum,
            in_sections: &["attribute-unchanged"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bestpath",
            value_type: ValueKind::String,
            block: &[
                BigipPropertySpec {
                    name: "as-path-ignore",
                    value_type: ValueKind::Enum,
                    in_sections: &["bestpath"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "compare-confed-aspath",
                    value_type: ValueKind::Enum,
                    in_sections: &["bestpath"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "compare-originator-id",
                    value_type: ValueKind::Enum,
                    in_sections: &["bestpath"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "compare-routerid",
                    value_type: ValueKind::Enum,
                    in_sections: &["bestpath"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "med",
                    value_type: ValueKind::String,
                    in_sections: &["bestpath"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "tie-break-on-age",
                    value_type: ValueKind::Enum,
                    in_sections: &["bestpath"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "as-path-ignore",
            value_type: ValueKind::Enum,
            in_sections: &["bestpath"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "compare-confed-aspath",
            value_type: ValueKind::Enum,
            in_sections: &["bestpath"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "compare-originator-id",
            value_type: ValueKind::Enum,
            in_sections: &["bestpath"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "compare-routerid",
            value_type: ValueKind::Enum,
            in_sections: &["bestpath"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "med",
            value_type: ValueKind::String,
            in_sections: &["bestpath"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tie-break-on-age",
            value_type: ValueKind::Enum,
            in_sections: &["bestpath"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "override",
            value_type: ValueKind::Enum,
            in_sections: &["capability-negotiate"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "state",
            value_type: ValueKind::Enum,
            in_sections: &["capability-negotiate"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "strict-match",
            value_type: ValueKind::Enum,
            in_sections: &["capability-negotiate"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dynamic",
            value_type: ValueKind::Enum,
            in_sections: &["capability"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "graceful-restart",
            value_type: ValueKind::Enum,
            in_sections: &["capability"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "orf",
            value_type: ValueKind::String,
            in_sections: &["capability"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-refresh",
            value_type: ValueKind::Enum,
            in_sections: &["capability"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-to-client-reflection",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cluster-id",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "confederation",
            value_type: ValueKind::String,
            block: &[
                BigipPropertySpec {
                    name: "identifier",
                    value_type: ValueKind::Integer,
                    in_sections: &["confederation"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "peers",
                    value_type: ValueKind::Boolean,
                    in_sections: &["confederation"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "identifier",
            value_type: ValueKind::Integer,
            in_sections: &["confederation"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "peers",
            value_type: ValueKind::Boolean,
            in_sections: &["confederation"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dampening",
            value_type: ValueKind::String,
            block: &[
                BigipPropertySpec {
                    name: "reachability-half-life",
                    value_type: ValueKind::Integer,
                    in_sections: &["dampening"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "reuse",
                    value_type: ValueKind::Integer,
                    in_sections: &["dampening"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "route-map",
                    value_type: ValueKind::Boolean,
                    in_sections: &["dampening"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "state",
                    value_type: ValueKind::Enum,
                    in_sections: &["dampening"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "suppress",
                    value_type: ValueKind::Integer,
                    in_sections: &["dampening"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "suppress-max",
                    value_type: ValueKind::Integer,
                    in_sections: &["dampening"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "unreachability-half-life",
                    value_type: ValueKind::Integer,
                    in_sections: &["dampening"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "reachability-half-life",
            value_type: ValueKind::Integer,
            in_sections: &["dampening"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "reuse",
            value_type: ValueKind::Integer,
            in_sections: &["dampening"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-map",
            value_type: ValueKind::Boolean,
            in_sections: &["dampening"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "state",
            value_type: ValueKind::Enum,
            in_sections: &["dampening"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "suppress",
            value_type: ValueKind::Integer,
            in_sections: &["dampening"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "suppress-max",
            value_type: ValueKind::Integer,
            in_sections: &["dampening"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "unreachability-half-life",
            value_type: ValueKind::Integer,
            in_sections: &["dampening"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "default-local-preference",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-map",
            value_type: ValueKind::Boolean,
            in_sections: &["default-originate"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "state",
            value_type: ValueKind::Enum,
            in_sections: &["default-originate"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::Boolean,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "deterministic-med",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "distance",
            value_type: ValueKind::Reference,
            shape_kind: Some(ValueKind::List),
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "access-list",
                    value_type: ValueKind::Boolean,
                    in_sections: &["distance"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "distance",
                    value_type: ValueKind::Integer,
                    in_sections: &["distance"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "external",
                    value_type: ValueKind::Integer,
                    in_sections: &["distance"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "internal",
                    value_type: ValueKind::Integer,
                    in_sections: &["distance"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "local",
                    value_type: ValueKind::Integer,
                    in_sections: &["distance"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "access-list",
            value_type: ValueKind::Boolean,
            in_sections: &["distance"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "distance",
            value_type: ValueKind::Integer,
            in_sections: &["distance"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "external",
            value_type: ValueKind::Integer,
            in_sections: &["distance"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "internal",
            value_type: ValueKind::Integer,
            in_sections: &["distance"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "local",
            value_type: ValueKind::Integer,
            in_sections: &["distance"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "in",
            value_type: ValueKind::Boolean,
            in_sections: &["distribute-list"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "out",
            value_type: ValueKind::Boolean,
            in_sections: &["distribute-list"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforce-first-as",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fast-external-failover",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "in",
            value_type: ValueKind::Boolean,
            in_sections: &["filter-list"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "out",
            value_type: ValueKind::Boolean,
            in_sections: &["filter-list"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "graceful-restart",
            value_type: ValueKind::String,
            block: &[
                BigipPropertySpec {
                    name: "graceful-reset",
                    value_type: ValueKind::Enum,
                    in_sections: &["graceful-restart"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "restart-time",
                    value_type: ValueKind::Integer,
                    in_sections: &["graceful-restart"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "stalepath-time",
                    value_type: ValueKind::Integer,
                    in_sections: &["graceful-restart"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "graceful-reset",
            value_type: ValueKind::Enum,
            in_sections: &["graceful-restart"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "restart-time",
            value_type: ValueKind::Integer,
            in_sections: &["graceful-restart"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "stalepath-time",
            value_type: ValueKind::Integer,
            in_sections: &["graceful-restart"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "graceful-shutdown",
            value_type: ValueKind::String,
            block: &[
                BigipPropertySpec {
                    name: "capable",
                    value_type: ValueKind::Enum,
                    in_sections: &["graceful-shutdown"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "local-preference",
                    value_type: ValueKind::Integer,
                    in_sections: &["graceful-shutdown"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["graceful-shutdown"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "timer",
                    value_type: ValueKind::Integer,
                    in_sections: &["graceful-shutdown"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "capable",
            value_type: ValueKind::Enum,
            in_sections: &["graceful-shutdown"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "local-preference",
            value_type: ValueKind::Integer,
            in_sections: &["graceful-shutdown"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mode",
            value_type: ValueKind::Enum,
            in_sections: &["graceful-shutdown"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "timer",
            value_type: ValueKind::Integer,
            in_sections: &["graceful-shutdown"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hold-time",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "keep-alive",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "local-as",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-neighbor-changes",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "threshold",
            value_type: ValueKind::Boolean,
            in_sections: &["maximum-prefix"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value",
            value_type: ValueKind::Integer,
            in_sections: &["maximum-prefix"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "warning-only",
            value_type: ValueKind::Enum,
            in_sections: &["maximum-prefix"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "confed",
            value_type: ValueKind::Enum,
            in_sections: &["med"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "missing-as-worst",
            value_type: ValueKind::Enum,
            in_sections: &["med"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "remove-recv-med",
            value_type: ValueKind::Enum,
            in_sections: &["med"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "remove-send-med",
            value_type: ValueKind::Enum,
            in_sections: &["med"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "neighbor",
            value_type: ValueKind::Reference,
            shape_kind: Some(ValueKind::List),
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "address-family",
                    value_type: ValueKind::Reference,
                    in_sections: &["neighbor"],
                    shape_kind: Some(ValueKind::List),
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "advertisement-interval",
                    value_type: ValueKind::Integer,
                    in_sections: &["neighbor"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "allow-infinite-hold-time",
                    value_type: ValueKind::Enum,
                    in_sections: &["neighbor"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "as-origination-interval",
                    value_type: ValueKind::Integer,
                    in_sections: &["neighbor"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "capability",
                    value_type: ValueKind::String,
                    in_sections: &["neighbor"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "capability-negotiate",
                    value_type: ValueKind::String,
                    in_sections: &["neighbor"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "collide-established",
                    value_type: ValueKind::Enum,
                    in_sections: &["neighbor"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "connect-timer",
                    value_type: ValueKind::Integer,
                    in_sections: &["neighbor"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::Boolean,
                    in_sections: &["neighbor"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ebgp-multihop",
                    value_type: ValueKind::Integer,
                    in_sections: &["neighbor"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enabled",
                    value_type: ValueKind::Enum,
                    in_sections: &["neighbor"],
                    enum_values: &["false", "true"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enforce-multihop",
                    value_type: ValueKind::Enum,
                    in_sections: &["neighbor"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "fall-over",
                    value_type: ValueKind::Boolean,
                    in_sections: &["neighbor"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "graceful-shutdown",
                    value_type: ValueKind::String,
                    in_sections: &["neighbor"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "hold-time",
                    value_type: ValueKind::Integer,
                    in_sections: &["neighbor"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "keep-alive",
                    value_type: ValueKind::Integer,
                    in_sections: &["neighbor"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "local-as",
                    value_type: ValueKind::Integer,
                    in_sections: &["neighbor"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "passive",
                    value_type: ValueKind::Enum,
                    in_sections: &["neighbor"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "password",
                    value_type: ValueKind::Boolean,
                    in_sections: &["neighbor"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "peer-group",
                    value_type: ValueKind::Boolean,
                    in_sections: &["neighbor"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "port",
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
                    name: "restart-time",
                    value_type: ValueKind::Integer,
                    in_sections: &["neighbor"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "update-source",
                    value_type: ValueKind::Boolean,
                    in_sections: &["neighbor"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "version",
                    value_type: ValueKind::Integer,
                    in_sections: &["neighbor"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "vlan",
                    value_type: ValueKind::Reference,
                    in_sections: &["neighbor"],
                    allow_none: true,
                    references: &["net_vlan"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "address-family",
            value_type: ValueKind::Reference,
            in_sections: &["neighbor"],
            shape_kind: Some(ValueKind::List),
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "advertisement-interval",
            value_type: ValueKind::Integer,
            in_sections: &["neighbor"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "allow-infinite-hold-time",
            value_type: ValueKind::Enum,
            in_sections: &["neighbor"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "as-origination-interval",
            value_type: ValueKind::Integer,
            in_sections: &["neighbor"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "capability",
            value_type: ValueKind::String,
            in_sections: &["neighbor"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "capability-negotiate",
            value_type: ValueKind::String,
            in_sections: &["neighbor"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collide-established",
            value_type: ValueKind::Enum,
            in_sections: &["neighbor"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "connect-timer",
            value_type: ValueKind::Integer,
            in_sections: &["neighbor"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::Boolean,
            in_sections: &["neighbor"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ebgp-multihop",
            value_type: ValueKind::Integer,
            in_sections: &["neighbor"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Enum,
            in_sections: &["neighbor"],
            enum_values: &["false", "true"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforce-multihop",
            value_type: ValueKind::Enum,
            in_sections: &["neighbor"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fall-over",
            value_type: ValueKind::Boolean,
            in_sections: &["neighbor"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "graceful-shutdown",
            value_type: ValueKind::String,
            in_sections: &["neighbor"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hold-time",
            value_type: ValueKind::Integer,
            in_sections: &["neighbor"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "keep-alive",
            value_type: ValueKind::Integer,
            in_sections: &["neighbor"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "local-as",
            value_type: ValueKind::Integer,
            in_sections: &["neighbor"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "passive",
            value_type: ValueKind::Enum,
            in_sections: &["neighbor"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "password",
            value_type: ValueKind::Boolean,
            in_sections: &["neighbor"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "peer-group",
            value_type: ValueKind::Boolean,
            in_sections: &["neighbor"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port",
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
            name: "restart-time",
            value_type: ValueKind::Integer,
            in_sections: &["neighbor"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "update-source",
            value_type: ValueKind::Boolean,
            in_sections: &["neighbor"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "version",
            value_type: ValueKind::Integer,
            in_sections: &["neighbor"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "vlan",
            value_type: ValueKind::Reference,
            in_sections: &["neighbor"],
            allow_none: true,
            references: &["net_vlan"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "network",
            value_type: ValueKind::Reference,
            shape_kind: Some(ValueKind::List),
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "backdoor",
                    value_type: ValueKind::Enum,
                    in_sections: &["network"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "route-map",
                    value_type: ValueKind::Boolean,
                    in_sections: &["network"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "backdoor",
            value_type: ValueKind::Enum,
            in_sections: &["network"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-map",
            value_type: ValueKind::Boolean,
            in_sections: &["network"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "prefix-list",
            value_type: ValueKind::Boolean,
            in_sections: &["orf"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "peer-group",
            value_type: ValueKind::Reference,
            shape_kind: Some(ValueKind::List),
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "address-family",
                    value_type: ValueKind::Reference,
                    in_sections: &["peer-group"],
                    shape_kind: Some(ValueKind::List),
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "advertisement-interval",
                    value_type: ValueKind::Integer,
                    in_sections: &["peer-group"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "allow-infinite-hold-time",
                    value_type: ValueKind::Enum,
                    in_sections: &["peer-group"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "as-origination-interval",
                    value_type: ValueKind::Integer,
                    in_sections: &["peer-group"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "capability",
                    value_type: ValueKind::String,
                    in_sections: &["peer-group"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "capability-negotiate",
                    value_type: ValueKind::String,
                    in_sections: &["peer-group"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "collide-established",
                    value_type: ValueKind::Enum,
                    in_sections: &["peer-group"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "connect-timer",
                    value_type: ValueKind::Integer,
                    in_sections: &["peer-group"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::Boolean,
                    in_sections: &["peer-group"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ebgp-multihop",
                    value_type: ValueKind::Integer,
                    in_sections: &["peer-group"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enabled",
                    value_type: ValueKind::Enum,
                    in_sections: &["peer-group"],
                    enum_values: &["false", "true"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enforce-multihop",
                    value_type: ValueKind::Enum,
                    in_sections: &["peer-group"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "fall-over",
                    value_type: ValueKind::Boolean,
                    in_sections: &["peer-group"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "graceful-shutdown",
                    value_type: ValueKind::String,
                    in_sections: &["peer-group"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "hold-time",
                    value_type: ValueKind::Integer,
                    in_sections: &["peer-group"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "keep-alive",
                    value_type: ValueKind::Integer,
                    in_sections: &["peer-group"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "local-as",
                    value_type: ValueKind::Integer,
                    in_sections: &["peer-group"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "passive",
                    value_type: ValueKind::Enum,
                    in_sections: &["peer-group"],
                    enum_values: &["disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "password",
                    value_type: ValueKind::Boolean,
                    in_sections: &["peer-group"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "port",
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
                    name: "restart-time",
                    value_type: ValueKind::Integer,
                    in_sections: &["peer-group"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "update-source",
                    value_type: ValueKind::Boolean,
                    in_sections: &["peer-group"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "version",
                    value_type: ValueKind::Integer,
                    in_sections: &["peer-group"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "address-family",
            value_type: ValueKind::Reference,
            in_sections: &["peer-group"],
            shape_kind: Some(ValueKind::List),
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "advertisement-interval",
            value_type: ValueKind::Integer,
            in_sections: &["peer-group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "allow-infinite-hold-time",
            value_type: ValueKind::Enum,
            in_sections: &["peer-group"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "as-origination-interval",
            value_type: ValueKind::Integer,
            in_sections: &["peer-group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "capability",
            value_type: ValueKind::String,
            in_sections: &["peer-group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "capability-negotiate",
            value_type: ValueKind::String,
            in_sections: &["peer-group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collide-established",
            value_type: ValueKind::Enum,
            in_sections: &["peer-group"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "connect-timer",
            value_type: ValueKind::Integer,
            in_sections: &["peer-group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::Boolean,
            in_sections: &["peer-group"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ebgp-multihop",
            value_type: ValueKind::Integer,
            in_sections: &["peer-group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Enum,
            in_sections: &["peer-group"],
            enum_values: &["false", "true"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforce-multihop",
            value_type: ValueKind::Enum,
            in_sections: &["peer-group"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fall-over",
            value_type: ValueKind::Boolean,
            in_sections: &["peer-group"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "graceful-shutdown",
            value_type: ValueKind::String,
            in_sections: &["peer-group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hold-time",
            value_type: ValueKind::Integer,
            in_sections: &["peer-group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "keep-alive",
            value_type: ValueKind::Integer,
            in_sections: &["peer-group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "local-as",
            value_type: ValueKind::Integer,
            in_sections: &["peer-group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "passive",
            value_type: ValueKind::Enum,
            in_sections: &["peer-group"],
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "password",
            value_type: ValueKind::Boolean,
            in_sections: &["peer-group"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port",
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
            name: "restart-time",
            value_type: ValueKind::Integer,
            in_sections: &["peer-group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "update-source",
            value_type: ValueKind::Boolean,
            in_sections: &["peer-group"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "version",
            value_type: ValueKind::Integer,
            in_sections: &["peer-group"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "in",
            value_type: ValueKind::Boolean,
            in_sections: &["prefix-list"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "out",
            value_type: ValueKind::Boolean,
            in_sections: &["prefix-list"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "profile",
            value_type: ValueKind::Boolean,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "redistribute",
            value_type: ValueKind::Reference,
            shape_kind: Some(ValueKind::List),
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[BigipPropertySpec {
                name: "route-map",
                value_type: ValueKind::Boolean,
                in_sections: &["redistribute"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-map",
            value_type: ValueKind::Boolean,
            in_sections: &["redistribute"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-domain",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["net_route_domain"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "in",
            value_type: ValueKind::Boolean,
            in_sections: &["route-map"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "out",
            value_type: ValueKind::Boolean,
            in_sections: &["route-map"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "router-id",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scan-time",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "synchronization",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "update-delay",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "view",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
