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

//! Properties `custom-signatures` to `protocol-dns` of `security dos profile`, each with the properties of its sections.

use crate::bigip::{BigipPropertySpec, ValueKind};

pub(super) const PROPERTIES: &[BigipPropertySpec] = &[
    BigipPropertySpec {
        name: "custom-signatures",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "manual-detection-threshold",
                value_type: ValueKind::Integer,
                in_sections: &["custom-signatures"],
                default: Some("infinite"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "manual-mitigation-threshold",
                value_type: ValueKind::Integer,
                in_sections: &["custom-signatures"],
                default: Some("infinite"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Enum,
                in_sections: &["custom-signatures"],
                enum_values: &["detect-only", "disabled", "learn-only", "mitigate"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "threshold-mode",
                value_type: ValueKind::Enum,
                in_sections: &["custom-signatures"],
                enum_values: &["fully-automatic", "manual", "stress-based-mitigation"],
                default: Some("manual"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "manual-detection-threshold",
        value_type: ValueKind::Integer,
        in_sections: &["custom-signatures"],
        default: Some("infinite"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "manual-mitigation-threshold",
        value_type: ValueKind::Integer,
        in_sections: &["custom-signatures"],
        default: Some("infinite"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "state",
        value_type: ValueKind::Enum,
        in_sections: &["custom-signatures"],
        enum_values: &["detect-only", "disabled", "learn-only", "mitigate"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "threshold-mode",
        value_type: ValueKind::Enum,
        in_sections: &["custom-signatures"],
        enum_values: &["fully-automatic", "manual", "stress-based-mitigation"],
        default: Some("manual"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "description",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dos-dnsnxdomain-stat",
        value_type: ValueKind::Unknown,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dos-network",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "dynamic-signatures",
                value_type: ValueKind::List,
                in_sections: &["dos-network"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multiplier-mitigation-percentage",
                value_type: ValueKind::Integer,
                in_sections: &["dos-network"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "network-attack-vector",
                value_type: ValueKind::List,
                in_sections: &["dos-network"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dynamic-signatures",
        value_type: ValueKind::List,
        in_sections: &["dos-network"],
        block: &[
            BigipPropertySpec {
                name: "detection",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "dynamic-signatures"],
                enum_values: &["disabled", "enabled", "learn-only"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mitigation",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "dynamic-signatures"],
                allow_none: true,
                enum_values: &["high", "low", "manual-multiplier", "medium", "none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scrubber-advertisement-period",
                value_type: ValueKind::Integer,
                in_sections: &["dos-network", "dynamic-signatures"],
                default: Some("300 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scrubber-category",
                value_type: ValueKind::Reference,
                in_sections: &["dos-network", "dynamic-signatures"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scrubber-enable",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "dynamic-signatures"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "detection",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "dynamic-signatures"],
        enum_values: &["disabled", "enabled", "learn-only"],
        default: Some("disabled"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "mitigation",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "dynamic-signatures"],
        allow_none: true,
        enum_values: &["high", "low", "manual-multiplier", "medium", "none"],
        default: Some("none"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "scrubber-advertisement-period",
        value_type: ValueKind::Integer,
        in_sections: &["dos-network", "dynamic-signatures"],
        default: Some("300 seconds"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "scrubber-category",
        value_type: ValueKind::Reference,
        in_sections: &["dos-network", "dynamic-signatures"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "scrubber-enable",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "dynamic-signatures"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("no"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "multiplier-mitigation-percentage",
        value_type: ValueKind::Integer,
        in_sections: &["dos-network"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "network-attack-vector",
        value_type: ValueKind::List,
        in_sections: &["dos-network"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "allow-upstream-scrubbing",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "network-attack-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attack-type",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "network-attack-vector"],
                enum_values: &[
                    "ext-hdr-too-large",
                    "hop-cnt-low",
                    "host-unreachable",
                    "icmp-frag",
                    "icmpv4-flood",
                    "icmpv6-flood",
                    "ip-frag-flood",
                    "ip-opt-frames",
                    "ipv6-ext-hdr-frames",
                    "ipv6-frag-flood",
                    "non-tcp-connection",
                    "opt-present-with-illegal-len",
                    "sweep",
                    "tcp-bad-urg",
                    "tcp-half-open",
                    "tcp-opt-overruns-tcp-hdr",
                    "tcp-psh-flood",
                    "tcp-rst-flood",
                    "tcp-syn-flood",
                    "tcp-syn-oversize",
                    "tcp-synack-flood",
                    "tcp-window-size",
                    "tidcmp",
                    "too-many-ext-hdrs",
                    "udp-flood",
                    "unk-tcp-opt-type",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attacked-dst",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "network-attack-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-blacklisting",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "network-attack-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-scrubbing",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "network-attack-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-threshold",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "network-attack-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bad-actor",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "network-attack-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blacklist-category",
                value_type: ValueKind::Reference,
                in_sections: &["dos-network", "network-attack-vector"],
                references: &["security_ip_intelligence_blacklist_category"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blacklist-detection-seconds",
                value_type: ValueKind::Integer,
                in_sections: &["dos-network", "network-attack-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blacklist-duration",
                value_type: ValueKind::Integer,
                in_sections: &["dos-network", "network-attack-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "network-attack-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multiplier-mitigation-percentage",
                value_type: ValueKind::Integer,
                in_sections: &["dos-network", "network-attack-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "packet-types",
                value_type: ValueKind::Unknown,
                in_sections: &["dos-network", "network-attack-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "per-dst-ip-detection-pps",
                value_type: ValueKind::Integer,
                in_sections: &["dos-network", "network-attack-vector"],
                default: Some("infinite"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "per-dst-ip-limit-pps",
                value_type: ValueKind::Integer,
                in_sections: &["dos-network", "network-attack-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "per-source-ip-detection-pps",
                value_type: ValueKind::Integer,
                in_sections: &["dos-network", "network-attack-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "per-source-ip-limit-pps",
                value_type: ValueKind::Integer,
                in_sections: &["dos-network", "network-attack-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-increase",
                value_type: ValueKind::Integer,
                in_sections: &["dos-network", "network-attack-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-limit",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "network-attack-vector"],
                enum_values: &["infinite"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-threshold",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "network-attack-vector"],
                enum_values: &["infinite"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scrubbing-category",
                value_type: ValueKind::Reference,
                in_sections: &["dos-network", "network-attack-vector"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scrubbing-detection-seconds",
                value_type: ValueKind::Integer,
                in_sections: &["dos-network", "network-attack-vector"],
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scrubbing-duration",
                value_type: ValueKind::Integer,
                in_sections: &["dos-network", "network-attack-vector"],
                default: Some("900"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "network-attack-vector"],
                enum_values: &["detect-only", "disabled", "learn-only", "mitigate"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "suspicious",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "network-attack-vector"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "threshold-mode",
                value_type: ValueKind::Enum,
                in_sections: &["dos-network", "network-attack-vector"],
                enum_values: &["fully-automatic", "manual", "stress-based-mitigation"],
                default: Some("manual"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "allow-upstream-scrubbing",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "network-attack-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("disabled"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "attack-type",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "network-attack-vector"],
        enum_values: &[
            "ext-hdr-too-large",
            "hop-cnt-low",
            "host-unreachable",
            "icmp-frag",
            "icmpv4-flood",
            "icmpv6-flood",
            "ip-frag-flood",
            "ip-opt-frames",
            "ipv6-ext-hdr-frames",
            "ipv6-frag-flood",
            "non-tcp-connection",
            "opt-present-with-illegal-len",
            "sweep",
            "tcp-bad-urg",
            "tcp-half-open",
            "tcp-opt-overruns-tcp-hdr",
            "tcp-psh-flood",
            "tcp-rst-flood",
            "tcp-syn-flood",
            "tcp-syn-oversize",
            "tcp-synack-flood",
            "tcp-window-size",
            "tidcmp",
            "too-many-ext-hdrs",
            "udp-flood",
            "unk-tcp-opt-type",
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "attacked-dst",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "network-attack-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("disabled"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "auto-blacklisting",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "network-attack-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "auto-scrubbing",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "network-attack-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("disabled"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "auto-threshold",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "network-attack-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("disabled"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "bad-actor",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "network-attack-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-category",
        value_type: ValueKind::Reference,
        in_sections: &["dos-network", "network-attack-vector"],
        references: &["security_ip_intelligence_blacklist_category"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-detection-seconds",
        value_type: ValueKind::Integer,
        in_sections: &["dos-network", "network-attack-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-duration",
        value_type: ValueKind::Integer,
        in_sections: &["dos-network", "network-attack-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "enforce",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "network-attack-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "multiplier-mitigation-percentage",
        value_type: ValueKind::Integer,
        in_sections: &["dos-network", "network-attack-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "packet-types",
        value_type: ValueKind::Unknown,
        in_sections: &["dos-network", "network-attack-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "per-dst-ip-detection-pps",
        value_type: ValueKind::Integer,
        in_sections: &["dos-network", "network-attack-vector"],
        default: Some("infinite"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "per-dst-ip-limit-pps",
        value_type: ValueKind::Integer,
        in_sections: &["dos-network", "network-attack-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "per-source-ip-detection-pps",
        value_type: ValueKind::Integer,
        in_sections: &["dos-network", "network-attack-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "per-source-ip-limit-pps",
        value_type: ValueKind::Integer,
        in_sections: &["dos-network", "network-attack-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rate-increase",
        value_type: ValueKind::Integer,
        in_sections: &["dos-network", "network-attack-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rate-limit",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "network-attack-vector"],
        enum_values: &["infinite"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rate-threshold",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "network-attack-vector"],
        enum_values: &["infinite"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "scrubbing-category",
        value_type: ValueKind::Reference,
        in_sections: &["dos-network", "network-attack-vector"],
        allow_none: true,
        default: Some("none"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "scrubbing-detection-seconds",
        value_type: ValueKind::Integer,
        in_sections: &["dos-network", "network-attack-vector"],
        default: Some("10"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "scrubbing-duration",
        value_type: ValueKind::Integer,
        in_sections: &["dos-network", "network-attack-vector"],
        default: Some("900"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "state",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "network-attack-vector"],
        enum_values: &["detect-only", "disabled", "learn-only", "mitigate"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "suspicious",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "network-attack-vector"],
        enum_values: &["false", "true"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("false"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "threshold-mode",
        value_type: ValueKind::Enum,
        in_sections: &["dos-network", "network-attack-vector"],
        enum_values: &["fully-automatic", "manual", "stress-based-mitigation"],
        default: Some("manual"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "http-whitelist",
        value_type: ValueKind::Reference,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "protocol-dns",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "dns-query-vector",
                value_type: ValueKind::List,
                in_sections: &["protocol-dns"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multiplier-mitigation-percentage",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-dns"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prot-err-atck-rate-incr",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-dns"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prot-err-attack-detection",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-dns"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dns-query-vector",
        value_type: ValueKind::List,
        in_sections: &["protocol-dns"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "allow-upstream-scrubbing",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "dns-query-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attacked-dst",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "dns-query-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-blacklisting",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "dns-query-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-scrubbing",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "dns-query-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-threshold",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "dns-query-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bad-actor",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "dns-query-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blacklist-category",
                value_type: ValueKind::Reference,
                in_sections: &["protocol-dns", "dns-query-vector"],
                references: &["security_ip_intelligence_blacklist_category"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blacklist-detection-seconds",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-dns", "dns-query-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blacklist-duration",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-dns", "dns-query-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "dns-query-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multiplier-mitigation-percentage",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-dns", "dns-query-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "per-dst-ip-detection-pps",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-dns", "dns-query-vector"],
                default: Some("infinite"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "per-dst-ip-limit-pps",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-dns", "dns-query-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "per-source-ip-detection-pps",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-dns", "dns-query-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "per-source-ip-limit-pps",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-dns", "dns-query-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "query-type",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "dns-query-vector"],
                enum_values: &[
                    "a", "aaaa", "any", "axfr", "ixfr", "mx", "ns", "nxdomain", "other", "ptr",
                    "soa", "srv", "txt",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-increase",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-dns", "dns-query-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-limit",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "dns-query-vector"],
                enum_values: &["infinite"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-threshold",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "dns-query-vector"],
                enum_values: &["infinite"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scrubbing-category",
                value_type: ValueKind::Reference,
                in_sections: &["protocol-dns", "dns-query-vector"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scrubbing-detection-seconds",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-dns", "dns-query-vector"],
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scrubbing-duration",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-dns", "dns-query-vector"],
                default: Some("900"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "dns-query-vector"],
                enum_values: &["detect-only", "disabled", "learn-only", "mitigate"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "suspicious",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "dns-query-vector"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "threshold-mode",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-dns", "dns-query-vector"],
                enum_values: &["fully-automatic", "manual", "stress-based-mitigation"],
                default: Some("manual"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "valid-domains",
                value_type: ValueKind::List,
                in_sections: &["protocol-dns", "dns-query-vector"],
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "allow-upstream-scrubbing",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "dns-query-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("disabled"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "attacked-dst",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "dns-query-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("disabled"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "auto-blacklisting",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "dns-query-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "auto-scrubbing",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "dns-query-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("disabled"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "auto-threshold",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "dns-query-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("disabled"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "bad-actor",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "dns-query-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-category",
        value_type: ValueKind::Reference,
        in_sections: &["protocol-dns", "dns-query-vector"],
        references: &["security_ip_intelligence_blacklist_category"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-detection-seconds",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-dns", "dns-query-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-duration",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-dns", "dns-query-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "enforce",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "dns-query-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "multiplier-mitigation-percentage",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-dns", "dns-query-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "per-dst-ip-detection-pps",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-dns", "dns-query-vector"],
        default: Some("infinite"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "per-dst-ip-limit-pps",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-dns", "dns-query-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "per-source-ip-detection-pps",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-dns", "dns-query-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "per-source-ip-limit-pps",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-dns", "dns-query-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "query-type",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "dns-query-vector"],
        enum_values: &[
            "a", "aaaa", "any", "axfr", "ixfr", "mx", "ns", "nxdomain", "other", "ptr", "soa",
            "srv", "txt",
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rate-increase",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-dns", "dns-query-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rate-limit",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "dns-query-vector"],
        enum_values: &["infinite"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rate-threshold",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "dns-query-vector"],
        enum_values: &["infinite"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "scrubbing-category",
        value_type: ValueKind::Reference,
        in_sections: &["protocol-dns", "dns-query-vector"],
        allow_none: true,
        default: Some("none"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "scrubbing-detection-seconds",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-dns", "dns-query-vector"],
        default: Some("10"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "scrubbing-duration",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-dns", "dns-query-vector"],
        default: Some("900"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "state",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "dns-query-vector"],
        enum_values: &["detect-only", "disabled", "learn-only", "mitigate"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "suspicious",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "dns-query-vector"],
        enum_values: &["false", "true"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("false"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "threshold-mode",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-dns", "dns-query-vector"],
        enum_values: &["fully-automatic", "manual", "stress-based-mitigation"],
        default: Some("manual"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "valid-domains",
        value_type: ValueKind::List,
        in_sections: &["protocol-dns", "dns-query-vector"],
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "multiplier-mitigation-percentage",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-dns"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "prot-err-atck-rate-incr",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-dns"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "prot-err-attack-detection",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-dns"],
        ..BigipPropertySpec::DEFAULT
    },
];
