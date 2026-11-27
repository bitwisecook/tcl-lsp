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

//! Properties `application` to `stress-based` of `security dos profile`, each with the properties of its sections.

use crate::bigip::{BigipPropertySpec, ValueKind};

pub(super) const PROPERTIES: &[BigipPropertySpec] = &[
    BigipPropertySpec {
        name: "application",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "bot-defense",
                value_type: ValueKind::Unknown,
                in_sections: &["application"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bot-signatures",
                value_type: ValueKind::List,
                in_sections: &["application"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "captcha-response",
                value_type: ValueKind::Unknown,
                in_sections: &["application"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fastl4-acceleration-profile",
                value_type: ValueKind::Reference,
                in_sections: &["application"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geolocations",
                value_type: ValueKind::List,
                in_sections: &["application"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "heavy-urls",
                value_type: ValueKind::List,
                in_sections: &["application"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-whitelist",
                value_type: ValueKind::List,
                in_sections: &["application"],
                allow_none: true,
                usage_flags: &["deprecated"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rtbh-duration-sec",
                value_type: ValueKind::Integer,
                in_sections: &["application"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rtbh-enable",
                value_type: ValueKind::Enum,
                in_sections: &["application"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scrubbing-duration-sec",
                value_type: ValueKind::Integer,
                in_sections: &["application"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scrubbing-enable",
                value_type: ValueKind::Enum,
                in_sections: &["application"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "single-page-application",
                value_type: ValueKind::Enum,
                in_sections: &["application"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "stress-based",
                value_type: ValueKind::Unknown,
                in_sections: &["application"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tcp-dump",
                value_type: ValueKind::Unknown,
                in_sections: &["application"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tps-based",
                value_type: ValueKind::Unknown,
                in_sections: &["application"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trigger-irule",
                value_type: ValueKind::Enum,
                in_sections: &["application"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "bot-defense",
        value_type: ValueKind::Unknown,
        in_sections: &["application"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "browser-legit-captcha",
                value_type: ValueKind::Enum,
                in_sections: &["application", "bot-defense"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "browser-legit-enabled",
                value_type: ValueKind::Enum,
                in_sections: &["application", "bot-defense"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "collect-stats",
                value_type: ValueKind::Enum,
                in_sections: &["application", "bot-defense"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cross-domain-requests",
                value_type: ValueKind::Enum,
                in_sections: &["application", "bot-defense"],
                enum_values: &["allow-all", "validate-bulk", "validate-upon-request"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "external-domains",
                value_type: ValueKind::List,
                in_sections: &["application", "bot-defense"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "grace-period",
                value_type: ValueKind::Integer,
                in_sections: &["application", "bot-defense"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                in_sections: &["application", "bot-defense"],
                enum_values: &["always", "disabled", "during-attacks"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-domains",
                value_type: ValueKind::List,
                in_sections: &["application", "bot-defense"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-whitelist",
                value_type: ValueKind::List,
                in_sections: &["application", "bot-defense"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "browser-legit-captcha",
        value_type: ValueKind::Enum,
        in_sections: &["application", "bot-defense"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "browser-legit-enabled",
        value_type: ValueKind::Enum,
        in_sections: &["application", "bot-defense"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "collect-stats",
        value_type: ValueKind::Enum,
        in_sections: &["application", "bot-defense"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "cross-domain-requests",
        value_type: ValueKind::Enum,
        in_sections: &["application", "bot-defense"],
        enum_values: &["allow-all", "validate-bulk", "validate-upon-request"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "external-domains",
        value_type: ValueKind::List,
        in_sections: &["application", "bot-defense"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "grace-period",
        value_type: ValueKind::Integer,
        in_sections: &["application", "bot-defense"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "mode",
        value_type: ValueKind::Enum,
        in_sections: &["application", "bot-defense"],
        enum_values: &["always", "disabled", "during-attacks"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-domains",
        value_type: ValueKind::List,
        in_sections: &["application", "bot-defense"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-whitelist",
        value_type: ValueKind::List,
        in_sections: &["application", "bot-defense"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "bot-signatures",
        value_type: ValueKind::List,
        in_sections: &["application"],
        block: &[
            BigipPropertySpec {
                name: "categories",
                value_type: ValueKind::List,
                in_sections: &["application", "bot-signatures"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "check",
                value_type: ValueKind::Enum,
                in_sections: &["application", "bot-signatures"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "disabled-signatures",
                value_type: ValueKind::List,
                in_sections: &["application", "bot-signatures"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "categories",
        value_type: ValueKind::List,
        in_sections: &["application", "bot-signatures"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[BigipPropertySpec {
            name: "action",
            value_type: ValueKind::Unknown,
            in_sections: &["application", "bot-signatures", "categories"],
            shape_kind: Some(ValueKind::Object),
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "action",
        value_type: ValueKind::Unknown,
        in_sections: &["application", "bot-signatures", "categories"],
        shape_kind: Some(ValueKind::Object),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "check",
        value_type: ValueKind::Enum,
        in_sections: &["application", "bot-signatures"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "disabled-signatures",
        value_type: ValueKind::List,
        in_sections: &["application", "bot-signatures"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "captcha-response",
        value_type: ValueKind::Unknown,
        in_sections: &["application"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "failure",
                value_type: ValueKind::Unknown,
                in_sections: &["application", "captcha-response"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first",
                value_type: ValueKind::Unknown,
                in_sections: &["application", "captcha-response"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "failure",
        value_type: ValueKind::Unknown,
        in_sections: &["application", "captcha-response"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "body",
                value_type: ValueKind::String,
                in_sections: &["application", "captcha-response", "failure"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["application", "captcha-response", "failure"],
                enum_values: &["custom", "default"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "body",
        value_type: ValueKind::String,
        in_sections: &["application", "captcha-response", "failure"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        in_sections: &["application", "captcha-response", "failure"],
        enum_values: &["custom", "default"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "first",
        value_type: ValueKind::Unknown,
        in_sections: &["application", "captcha-response"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "body",
                value_type: ValueKind::String,
                in_sections: &["application", "captcha-response", "first"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["application", "captcha-response", "first"],
                enum_values: &["custom", "default"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "body",
        value_type: ValueKind::String,
        in_sections: &["application", "captcha-response", "first"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        in_sections: &["application", "captcha-response", "first"],
        enum_values: &["custom", "default"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "fastl4-acceleration-profile",
        value_type: ValueKind::Reference,
        in_sections: &["application"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geolocations",
        value_type: ValueKind::List,
        in_sections: &["application"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "heavy-urls",
        value_type: ValueKind::List,
        in_sections: &["application"],
        block: &[
            BigipPropertySpec {
                name: "automatic-detection",
                value_type: ValueKind::Enum,
                in_sections: &["application", "heavy-urls"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "exclude",
                value_type: ValueKind::List,
                in_sections: &["application", "heavy-urls"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include",
                value_type: ValueKind::List,
                in_sections: &["application", "heavy-urls"],
                repeated: true,
                allow_none: true,
                usage_flags: &["deprecated"],
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-list",
                value_type: ValueKind::List,
                in_sections: &["application", "heavy-urls"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "latency-threshold",
                value_type: ValueKind::Integer,
                in_sections: &["application", "heavy-urls"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protection",
                value_type: ValueKind::Enum,
                in_sections: &["application", "heavy-urls"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "automatic-detection",
        value_type: ValueKind::Enum,
        in_sections: &["application", "heavy-urls"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "exclude",
        value_type: ValueKind::List,
        in_sections: &["application", "heavy-urls"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "include",
        value_type: ValueKind::List,
        in_sections: &["application", "heavy-urls"],
        repeated: true,
        allow_none: true,
        usage_flags: &["deprecated"],
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "include-list",
        value_type: ValueKind::List,
        in_sections: &["application", "heavy-urls"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "latency-threshold",
        value_type: ValueKind::Integer,
        in_sections: &["application", "heavy-urls"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "protection",
        value_type: ValueKind::Enum,
        in_sections: &["application", "heavy-urls"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-whitelist",
        value_type: ValueKind::List,
        in_sections: &["application"],
        allow_none: true,
        usage_flags: &["deprecated"],
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rtbh-duration-sec",
        value_type: ValueKind::Integer,
        in_sections: &["application"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rtbh-enable",
        value_type: ValueKind::Enum,
        in_sections: &["application"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "scrubbing-duration-sec",
        value_type: ValueKind::Integer,
        in_sections: &["application"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "scrubbing-enable",
        value_type: ValueKind::Enum,
        in_sections: &["application"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "single-page-application",
        value_type: ValueKind::Enum,
        in_sections: &["application"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("disabled"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "stress-based",
        value_type: ValueKind::Unknown,
        in_sections: &["application"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "behavioral",
                value_type: ValueKind::Unknown,
                in_sections: &["application", "stress-based"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "de-escalation-period",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-captcha-challenge",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-client-side-defense",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-maximum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-maximum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-minimum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-minimum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-rate-limiting",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-request-blocking-mode",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["block-all", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-tps-increase-rate",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "escalation-period",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geo-captcha-challenge",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geo-client-side-defense",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geo-maximum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geo-minimum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geo-minimum-share",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geo-rate-limiting",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geo-request-blocking-mode",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["block-all", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geo-share-increase-rate",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-captcha-challenge",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-client-side-defense",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-maximum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-maximum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-minimum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-minimum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-rate-limiting",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-request-blocking-mode",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["block-all", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-tps-increase-rate",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["blocking", "off", "transparent"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-captcha-challenge",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-client-side-defense",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-maximum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-maximum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-minimum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-minimum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-rate-limiting",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-tps-increase-rate",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "static-url-mitigation",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "thresholds-mode",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["automatic", "manual"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-captcha-challenge",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-client-side-defense",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-enable-heavy",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-maximum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-maximum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-minimum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-minimum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-rate-limiting",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-tps-increase-rate",
                value_type: ValueKind::Integer,
                in_sections: &["application", "stress-based"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "behavioral",
        value_type: ValueKind::Unknown,
        in_sections: &["application", "stress-based"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "accelerated-signatures",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based", "behavioral"],
                enum_values: &["disabled", "enables"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dos-detection",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based", "behavioral"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mitigation-mode",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based", "behavioral"],
                allow_none: true,
                enum_values: &["aggressive", "conservative", "none", "standard"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signatures",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based", "behavioral"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signatures-approved-only",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based", "behavioral"],
                enum_values: &["disabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tls-fp",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based", "behavioral"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tls-signatures",
                value_type: ValueKind::Enum,
                in_sections: &["application", "stress-based", "behavioral"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "accelerated-signatures",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based", "behavioral"],
        enum_values: &["disabled", "enables"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dos-detection",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based", "behavioral"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "mitigation-mode",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based", "behavioral"],
        allow_none: true,
        enum_values: &["aggressive", "conservative", "none", "standard"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "signatures",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based", "behavioral"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "signatures-approved-only",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based", "behavioral"],
        enum_values: &["disabled"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "tls-fp",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based", "behavioral"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "tls-signatures",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based", "behavioral"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "de-escalation-period",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-captcha-challenge",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-client-side-defense",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-maximum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-maximum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-minimum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-minimum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-rate-limiting",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-request-blocking-mode",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["block-all", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-tps-increase-rate",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "escalation-period",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geo-captcha-challenge",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geo-client-side-defense",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geo-maximum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geo-minimum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geo-minimum-share",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geo-rate-limiting",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geo-request-blocking-mode",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["block-all", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geo-share-increase-rate",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-captcha-challenge",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-client-side-defense",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-maximum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-maximum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-minimum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-minimum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-rate-limiting",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-request-blocking-mode",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["block-all", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-tps-increase-rate",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "mode",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["blocking", "off", "transparent"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-captcha-challenge",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-client-side-defense",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-maximum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-maximum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-minimum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-minimum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-rate-limiting",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-tps-increase-rate",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "static-url-mitigation",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "thresholds-mode",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["automatic", "manual"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-captcha-challenge",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-client-side-defense",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-enable-heavy",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-maximum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-maximum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-minimum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-minimum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-rate-limiting",
        value_type: ValueKind::Enum,
        in_sections: &["application", "stress-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-tps-increase-rate",
        value_type: ValueKind::Integer,
        in_sections: &["application", "stress-based"],
        ..BigipPropertySpec::DEFAULT
    },
];
