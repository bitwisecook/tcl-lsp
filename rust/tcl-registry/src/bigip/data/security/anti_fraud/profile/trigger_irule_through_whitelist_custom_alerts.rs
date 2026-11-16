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

//! Properties `trigger-irule` to `whitelist-custom-alerts` of `security anti-fraud profile`, each with the properties of its sections.

use crate::bigip::{BigipPropertySpec, ValueKind};

pub(super) const PROPERTIES: &[BigipPropertySpec] = &[
    BigipPropertySpec {
        name: "trigger-irule",
        value_type: ValueKind::Enum,
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("disabled"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        enum_values: &["explicit", "wildcard"],
        usage_flags: &["read_only"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "urls",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "app-layer-encryption",
                value_type: ValueKind::Unknown,
                in_sections: &["urls"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-transactions",
                value_type: ValueKind::List,
                in_sections: &["urls"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "before-load-function",
                value_type: ValueKind::String,
                in_sections: &["urls"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "custom-alerts",
                value_type: ValueKind::List,
                in_sections: &["urls"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "app-layer-encryption",
        value_type: ValueKind::Unknown,
        in_sections: &["urls"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "add-decoy-inputs",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "app-layer-encryption"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-complete-block",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "app-layer-encryption"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-complete-whitelist-functions",
                value_type: ValueKind::List,
                in_sections: &["urls", "app-layer-encryption"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "custom-encryption-function",
                value_type: ValueKind::String,
                in_sections: &["urls", "app-layer-encryption"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fake-strokes",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "app-layer-encryption"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "full-ajax-encryption",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "app-layer-encryption"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hide-password-revealer",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "app-layer-encryption"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "html-field-obfuscation",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "app-layer-encryption"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "real-time-encryption",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "app-layer-encryption"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remove-element-ids",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "app-layer-encryption"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remove-event-listeners",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "app-layer-encryption"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "stolen-creds",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "app-layer-encryption"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "substitute-value-function",
                value_type: ValueKind::String,
                in_sections: &["urls", "app-layer-encryption"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "add-decoy-inputs",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "app-layer-encryption"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "auto-complete-block",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "app-layer-encryption"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "auto-complete-whitelist-functions",
        value_type: ValueKind::List,
        in_sections: &["urls", "app-layer-encryption"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "custom-encryption-function",
        value_type: ValueKind::String,
        in_sections: &["urls", "app-layer-encryption"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "fake-strokes",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "app-layer-encryption"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "full-ajax-encryption",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "app-layer-encryption"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "hide-password-revealer",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "app-layer-encryption"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "html-field-obfuscation",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "app-layer-encryption"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "real-time-encryption",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "app-layer-encryption"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "remove-element-ids",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "app-layer-encryption"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "remove-event-listeners",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "app-layer-encryption"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "stolen-creds",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "app-layer-encryption"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "substitute-value-function",
        value_type: ValueKind::String,
        in_sections: &["urls", "app-layer-encryption"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "auto-transactions",
        value_type: ValueKind::List,
        in_sections: &["urls"],
        block: &[
            BigipPropertySpec {
                name: "attach-ajax-payload-to-alerts",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "auto-transactions"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bot-score",
                value_type: ValueKind::Integer,
                in_sections: &["urls", "auto-transactions"],
                default: Some("a score of 50"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "browser",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "auto-transactions"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "click-score",
                value_type: ValueKind::Integer,
                in_sections: &["urls", "auto-transactions"],
                default: Some("a score of 40"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "full-ajax-integrity",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "auto-transactions"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "integrity-fail-max-score",
                value_type: ValueKind::Integer,
                in_sections: &["urls", "auto-transactions"],
                default: Some("a score of 100"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "integrity-fail-score",
                value_type: ValueKind::Integer,
                in_sections: &["urls", "auto-transactions"],
                default: Some("a score of 40"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-mouse-move-count",
                value_type: ValueKind::Integer,
                in_sections: &["urls", "auto-transactions"],
                default: Some("5 movements"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-mouse-over-count",
                value_type: ValueKind::Integer,
                in_sections: &["urls", "auto-transactions"],
                default: Some("2 button interactions"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-report-score",
                value_type: ValueKind::Integer,
                in_sections: &["urls", "auto-transactions"],
                default: Some("50"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-time-to-request",
                value_type: ValueKind::Integer,
                in_sections: &["urls", "auto-transactions"],
                default: Some("2 seconds"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "non-browser",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "auto-transactions"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "not-human-score",
                value_type: ValueKind::Integer,
                in_sections: &["urls", "auto-transactions"],
                default: Some("a score of 25"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "strong-integrity",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "auto-transactions"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "strong-integrity-user-functions",
                value_type: ValueKind::List,
                in_sections: &["urls", "auto-transactions"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-buttons",
                value_type: ValueKind::List,
                in_sections: &["urls", "auto-transactions"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tampered-cookie-score",
                value_type: ValueKind::Integer,
                in_sections: &["urls", "auto-transactions"],
                default: Some("a score of 50"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-fail-score",
                value_type: ValueKind::Integer,
                in_sections: &["urls", "auto-transactions"],
                default: Some("a score of 20"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "attach-ajax-payload-to-alerts",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "auto-transactions"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "bot-score",
        value_type: ValueKind::Integer,
        in_sections: &["urls", "auto-transactions"],
        default: Some("a score of 50"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "browser",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "auto-transactions"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "click-score",
        value_type: ValueKind::Integer,
        in_sections: &["urls", "auto-transactions"],
        default: Some("a score of 40"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "full-ajax-integrity",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "auto-transactions"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "integrity-fail-max-score",
        value_type: ValueKind::Integer,
        in_sections: &["urls", "auto-transactions"],
        default: Some("a score of 100"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "integrity-fail-score",
        value_type: ValueKind::Integer,
        in_sections: &["urls", "auto-transactions"],
        default: Some("a score of 40"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "min-mouse-move-count",
        value_type: ValueKind::Integer,
        in_sections: &["urls", "auto-transactions"],
        default: Some("5 movements"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "min-mouse-over-count",
        value_type: ValueKind::Integer,
        in_sections: &["urls", "auto-transactions"],
        default: Some("2 button interactions"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "min-report-score",
        value_type: ValueKind::Integer,
        in_sections: &["urls", "auto-transactions"],
        default: Some("50"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "min-time-to-request",
        value_type: ValueKind::Integer,
        in_sections: &["urls", "auto-transactions"],
        default: Some("2 seconds"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "non-browser",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "auto-transactions"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "not-human-score",
        value_type: ValueKind::Integer,
        in_sections: &["urls", "auto-transactions"],
        default: Some("a score of 25"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "strong-integrity",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "auto-transactions"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "strong-integrity-user-functions",
        value_type: ValueKind::List,
        in_sections: &["urls", "auto-transactions"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "submit-buttons",
        value_type: ValueKind::List,
        in_sections: &["urls", "auto-transactions"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "tampered-cookie-score",
        value_type: ValueKind::Integer,
        in_sections: &["urls", "auto-transactions"],
        default: Some("a score of 50"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "time-fail-score",
        value_type: ValueKind::Integer,
        in_sections: &["urls", "auto-transactions"],
        default: Some("a score of 20"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "before-load-function",
        value_type: ValueKind::String,
        in_sections: &["urls"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "custom-alerts",
        value_type: ValueKind::List,
        in_sections: &["urls"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "attach-request-part",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "custom-alerts"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "component",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "custom-alerts"],
                enum_values: &["auto-transactions", "malware", "mobilesafe", "phishing"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "header-name",
                value_type: ValueKind::String,
                in_sections: &["urls", "custom-alerts"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "malware-name",
                value_type: ValueKind::String,
                in_sections: &["urls", "custom-alerts"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "message",
                value_type: ValueKind::String,
                in_sections: &["urls", "custom-alerts"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "search-in",
                value_type: ValueKind::Enum,
                in_sections: &["urls", "custom-alerts"],
                enum_values: &["client-ip", "header", "payload"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                in_sections: &["urls", "custom-alerts"],
                allow_none: true,
                default: Some("none, which means that the system searches for any value"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "attach-request-part",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "custom-alerts"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "component",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "custom-alerts"],
        enum_values: &["auto-transactions", "malware", "mobilesafe", "phishing"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "header-name",
        value_type: ValueKind::String,
        in_sections: &["urls", "custom-alerts"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "malware-name",
        value_type: ValueKind::String,
        in_sections: &["urls", "custom-alerts"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "message",
        value_type: ValueKind::String,
        in_sections: &["urls", "custom-alerts"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "search-in",
        value_type: ValueKind::Enum,
        in_sections: &["urls", "custom-alerts"],
        enum_values: &["client-ip", "header", "payload"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "value",
        value_type: ValueKind::String,
        in_sections: &["urls", "custom-alerts"],
        allow_none: true,
        default: Some("none, which means that the system searches for any value"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "users",
        value_type: ValueKind::List,
        list_operators: &["add", "delete", "modify"],
        block: &[BigipPropertySpec {
            name: "modes",
            value_type: ValueKind::List,
            in_sections: &["users"],
            list_operators: &["add", "delete"],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "modes",
        value_type: ValueKind::List,
        in_sections: &["users"],
        list_operators: &["add", "delete"],
        block: &[BigipPropertySpec {
            name: "mode",
            value_type: ValueKind::Unknown,
            in_sections: &["users", "modes"],
            required: true,
            enum_values: &["block", "forensic", "inspection", "remediation"],
            shape_kind: Some(ValueKind::Object),
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "mode",
        value_type: ValueKind::Unknown,
        in_sections: &["users", "modes"],
        required: true,
        enum_values: &["block", "forensic", "inspection", "remediation"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "duration",
                value_type: ValueKind::Integer,
                in_sections: &["users", "modes", "mode"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce-policy",
                value_type: ValueKind::Enum,
                in_sections: &["users", "modes", "mode"],
                enum_values: &["enforce", "time-limited", "unlimited"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-login-time",
                value_type: ValueKind::Unknown,
                in_sections: &["users", "modes", "mode"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "duration",
        value_type: ValueKind::Integer,
        in_sections: &["users", "modes", "mode"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "enforce-policy",
        value_type: ValueKind::Enum,
        in_sections: &["users", "modes", "mode"],
        enum_values: &["enforce", "time-limited", "unlimited"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "first-login-time",
        value_type: ValueKind::Unknown,
        in_sections: &["users", "modes", "mode"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "web-rootkit",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "blacklist-functions",
                value_type: ValueKind::List,
                in_sections: &["web-rootkit"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "whitelist-functions",
                value_type: ValueKind::List,
                in_sections: &["web-rootkit"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-functions",
        value_type: ValueKind::List,
        in_sections: &["web-rootkit"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "whitelist-functions",
        value_type: ValueKind::List,
        in_sections: &["web-rootkit"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "whitelist-custom-alerts",
        value_type: ValueKind::List,
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
];
