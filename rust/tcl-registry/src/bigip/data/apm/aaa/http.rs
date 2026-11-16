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

//! BIG-IP object specs for `apm aaa http`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_aaa_http",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["aaa http"],
    },
    header_types: &[("apm", "aaa http")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auth-type",
            value_type: ValueKind::Enum,
            enum_values: &["basic-ntlm", "custom-post", "form-based"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "content-type",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["none", "url-encoded-utf8", "xml-utf8"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "custom-body",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "follow-redirect",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "form-action",
            value_type: ValueKind::String,
            allow_none: true,
            usage_flags: &["optional"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "form-fields",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "form-method",
            value_type: ValueKind::Enum,
            enum_values: &["get", "post"],
            default: Some("POST"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "form-params",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "form-password",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "form-username",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "headers",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["headers"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "hname",
                    value_type: ValueKind::String,
                    in_sections: &["headers"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "hvalue",
                    value_type: ValueKind::String,
                    in_sections: &["headers"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["headers"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hname",
            value_type: ValueKind::String,
            in_sections: &["headers"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hvalue",
            value_type: ValueKind::String,
            in_sections: &["headers"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "location-specific",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "start-uri",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "success-match-type",
            value_type: ValueKind::Enum,
            enum_values: &["cookie", "exact-cookie", "url"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "success-match-value",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
