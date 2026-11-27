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

//! BIG-IP object specs for `security http`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "security_http_profile",
        table_name: None,
        resolver_name: None,
        module: Some("security"),
        object_types: &["http profile"],
    },
    header_types: &[("security", "http profile")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["security_http_profile"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "evasion-techniques",
            value_type: ValueKind::List,
            block: &[
                BigipPropertySpec {
                    name: "alarm",
                    value_type: ValueKind::Enum,
                    in_sections: &["evasion-techniques"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "block",
                    value_type: ValueKind::Enum,
                    in_sections: &["evasion-techniques"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "alarm",
            value_type: ValueKind::Enum,
            in_sections: &["evasion-techniques"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "block",
            value_type: ValueKind::Enum,
            in_sections: &["evasion-techniques"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "file-types",
            value_type: ValueKind::List,
            block: &[
                BigipPropertySpec {
                    name: "alarm",
                    value_type: ValueKind::Enum,
                    in_sections: &["file-types"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "block",
                    value_type: ValueKind::Enum,
                    in_sections: &["file-types"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "values",
                    value_type: ValueKind::List,
                    in_sections: &["file-types"],
                    repeated: true,
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "alarm",
            value_type: ValueKind::Enum,
            in_sections: &["file-types"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "block",
            value_type: ValueKind::Enum,
            in_sections: &["file-types"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "values",
            value_type: ValueKind::List,
            in_sections: &["file-types"],
            repeated: true,
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "http-rfc",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "alarm",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bad-host-header",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "bad-version",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "block",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "body-in-get-head",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "chunked-with-content-length",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "content-length-is-positive",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "header-name-without-value",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "high-ascii-in-headers",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "host-header-is-ip",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "linefolding-in-headers",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "maximum-headers",
                    value_type: ValueKind::Integer,
                    in_sections: &["http-rfc"],
                    default: Some("a maximum of 20 headers"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "null-in-body",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "null-in-headers",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "post-with-zero-length",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "several-content-length",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "unparsable-content",
                    value_type: ValueKind::Enum,
                    in_sections: &["http-rfc"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "alarm",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bad-host-header",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bad-version",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "block",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "body-in-get-head",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "chunked-with-content-length",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "content-length-is-positive",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "header-name-without-value",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "high-ascii-in-headers",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "host-header-is-ip",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "linefolding-in-headers",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "maximum-headers",
            value_type: ValueKind::Integer,
            in_sections: &["http-rfc"],
            default: Some("a maximum of 20 headers"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "null-in-body",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "null-in-headers",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "post-with-zero-length",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "several-content-length",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "unparsable-content",
            value_type: ValueKind::Enum,
            in_sections: &["http-rfc"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mandatory-headers",
            value_type: ValueKind::List,
            block: &[
                BigipPropertySpec {
                    name: "alarm",
                    value_type: ValueKind::Enum,
                    in_sections: &["mandatory-headers"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "block",
                    value_type: ValueKind::Enum,
                    in_sections: &["mandatory-headers"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "values",
                    value_type: ValueKind::List,
                    in_sections: &["mandatory-headers"],
                    repeated: true,
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "alarm",
            value_type: ValueKind::Enum,
            in_sections: &["mandatory-headers"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "block",
            value_type: ValueKind::Enum,
            in_sections: &["mandatory-headers"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "values",
            value_type: ValueKind::List,
            in_sections: &["mandatory-headers"],
            repeated: true,
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "maximum-length",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "alarm",
                    value_type: ValueKind::Enum,
                    in_sections: &["maximum-length"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "block",
                    value_type: ValueKind::Enum,
                    in_sections: &["maximum-length"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "post-data",
                    value_type: ValueKind::Integer,
                    in_sections: &["maximum-length"],
                    default: Some("any (no restriction)"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "query-string",
                    value_type: ValueKind::Integer,
                    in_sections: &["maximum-length"],
                    default: Some("1024 bytes"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "request",
                    value_type: ValueKind::Integer,
                    in_sections: &["maximum-length"],
                    default: Some("any (no restriction)"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "uri",
                    value_type: ValueKind::Integer,
                    in_sections: &["maximum-length"],
                    default: Some("1024 bytes"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "alarm",
            value_type: ValueKind::Enum,
            in_sections: &["maximum-length"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "block",
            value_type: ValueKind::Enum,
            in_sections: &["maximum-length"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "post-data",
            value_type: ValueKind::Integer,
            in_sections: &["maximum-length"],
            default: Some("any (no restriction)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "query-string",
            value_type: ValueKind::Integer,
            in_sections: &["maximum-length"],
            default: Some("1024 bytes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request",
            value_type: ValueKind::Integer,
            in_sections: &["maximum-length"],
            default: Some("any (no restriction)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "uri",
            value_type: ValueKind::Integer,
            in_sections: &["maximum-length"],
            default: Some("1024 bytes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "methods",
            value_type: ValueKind::List,
            block: &[
                BigipPropertySpec {
                    name: "alarm",
                    value_type: ValueKind::Enum,
                    in_sections: &["methods"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("enabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "block",
                    value_type: ValueKind::Enum,
                    in_sections: &["methods"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("disabled"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "values",
                    value_type: ValueKind::List,
                    in_sections: &["methods"],
                    repeated: true,
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "alarm",
            value_type: ValueKind::Enum,
            in_sections: &["methods"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "block",
            value_type: ValueKind::Enum,
            in_sections: &["methods"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "values",
            value_type: ValueKind::List,
            in_sections: &["methods"],
            repeated: true,
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "response",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "body",
                    value_type: ValueKind::String,
                    in_sections: &["response"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "headers",
                    value_type: ValueKind::Unknown,
                    in_sections: &["response"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "type",
                    value_type: ValueKind::Enum,
                    in_sections: &["response"],
                    enum_values: &["custom", "default", "redirect", "soap-fault"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "url",
                    value_type: ValueKind::String,
                    in_sections: &["response"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "body",
            value_type: ValueKind::String,
            in_sections: &["response"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "headers",
            value_type: ValueKind::Unknown,
            in_sections: &["response"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Enum,
            in_sections: &["response"],
            enum_values: &["custom", "default", "redirect", "soap-fault"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "url",
            value_type: ValueKind::String,
            in_sections: &["response"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
