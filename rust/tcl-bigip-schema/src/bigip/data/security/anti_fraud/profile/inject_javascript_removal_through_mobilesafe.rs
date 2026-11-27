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

//! Properties `inject-javascript-removal` to `mobilesafe` of `security anti-fraud profile`, each with the properties of its sections.

use crate::bigip::{BigipPropertySpec, ValueKind};

pub(super) const PROPERTIES: &[BigipPropertySpec] = &[
    BigipPropertySpec {
        name: "inject-javascript-removal",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[BigipPropertySpec {
            name: "tag",
            value_type: ValueKind::String,
            in_sections: &["inject-javascript-removal"],
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "tag",
        value_type: ValueKind::String,
        in_sections: &["inject-javascript-removal"],
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "inject-main-javascript",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        usage_flags: &["deprecated"],
        block: &[BigipPropertySpec {
            name: "tag",
            value_type: ValueKind::String,
            in_sections: &["inject-main-javascript"],
            usage_flags: &["deprecated"],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "tag",
        value_type: ValueKind::String,
        in_sections: &["inject-main-javascript"],
        usage_flags: &["deprecated"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "inline-scripts-whitelist-signatures",
        value_type: ValueKind::List,
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ios",
        value_type: ValueKind::List,
        block: &[BigipPropertySpec {
            name: "custom-malware",
            value_type: ValueKind::List,
            in_sections: &["ios"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "custom-malware",
        value_type: ValueKind::List,
        in_sections: &["ios"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "path",
                value_type: ValueKind::String,
                in_sections: &["ios", "custom-malware"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "score",
                value_type: ValueKind::Integer,
                in_sections: &["ios", "custom-malware"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "path",
        value_type: ValueKind::String,
        in_sections: &["ios", "custom-malware"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "score",
        value_type: ValueKind::Integer,
        in_sections: &["ios", "custom-malware"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "javascript-grace-threshold",
        value_type: ValueKind::Integer,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "javascript-location",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "javascript-removal-location",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "local-syslog-publisher",
        value_type: ValueKind::Reference,
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "login-response",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "domain-cookie",
                value_type: ValueKind::String,
                in_sections: &["login-response"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "exclude-string",
                value_type: ValueKind::String,
                in_sections: &["login-response"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "header",
                value_type: ValueKind::String,
                in_sections: &["login-response"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-string",
                value_type: ValueKind::String,
                in_sections: &["login-response"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "status-code",
                value_type: ValueKind::Integer,
                in_sections: &["login-response"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "validation",
                value_type: ValueKind::Enum,
                in_sections: &["login-response"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "domain-cookie",
        value_type: ValueKind::String,
        in_sections: &["login-response"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "exclude-string",
        value_type: ValueKind::String,
        in_sections: &["login-response"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "header",
        value_type: ValueKind::String,
        in_sections: &["login-response"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "include-string",
        value_type: ValueKind::String,
        in_sections: &["login-response"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "status-code",
        value_type: ValueKind::Integer,
        in_sections: &["login-response"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "validation",
        value_type: ValueKind::Enum,
        in_sections: &["login-response"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "malware",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "allowed-domains",
                value_type: ValueKind::List,
                in_sections: &["malware"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attach-html-to-alerts",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-learn-form-tags",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-learn-input-tags",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-learn-script-tags",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bait-check-generic",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bait-location",
                value_type: ValueKind::String,
                in_sections: &["malware"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blacklist-functions",
                value_type: ValueKind::List,
                in_sections: &["malware"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blacklist-js-words",
                value_type: ValueKind::List,
                in_sections: &["malware"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blacklist-urls",
                value_type: ValueKind::List,
                in_sections: &["malware"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blacklist-words",
                value_type: ValueKind::List,
                in_sections: &["malware"],
                repeated: true,
                allow_none: true,
                usage_flags: &["deprecated"],
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blocked-enter-key-detection",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "browser-cache",
                value_type: ValueKind::Unknown,
                in_sections: &["malware"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "deferred-execution",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "detected-malware",
                value_type: ValueKind::List,
                in_sections: &["malware"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dom-signatures",
                value_type: ValueKind::List,
                in_sections: &["malware"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain-availability",
                value_type: ValueKind::Unknown,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enable-symbols",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "external-injection",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "generic-malware",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "manual-count-form-tags",
                value_type: ValueKind::Integer,
                in_sections: &["malware"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "manual-count-input-tags",
                value_type: ValueKind::Integer,
                in_sections: &["malware"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "manual-count-script-tags",
                value_type: ValueKind::Integer,
                in_sections: &["malware"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-exfiltration-detection",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rat-detection",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "removed-scripts-detection",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "same-domain-scripts-validation",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "self-bait",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-integrity",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vbklip-detection",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "visibility-check",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "visibility-check-items",
                value_type: ValueKind::List,
                in_sections: &["malware"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "web-rootkit-detection",
                value_type: ValueKind::Enum,
                in_sections: &["malware"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "whitelist-dom-signatures",
                value_type: ValueKind::List,
                in_sections: &["malware"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "whitelist-words",
                value_type: ValueKind::List,
                in_sections: &["malware"],
                repeated: true,
                allow_none: true,
                usage_flags: &["deprecated"],
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "allowed-domains",
        value_type: ValueKind::List,
        in_sections: &["malware"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "attach-html-to-alerts",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "auto-learn-form-tags",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "auto-learn-input-tags",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "auto-learn-script-tags",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "bait-check-generic",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "bait-location",
        value_type: ValueKind::String,
        in_sections: &["malware"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-functions",
        value_type: ValueKind::List,
        in_sections: &["malware"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-js-words",
        value_type: ValueKind::List,
        in_sections: &["malware"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-urls",
        value_type: ValueKind::List,
        in_sections: &["malware"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-words",
        value_type: ValueKind::List,
        in_sections: &["malware"],
        repeated: true,
        allow_none: true,
        usage_flags: &["deprecated"],
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blocked-enter-key-detection",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "browser-cache",
        value_type: ValueKind::Unknown,
        in_sections: &["malware"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "blacklist-urls",
                value_type: ValueKind::List,
                in_sections: &["malware", "browser-cache"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "whitelist-urls",
                value_type: ValueKind::List,
                in_sections: &["malware", "browser-cache"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-urls",
        value_type: ValueKind::List,
        in_sections: &["malware", "browser-cache"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "whitelist-urls",
        value_type: ValueKind::List,
        in_sections: &["malware", "browser-cache"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "deferred-execution",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "detected-malware",
        value_type: ValueKind::List,
        in_sections: &["malware"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[BigipPropertySpec {
            name: "baits",
            value_type: ValueKind::List,
            in_sections: &["malware", "detected-malware"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "baits",
        value_type: ValueKind::List,
        in_sections: &["malware", "detected-malware"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "data-before",
                value_type: ValueKind::String,
                in_sections: &["malware", "detected-malware", "baits"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "data-inject",
                value_type: ValueKind::String,
                in_sections: &["malware", "detected-malware", "baits"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trigger-url",
                value_type: ValueKind::Unknown,
                in_sections: &["malware", "detected-malware", "baits"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "data-before",
        value_type: ValueKind::String,
        in_sections: &["malware", "detected-malware", "baits"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "data-inject",
        value_type: ValueKind::String,
        in_sections: &["malware", "detected-malware", "baits"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "trigger-url",
        value_type: ValueKind::Unknown,
        in_sections: &["malware", "detected-malware", "baits"],
        shape_kind: Some(ValueKind::Object),
        block: &[BigipPropertySpec {
            name: "position",
            value_type: ValueKind::Enum,
            in_sections: &["malware", "detected-malware", "baits", "trigger-url"],
            enum_values: &["alone", "any", "last"],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "position",
        value_type: ValueKind::Enum,
        in_sections: &["malware", "detected-malware", "baits", "trigger-url"],
        enum_values: &["alone", "any", "last"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dom-signatures",
        value_type: ValueKind::List,
        in_sections: &["malware"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "attribute-name",
                value_type: ValueKind::String,
                in_sections: &["malware", "dom-signatures"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hash-id",
                value_type: ValueKind::String,
                in_sections: &["malware", "dom-signatures"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "html-tag",
                value_type: ValueKind::String,
                in_sections: &["malware", "dom-signatures"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-type",
                value_type: ValueKind::Enum,
                in_sections: &["malware", "dom-signatures"],
                enum_values: &["contains", "is"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "search-for",
                value_type: ValueKind::String,
                in_sections: &["malware", "dom-signatures"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "search-in",
                value_type: ValueKind::Enum,
                in_sections: &["malware", "dom-signatures"],
                enum_values: &["all", "attribute", "html", "text"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "attribute-name",
        value_type: ValueKind::String,
        in_sections: &["malware", "dom-signatures"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "hash-id",
        value_type: ValueKind::String,
        in_sections: &["malware", "dom-signatures"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "html-tag",
        value_type: ValueKind::String,
        in_sections: &["malware", "dom-signatures"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "match-type",
        value_type: ValueKind::Enum,
        in_sections: &["malware", "dom-signatures"],
        enum_values: &["contains", "is"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "search-for",
        value_type: ValueKind::String,
        in_sections: &["malware", "dom-signatures"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "search-in",
        value_type: ValueKind::Enum,
        in_sections: &["malware", "dom-signatures"],
        enum_values: &["all", "attribute", "html", "text"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "domain-availability",
        value_type: ValueKind::Unknown,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "blacklist-urls",
                value_type: ValueKind::List,
                in_sections: &["malware", "domain-availability"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "whitelist-urls",
                value_type: ValueKind::List,
                in_sections: &["malware", "domain-availability"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "blacklist-urls",
        value_type: ValueKind::List,
        in_sections: &["malware", "domain-availability"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "whitelist-urls",
        value_type: ValueKind::List,
        in_sections: &["malware", "domain-availability"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "enable-symbols",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "external-injection",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "generic-malware",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "manual-count-form-tags",
        value_type: ValueKind::Integer,
        in_sections: &["malware"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "manual-count-input-tags",
        value_type: ValueKind::Integer,
        in_sections: &["malware"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "manual-count-script-tags",
        value_type: ValueKind::Integer,
        in_sections: &["malware"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "password-exfiltration-detection",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rat-detection",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "removed-scripts-detection",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "same-domain-scripts-validation",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "self-bait",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "source-integrity",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "vbklip-detection",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "visibility-check",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "visibility-check-items",
        value_type: ValueKind::List,
        in_sections: &["malware"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "web-rootkit-detection",
        value_type: ValueKind::Enum,
        in_sections: &["malware"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "whitelist-dom-signatures",
        value_type: ValueKind::List,
        in_sections: &["malware"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "whitelist-words",
        value_type: ValueKind::List,
        in_sections: &["malware"],
        repeated: true,
        allow_none: true,
        usage_flags: &["deprecated"],
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "mitm",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "certificate-custom-config",
                value_type: ValueKind::String,
                in_sections: &["mitm"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-custom-config",
                value_type: ValueKind::String,
                in_sections: &["mitm"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domains",
                value_type: ValueKind::List,
                in_sections: &["mitm"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "certificate-custom-config",
        value_type: ValueKind::String,
        in_sections: &["mitm"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dns-custom-config",
        value_type: ValueKind::String,
        in_sections: &["mitm"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "domains",
        value_type: ValueKind::List,
        in_sections: &["mitm"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "certificate",
                value_type: ValueKind::Unknown,
                in_sections: &["mitm", "domains"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns",
                value_type: ValueKind::List,
                in_sections: &["mitm", "domains"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "certificate",
        value_type: ValueKind::Unknown,
        in_sections: &["mitm", "domains"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "forging-score",
                value_type: ValueKind::Integer,
                in_sections: &["mitm", "domains", "certificate"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hash",
                value_type: ValueKind::String,
                in_sections: &["mitm", "domains", "certificate"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "forging-score",
        value_type: ValueKind::Integer,
        in_sections: &["mitm", "domains", "certificate"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "hash",
        value_type: ValueKind::String,
        in_sections: &["mitm", "domains", "certificate"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dns",
        value_type: ValueKind::List,
        in_sections: &["mitm", "domains"],
        block: &[
            BigipPropertySpec {
                name: "ip-ranges",
                value_type: ValueKind::List,
                in_sections: &["mitm", "domains", "dns"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "spoofing-score",
                value_type: ValueKind::Integer,
                in_sections: &["mitm", "domains", "dns"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-ranges",
        value_type: ValueKind::List,
        in_sections: &["mitm", "domains", "dns"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "spoofing-score",
        value_type: ValueKind::Integer,
        in_sections: &["mitm", "domains", "dns"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "mobilesafe",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[
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
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
];
