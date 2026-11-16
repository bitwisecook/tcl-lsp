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

//! Properties `mobilesafe-encryption` to `suggested-username-header` of `security anti-fraud profile`, each with the properties of its sections.

use crate::bigip::{BigipPropertySpec, ValueKind};

pub(super) const PROPERTIES: &[BigipPropertySpec] = &[
    BigipPropertySpec {
        name: "mobilesafe-encryption",
        value_type: ValueKind::Enum,
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "alert-custom-config",
        value_type: ValueKind::String,
        in_sections: &["mobilesafe"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "alert-threshold",
        value_type: ValueKind::Integer,
        in_sections: &["mobilesafe"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "app-integrity",
        value_type: ValueKind::Unknown,
        in_sections: &["mobilesafe"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "android",
                value_type: ValueKind::Unknown,
                in_sections: &["mobilesafe", "app-integrity"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "custom-config",
                value_type: ValueKind::String,
                in_sections: &["mobilesafe", "app-integrity"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ios",
                value_type: ValueKind::List,
                in_sections: &["mobilesafe", "app-integrity"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "android",
        value_type: ValueKind::Unknown,
        in_sections: &["mobilesafe", "app-integrity"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "score",
                value_type: ValueKind::Integer,
                in_sections: &["mobilesafe", "app-integrity", "android"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signature",
                value_type: ValueKind::String,
                in_sections: &["mobilesafe", "app-integrity", "android"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "score",
        value_type: ValueKind::Integer,
        in_sections: &["mobilesafe", "app-integrity", "android"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "signature",
        value_type: ValueKind::String,
        in_sections: &["mobilesafe", "app-integrity", "android"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "custom-config",
        value_type: ValueKind::String,
        in_sections: &["mobilesafe", "app-integrity"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ios",
        value_type: ValueKind::List,
        in_sections: &["mobilesafe", "app-integrity"],
        block: &[
            BigipPropertySpec {
                name: "hashes",
                value_type: ValueKind::List,
                in_sections: &["mobilesafe", "app-integrity", "ios"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "score",
                value_type: ValueKind::Integer,
                in_sections: &["mobilesafe", "app-integrity", "ios"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "hashes",
        value_type: ValueKind::List,
        in_sections: &["mobilesafe", "app-integrity", "ios"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[BigipPropertySpec {
            name: "value",
            value_type: ValueKind::Unknown,
            in_sections: &["mobilesafe", "app-integrity", "ios", "hashes"],
            shape_kind: Some(ValueKind::Object),
            default: Some("none, which means that the system searches for any value"),
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "value",
        value_type: ValueKind::Unknown,
        in_sections: &["mobilesafe", "app-integrity", "ios", "hashes"],
        shape_kind: Some(ValueKind::Object),
        default: Some("none, which means that the system searches for any value"),
        block: &[BigipPropertySpec {
            name: "version",
            value_type: ValueKind::String,
            in_sections: &["mobilesafe", "app-integrity", "ios", "hashes", "value"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "version",
        value_type: ValueKind::String,
        in_sections: &["mobilesafe", "app-integrity", "ios", "hashes", "value"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "score",
        value_type: ValueKind::Integer,
        in_sections: &["mobilesafe", "app-integrity", "ios"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "general-custom-config",
        value_type: ValueKind::String,
        in_sections: &["mobilesafe"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "malware",
        value_type: ValueKind::Unknown,
        in_sections: &["mobilesafe"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "android",
                value_type: ValueKind::Unknown,
                in_sections: &["mobilesafe", "malware"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "custom-whitelist",
                value_type: ValueKind::List,
                in_sections: &["mobilesafe", "malware"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "android",
        value_type: ValueKind::Unknown,
        in_sections: &["mobilesafe", "malware"],
        shape_kind: Some(ValueKind::Object),
        block: &[BigipPropertySpec {
            name: "custom-malware",
            value_type: ValueKind::List,
            in_sections: &["mobilesafe", "malware", "android"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "custom-malware",
        value_type: ValueKind::List,
        in_sections: &["mobilesafe", "malware", "android"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "package",
                value_type: ValueKind::String,
                in_sections: &["mobilesafe", "malware", "android", "custom-malware"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "score",
                value_type: ValueKind::Integer,
                in_sections: &["mobilesafe", "malware", "android", "custom-malware"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "package",
        value_type: ValueKind::String,
        in_sections: &["mobilesafe", "malware", "android", "custom-malware"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "score",
        value_type: ValueKind::Integer,
        in_sections: &["mobilesafe", "malware", "android", "custom-malware"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "custom-whitelist",
        value_type: ValueKind::List,
        in_sections: &["mobilesafe", "malware"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[BigipPropertySpec {
            name: "package",
            value_type: ValueKind::String,
            in_sections: &["mobilesafe", "malware", "custom-whitelist"],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "package",
        value_type: ValueKind::String,
        in_sections: &["mobilesafe", "malware", "custom-whitelist"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "os-security",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "android",
                value_type: ValueKind::Unknown,
                in_sections: &["os-security"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "custom-config",
                value_type: ValueKind::String,
                in_sections: &["os-security"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ios",
                value_type: ValueKind::List,
                in_sections: &["os-security"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "android",
        value_type: ValueKind::Unknown,
        in_sections: &["os-security"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "untrusted-apps-score",
                value_type: ValueKind::Integer,
                in_sections: &["os-security", "android"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "versions",
                value_type: ValueKind::List,
                in_sections: &["os-security", "android"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "untrusted-apps-score",
        value_type: ValueKind::Integer,
        in_sections: &["os-security", "android"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "versions",
        value_type: ValueKind::List,
        in_sections: &["os-security", "android"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[BigipPropertySpec {
            name: "priority",
            value_type: ValueKind::Unknown,
            in_sections: &["os-security", "android", "versions"],
            required: true,
            shape_kind: Some(ValueKind::Object),
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "priority",
        value_type: ValueKind::Unknown,
        in_sections: &["os-security", "android", "versions"],
        required: true,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "from",
                value_type: ValueKind::String,
                in_sections: &["os-security", "android", "versions", "priority"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "score",
                value_type: ValueKind::Integer,
                in_sections: &["os-security", "android", "versions", "priority"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "to",
                value_type: ValueKind::String,
                in_sections: &["os-security", "android", "versions", "priority"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "from",
        value_type: ValueKind::String,
        in_sections: &["os-security", "android", "versions", "priority"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "score",
        value_type: ValueKind::Integer,
        in_sections: &["os-security", "android", "versions", "priority"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "to",
        value_type: ValueKind::String,
        in_sections: &["os-security", "android", "versions", "priority"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "custom-config",
        value_type: ValueKind::String,
        in_sections: &["os-security"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ios",
        value_type: ValueKind::List,
        in_sections: &["os-security"],
        block: &[BigipPropertySpec {
            name: "versions",
            value_type: ValueKind::List,
            in_sections: &["os-security", "ios"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "versions",
        value_type: ValueKind::List,
        in_sections: &["os-security", "ios"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[BigipPropertySpec {
            name: "priority",
            value_type: ValueKind::Unknown,
            in_sections: &["os-security", "ios", "versions"],
            required: true,
            shape_kind: Some(ValueKind::Object),
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "priority",
        value_type: ValueKind::Unknown,
        in_sections: &["os-security", "ios", "versions"],
        required: true,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "from",
                value_type: ValueKind::String,
                in_sections: &["os-security", "ios", "versions", "priority"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "score",
                value_type: ValueKind::Integer,
                in_sections: &["os-security", "ios", "versions", "priority"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "to",
                value_type: ValueKind::String,
                in_sections: &["os-security", "ios", "versions", "priority"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "from",
        value_type: ValueKind::String,
        in_sections: &["os-security", "ios", "versions", "priority"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "score",
        value_type: ValueKind::Integer,
        in_sections: &["os-security", "ios", "versions", "priority"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "to",
        value_type: ValueKind::String,
        in_sections: &["os-security", "ios", "versions", "priority"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "parameters",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "ajax-mapping",
                value_type: ValueKind::String,
                in_sections: &["parameters"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attach-to-vtoken-report",
                value_type: ValueKind::Enum,
                in_sections: &["parameters"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "check-integrity",
                value_type: ValueKind::Enum,
                in_sections: &["parameters"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encrypt",
                value_type: ValueKind::Enum,
                in_sections: &["parameters"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "identify-as-username",
                value_type: ValueKind::Enum,
                in_sections: &["parameters"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "method",
                value_type: ValueKind::Enum,
                in_sections: &["parameters"],
                enum_values: &["GET", "POST"],
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobilesafe-encrypt",
                value_type: ValueKind::Enum,
                in_sections: &["parameters"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobilesafe-entangle",
                value_type: ValueKind::Enum,
                in_sections: &["parameters"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "obfuscate",
                value_type: ValueKind::Enum,
                in_sections: &["parameters"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "priority",
                value_type: ValueKind::Integer,
                in_sections: &["parameters"],
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protect-by-selector",
                value_type: ValueKind::Enum,
                in_sections: &["parameters"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "search-in",
                value_type: ValueKind::Enum,
                in_sections: &["parameters"],
                enum_values: &["any", "payload"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "substitute-value",
                value_type: ValueKind::Enum,
                in_sections: &["parameters"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["parameters"],
                enum_values: &["explicit", "wildcard"],
                usage_flags: &["read_only"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ajax-mapping",
        value_type: ValueKind::String,
        in_sections: &["parameters"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "attach-to-vtoken-report",
        value_type: ValueKind::Enum,
        in_sections: &["parameters"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "check-integrity",
        value_type: ValueKind::Enum,
        in_sections: &["parameters"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "encrypt",
        value_type: ValueKind::Enum,
        in_sections: &["parameters"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "identify-as-username",
        value_type: ValueKind::Enum,
        in_sections: &["parameters"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "method",
        value_type: ValueKind::Enum,
        in_sections: &["parameters"],
        enum_values: &["GET", "POST"],
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "mobilesafe-encrypt",
        value_type: ValueKind::Enum,
        in_sections: &["parameters"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "mobilesafe-entangle",
        value_type: ValueKind::Enum,
        in_sections: &["parameters"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "obfuscate",
        value_type: ValueKind::Enum,
        in_sections: &["parameters"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "priority",
        value_type: ValueKind::Integer,
        in_sections: &["parameters"],
        required: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "protect-by-selector",
        value_type: ValueKind::Enum,
        in_sections: &["parameters"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "search-in",
        value_type: ValueKind::Enum,
        in_sections: &["parameters"],
        enum_values: &["any", "payload"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "substitute-value",
        value_type: ValueKind::Enum,
        in_sections: &["parameters"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        in_sections: &["parameters"],
        enum_values: &["explicit", "wildcard"],
        usage_flags: &["read_only"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "phishing",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "alert-path",
                value_type: ValueKind::String,
                in_sections: &["phishing"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allowed-elements",
                value_type: ValueKind::List,
                in_sections: &["phishing"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allowed-referrers",
                value_type: ValueKind::List,
                in_sections: &["phishing"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "application-css",
                value_type: ValueKind::Enum,
                in_sections: &["phishing"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "application-css-locations",
                value_type: ValueKind::List,
                in_sections: &["phishing"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "capture-users",
                value_type: ValueKind::Enum,
                in_sections: &["phishing"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "copy-detection",
                value_type: ValueKind::Enum,
                in_sections: &["phishing"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "css-attribute-name",
                value_type: ValueKind::String,
                in_sections: &["phishing"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "css-location",
                value_type: ValueKind::String,
                in_sections: &["phishing"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "css-protection",
                value_type: ValueKind::Enum,
                in_sections: &["phishing"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "expiration-checks",
                value_type: ValueKind::Enum,
                in_sections: &["phishing"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-types-to-send",
                value_type: ValueKind::List,
                in_sections: &["phishing"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "image-location",
                value_type: ValueKind::String,
                in_sections: &["phishing"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "inject-css-element",
                value_type: ValueKind::Unknown,
                in_sections: &["phishing"],
                shape_kind: Some(ValueKind::Object),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "inject-css-link",
                value_type: ValueKind::Unknown,
                in_sections: &["phishing"],
                shape_kind: Some(ValueKind::Object),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "inject-inline-javascript",
                value_type: ValueKind::Unknown,
                in_sections: &["phishing"],
                shape_kind: Some(ValueKind::Object),
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protected-elements",
                value_type: ValueKind::List,
                in_sections: &["phishing"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "referrer-checks",
                value_type: ValueKind::Enum,
                in_sections: &["phishing"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "alert-path",
        value_type: ValueKind::String,
        in_sections: &["phishing"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "allowed-elements",
        value_type: ValueKind::List,
        in_sections: &["phishing"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "allowed-referrers",
        value_type: ValueKind::List,
        in_sections: &["phishing"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "application-css",
        value_type: ValueKind::Enum,
        in_sections: &["phishing"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "application-css-locations",
        value_type: ValueKind::List,
        in_sections: &["phishing"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "capture-users",
        value_type: ValueKind::Enum,
        in_sections: &["phishing"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "copy-detection",
        value_type: ValueKind::Enum,
        in_sections: &["phishing"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "css-attribute-name",
        value_type: ValueKind::String,
        in_sections: &["phishing"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "css-location",
        value_type: ValueKind::String,
        in_sections: &["phishing"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "css-protection",
        value_type: ValueKind::Enum,
        in_sections: &["phishing"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "expiration-checks",
        value_type: ValueKind::Enum,
        in_sections: &["phishing"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-types-to-send",
        value_type: ValueKind::List,
        in_sections: &["phishing"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "image-location",
        value_type: ValueKind::String,
        in_sections: &["phishing"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "inject-css-element",
        value_type: ValueKind::Unknown,
        in_sections: &["phishing"],
        shape_kind: Some(ValueKind::Object),
        usage_flags: &["deprecated"],
        block: &[BigipPropertySpec {
            name: "tag",
            value_type: ValueKind::String,
            in_sections: &["phishing", "inject-css-element"],
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "tag",
        value_type: ValueKind::String,
        in_sections: &["phishing", "inject-css-element"],
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "inject-css-link",
        value_type: ValueKind::Unknown,
        in_sections: &["phishing"],
        shape_kind: Some(ValueKind::Object),
        usage_flags: &["deprecated"],
        block: &[BigipPropertySpec {
            name: "tag",
            value_type: ValueKind::String,
            in_sections: &["phishing", "inject-css-link"],
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "tag",
        value_type: ValueKind::String,
        in_sections: &["phishing", "inject-css-link"],
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "inject-inline-javascript",
        value_type: ValueKind::Unknown,
        in_sections: &["phishing"],
        shape_kind: Some(ValueKind::Object),
        usage_flags: &["deprecated"],
        block: &[BigipPropertySpec {
            name: "tag",
            value_type: ValueKind::String,
            in_sections: &["phishing", "inject-inline-javascript"],
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "tag",
        value_type: ValueKind::String,
        in_sections: &["phishing", "inject-inline-javascript"],
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "protected-elements",
        value_type: ValueKind::List,
        in_sections: &["phishing"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "referrer-checks",
        value_type: ValueKind::Enum,
        in_sections: &["phishing"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "priority",
        value_type: ValueKind::Integer,
        required: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "referrer-info-header",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "removed-scripts",
        value_type: ValueKind::List,
        block: &[
            BigipPropertySpec {
                name: "blacklist-functions",
                value_type: ValueKind::List,
                in_sections: &["removed-scripts"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "whitelist-functions",
                value_type: ValueKind::List,
                in_sections: &["removed-scripts"],
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
        in_sections: &["removed-scripts"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "whitelist-functions",
        value_type: ValueKind::List,
        in_sections: &["removed-scripts"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "risk-engine-path",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "risk-engine-publisher",
        value_type: ValueKind::Reference,
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rooting-jailbreak",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "custom-config",
                value_type: ValueKind::String,
                in_sections: &["rooting-jailbreak"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jailbreak-score",
                value_type: ValueKind::Integer,
                in_sections: &["rooting-jailbreak"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rooting-score",
                value_type: ValueKind::Integer,
                in_sections: &["rooting-jailbreak"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "custom-config",
        value_type: ValueKind::String,
        in_sections: &["rooting-jailbreak"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "jailbreak-score",
        value_type: ValueKind::Integer,
        in_sections: &["rooting-jailbreak"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rooting-score",
        value_type: ValueKind::Integer,
        in_sections: &["rooting-jailbreak"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rules",
        value_type: ValueKind::List,
        allow_none: true,
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
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[BigipPropertySpec {
            name: "event",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            required: true,
            enum_values: &[
                "auto-transaction",
                "client-network-connection",
                "client-side-missing-components",
                "encryption-failure",
                "generic-malware",
                "mandatory-words",
                "phishing",
                "phishing-user",
                "rat-detection",
                "referrer-checks",
                "server-side-missing-components",
                "source-integrity",
                "web-injection",
            ],
            shape_kind: Some(ValueKind::Object),
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "event",
        value_type: ValueKind::Unknown,
        in_sections: &["rules"],
        required: true,
        enum_values: &[
            "auto-transaction",
            "client-network-connection",
            "client-side-missing-components",
            "encryption-failure",
            "generic-malware",
            "mandatory-words",
            "phishing",
            "phishing-user",
            "rat-detection",
            "referrer-checks",
            "server-side-missing-components",
            "source-integrity",
            "web-injection",
        ],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["rules", "event"],
                enum_values: &[
                    "block-user",
                    "forensic",
                    "inspection",
                    "redirect",
                    "remediation",
                    "route",
                    "web-service",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "duration",
                value_type: ValueKind::Integer,
                in_sections: &["rules", "event"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforce-policy",
                value_type: ValueKind::Enum,
                in_sections: &["rules", "event"],
                enum_values: &["enforce", "time-limited", "unlimited"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-score",
                value_type: ValueKind::Integer,
                in_sections: &["rules", "event"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "payload",
                value_type: ValueKind::String,
                in_sections: &["rules", "event"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                in_sections: &["rules", "event"],
                allow_none: true,
                references: &[
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "publisher",
                value_type: ValueKind::Reference,
                in_sections: &["rules", "event"],
                allow_none: true,
                references: &[
                    "security_blacklist_publisher_all_blacklist_publisher",
                    "security_blacklist_publisher_blacklist_publisher_stats",
                    "security_blacklist_publisher_by_addr",
                    "security_blacklist_publisher_by_category",
                    "security_blacklist_publisher_category",
                    "security_blacklist_publisher_profile",
                    "sys_icall_publisher",
                    "sys_log_config_publisher",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::String,
                in_sections: &["rules", "event"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "action",
        value_type: ValueKind::Enum,
        in_sections: &["rules", "event"],
        enum_values: &[
            "block-user",
            "forensic",
            "inspection",
            "redirect",
            "remediation",
            "route",
            "web-service",
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "duration",
        value_type: ValueKind::Integer,
        in_sections: &["rules", "event"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "enforce-policy",
        value_type: ValueKind::Enum,
        in_sections: &["rules", "event"],
        enum_values: &["enforce", "time-limited", "unlimited"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "min-score",
        value_type: ValueKind::Integer,
        in_sections: &["rules", "event"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "payload",
        value_type: ValueKind::String,
        in_sections: &["rules", "event"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "pool",
        value_type: ValueKind::Reference,
        in_sections: &["rules", "event"],
        allow_none: true,
        references: &[
            "analytics_lsn_pool_report",
            "analytics_lsn_pool_scheduled_report",
            "analytics_pool_traffic_report",
            "analytics_pool_traffic_scheduled_report",
            "gtm_pool_a",
            "gtm_pool_aaaa",
            "gtm_pool_cname",
            "gtm_pool_https",
            "gtm_pool_mx",
            "gtm_pool_naptr",
            "gtm_pool_srv",
            "gtm_pool_svcb",
            "gtm_prober_pool",
            "ltm_lsn_pool",
            "ltm_pool",
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "publisher",
        value_type: ValueKind::Reference,
        in_sections: &["rules", "event"],
        allow_none: true,
        references: &[
            "security_blacklist_publisher_all_blacklist_publisher",
            "security_blacklist_publisher_blacklist_publisher_stats",
            "security_blacklist_publisher_by_addr",
            "security_blacklist_publisher_by_category",
            "security_blacklist_publisher_category",
            "security_blacklist_publisher_profile",
            "sys_icall_publisher",
            "sys_log_config_publisher",
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "url",
        value_type: ValueKind::String,
        in_sections: &["rules", "event"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "same-domain-scripts-validation-header",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "self-bait-header",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "source-integrity-location",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "suggested-username-header",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
];
