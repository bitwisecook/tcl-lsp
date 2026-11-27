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

//! BIG-IP object specs for `apm oauth`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_db_instance",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth db-instance"],
        },
        header_types: &[("apm", "oauth db-instance")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
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
                name: "purge-frequency",
                value_type: ValueKind::Enum,
                enum_values: &["daily", "hourly", "monthly", "never", "weekly"],
                default: Some("daily"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "purge-now",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "purge-time",
                value_type: ValueKind::String,
                default: Some("02:00"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_jwk_config",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth jwk-config"],
        },
        header_types: &[("apm", "oauth jwk-config")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_jwt_config",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth jwt-config"],
        },
        header_types: &[("apm", "oauth jwt-config")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_jwt_provider_list",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth jwt-provider-list"],
        },
        header_types: &[("apm", "oauth jwt-provider-list")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_oauth_claim",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth oauth-claim"],
        },
        header_types: &[("apm", "oauth oauth-claim")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "claim-description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "claim-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "claim-type",
                value_type: ValueKind::Enum,
                enum_values: &["boolean", "custom", "number"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "claim-value",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_oauth_client_app",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth oauth-client-app"],
        },
        header_types: &[("apm", "oauth oauth-client-app")],
        properties: &[
            BigipPropertySpec {
                name: "access-token-lifetime",
                value_type: ValueKind::Integer,
                default: Some("5 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-plain-code-challenge",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-description",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-name",
                value_type: ValueKind::String,
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
                name: "audience",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-code-lifetime",
                value_type: ValueKind::Integer,
                default: Some("5 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["certificate", "none", "secret"],
                default: Some("secret and other possible values are none and certificate"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "client-cert-dn",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "contact",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "generate-jwt-refresh-token",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "generate-refresh-token",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "grant-code",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "grant-password",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "grant-token",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "id-token-claims",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "id-token-lifetime",
                value_type: ValueKind::Integer,
                default: Some("5 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jwt-access-token-claims",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jwt-access-token-lifetime",
                value_type: ValueKind::Integer,
                default: Some("5 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jwt-refresh-token-lifetime",
                value_type: ValueKind::Integer,
                default: Some("60 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "logo-url",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "openid-connect",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirect-uris",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "refresh-token-lifetime",
                value_type: ValueKind::Integer,
                default: Some("480 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "refresh-token-usage-limit",
                value_type: ValueKind::Integer,
                default: Some("64"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "regenerate-client-secret",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "require-pkce",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reuse-access-token",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "reuse-refresh-token",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scopes",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-profile-token-mgmt-settings",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "userinfo-claims",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "website-url",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_oauth_resource_server",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth oauth-resource-server"],
        },
        header_types: &[("apm", "oauth oauth-resource-server")],
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
                allow_none: true,
                enum_values: &["certificate", "none", "secret"],
                default: Some("certificate and other possible values are none and secret"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "regenerate-resource-server-secret",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "resource-server-cert-dn",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_oauth_oauth_scope",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["oauth oauth-scope"],
        },
        header_types: &[("apm", "oauth oauth-scope")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scope-description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scope-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scope-value",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
