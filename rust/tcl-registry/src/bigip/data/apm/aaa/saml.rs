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

//! BIG-IP object specs for `apm aaa saml`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_aaa_saml",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["aaa saml"],
    },
    header_types: &[("apm", "aaa saml")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "assertion-consumer-binding",
            value_type: ValueKind::Enum,
            enum_values: &["http-artifact", "http-post"],
            default: Some("http-post"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "attribute-consuming-services",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[BigipPropertySpec {
                name: "attribute-consuming-service-index",
                value_type: ValueKind::Integer,
                in_sections: &["attribute-consuming-services"],
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "attribute-consuming-service-index",
            value_type: ValueKind::Integer,
            in_sections: &["attribute-consuming-services"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auth-context-class-list",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auth-context-comparison-method",
            value_type: ValueKind::Enum,
            enum_values: &["better", "exact", "maximum", "minimum"],
            default: Some("exact"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auth-context-methods",
            value_type: ValueKind::List,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "default-attribute-consuming-service",
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
            name: "entity-id",
            value_type: ValueKind::String,
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "export-metadata",
            value_type: ValueKind::Enum,
            enum_values: &["no-signing", "with-signing"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "force-authn",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "idp-connectors",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "idp-matching-source",
                    value_type: ValueKind::String,
                    in_sections: &["idp-connectors"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "idp-matching-value",
                    value_type: ValueKind::String,
                    in_sections: &["idp-connectors"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "idp-matching-source",
            value_type: ValueKind::String,
            in_sections: &["idp-connectors"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "idp-matching-value",
            value_type: ValueKind::String,
            in_sections: &["idp-connectors"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "is-authn-request-signed",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
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
            name: "metadata-cert",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "metadata-file",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "metadata-signkey",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "name-id-policy-allow-create",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "name-id-policy-format",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "name-id-policy-sp-name-qualifier",
            value_type: ValueKind::String,
            allow_none: true,
            usage_flags: &["optional"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "provider-name",
            value_type: ValueKind::String,
            allow_none: true,
            usage_flags: &["optional"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "relay-state",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sp-certificate",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sp-host",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sp-scheme",
            value_type: ValueKind::Enum,
            enum_values: &["http", "https"],
            default: Some("https"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sp-signkey",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "want-assertion-encrypted",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "want-assertion-signed",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
