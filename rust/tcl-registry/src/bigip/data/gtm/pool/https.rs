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

//! BIG-IP object specs for `gtm pool https`. Hand-maintained: edit them directly.

// Large tmsh bounds appear as bare f64 literals.
#![allow(clippy::unreadable_literal)]
use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "gtm_pool_https",
        table_name: None,
        resolver_name: None,
        module: Some("gtm"),
        object_types: &["pool https"],
    },
    header_types: &[("gtm", "pool https")],
    properties: &[
        BigipPropertySpec {
            name: "alpn",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "alternate-mode",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &[
                "drop-packet",
                "fallback-ip",
                "none",
                "packet-rate",
                "ratio",
                "return-to-dns",
                "round-robin",
                "static-persistence",
                "topology",
                "virtual-server-capacity",
                "virtual-server-score",
            ],
            default: Some("round-robin"),
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
            name: "dynamic",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fallback-mode",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &[
                "completion-rate",
                "cpu",
                "drop-packet",
                "fallback-ip",
                "fewest-hops",
                "kilobytes-per-second",
                "least-connections",
                "lowest-round-trip-time",
                "none",
                "packet-rate",
                "quality-of-service",
                "ratio",
                "return-to-dns",
                "round-robin",
                "static-persistence",
                "topology",
                "virtual-server-capacity",
                "virtual-server-score",
            ],
            default: Some("return-to-dns"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hpke-key",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "load-balancing-mode",
            value_type: ValueKind::Enum,
            enum_values: &[
                "completion-rate",
                "cpu",
                "drop-packet",
                "fallback-ip",
                "fewest-hops",
                "kilobytes-per-second",
                "least-connections",
                "lowest-round-trip-time",
                "packet-rate",
                "quality-of-service",
                "ratio",
                "return-to-dns",
                "round-robin",
                "static-persistence",
                "topology",
                "virtual-server-capacity",
                "virtual-server-score",
            ],
            default: Some("round-robin"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "manual-resume",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-answers-returned",
            value_type: ValueKind::Integer,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "member-order",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "members",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("none"),
            address_grammar: Some(crate::bigip::AddressGrammar::GtmMemberKey),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "metadata",
            value_type: ValueKind::Unknown,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "min-members-up-mode",
            value_type: ValueKind::Enum,
            enum_values: &["number", "off", "percent"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "min-members-up-value",
            value_type: ValueKind::Integer,
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "path",
            value_type: ValueKind::String,
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
            name: "qos-hit-ratio",
            value_type: ValueKind::Integer,
            default: Some("5"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-hops",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-kilobytes-second",
            value_type: ValueKind::Integer,
            default: Some("3"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-lcs",
            value_type: ValueKind::Integer,
            default: Some("30"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-packet-rate",
            value_type: ValueKind::Integer,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-rtt",
            value_type: ValueKind::Integer,
            default: Some("50"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-topology",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-vs-capacity",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "qos-vs-score",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ttl",
            value_type: ValueKind::Integer,
            min_value: Some(0f64),
            max_value: Some(4294967295f64),
            default: Some("30"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "verify-member-availability",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
