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

//! Properties `antifraud` to `ip-intelligence` of `security log`, each with the properties of its sections.

use crate::bigip::{BigipPropertySpec, ValueKind};

pub(super) const PROPERTIES: &[BigipPropertySpec] = &[
    BigipPropertySpec {
        name: "antifraud",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "encode-fields",
                value_type: ValueKind::List,
                in_sections: &["antifraud"],
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "events",
                value_type: ValueKind::List,
                in_sections: &["antifraud"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-limit-template",
                value_type: ValueKind::String,
                in_sections: &["antifraud"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-publisher",
                value_type: ValueKind::Reference,
                in_sections: &["antifraud"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "encode-fields",
        value_type: ValueKind::List,
        in_sections: &["antifraud"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "events",
        value_type: ValueKind::List,
        in_sections: &["antifraud"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Unknown,
            in_sections: &["antifraud", "events"],
            enum_values: &["alert", "login"],
            shape_kind: Some(ValueKind::Object),
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Unknown,
        in_sections: &["antifraud", "events"],
        enum_values: &["alert", "login"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "format",
                value_type: ValueKind::Unknown,
                in_sections: &["antifraud", "events", "type"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-limit",
                value_type: ValueKind::Integer,
                in_sections: &["antifraud", "events", "type"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "format",
        value_type: ValueKind::Unknown,
        in_sections: &["antifraud", "events", "type"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["antifraud", "events", "type", "format"],
                allow_none: true,
                enum_values: &["default", "none", "user-defined"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-template",
                value_type: ValueKind::String,
                in_sections: &["antifraud", "events", "type", "format"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        in_sections: &["antifraud", "events", "type", "format"],
        allow_none: true,
        enum_values: &["default", "none", "user-defined"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-template",
        value_type: ValueKind::String,
        in_sections: &["antifraud", "events", "type", "format"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rate-limit",
        value_type: ValueKind::Integer,
        in_sections: &["antifraud", "events", "type"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rate-limit-template",
        value_type: ValueKind::String,
        in_sections: &["antifraud"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "remote-publisher",
        value_type: ValueKind::Reference,
        in_sections: &["antifraud"],
        allow_none: true,
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
        name: "application",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "facility",
                value_type: ValueKind::Enum,
                in_sections: &["application"],
                enum_values: &[
                    "local0", "local1", "local2", "local3", "local4", "local5", "local6", "local7",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "filter",
                value_type: ValueKind::List,
                in_sections: &["application"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "format",
                value_type: ValueKind::Unknown,
                in_sections: &["application"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "guarantee-logging",
                value_type: ValueKind::Enum,
                in_sections: &["application"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "guarantee-response-logging",
                value_type: ValueKind::Enum,
                in_sections: &["application"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-storage",
                value_type: ValueKind::Enum,
                in_sections: &["application"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "logic-operation",
                value_type: ValueKind::Enum,
                in_sections: &["application"],
                enum_values: &["and", "or"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "maximum-entry-length",
                value_type: ValueKind::Enum,
                in_sections: &["application"],
                enum_values: &["10k", "1k", "2k", "64k"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "maximum-header-size",
                value_type: ValueKind::Integer,
                in_sections: &["application"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "maximum-query-size",
                value_type: ValueKind::Integer,
                in_sections: &["application"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "maximum-request-size",
                value_type: ValueKind::Integer,
                in_sections: &["application"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Enum,
                in_sections: &["application"],
                enum_values: &["tcp", "tcp-rfc3195", "udp"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-storage",
                value_type: ValueKind::Enum,
                in_sections: &["application"],
                allow_none: true,
                enum_values: &["arcsight", "none", "remote", "splunk"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "report-anomalies",
                value_type: ValueKind::Enum,
                in_sections: &["application"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "response-logging",
                value_type: ValueKind::Enum,
                in_sections: &["application"],
                allow_none: true,
                enum_values: &["all", "illegal", "none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "servers",
                value_type: ValueKind::List,
                in_sections: &["application"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "facility",
        value_type: ValueKind::Enum,
        in_sections: &["application"],
        enum_values: &[
            "local0", "local1", "local2", "local3", "local4", "local5", "local6", "local7",
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "filter",
        value_type: ValueKind::List,
        in_sections: &["application"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[BigipPropertySpec {
            name: "key",
            value_type: ValueKind::Unknown,
            in_sections: &["application", "filter"],
            required: true,
            enum_values: &[
                "http-method",
                "protocol",
                "request-type",
                "response-code",
                "search-all",
                "search-in-headers",
                "search-in-post-data",
                "search-in-request",
                "search-in-uri",
            ],
            shape_kind: Some(ValueKind::Object),
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "key",
        value_type: ValueKind::Unknown,
        in_sections: &["application", "filter"],
        required: true,
        enum_values: &[
            "http-method",
            "protocol",
            "request-type",
            "response-code",
            "search-all",
            "search-in-headers",
            "search-in-post-data",
            "search-in-request",
            "search-in-uri",
        ],
        shape_kind: Some(ValueKind::Object),
        block: &[BigipPropertySpec {
            name: "values",
            value_type: ValueKind::List,
            in_sections: &["application", "filter", "key"],
            repeated: true,
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "values",
        value_type: ValueKind::List,
        in_sections: &["application", "filter", "key"],
        repeated: true,
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "format",
        value_type: ValueKind::Unknown,
        in_sections: &["application"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "field-delimiter",
                value_type: ValueKind::String,
                in_sections: &["application", "format"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-format",
                value_type: ValueKind::String,
                in_sections: &["application", "format"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fields",
                value_type: ValueKind::List,
                in_sections: &["application", "format"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["application", "format"],
                enum_values: &["predefined", "user-defined"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-string",
                value_type: ValueKind::String,
                in_sections: &["application", "format"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-delimiter",
        value_type: ValueKind::String,
        in_sections: &["application", "format"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-format",
        value_type: ValueKind::String,
        in_sections: &["application", "format"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "fields",
        value_type: ValueKind::List,
        in_sections: &["application", "format"],
        repeated: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        in_sections: &["application", "format"],
        enum_values: &["predefined", "user-defined"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-string",
        value_type: ValueKind::String,
        in_sections: &["application", "format"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "guarantee-logging",
        value_type: ValueKind::Enum,
        in_sections: &["application"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "guarantee-response-logging",
        value_type: ValueKind::Enum,
        in_sections: &["application"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "local-storage",
        value_type: ValueKind::Enum,
        in_sections: &["application"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "logic-operation",
        value_type: ValueKind::Enum,
        in_sections: &["application"],
        enum_values: &["and", "or"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "maximum-entry-length",
        value_type: ValueKind::Enum,
        in_sections: &["application"],
        enum_values: &["10k", "1k", "2k", "64k"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "maximum-header-size",
        value_type: ValueKind::Integer,
        in_sections: &["application"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "maximum-query-size",
        value_type: ValueKind::Integer,
        in_sections: &["application"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "maximum-request-size",
        value_type: ValueKind::Integer,
        in_sections: &["application"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "protocol",
        value_type: ValueKind::Enum,
        in_sections: &["application"],
        enum_values: &["tcp", "tcp-rfc3195", "udp"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "remote-storage",
        value_type: ValueKind::Enum,
        in_sections: &["application"],
        allow_none: true,
        enum_values: &["arcsight", "none", "remote", "splunk"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "report-anomalies",
        value_type: ValueKind::Enum,
        in_sections: &["application"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "response-logging",
        value_type: ValueKind::Enum,
        in_sections: &["application"],
        allow_none: true,
        enum_values: &["all", "illegal", "none"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "servers",
        value_type: ValueKind::List,
        in_sections: &["application"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "bot-defense",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "filter",
                value_type: ValueKind::Unknown,
                in_sections: &["bot-defense"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-publisher",
                value_type: ValueKind::Reference,
                in_sections: &["bot-defense"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-publisher",
                value_type: ValueKind::Reference,
                in_sections: &["bot-defense"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "filter",
        value_type: ValueKind::Unknown,
        in_sections: &["bot-defense"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "log-bot-signature-matched-requests",
                value_type: ValueKind::Enum,
                in_sections: &["bot-defense", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-captcha-challenged-requests",
                value_type: ValueKind::Enum,
                in_sections: &["bot-defense", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-challenged-requests",
                value_type: ValueKind::Enum,
                in_sections: &["bot-defense", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-illegal-requests",
                value_type: ValueKind::Enum,
                in_sections: &["bot-defense", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-legal-requests",
                value_type: ValueKind::Enum,
                in_sections: &["bot-defense", "filter"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-bot-signature-matched-requests",
        value_type: ValueKind::Enum,
        in_sections: &["bot-defense", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-captcha-challenged-requests",
        value_type: ValueKind::Enum,
        in_sections: &["bot-defense", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-challenged-requests",
        value_type: ValueKind::Enum,
        in_sections: &["bot-defense", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-illegal-requests",
        value_type: ValueKind::Enum,
        in_sections: &["bot-defense", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-legal-requests",
        value_type: ValueKind::Enum,
        in_sections: &["bot-defense", "filter"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "local-publisher",
        value_type: ValueKind::Reference,
        in_sections: &["bot-defense"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "remote-publisher",
        value_type: ValueKind::Reference,
        in_sections: &["bot-defense"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "built-in",
        value_type: ValueKind::Enum,
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "description",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dos-application",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "local-publisher",
                value_type: ValueKind::Reference,
                in_sections: &["dos-application"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-publisher",
                value_type: ValueKind::Reference,
                in_sections: &["dos-application"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "local-publisher",
        value_type: ValueKind::Reference,
        in_sections: &["dos-application"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "remote-publisher",
        value_type: ValueKind::Reference,
        in_sections: &["dos-application"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dos-network-publisher",
        value_type: ValueKind::Reference,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "flowspec",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[BigipPropertySpec {
            name: "log-publisher",
            value_type: ValueKind::Reference,
            in_sections: &["flowspec"],
            ..BigipPropertySpec::DEFAULT
        }],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-publisher",
        value_type: ValueKind::Reference,
        in_sections: &["flowspec"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "ip-intelligence",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "aggregate-rate",
                value_type: ValueKind::Integer,
                in_sections: &["ip-intelligence"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-geo",
                value_type: ValueKind::Enum,
                in_sections: &["ip-intelligence"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-publisher",
                value_type: ValueKind::Reference,
                in_sections: &["ip-intelligence"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-rtbh",
                value_type: ValueKind::Enum,
                in_sections: &["ip-intelligence"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-scrubber",
                value_type: ValueKind::Enum,
                in_sections: &["ip-intelligence"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-shun",
                value_type: ValueKind::Enum,
                in_sections: &["ip-intelligence"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-translation-fields",
                value_type: ValueKind::Enum,
                in_sections: &["ip-intelligence"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "aggregate-rate",
        value_type: ValueKind::Integer,
        in_sections: &["ip-intelligence"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-geo",
        value_type: ValueKind::Enum,
        in_sections: &["ip-intelligence"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-publisher",
        value_type: ValueKind::Reference,
        in_sections: &["ip-intelligence"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-rtbh",
        value_type: ValueKind::Enum,
        in_sections: &["ip-intelligence"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-scrubber",
        value_type: ValueKind::Enum,
        in_sections: &["ip-intelligence"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-shun",
        value_type: ValueKind::Enum,
        in_sections: &["ip-intelligence"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-translation-fields",
        value_type: ValueKind::Enum,
        in_sections: &["ip-intelligence"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
];
