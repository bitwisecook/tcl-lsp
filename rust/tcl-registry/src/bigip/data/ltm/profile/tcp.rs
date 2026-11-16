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

//! BIG-IP object specs for `ltm profile tcp`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_tcp",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile tcp"],
    },
    header_types: &[("ltm", "profile tcp")],
    properties: &[
        BigipPropertySpec {
            name: "abc",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ack-on-push",
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
            name: "auto-proxy-buffer-size",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auto-receive-window-size",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auto-send-buffer-size",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "close-wait-timeout",
            value_type: ValueKind::Integer,
            default: Some("5 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cmetrics-cache",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cmetrics-cache-timeout",
            value_type: ValueKind::Integer,
            default: Some("0, which defers to the sys db variable route"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "congestion-control",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &[
                "bbr",
                "cdg",
                "chd",
                "cubic",
                "high-speed",
                "illinois",
                "new-reno",
                "none",
                "reno",
                "scalable",
                "vegas",
                "westwood",
                "woodside",
            ],
            default: Some("high-speed"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_profile_tcp"],
            default: Some("tcp"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "deferred-accept",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "delay-window-control",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "delayed-acks",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dsack",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "early-retransmit",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ecn",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enhanced-loss-recovery",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fast-open",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fast-open-cookie-expiration",
            value_type: ValueKind::Integer,
            default: Some("21600 seconds (6 hours)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fin-wait-2-timeout",
            value_type: ValueKind::Integer,
            default: Some("300 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fin-wait-timeout",
            value_type: ValueKind::Integer,
            default: Some("5 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hardware-syn-cookie",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
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
            name: "init-cwnd",
            value_type: ValueKind::Integer,
            default: Some("10"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "init-rwnd",
            value_type: ValueKind::Integer,
            default: Some("10"),
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
            default: Some("0 (zero)"),
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
            default: Some("1800 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "limited-transmit",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "link-qos-to-client",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-retrans",
            value_type: ValueKind::Integer,
            default: Some("8"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-segment-size",
            value_type: ValueKind::Integer,
            default: Some("1460 bytes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "md5-signature",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "md5-signature-passphrase",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "minimum-rto",
            value_type: ValueKind::Integer,
            default: Some("1000 milliseconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mptcp",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled", "passthrough"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mptcp-csum",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mptcp-csum-verify",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mptcp-debug",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mptcp-fallback",
            value_type: ValueKind::Enum,
            enum_values: &["accept", "active-accept", "reset", "retransmit"],
            default: Some("reset"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mptcp-fastjoin",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mptcp-join-max",
            value_type: ValueKind::Integer,
            default: Some("5"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mptcp-makeafterbreak",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mptcp-nojoindssack",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mptcp-rtomax",
            value_type: ValueKind::Integer,
            default: Some("5"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mptcp-rxmitmin",
            value_type: ValueKind::Integer,
            default: Some("1000"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mptcp-subflowmax",
            value_type: ValueKind::Integer,
            default: Some("6"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mptcp-timeout",
            value_type: ValueKind::Integer,
            default: Some("3600"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "nagle",
            value_type: ValueKind::Enum,
            enum_values: &["auto", "disabled", "enabled"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pkt-loss-ignore-burst",
            value_type: ValueKind::Integer,
            default: Some(
                "0 (zero), which means that the system performs congestion control, if any packets are lost",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pkt-loss-ignore-rate",
            value_type: ValueKind::Integer,
            default: Some(
                "0 (zero), which means that the system performs congestion control, if any packet loss occurs",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "proxy-buffer-high",
            value_type: ValueKind::Integer,
            default: Some("131072"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "proxy-buffer-low",
            value_type: ValueKind::Integer,
            default: Some("98304"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "proxy-mss",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "proxy-options",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "push-flag",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["auto", "default", "none", "one"],
            default: Some("default"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rate-pace",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rate-pace-max-rate",
            value_type: ValueKind::Integer,
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "receive-window-size",
            value_type: ValueKind::Integer,
            default: Some("65535 bytes"),
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
            name: "rexmt-thresh",
            value_type: ValueKind::Integer,
            default: Some("3"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "selective-acks",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "selective-nack",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "send-buffer-size",
            value_type: ValueKind::Integer,
            default: Some("131072 bytes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "slow-start",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
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
            name: "syn-cookie-whitelist",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "syn-max-retrans",
            value_type: ValueKind::Integer,
            default: Some("3"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "syn-rto-base",
            value_type: ValueKind::Integer,
            default: Some("3000"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tail-loss-probe",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "time-wait-recycle",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "time-wait-timeout",
            value_type: ValueKind::Integer,
            default: Some("2000 milliseconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "timestamps",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "verified-accept",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "zero-window-timeout",
            value_type: ValueKind::Integer,
            default: Some("20000 milliseconds"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
