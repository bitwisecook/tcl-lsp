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

//! BIG-IP object specs for `security dos device-config`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "security_dos_device_config",
        table_name: None,
        resolver_name: None,
        module: Some("security"),
        object_types: &["dos device-config"],
    },
    header_types: &[("security", "dos device-config")],
    properties: &[
        BigipPropertySpec {
            name: "auto-threshold-sensitivity",
            value_type: ValueKind::Unknown,
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        },
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
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "threshold-mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["custom-signatures"],
                    enum_values: &[
                        "fully-automatic",
                        "manual",
                        "manual-multiplier-mitigation",
                        "stress-based-mitigation",
                    ],
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
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "threshold-mode",
            value_type: ValueKind::Enum,
            in_sections: &["custom-signatures"],
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
            name: "dns-dos-mitigation-percentage",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dns-nxdomain-stat",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dos-device-vector",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "allow-advertisement",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "allow-upstream-scrubbing",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "attacked-dst",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "auto-blacklisting",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "auto-scrubbing",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "auto-threshold",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bad-actor",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "blacklist-category",
                    value_type: ValueKind::Reference,
                    in_sections: &["dos-device-vector"],
                    references: &["security_ip_intelligence_blacklist_category"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "blacklist-detection-seconds",
                    value_type: ValueKind::Integer,
                    in_sections: &["dos-device-vector"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "blacklist-duration",
                    value_type: ValueKind::Integer,
                    in_sections: &["dos-device-vector"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ceiling",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["infinite"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "default-internal-rate-limit",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["infinite"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "detection-threshold-percent",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["infinite"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "detection-threshold-pps",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["infinite"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enforce",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "floor",
                    value_type: ValueKind::Integer,
                    in_sections: &["dos-device-vector"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "packet-types",
                    value_type: ValueKind::List,
                    in_sections: &["dos-device-vector"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "per-dst-ip-detection-pps",
                    value_type: ValueKind::Integer,
                    in_sections: &["dos-device-vector"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "per-dst-ip-limit-pps",
                    value_type: ValueKind::Integer,
                    in_sections: &["dos-device-vector"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "per-source-ip-detection-pps",
                    value_type: ValueKind::Integer,
                    in_sections: &["dos-device-vector"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "per-source-ip-limit-pps",
                    value_type: ValueKind::Integer,
                    in_sections: &["dos-device-vector"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scrubbing-category",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["\"none\""],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scrubbing-detection-seconds",
                    value_type: ValueKind::Integer,
                    in_sections: &["dos-device-vector"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scrubbing-duration",
                    value_type: ValueKind::Integer,
                    in_sections: &["dos-device-vector"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "simulate-auto-threshold",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["disable", "enable"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "state",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["detect-only", "disabled", "learn-only", "mitigate"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "suspicious",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "threshold-mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
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
                    name: "tscookie",
                    value_type: ValueKind::Enum,
                    in_sections: &["dos-device-vector"],
                    enum_values: &["disable", "enable"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "valid-domains",
                    value_type: ValueKind::List,
                    in_sections: &["dos-device-vector"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "allow-advertisement",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "allow-upstream-scrubbing",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "attacked-dst",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auto-blacklisting",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auto-scrubbing",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auto-threshold",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bad-actor",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "blacklist-category",
            value_type: ValueKind::Reference,
            in_sections: &["dos-device-vector"],
            references: &["security_ip_intelligence_blacklist_category"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "blacklist-detection-seconds",
            value_type: ValueKind::Integer,
            in_sections: &["dos-device-vector"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "blacklist-duration",
            value_type: ValueKind::Integer,
            in_sections: &["dos-device-vector"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ceiling",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["infinite"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "default-internal-rate-limit",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["infinite"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "detection-threshold-percent",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["infinite"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "detection-threshold-pps",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["infinite"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforce",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "floor",
            value_type: ValueKind::Integer,
            in_sections: &["dos-device-vector"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "packet-types",
            value_type: ValueKind::List,
            in_sections: &["dos-device-vector"],
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "per-dst-ip-detection-pps",
            value_type: ValueKind::Integer,
            in_sections: &["dos-device-vector"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "per-dst-ip-limit-pps",
            value_type: ValueKind::Integer,
            in_sections: &["dos-device-vector"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "per-source-ip-detection-pps",
            value_type: ValueKind::Integer,
            in_sections: &["dos-device-vector"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "per-source-ip-limit-pps",
            value_type: ValueKind::Integer,
            in_sections: &["dos-device-vector"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scrubbing-category",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["\"none\""],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scrubbing-detection-seconds",
            value_type: ValueKind::Integer,
            in_sections: &["dos-device-vector"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scrubbing-duration",
            value_type: ValueKind::Integer,
            in_sections: &["dos-device-vector"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "simulate-auto-threshold",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["disable", "enable"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "state",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["detect-only", "disabled", "learn-only", "mitigate"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "suspicious",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "threshold-mode",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
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
            name: "tscookie",
            value_type: ValueKind::Enum,
            in_sections: &["dos-device-vector"],
            enum_values: &["disable", "enable"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "valid-domains",
            value_type: ValueKind::List,
            in_sections: &["dos-device-vector"],
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dynamic-signatures",
            value_type: ValueKind::List,
            block: &[
                BigipPropertySpec {
                    name: "detection",
                    value_type: ValueKind::Enum,
                    in_sections: &["dynamic-signatures"],
                    enum_values: &["disabled", "enabled", "learn-only"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "dns",
                    value_type: ValueKind::List,
                    in_sections: &["dynamic-signatures"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "mitigation",
                    value_type: ValueKind::Enum,
                    in_sections: &["dynamic-signatures"],
                    allow_none: true,
                    enum_values: &["high", "low", "medium", "none"],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "network",
                    value_type: ValueKind::Unknown,
                    in_sections: &["dynamic-signatures"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scrubber-advertisement-period",
                    value_type: ValueKind::Integer,
                    in_sections: &["dynamic-signatures"],
                    default: Some("300 seconds"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scrubber-category",
                    value_type: ValueKind::Reference,
                    in_sections: &["dynamic-signatures"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scrubber-enable",
                    value_type: ValueKind::Enum,
                    in_sections: &["dynamic-signatures"],
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
            in_sections: &["dynamic-signatures"],
            enum_values: &["disabled", "enabled", "learn-only"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dns",
            value_type: ValueKind::List,
            in_sections: &["dynamic-signatures"],
            block: &[
                BigipPropertySpec {
                    name: "detection",
                    value_type: ValueKind::Enum,
                    in_sections: &["dynamic-signatures", "dns"],
                    enum_values: &["disabled", "enabled", "learn-only"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "mitigation",
                    value_type: ValueKind::Enum,
                    in_sections: &["dynamic-signatures", "dns"],
                    allow_none: true,
                    enum_values: &["high", "low", "manual-multiplier", "medium", "none"],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "detection",
            value_type: ValueKind::Enum,
            in_sections: &["dynamic-signatures", "dns"],
            enum_values: &["disabled", "enabled", "learn-only"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mitigation",
            value_type: ValueKind::Enum,
            in_sections: &["dynamic-signatures", "dns"],
            allow_none: true,
            enum_values: &["high", "low", "manual-multiplier", "medium", "none"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mitigation",
            value_type: ValueKind::Enum,
            in_sections: &["dynamic-signatures"],
            allow_none: true,
            enum_values: &["high", "low", "medium", "none"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "network",
            value_type: ValueKind::Unknown,
            in_sections: &["dynamic-signatures"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "detection",
                    value_type: ValueKind::Enum,
                    in_sections: &["dynamic-signatures", "network"],
                    enum_values: &["disabled", "enabled", "learn-only"],
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "mitigation",
                    value_type: ValueKind::Enum,
                    in_sections: &["dynamic-signatures", "network"],
                    allow_none: true,
                    enum_values: &["high", "low", "manual-multiplier", "medium", "none"],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scrubber-advertisement-period",
                    value_type: ValueKind::Integer,
                    in_sections: &["dynamic-signatures", "network"],
                    default: Some("300 seconds"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scrubber-category",
                    value_type: ValueKind::Reference,
                    in_sections: &["dynamic-signatures", "network"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scrubber-enable",
                    value_type: ValueKind::Enum,
                    in_sections: &["dynamic-signatures", "network"],
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
            in_sections: &["dynamic-signatures", "network"],
            enum_values: &["disabled", "enabled", "learn-only"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mitigation",
            value_type: ValueKind::Enum,
            in_sections: &["dynamic-signatures", "network"],
            allow_none: true,
            enum_values: &["high", "low", "manual-multiplier", "medium", "none"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scrubber-advertisement-period",
            value_type: ValueKind::Integer,
            in_sections: &["dynamic-signatures", "network"],
            default: Some("300 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scrubber-category",
            value_type: ValueKind::Reference,
            in_sections: &["dynamic-signatures", "network"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scrubber-enable",
            value_type: ValueKind::Enum,
            in_sections: &["dynamic-signatures", "network"],
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("no"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scrubber-advertisement-period",
            value_type: ValueKind::Integer,
            in_sections: &["dynamic-signatures"],
            default: Some("300 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scrubber-category",
            value_type: ValueKind::Reference,
            in_sections: &["dynamic-signatures"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scrubber-enable",
            value_type: ValueKind::Enum,
            in_sections: &["dynamic-signatures"],
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("no"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-uncommon-protolist",
            value_type: ValueKind::String,
            default: Some("/Common/ip-uncommon-protolist"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-publisher",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "network-dos-mitigation-percentage",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sip-dos-mitigation-percentage",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "syn-cookie-dsr-flow-reset-by",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["bigip", "client", "none"],
            default: Some("none"),
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
            name: "threshold-sensitivity",
            value_type: ValueKind::Enum,
            enum_values: &["high", "low", "medium"],
            default: Some("set to medium"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
