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

//! Properties `alert-client-side-caching` to `inject-javascript` of `security anti-fraud profile`, each with the properties of its sections.

use crate::bigip::{BigipPropertySpec, ValueKind};

pub(super) const PROPERTIES: &[BigipPropertySpec] = &[
    BigipPropertySpec {
        name: "alert-client-side-caching",
        value_type: ValueKind::Enum,
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "alert-identifier",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "alert-path",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "alert-pool",
        value_type: ValueKind::Reference,
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "alert-publisher",
        value_type: ValueKind::Reference,
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "alert-token-header",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "app-layer-encryption",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[BigipPropertySpec {
            name: "fail-open",
            value_type: ValueKind::Enum,
            in_sections: &["app-layer-encryption"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "fail-open",
        value_type: ValueKind::Enum,
        in_sections: &["app-layer-encryption"],
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
        name: "auto-transactions",
        value_type: ValueKind::List,
        block: &[
            BigipPropertySpec {
                name: "bot-score",
                value_type: ValueKind::Integer,
                in_sections: &["auto-transactions"],
                default: Some("a score of 50"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "click-score",
                value_type: ValueKind::Integer,
                in_sections: &["auto-transactions"],
                default: Some("a score of 40"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "integrity-fail-score",
                value_type: ValueKind::Integer,
                in_sections: &["auto-transactions"],
                default: Some("a score of 40"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-mouse-move-count",
                value_type: ValueKind::Integer,
                in_sections: &["auto-transactions"],
                default: Some("5 movements"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-mouse-over-count",
                value_type: ValueKind::Integer,
                in_sections: &["auto-transactions"],
                default: Some("2 button interactions"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-report-score",
                value_type: ValueKind::Integer,
                in_sections: &["auto-transactions"],
                default: Some("50"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-time-to-request",
                value_type: ValueKind::Integer,
                in_sections: &["auto-transactions"],
                default: Some("2 seconds"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "not-human-score",
                value_type: ValueKind::Integer,
                in_sections: &["auto-transactions"],
                default: Some("a score of 25"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "strong-integrity",
                value_type: ValueKind::Unknown,
                in_sections: &["auto-transactions"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tampered-cookie-score",
                value_type: ValueKind::Integer,
                in_sections: &["auto-transactions"],
                default: Some("a score of 50"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-fail-score",
                value_type: ValueKind::Integer,
                in_sections: &["auto-transactions"],
                default: Some("a score of 20"),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "bot-score",
        value_type: ValueKind::Integer,
        in_sections: &["auto-transactions"],
        default: Some("a score of 50"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "click-score",
        value_type: ValueKind::Integer,
        in_sections: &["auto-transactions"],
        default: Some("a score of 40"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "integrity-fail-score",
        value_type: ValueKind::Integer,
        in_sections: &["auto-transactions"],
        default: Some("a score of 40"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "min-mouse-move-count",
        value_type: ValueKind::Integer,
        in_sections: &["auto-transactions"],
        default: Some("5 movements"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "min-mouse-over-count",
        value_type: ValueKind::Integer,
        in_sections: &["auto-transactions"],
        default: Some("2 button interactions"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "min-report-score",
        value_type: ValueKind::Integer,
        in_sections: &["auto-transactions"],
        default: Some("50"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "min-time-to-request",
        value_type: ValueKind::Integer,
        in_sections: &["auto-transactions"],
        default: Some("2 seconds"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "not-human-score",
        value_type: ValueKind::Integer,
        in_sections: &["auto-transactions"],
        default: Some("a score of 25"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "strong-integrity",
        value_type: ValueKind::Unknown,
        in_sections: &["auto-transactions"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "hide-encrypted-parameters",
                value_type: ValueKind::Enum,
                in_sections: &["auto-transactions", "strong-integrity"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "parameter",
                value_type: ValueKind::String,
                in_sections: &["auto-transactions", "strong-integrity"],
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "hide-encrypted-parameters",
        value_type: ValueKind::Enum,
        in_sections: &["auto-transactions", "strong-integrity"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "parameter",
        value_type: ValueKind::String,
        in_sections: &["auto-transactions", "strong-integrity"],
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "tampered-cookie-score",
        value_type: ValueKind::Integer,
        in_sections: &["auto-transactions"],
        default: Some("a score of 50"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "time-fail-score",
        value_type: ValueKind::Integer,
        in_sections: &["auto-transactions"],
        default: Some("a score of 20"),
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "before-load-function",
        value_type: ValueKind::String,
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "behaviour-analysis",
        value_type: ValueKind::List,
        block: &[
            BigipPropertySpec {
                name: "run",
                value_type: ValueKind::Enum,
                in_sections: &["behaviour-analysis"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "score",
                value_type: ValueKind::Integer,
                in_sections: &["behaviour-analysis"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "run",
        value_type: ValueKind::Enum,
        in_sections: &["behaviour-analysis"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "score",
        value_type: ValueKind::Integer,
        in_sections: &["behaviour-analysis"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blocking-page",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "response-body",
                value_type: ValueKind::String,
                in_sections: &["blocking-page"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "response-headers",
                value_type: ValueKind::String,
                in_sections: &["blocking-page"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "response-body",
        value_type: ValueKind::String,
        in_sections: &["blocking-page"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "response-headers",
        value_type: ValueKind::String,
        in_sections: &["blocking-page"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "check-custom",
        value_type: ValueKind::Enum,
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "check-generic",
        value_type: ValueKind::Enum,
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "cloud-service-pool",
        value_type: ValueKind::Reference,
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "config-location",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "cookies",
        value_type: ValueKind::List,
        block: &[
            BigipPropertySpec {
                name: "application",
                value_type: ValueKind::List,
                in_sections: &["cookies"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "base-domain",
                value_type: ValueKind::Unknown,
                in_sections: &["cookies"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-side",
                value_type: ValueKind::String,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-side-lifetime",
                value_type: ValueKind::Integer,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "components-state",
                value_type: ValueKind::String,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "components-state-lifetime",
                value_type: ValueKind::Integer,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "components-state-removal-protection",
                value_type: ValueKind::Enum,
                in_sections: &["cookies"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encryption-disabled",
                value_type: ValueKind::String,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encryption-disabled-lifetime",
                value_type: ValueKind::Integer,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encryption-disabled-removal-protection",
                value_type: ValueKind::Enum,
                in_sections: &["cookies"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fingerprint",
                value_type: ValueKind::String,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fingerprint-lifetime",
                value_type: ValueKind::Integer,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fingerprint-removal-protection",
                value_type: ValueKind::Enum,
                in_sections: &["cookies"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "html-field-obfuscation",
                value_type: ValueKind::String,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "html-field-obfuscation-lifetime",
                value_type: ValueKind::Integer,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "malware-forensic",
                value_type: ValueKind::String,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "malware-forensic-lifetime",
                value_type: ValueKind::Integer,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "malware-guid",
                value_type: ValueKind::String,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "malware-guid-lifetime",
                value_type: ValueKind::Integer,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "malware-guid-removal-protection",
                value_type: ValueKind::Enum,
                in_sections: &["cookies"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::String,
                in_sections: &["cookies"],
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
                name: "rules-lifetime",
                value_type: ValueKind::Integer,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules-removal-protection",
                value_type: ValueKind::Enum,
                in_sections: &["cookies"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secure-alert",
                value_type: ValueKind::String,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secure-alert-lifetime",
                value_type: ValueKind::Integer,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secure-alert-removal-protection",
                value_type: ValueKind::Enum,
                in_sections: &["cookies"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secure-channel",
                value_type: ValueKind::String,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secure-channel-lifetime",
                value_type: ValueKind::Integer,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secure-channel-removal-protection",
                value_type: ValueKind::Enum,
                in_sections: &["cookies"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secure-mode",
                value_type: ValueKind::Enum,
                in_sections: &["cookies"],
                enum_values: &["auto", "disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transaction-data",
                value_type: ValueKind::String,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transaction-data-lifetime",
                value_type: ValueKind::Integer,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-inspection",
                value_type: ValueKind::String,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-name",
                value_type: ValueKind::String,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-name-lifetime",
                value_type: ValueKind::Integer,
                in_sections: &["cookies"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-name-removal-protection",
                value_type: ValueKind::Enum,
                in_sections: &["cookies"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "application",
        value_type: ValueKind::List,
        in_sections: &["cookies"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "base-domain",
        value_type: ValueKind::Unknown,
        in_sections: &["cookies"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "apply",
                value_type: ValueKind::Enum,
                in_sections: &["cookies", "base-domain"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "exceptions",
                value_type: ValueKind::List,
                in_sections: &["cookies", "base-domain"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "apply",
        value_type: ValueKind::Enum,
        in_sections: &["cookies", "base-domain"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "exceptions",
        value_type: ValueKind::List,
        in_sections: &["cookies", "base-domain"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "client-side",
        value_type: ValueKind::String,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "client-side-lifetime",
        value_type: ValueKind::Integer,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "components-state",
        value_type: ValueKind::String,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "components-state-lifetime",
        value_type: ValueKind::Integer,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "components-state-removal-protection",
        value_type: ValueKind::Enum,
        in_sections: &["cookies"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "encryption-disabled",
        value_type: ValueKind::String,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "encryption-disabled-lifetime",
        value_type: ValueKind::Integer,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "encryption-disabled-removal-protection",
        value_type: ValueKind::Enum,
        in_sections: &["cookies"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "fingerprint",
        value_type: ValueKind::String,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "fingerprint-lifetime",
        value_type: ValueKind::Integer,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "fingerprint-removal-protection",
        value_type: ValueKind::Enum,
        in_sections: &["cookies"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "html-field-obfuscation",
        value_type: ValueKind::String,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "html-field-obfuscation-lifetime",
        value_type: ValueKind::Integer,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "malware-forensic",
        value_type: ValueKind::String,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "malware-forensic-lifetime",
        value_type: ValueKind::Integer,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "malware-guid",
        value_type: ValueKind::String,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "malware-guid-lifetime",
        value_type: ValueKind::Integer,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "malware-guid-removal-protection",
        value_type: ValueKind::Enum,
        in_sections: &["cookies"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rules",
        value_type: ValueKind::String,
        in_sections: &["cookies"],
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
        name: "rules-lifetime",
        value_type: ValueKind::Integer,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rules-removal-protection",
        value_type: ValueKind::Enum,
        in_sections: &["cookies"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "secure-alert",
        value_type: ValueKind::String,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "secure-alert-lifetime",
        value_type: ValueKind::Integer,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "secure-alert-removal-protection",
        value_type: ValueKind::Enum,
        in_sections: &["cookies"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "secure-channel",
        value_type: ValueKind::String,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "secure-channel-lifetime",
        value_type: ValueKind::Integer,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "secure-channel-removal-protection",
        value_type: ValueKind::Enum,
        in_sections: &["cookies"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "secure-mode",
        value_type: ValueKind::Enum,
        in_sections: &["cookies"],
        enum_values: &["auto", "disabled", "enabled"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "transaction-data",
        value_type: ValueKind::String,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "transaction-data-lifetime",
        value_type: ValueKind::Integer,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-inspection",
        value_type: ValueKind::String,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-name",
        value_type: ValueKind::String,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-name-lifetime",
        value_type: ValueKind::Integer,
        in_sections: &["cookies"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-name-removal-protection",
        value_type: ValueKind::Enum,
        in_sections: &["cookies"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "custom-config",
        value_type: ValueKind::String,
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "custom-whitelist",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[BigipPropertySpec {
            name: "path",
            value_type: ValueKind::String,
            in_sections: &["custom-whitelist"],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "path",
        value_type: ValueKind::String,
        in_sections: &["custom-whitelist"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "debug",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "console-log",
                value_type: ValueKind::Unknown,
                in_sections: &["debug"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send-alert",
                value_type: ValueKind::Unknown,
                in_sections: &["debug"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "console-log",
        value_type: ValueKind::Unknown,
        in_sections: &["debug"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "client-ips",
                value_type: ValueKind::List,
                in_sections: &["debug", "console-log"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fingerprints",
                value_type: ValueKind::List,
                in_sections: &["debug", "console-log"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-agents",
                value_type: ValueKind::List,
                in_sections: &["debug", "console-log"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "client-ips",
        value_type: ValueKind::List,
        in_sections: &["debug", "console-log"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "fingerprints",
        value_type: ValueKind::List,
        in_sections: &["debug", "console-log"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-agents",
        value_type: ValueKind::List,
        in_sections: &["debug", "console-log"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "send-alert",
        value_type: ValueKind::Unknown,
        in_sections: &["debug"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "client-ips",
                value_type: ValueKind::List,
                in_sections: &["debug", "send-alert"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fingerprints",
                value_type: ValueKind::List,
                in_sections: &["debug", "send-alert"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-agents",
                value_type: ValueKind::List,
                in_sections: &["debug", "send-alert"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "client-ips",
        value_type: ValueKind::List,
        in_sections: &["debug", "send-alert"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "fingerprints",
        value_type: ValueKind::List,
        in_sections: &["debug", "send-alert"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-agents",
        value_type: ValueKind::List,
        in_sections: &["debug", "send-alert"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "defaults-from",
        value_type: ValueKind::Reference,
        allow_none: true,
        references: &["security_anti_fraud_profile"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "description",
        value_type: ValueKind::String,
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "destination-urls",
        value_type: ValueKind::List,
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "domain-availability-urls",
        value_type: ValueKind::String,
        allow_none: true,
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dummy-alert-html-maximum-length",
        value_type: ValueKind::Integer,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "encryption-staging-mode",
        value_type: ValueKind::Enum,
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "external-sources-targets",
        value_type: ValueKind::List,
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "fallback-to-base-url",
        value_type: ValueKind::Enum,
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "fingerprint",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "collect",
                value_type: ValueKind::Enum,
                in_sections: &["fingerprint"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location",
                value_type: ValueKind::String,
                in_sections: &["fingerprint"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "collect",
        value_type: ValueKind::Enum,
        in_sections: &["fingerprint"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "location",
        value_type: ValueKind::String,
        in_sections: &["fingerprint"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "flash-cookie-content",
        value_type: ValueKind::String,
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "flash-cookie-location",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "flash-cookies",
        value_type: ValueKind::Enum,
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "forensic",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "alert-path",
                value_type: ValueKind::String,
                in_sections: &["forensic"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-domains",
                value_type: ValueKind::List,
                in_sections: &["forensic"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cloud-config-path",
                value_type: ValueKind::String,
                in_sections: &["forensic"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cloud-forensics-mode",
                value_type: ValueKind::Integer,
                in_sections: &["forensic"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cloud-remediation-mode",
                value_type: ValueKind::Integer,
                in_sections: &["forensic"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "continue-element",
                value_type: ValueKind::String,
                in_sections: &["forensic"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "exe-location",
                value_type: ValueKind::String,
                in_sections: &["forensic"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "html",
                value_type: ValueKind::String,
                in_sections: &["forensic"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "self-post-location",
                value_type: ValueKind::String,
                in_sections: &["forensic"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "skip-element",
                value_type: ValueKind::String,
                in_sections: &["forensic"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "skip-path",
                value_type: ValueKind::String,
                in_sections: &["forensic"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "alert-path",
        value_type: ValueKind::String,
        in_sections: &["forensic"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "client-domains",
        value_type: ValueKind::List,
        in_sections: &["forensic"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "cloud-config-path",
        value_type: ValueKind::String,
        in_sections: &["forensic"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "cloud-forensics-mode",
        value_type: ValueKind::Integer,
        in_sections: &["forensic"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "cloud-remediation-mode",
        value_type: ValueKind::Integer,
        in_sections: &["forensic"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "continue-element",
        value_type: ValueKind::String,
        in_sections: &["forensic"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "exe-location",
        value_type: ValueKind::String,
        in_sections: &["forensic"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "html",
        value_type: ValueKind::String,
        in_sections: &["forensic"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "self-post-location",
        value_type: ValueKind::String,
        in_sections: &["forensic"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "skip-element",
        value_type: ValueKind::String,
        in_sections: &["forensic"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "skip-path",
        value_type: ValueKind::String,
        in_sections: &["forensic"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "generic-whitelist-words",
        value_type: ValueKind::List,
        repeated: true,
        allow_none: true,
        usage_flags: &["deprecated"],
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "geolocation",
        value_type: ValueKind::Enum,
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "include-query-string",
        value_type: ValueKind::Enum,
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("disabled"),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "inject-javascript",
        value_type: ValueKind::Enum,
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        default: Some("enabled"),
        ..BigipPropertySpec::DEFAULT
    },
];
