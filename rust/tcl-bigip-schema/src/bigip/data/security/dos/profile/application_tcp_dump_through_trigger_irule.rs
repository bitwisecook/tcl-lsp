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

//! Properties `tcp-dump` to `trigger-irule` of `security dos profile`, each with the properties of its sections.

use crate::bigip::{BigipPropertySpec, ValueKind};

pub(super) const PROPERTIES: &[BigipPropertySpec] = &[
    BigipPropertySpec {
        name: "tcp-dump",
        value_type: ValueKind::Unknown,
        in_sections: &["application"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "maximum-duration",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tcp-dump"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "maximum-size",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tcp-dump"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "record-traffic",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tcp-dump"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "repetition-interval",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tcp-dump"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "maximum-duration",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tcp-dump"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "maximum-size",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tcp-dump"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "record-traffic",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tcp-dump"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "repetition-interval",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tcp-dump"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "tps-based",
        value_type: ValueKind::Unknown,
        in_sections: &["application"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "de-escalation-period",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-captcha-challenge",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-client-side-defense",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-maximum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-maximum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-minimum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-minimum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-rate-limiting",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-request-blocking-mode",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["block-all", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "device-tps-increase-rate",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "escalation-period",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geo-captcha-challenge",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geo-client-side-defense",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geo-minimum-share",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geo-rate-limiting",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geo-request-blocking-mode",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["block-all", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "geo-share-increase-rate",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-captcha-challenge",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-client-side-defense",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-maximum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-maximum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-minimum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-minimum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-rate-limiting",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-request-blocking-mode",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["block-all", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-tps-increase-rate",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["blocking", "off", "transparent"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-captcha-challenge",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-client-side-defense",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-maximum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-maximum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-minimum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-minimum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-rate-limiting",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "site-tps-increase-rate",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "static-url-mitigation",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "thresholds-mode",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["automatic", "manual"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-captcha-challenge",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-client-side-defense",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-enable-heavy",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-maximum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-maximum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-minimum-auto-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-minimum-tps",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-rate-limiting",
                value_type: ValueKind::Enum,
                in_sections: &["application", "tps-based"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-tps-increase-rate",
                value_type: ValueKind::Integer,
                in_sections: &["application", "tps-based"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "de-escalation-period",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-captcha-challenge",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-client-side-defense",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-maximum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-maximum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-minimum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-minimum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-rate-limiting",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-request-blocking-mode",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["block-all", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "device-tps-increase-rate",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "escalation-period",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geo-captcha-challenge",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geo-client-side-defense",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geo-minimum-share",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geo-rate-limiting",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geo-request-blocking-mode",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["block-all", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geo-share-increase-rate",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-captcha-challenge",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-client-side-defense",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-maximum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-maximum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-minimum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-minimum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-rate-limiting",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-request-blocking-mode",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["block-all", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-tps-increase-rate",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "mode",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["blocking", "off", "transparent"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-captcha-challenge",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-client-side-defense",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-maximum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-maximum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-minimum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-minimum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-rate-limiting",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "site-tps-increase-rate",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "static-url-mitigation",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "thresholds-mode",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["automatic", "manual"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-captcha-challenge",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-client-side-defense",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-enable-heavy",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-maximum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-maximum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-minimum-auto-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-minimum-tps",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-rate-limiting",
        value_type: ValueKind::Enum,
        in_sections: &["application", "tps-based"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url-tps-increase-rate",
        value_type: ValueKind::Integer,
        in_sections: &["application", "tps-based"],
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
];
