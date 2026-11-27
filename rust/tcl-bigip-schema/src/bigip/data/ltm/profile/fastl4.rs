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

//! BIG-IP object specs for `ltm profile fastl4`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_fastl4",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile fastl4"],
    },
    header_types: &[("ltm", "profile fastl4")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-timeout",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_profile_fastl4"],
            default: Some("fastl4"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "explicit-flow-migration",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hardware-syn-cookie",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "idle-timeout",
            value_type: ValueKind::Integer,
            default: Some("300 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-df-mode",
            value_type: ValueKind::Enum,
            enum_values: &["clear", "preserve", "set"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-tos-to-client",
            value_type: ValueKind::Integer,
            default: Some("65535, which indicates, do not modify"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-tos-to-server",
            value_type: ValueKind::Integer,
            default: Some("65535, which indicates, do not modify"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-ttl-mode",
            value_type: ValueKind::Enum,
            enum_values: &["decrement", "preserve", "proxy", "set"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-ttl-value",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "keep-alive-interval",
            value_type: ValueKind::Integer,
            default: Some("disabled (0 seconds)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "late-binding",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "link-qos-to-client",
            value_type: ValueKind::Integer,
            default: Some("65535, which indicates, do not modify"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "link-qos-to-server",
            value_type: ValueKind::Integer,
            default: Some("65535, which indicates, do not modify"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "loose-close",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "loose-initialization",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mss-override",
            value_type: ValueKind::Integer,
            default: Some("0 (zero), which disables this option"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "other-pva-clientpkts-threshold",
            value_type: ValueKind::Integer,
            default: Some("2"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "other-pva-offload-direction",
            value_type: ValueKind::Enum,
            enum_values: &[
                "bidirectional",
                "client-to-server-only",
                "server-to-client-only",
            ],
            default: Some(
                "bidirectional which implies both side is permitted to offload if threshold exceeds",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "other-pva-serverpkts-threshold",
            value_type: ValueKind::Integer,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "other-pva-whento-offload",
            value_type: ValueKind::Enum,
            enum_values: &[
                "after-packets-both-direction",
                "after-packets-per-direction",
            ],
            default: Some(
                "after-packets-per-direction and implies the client and server traffic is offloaded independently after exceeding their own thresholds",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "priority-to-client",
            value_type: ValueKind::Integer,
            default: Some("65535, which indicates, do not modify"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "priority-to-server",
            value_type: ValueKind::Integer,
            default: Some("65535, which indicates, do not modify"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pva-acceleration",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["dedicated", "full", "none", "partial"],
            default: Some("full"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pva-dynamic-client-packets",
            value_type: ValueKind::Integer,
            default: Some("2"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pva-dynamic-server-packets",
            value_type: ValueKind::Integer,
            default: Some("2"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pva-flow-aging",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pva-flow-evict",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pva-offload-dynamic",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pva-offload-dynamic-priority",
            value_type: ValueKind::Enum,
            enum_values: &["disable", "enable"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pva-offload-initial-priority",
            value_type: ValueKind::Enum,
            enum_values: &["high", "low", "medium"],
            default: Some("medium"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pva-offload-state",
            value_type: ValueKind::Enum,
            enum_values: &["embryonic", "establish"],
            default: Some("embryonic and implies at TCP CSYN or the first client UDP packet"),
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "reassemble-fragments",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "receive-window-size",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "reset-on-client-fin",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "reset-on-timeout",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rtt-from-client",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rtt-from-server",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "server-sack",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "server-timestamp",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "software-syn-cookie",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "syn-cookie-dsr-flow-reset-by",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["bigip", "client", "none"],
            default: Some(
                "none, which is backward-compatible with syn-cookie-whitelist feature in non-DSR mode",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "syn-cookie-enable",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "syn-cookie-mss",
            value_type: ValueKind::Integer,
            default: Some("0 (zero), which disables this option"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "syn-cookie-whitelist",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-close-timeout",
            value_type: ValueKind::Integer,
            default: Some("5 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-generate-is",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-handshake-timeout",
            value_type: ValueKind::Integer,
            default: Some("5 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-pva-offload-direction",
            value_type: ValueKind::Enum,
            enum_values: &[
                "bidirectional",
                "client-to-server-only",
                "server-to-client-only",
            ],
            default: Some(
                "bidirectional which implies both side is permitted to offload if threshold exceeds",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-pva-whento-offload",
            value_type: ValueKind::Enum,
            enum_values: &["embryonic", "establish"],
            default: Some("embryonic and implies at TCP SYN packet"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-strip-sack",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-time-wait-timeout",
            value_type: ValueKind::Integer,
            default: Some("0 milliseconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-timestamp-mode",
            value_type: ValueKind::Enum,
            enum_values: &["preserve", "rewrite", "strip"],
            default: Some("preserve"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-wscale-mode",
            value_type: ValueKind::Enum,
            enum_values: &["preserve", "rewrite", "strip"],
            default: Some("preserve"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "timeout-recovery",
            value_type: ValueKind::Enum,
            enum_values: &["disconnect", "fallback"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
