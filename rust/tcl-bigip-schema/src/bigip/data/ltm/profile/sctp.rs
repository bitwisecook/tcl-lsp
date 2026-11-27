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

//! BIG-IP object specs for `ltm profile sctp`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_sctp",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile sctp"],
    },
    header_types: &[("ltm", "profile sctp")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cookie-expiration",
            value_type: ValueKind::Integer,
            default: Some("60 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            references: &["ltm_profile_sctp"],
            default: Some("sctp"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "heartbeat-interval",
            value_type: ValueKind::Integer,
            default: Some("30 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "heartbeat-max-burst",
            value_type: ValueKind::Integer,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "idle-timeout",
            value_type: ValueKind::Integer,
            default: Some("300 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "in-streams",
            value_type: ValueKind::Integer,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "init-max-retries",
            value_type: ValueKind::Integer,
            default: Some("4"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-tos",
            value_type: ValueKind::Integer,
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "link-qos",
            value_type: ValueKind::Integer,
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-burst",
            value_type: ValueKind::Integer,
            default: Some("4"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "out-streams",
            value_type: ValueKind::Integer,
            default: Some("2"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "proxy-buffer-high",
            value_type: ValueKind::Integer,
            default: Some("16384"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "proxy-buffer-low",
            value_type: ValueKind::Integer,
            default: Some("4096"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "receive-chunks",
            value_type: ValueKind::Integer,
            default: Some("256"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "receive-ordered",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "receive-window-size",
            value_type: ValueKind::Integer,
            default: Some("65535"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "reset-on-timeout",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rto-initial",
            value_type: ValueKind::Integer,
            default: Some("3000 milliseconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rto-max",
            value_type: ValueKind::Integer,
            default: Some("60000 milliseconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rto-min",
            value_type: ValueKind::Integer,
            default: Some("1000 milliseconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sack-timeout",
            value_type: ValueKind::Integer,
            default: Some("200 milliseconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "secret",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "send-buffer-size",
            value_type: ValueKind::Integer,
            default: Some("65536"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "send-max-retries",
            value_type: ValueKind::Integer,
            default: Some("8"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "send-partial",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-shutdown",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "transmit-chunks",
            value_type: ValueKind::Integer,
            default: Some("256"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
