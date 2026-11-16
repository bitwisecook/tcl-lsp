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

//! BIG-IP object specs for `security bot-defense`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_bot_defense_profile",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["bot-defense profile"],
        },
        header_types: &[("security", "bot-defense profile")],
        properties: &[
            BigipPropertySpec {
                name: "anomaly-category-overrides",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "action",
                        value_type: ValueKind::Enum,
                        in_sections: &["anomaly-category-overrides"],
                        allow_none: true,
                        enum_values: &[
                            "alarm",
                            "block",
                            "captcha",
                            "honeypot-page",
                            "none",
                            "redirect-to-pool",
                            "tcp-reset",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["anomaly-category-overrides"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["anomaly-category-overrides"],
                allow_none: true,
                enum_values: &[
                    "alarm",
                    "block",
                    "captcha",
                    "honeypot-page",
                    "none",
                    "redirect-to-pool",
                    "tcp-reset",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["anomaly-category-overrides"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "anomaly-overrides",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "action",
                    value_type: ValueKind::Enum,
                    in_sections: &["anomaly-overrides"],
                    allow_none: true,
                    enum_values: &[
                        "alarm",
                        "block",
                        "captcha",
                        "honeypot-page",
                        "none",
                        "redirect-to-pool",
                        "tcp-reset",
                    ],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["anomaly-overrides"],
                allow_none: true,
                enum_values: &[
                    "alarm",
                    "block",
                    "captcha",
                    "honeypot-page",
                    "none",
                    "redirect-to-pool",
                    "tcp-reset",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-access-strict-mitigation",
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
                name: "blocking-page",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "body",
                        value_type: ValueKind::String,
                        in_sections: &["blocking-page"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "headers",
                        value_type: ValueKind::String,
                        in_sections: &["blocking-page"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "status-code",
                        value_type: ValueKind::Integer,
                        in_sections: &["blocking-page"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "type",
                        value_type: ValueKind::Enum,
                        in_sections: &["blocking-page"],
                        enum_values: &["custom", "default"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "body",
                value_type: ValueKind::String,
                in_sections: &["blocking-page"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "headers",
                value_type: ValueKind::String,
                in_sections: &["blocking-page"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "status-code",
                value_type: ValueKind::Integer,
                in_sections: &["blocking-page"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["blocking-page"],
                enum_values: &["custom", "default"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "captcha-response",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "failure",
                        value_type: ValueKind::Unknown,
                        in_sections: &["captcha-response"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "first",
                        value_type: ValueKind::Unknown,
                        in_sections: &["captcha-response"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure",
                value_type: ValueKind::Unknown,
                in_sections: &["captcha-response"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "body",
                        value_type: ValueKind::String,
                        in_sections: &["captcha-response", "failure"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "type",
                        value_type: ValueKind::Enum,
                        in_sections: &["captcha-response", "failure"],
                        enum_values: &["custom", "default"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "body",
                value_type: ValueKind::String,
                in_sections: &["captcha-response", "failure"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["captcha-response", "failure"],
                enum_values: &["custom", "default"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first",
                value_type: ValueKind::Unknown,
                in_sections: &["captcha-response"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "body",
                        value_type: ValueKind::String,
                        in_sections: &["captcha-response", "first"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "type",
                        value_type: ValueKind::Enum,
                        in_sections: &["captcha-response", "first"],
                        enum_values: &["custom", "default"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "body",
                value_type: ValueKind::String,
                in_sections: &["captcha-response", "first"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["captcha-response", "first"],
                enum_values: &["custom", "default"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "class-overrides",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "mitigation",
                        value_type: ValueKind::Unknown,
                        in_sections: &["class-overrides"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "verification",
                        value_type: ValueKind::Unknown,
                        in_sections: &["class-overrides"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mitigation",
                value_type: ValueKind::Unknown,
                in_sections: &["class-overrides"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "action",
                        value_type: ValueKind::Enum,
                        in_sections: &["class-overrides", "mitigation"],
                        allow_none: true,
                        enum_values: &[
                            "alarm",
                            "block",
                            "captcha",
                            "honeypot-page",
                            "none",
                            "rate-limit",
                            "redirect-to-pool",
                            "tcp-reset",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "rate-limit-tps",
                        value_type: ValueKind::Integer,
                        in_sections: &["class-overrides", "mitigation"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["class-overrides", "mitigation"],
                allow_none: true,
                enum_values: &[
                    "alarm",
                    "block",
                    "captcha",
                    "honeypot-page",
                    "none",
                    "rate-limit",
                    "redirect-to-pool",
                    "tcp-reset",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-limit-tps",
                value_type: ValueKind::Integer,
                in_sections: &["class-overrides", "mitigation"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verification",
                value_type: ValueKind::Unknown,
                in_sections: &["class-overrides"],
                shape_kind: Some(ValueKind::Object),
                block: &[BigipPropertySpec {
                    name: "action",
                    value_type: ValueKind::Enum,
                    in_sections: &["class-overrides", "verification"],
                    allow_none: true,
                    enum_values: &[
                        "browser-challenge-free-verification",
                        "browser-verify-after-access-detection",
                        "browser-verify-before-access",
                        "none",
                    ],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["class-overrides", "verification"],
                allow_none: true,
                enum_values: &[
                    "browser-challenge-free-verification",
                    "browser-verify-after-access-detection",
                    "browser-verify-before-access",
                    "none",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cross-domain-requests",
                value_type: ValueKind::Enum,
                enum_values: &["allow-all", "validate-bulk", "validate-upon-request"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Unknown,
                references: &["security_bot_defense_profile"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "deviceid-mode",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["generate-after-access", "generate-before-access", "none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dos-attack-strict-mitigation",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforcement-mode",
                value_type: ValueKind::Enum,
                enum_values: &["blocking", "transparent"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforcement-readiness-period",
                value_type: ValueKind::Integer,
                default: Some("604800 seconds (7 days)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "external-domains",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "grace-period",
                value_type: ValueKind::Integer,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "micro-services",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "action",
                        value_type: ValueKind::Enum,
                        in_sections: &["micro-services"],
                        allow_none: true,
                        enum_values: &[
                            "alarm",
                            "block",
                            "captcha",
                            "honeypot-page",
                            "none",
                            "redirect-to-pool",
                            "tcp-reset",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "class-overrides",
                        value_type: ValueKind::List,
                        in_sections: &["micro-services"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["micro-services"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "detection-threshold",
                        value_type: ValueKind::Integer,
                        in_sections: &["micro-services"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "detection-time",
                        value_type: ValueKind::Integer,
                        in_sections: &["micro-services"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enforcement-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["micro-services"],
                        enum_values: &["blocking", "profile-default", "transparent"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hostname",
                        value_type: ValueKind::String,
                        in_sections: &["micro-services"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "match-order",
                        value_type: ValueKind::Integer,
                        in_sections: &["micro-services"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "mitigation-time",
                        value_type: ValueKind::Integer,
                        in_sections: &["micro-services"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "type",
                        value_type: ValueKind::Unknown,
                        in_sections: &["micro-services"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "urls",
                        value_type: ValueKind::List,
                        in_sections: &["micro-services"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["micro-services"],
                allow_none: true,
                enum_values: &[
                    "alarm",
                    "block",
                    "captcha",
                    "honeypot-page",
                    "none",
                    "redirect-to-pool",
                    "tcp-reset",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "class-overrides",
                value_type: ValueKind::List,
                in_sections: &["micro-services"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "mitigation",
                        value_type: ValueKind::Unknown,
                        in_sections: &["micro-services", "class-overrides"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "verification",
                        value_type: ValueKind::Unknown,
                        in_sections: &["micro-services", "class-overrides"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mitigation",
                value_type: ValueKind::Unknown,
                in_sections: &["micro-services", "class-overrides"],
                shape_kind: Some(ValueKind::Object),
                block: &[BigipPropertySpec {
                    name: "action",
                    value_type: ValueKind::Enum,
                    in_sections: &["micro-services", "class-overrides", "mitigation"],
                    allow_none: true,
                    enum_values: &[
                        "alarm",
                        "block",
                        "captcha",
                        "honeypot-page",
                        "none",
                        "redirect-to-pool",
                        "tcp-reset",
                    ],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["micro-services", "class-overrides", "mitigation"],
                allow_none: true,
                enum_values: &[
                    "alarm",
                    "block",
                    "captcha",
                    "honeypot-page",
                    "none",
                    "redirect-to-pool",
                    "tcp-reset",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verification",
                value_type: ValueKind::Unknown,
                in_sections: &["micro-services", "class-overrides"],
                shape_kind: Some(ValueKind::Object),
                block: &[BigipPropertySpec {
                    name: "action",
                    value_type: ValueKind::Enum,
                    in_sections: &["micro-services", "class-overrides", "verification"],
                    allow_none: true,
                    enum_values: &[
                        "browser-captcha",
                        "browser-challenge-free-verification",
                        "browser-verify-after-access-detection",
                        "browser-verify-before-access",
                        "none",
                    ],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["micro-services", "class-overrides", "verification"],
                allow_none: true,
                enum_values: &[
                    "browser-captcha",
                    "browser-challenge-free-verification",
                    "browser-verify-after-access-detection",
                    "browser-verify-before-access",
                    "none",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["micro-services"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "detection-threshold",
                value_type: ValueKind::Integer,
                in_sections: &["micro-services"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "detection-time",
                value_type: ValueKind::Integer,
                in_sections: &["micro-services"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforcement-mode",
                value_type: ValueKind::Enum,
                in_sections: &["micro-services"],
                enum_values: &["blocking", "profile-default", "transparent"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hostname",
                value_type: ValueKind::String,
                in_sections: &["micro-services"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-order",
                value_type: ValueKind::Integer,
                in_sections: &["micro-services"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mitigation-time",
                value_type: ValueKind::Integer,
                in_sections: &["micro-services"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Unknown,
                in_sections: &["micro-services"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "urls",
                value_type: ValueKind::List,
                in_sections: &["micro-services"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "match-order",
                        value_type: ValueKind::Integer,
                        in_sections: &["micro-services", "urls"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "url",
                        value_type: ValueKind::String,
                        in_sections: &["micro-services", "urls"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-order",
                value_type: ValueKind::Integer,
                in_sections: &["micro-services", "urls"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::String,
                in_sections: &["micro-services", "urls"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-detection",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "allow-android-rooted-device",
                        value_type: ValueKind::Enum,
                        in_sections: &["mobile-detection"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "allow-any-android-package",
                        value_type: ValueKind::Enum,
                        in_sections: &["mobile-detection"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "allow-any-ios-package",
                        value_type: ValueKind::Enum,
                        in_sections: &["mobile-detection"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "allow-emulators",
                        value_type: ValueKind::Enum,
                        in_sections: &["mobile-detection"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "allow-jailbroken-devices",
                        value_type: ValueKind::Enum,
                        in_sections: &["mobile-detection"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "android-publishers",
                        value_type: ValueKind::List,
                        in_sections: &["mobile-detection"],
                        repeated: true,
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "block-debugger-enabled-device",
                        value_type: ValueKind::Enum,
                        in_sections: &["mobile-detection"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "client-side-challenge-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["mobile-detection"],
                        enum_values: &["cshui", "pass"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ios-allowed-packages",
                        value_type: ValueKind::List,
                        in_sections: &["mobile-detection"],
                        repeated: true,
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "signatures",
                        value_type: ValueKind::List,
                        in_sections: &["mobile-detection"],
                        repeated: true,
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-android-rooted-device",
                value_type: ValueKind::Enum,
                in_sections: &["mobile-detection"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-any-android-package",
                value_type: ValueKind::Enum,
                in_sections: &["mobile-detection"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-any-ios-package",
                value_type: ValueKind::Enum,
                in_sections: &["mobile-detection"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-emulators",
                value_type: ValueKind::Enum,
                in_sections: &["mobile-detection"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-jailbroken-devices",
                value_type: ValueKind::Enum,
                in_sections: &["mobile-detection"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "android-publishers",
                value_type: ValueKind::List,
                in_sections: &["mobile-detection"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "block-debugger-enabled-device",
                value_type: ValueKind::Enum,
                in_sections: &["mobile-detection"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-side-challenge-mode",
                value_type: ValueKind::Enum,
                in_sections: &["mobile-detection"],
                enum_values: &["cshui", "pass"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ios-allowed-packages",
                value_type: ValueKind::List,
                in_sections: &["mobile-detection"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signatures",
                value_type: ValueKind::List,
                in_sections: &["mobile-detection"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "perform-challenge-in-transparent",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signature-category-overrides",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "action",
                    value_type: ValueKind::Enum,
                    in_sections: &["signature-category-overrides"],
                    allow_none: true,
                    enum_values: &[
                        "alarm",
                        "block",
                        "captcha",
                        "honeypot-page",
                        "none",
                        "redirect-to-pool",
                        "tcp-reset",
                    ],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["signature-category-overrides"],
                allow_none: true,
                enum_values: &[
                    "alarm",
                    "block",
                    "captcha",
                    "honeypot-page",
                    "none",
                    "redirect-to-pool",
                    "tcp-reset",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signature-overrides",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "action",
                        value_type: ValueKind::Enum,
                        in_sections: &["signature-overrides"],
                        allow_none: true,
                        enum_values: &[
                            "alarm",
                            "block",
                            "captcha",
                            "honeypot-page",
                            "none",
                            "rate-limit",
                            "redirect-to-pool",
                            "tcp-reset",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "rate-limit-tps",
                        value_type: ValueKind::Integer,
                        in_sections: &["signature-overrides"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["signature-overrides"],
                allow_none: true,
                enum_values: &[
                    "alarm",
                    "block",
                    "captcha",
                    "honeypot-page",
                    "none",
                    "rate-limit",
                    "redirect-to-pool",
                    "tcp-reset",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-limit-tps",
                value_type: ValueKind::Integer,
                in_sections: &["signature-overrides"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signature-staging-upon-update",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "single-page-application",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("Disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-domains",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "staged-signatures",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "template",
                value_type: ValueKind::Enum,
                enum_values: &["balanced", "relaxed", "strict"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "whitelist",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "disable-mitigation",
                        value_type: ValueKind::Enum,
                        in_sections: &["whitelist"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "disable-verification",
                        value_type: ValueKind::Enum,
                        in_sections: &["whitelist"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "geolocation",
                        value_type: ValueKind::Unknown,
                        in_sections: &["whitelist"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "match-order",
                        value_type: ValueKind::Integer,
                        in_sections: &["whitelist"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "source-address",
                        value_type: ValueKind::Unknown,
                        in_sections: &["whitelist"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "url",
                        value_type: ValueKind::String,
                        in_sections: &["whitelist"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "disable-mitigation",
                value_type: ValueKind::Enum,
                in_sections: &["whitelist"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "disable-verification",
                value_type: ValueKind::Enum,
                in_sections: &["whitelist"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geolocation",
                value_type: ValueKind::Unknown,
                in_sections: &["whitelist"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-order",
                value_type: ValueKind::Integer,
                in_sections: &["whitelist"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-address",
                value_type: ValueKind::Unknown,
                in_sections: &["whitelist"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::String,
                in_sections: &["whitelist"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_bot_defense_signature",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["bot-defense signature"],
        },
        header_types: &[("security", "bot-defense signature")],
        properties: &[
            BigipPropertySpec {
                name: "category",
                value_type: ValueKind::Reference,
                references: &[
                    "ltm_classification_category",
                    "ltm_classification_stats_url_category",
                    "ltm_classification_url_category",
                    "security_blacklist_publisher_by_category",
                    "security_blacklist_publisher_category",
                    "security_bot_defense_anomaly_category",
                    "security_bot_defense_signature_category",
                    "security_dos_bot_signature_category",
                    "security_firewall_ipi_category_info",
                    "security_ip_intelligence_blacklist_category",
                    "security_scrubber_dwbl_scrubber_category_stats",
                    "sys_url_db_url_category",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domains",
                value_type: ValueKind::List,
                required: true,
                repeated: true,
                allow_none: true,
                usage_flags: &["optional"],
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "risk",
                value_type: ValueKind::Enum,
                enum_values: &["high", "low", "medium"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rule",
                value_type: ValueKind::String,
                references: &[
                    "gtm_rule",
                    "ltm_cipher_rule",
                    "ltm_global_settings_rule",
                    "ltm_rule",
                    "ltm_rule_profiler",
                    "security_firewall_matching_rule",
                    "security_firewall_on_demand_rule_deploy",
                    "security_firewall_rule_list",
                    "security_firewall_rule_stat",
                    "security_packet_filter_rule_stat",
                    "sys_file_rewrite_rule",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signature-references",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "match-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["url"],
                        enum_values: &["contains"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "search-string",
                        value_type: ValueKind::String,
                        in_sections: &["url"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-type",
                value_type: ValueKind::Enum,
                in_sections: &["url"],
                enum_values: &["contains"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "search-string",
                value_type: ValueKind::String,
                in_sections: &["url"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-agent",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "match-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["user-agent"],
                        enum_values: &["contains"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "search-string",
                        value_type: ValueKind::String,
                        in_sections: &["user-agent"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-type",
                value_type: ValueKind::Enum,
                in_sections: &["user-agent"],
                enum_values: &["contains"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "search-string",
                value_type: ValueKind::String,
                in_sections: &["user-agent"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_bot_defense_signature_category",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["bot-defense signature-category"],
        },
        header_types: &[("security", "bot-defense signature-category")],
        properties: &[BigipPropertySpec {
            name: "class",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
];
