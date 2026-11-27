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

//! BIG-IP object specs for `apm profile access`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_profile_access",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["profile access"],
    },
    header_types: &[("apm", "profile access")],
    properties: &[
        BigipPropertySpec {
            name: "accept-languages",
            value_type: ValueKind::List,
            required: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "access-policy",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "access-policy-timeout",
            value_type: ValueKind::Integer,
            default: Some("300 seconds"),
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
            name: "cache-generation",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "customization-group",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "default-language",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            default: Some("en (English)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            references: &["apm_profile_access"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "domain-cookie",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "domain-groups",
            value_type: ValueKind::List,
            required: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "domain-mode",
            value_type: ValueKind::Enum,
            enum_values: &["multi-domain", "single-domain"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforce-policy",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("true which means the access-policy is always enforced"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "eps-group",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "errormap-group",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "framework-installation-group",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "general-ui-group",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "generation-action",
            value_type: ValueKind::Enum,
            enum_values: &["increment", "noop"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "httponly-cookie",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "inactivity-timeout",
            value_type: ValueKind::Integer,
            default: Some("900 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-settings",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "logout-uri-include",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "logout-uri-timeout",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-concurrent-sessions",
            value_type: ValueKind::Integer,
            allow_none: true,
            default: Some("0 (zero), which represents unlimited sessions"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-concurrent-users",
            value_type: ValueKind::Integer,
            allow_none: true,
            default: Some("0 (zero), which represents unlimited sessions"),
            usage_flags: &["read_only"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-failure-delay",
            value_type: ValueKind::Integer,
            default: Some("5 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-in-progress-sessions",
            value_type: ValueKind::Integer,
            allow_none: true,
            default: Some("0, which represents an unlimited number of such sessions"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-session-timeout",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "min-failure-delay",
            value_type: ValueKind::Integer,
            default: Some("2 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "named-scope",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "oauth-profile",
            value_type: ValueKind::Unknown,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "persistent-cookie",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "primary-auth-service",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "restrict-to-single-client-ip",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sandboxes",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scope",
            value_type: ValueKind::Enum,
            enum_values: &["profile", "public", "virtual-server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "secure-cookie",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sso-name",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Enum,
            enum_values: &[
                "all",
                "identity-service",
                "ltm-apm",
                "oauth-resource-server",
                "rdg-rap",
                "ssl-vpn",
                "sso",
                "swg-explicit",
                "swg-transparent",
                "system-authentication",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "use-http-503-on-error",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "user-identity-method",
            value_type: ValueKind::Enum,
            enum_values: &["http"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
