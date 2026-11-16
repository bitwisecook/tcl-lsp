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

//! BIG-IP object specs for `apm profile oauth`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_profile_oauth",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["profile oauth"],
    },
    header_types: &[("apm", "profile oauth")],
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
            enum_values: &["disabled", "enalbed"],
            default: Some("enabled"),
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
            name: "auth-url",
            value_type: ValueKind::String,
            default: Some("/f5-oauth2/v1/authorize"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-apps",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "db-instance",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::String,
            allow_none: true,
            references: &["apm_profile_oauth"],
            default: Some("oauth"),
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
            name: "id-token-primary-key",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ignore-expired-cert",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "issuer",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "jwks-url",
            value_type: ValueKind::String,
            default: Some("/f5-oauth2/v1/jwks"),
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
            name: "jwt-ec-signature-format",
            value_type: ValueKind::Enum,
            enum_values: &["binary", "der"],
            default: Some("binary format"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "jwt-refresh-token-enc-secret",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "jwt-refresh-token-lifetime",
            value_type: ValueKind::Integer,
            default: Some("60 minutes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "jwt-token",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "opaque-token",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("true"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "openid-cfg-url",
            value_type: ValueKind::String,
            default: Some("/f5-oauth2/v1/"),
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
            name: "per-user-token-limit",
            value_type: ValueKind::Integer,
            default: Some("255"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "primary-key",
            value_type: ValueKind::Unknown,
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
            default: Some("0, which represents unlimited number of times"),
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
            name: "resource-servers",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "replace-all-with"],
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
            name: "rotation-keys",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "subject",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("%{session"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "token-introspection-url",
            value_type: ValueKind::String,
            default: Some("/f5-oauth2/v1/introspect"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "token-issuance-url",
            value_type: ValueKind::String,
            default: Some("/f5-oauth2/v1/token"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "token-revocation-url",
            value_type: ValueKind::String,
            default: Some("/f5-oauth2/v1/revoke"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "trusted-ca-bundle",
            value_type: ValueKind::Unknown,
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
            name: "userinfo-primary-key",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "userinfo-url",
            value_type: ValueKind::String,
            default: Some("/f5-oauth2/v1/userinfo"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
