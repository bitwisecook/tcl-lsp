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

//! BIG-IP object specs for `apm saml`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_saml_artifact_resolution_service",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["saml artifact-resolution-service"],
        },
        header_types: &[("apm", "saml artifact-resolution-service")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "artifact-resolution-service-host",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "artifact-resolution-service-port",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "artifact-send-method",
                value_type: ValueKind::Enum,
                enum_values: &["http-post", "http-redirect"],
                default: Some("http-redirect"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "artifact-validity",
                value_type: ValueKind::Integer,
                default: Some("60 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "basic-auth-password",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "basic-auth-username",
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
                name: "location-specific",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "virtual-server-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "want-artifact-resolution-rq-signed",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_saml_attribute_consuming_service",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["saml attribute-consuming-service"],
        },
        header_types: &[("apm", "saml attribute-consuming-service")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attributes",
                value_type: ValueKind::List,
                required: true,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["attributes"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "attribute-name",
                        value_type: ValueKind::String,
                        in_sections: &["attributes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "friendly-name",
                        value_type: ValueKind::String,
                        in_sections: &["attributes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "is-required",
                        value_type: ValueKind::Unknown,
                        in_sections: &["attributes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "name-format",
                        value_type: ValueKind::String,
                        in_sections: &["attributes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["attributes"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attribute-name",
                value_type: ValueKind::String,
                in_sections: &["attributes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "friendly-name",
                value_type: ValueKind::String,
                in_sections: &["attributes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "is-required",
                value_type: ValueKind::Unknown,
                in_sections: &["attributes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "name-format",
                value_type: ValueKind::String,
                in_sections: &["attributes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-name",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_saml_auth_context_class_list",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["saml auth-context-class-list"],
        },
        header_types: &[("apm", "saml auth-context-class-list")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "classes",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "order",
                        value_type: ValueKind::Integer,
                        in_sections: &["classes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "value",
                        value_type: ValueKind::String,
                        in_sections: &["classes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "order",
                value_type: ValueKind::Integer,
                in_sections: &["classes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                in_sections: &["classes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
