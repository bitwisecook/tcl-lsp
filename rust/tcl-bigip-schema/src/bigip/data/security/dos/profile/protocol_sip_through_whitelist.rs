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

//! Properties `protocol-sip` to `whitelist` of `security dos profile`, each with the properties of its sections.

use crate::bigip::{BigipPropertySpec, ValueKind};

pub(super) const PROPERTIES: &[BigipPropertySpec] = &[
    BigipPropertySpec {
        name: "protocol-sip",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "multiplier-mitigation-percentage",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-sip"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prot-err-atck-rate-increase",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-sip"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prot-err-atck-rate-threshold",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-sip"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prot-err-attack-detection",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-sip"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sip-attack-vector",
                value_type: ValueKind::List,
                in_sections: &["protocol-sip"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "multiplier-mitigation-percentage",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-sip"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "prot-err-atck-rate-increase",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-sip"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "prot-err-atck-rate-threshold",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-sip"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "prot-err-attack-detection",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-sip"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "sip-attack-vector",
        value_type: ValueKind::List,
        in_sections: &["protocol-sip"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "allow-upstream-scrubbing",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attacked-dst",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-blacklisting",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-scrubbing",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-threshold",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bad-actor",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blacklist-category",
                value_type: ValueKind::Reference,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                references: &["security_ip_intelligence_blacklist_category"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blacklist-detection-seconds",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blacklist-duration",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multiplier-mitigation-percentage",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "per-dst-ip-detection-pps",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                default: Some("infinite"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "per-dst-ip-limit-pps",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "per-source-ip-detection-pps",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "per-source-ip-limit-pps",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-increase",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-limit",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                enum_values: &["infinite"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-threshold",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                enum_values: &["infinite"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scrubbing-category",
                value_type: ValueKind::Reference,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scrubbing-detection-seconds",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scrubbing-duration",
                value_type: ValueKind::Integer,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                default: Some("900"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                enum_values: &["detect-only", "disabled", "learn-only", "mitigate"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "suspicious",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "threshold-mode",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                enum_values: &[
                    "fully-automatic",
                    "manual",
                    "manual-multiplier-mitigation",
                    "stress-based-mitigation",
                ],
                default: Some("manual"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["protocol-sip", "sip-attack-vector"],
                enum_values: &[
                    "ack",
                    "bye",
                    "cancel",
                    "invite",
                    "message",
                    "notify",
                    "options",
                    "other",
                    "prack",
                    "publish",
                    "register",
                    "subscribe",
                    "uri-limit",
                ],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "allow-upstream-scrubbing",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("disabled"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "attacked-dst",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("disabled"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "auto-blacklisting",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "auto-scrubbing",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("disabled"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "auto-threshold",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("disabled"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "bad-actor",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-category",
        value_type: ValueKind::Reference,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        references: &["security_ip_intelligence_blacklist_category"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-detection-seconds",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-duration",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "enforce",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "multiplier-mitigation-percentage",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "per-dst-ip-detection-pps",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        default: Some("infinite"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "per-dst-ip-limit-pps",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "per-source-ip-detection-pps",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "per-source-ip-limit-pps",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rate-increase",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rate-limit",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        enum_values: &["infinite"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rate-threshold",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        enum_values: &["infinite"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "scrubbing-category",
        value_type: ValueKind::Reference,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        allow_none: true,
        default: Some("none"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "scrubbing-detection-seconds",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        default: Some("10"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "scrubbing-duration",
        value_type: ValueKind::Integer,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        default: Some("900"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "state",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        enum_values: &["detect-only", "disabled", "learn-only", "mitigate"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "suspicious",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        enum_values: &["false", "true"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("false"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "threshold-mode",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        enum_values: &[
            "fully-automatic",
            "manual",
            "manual-multiplier-mitigation",
            "stress-based-mitigation",
        ],
        default: Some("manual"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        in_sections: &["protocol-sip", "sip-attack-vector"],
        enum_values: &[
            "ack",
            "bye",
            "cancel",
            "invite",
            "message",
            "notify",
            "options",
            "other",
            "prack",
            "publish",
            "register",
            "subscribe",
            "uri-limit",
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "whitelist",
        value_type: ValueKind::Reference,
        references: &["security_dos_network_whitelist"],
        ..BigipPropertySpec::DEFAULT
    },
];
