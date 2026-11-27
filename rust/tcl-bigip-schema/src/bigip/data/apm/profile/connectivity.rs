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

//! BIG-IP object specs for `apm profile connectivity`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_profile_connectivity",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["profile connectivity"],
    },
    header_types: &[("apm", "profile connectivity")],
    properties: &[
        BigipPropertySpec {
            name: "adaptive-compression",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
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
            name: "citrix-client-bundle",
            value_type: ValueKind::Reference,
            references: &["apm_resource_remote_desktop_citrix_client_bundle"],
            default: Some("default-citrix-client-bundle"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-policy",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "android-ec",
                    value_type: ValueKind::Unknown,
                    in_sections: &["client-policy"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "android-ep",
                    value_type: ValueKind::Unknown,
                    in_sections: &["client-policy"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "chromeos-ec",
                    value_type: ValueKind::Unknown,
                    in_sections: &["client-policy"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ec",
                    value_type: ValueKind::Unknown,
                    in_sections: &["client-policy"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ios-ec",
                    value_type: ValueKind::Unknown,
                    in_sections: &["client-policy"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ios-ep",
                    value_type: ValueKind::Unknown,
                    in_sections: &["client-policy"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "macos-ec",
                    value_type: ValueKind::Unknown,
                    in_sections: &["client-policy"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "oauth",
                    value_type: ValueKind::Unknown,
                    in_sections: &["client-policy"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "servers",
                    value_type: ValueKind::List,
                    in_sections: &["client-policy"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "android-ec",
            value_type: ValueKind::Unknown,
            in_sections: &["client-policy"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "device-lock-complexity",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ec"],
                    allow_none: true,
                    enum_values: &["high", "low", "medium", "none"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "device-lock-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ec"],
                    enum_values: &["alphabetic", "alphanumeric", "any", "numeric"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enable-mobilesafe",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enforce-device-lock",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("true"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enforce-logon-mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "logon-mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ec"],
                    enum_values: &["native", "web"],
                    default: Some("native"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "max-inactivity-time",
                    value_type: ValueKind::Integer,
                    in_sections: &["client-policy", "android-ec"],
                    default: Some("5"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "min-passcode-length",
                    value_type: ValueKind::Integer,
                    in_sections: &["client-policy", "android-ec"],
                    required: true,
                    default: Some("4"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "require-device-auth",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ec"],
                    enum_values: &["disk", "memory"],
                    default: Some("disk"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password-timeout",
                    value_type: ValueKind::Integer,
                    in_sections: &["client-policy", "android-ec"],
                    default: Some("240"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "device-lock-complexity",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ec"],
            allow_none: true,
            enum_values: &["high", "low", "medium", "none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "device-lock-method",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ec"],
            enum_values: &["alphabetic", "alphanumeric", "any", "numeric"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enable-mobilesafe",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforce-device-lock",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("true"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforce-logon-mode",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "logon-mode",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ec"],
            enum_values: &["native", "web"],
            default: Some("native"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-inactivity-time",
            value_type: ValueKind::Integer,
            in_sections: &["client-policy", "android-ec"],
            default: Some("5"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "min-passcode-length",
            value_type: ValueKind::Integer,
            in_sections: &["client-policy", "android-ec"],
            required: true,
            default: Some("4"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "require-device-auth",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password-method",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ec"],
            enum_values: &["disk", "memory"],
            default: Some("disk"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password-timeout",
            value_type: ValueKind::Integer,
            in_sections: &["client-policy", "android-ec"],
            default: Some("240"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "android-ep",
            value_type: ValueKind::Unknown,
            in_sections: &["client-policy"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "device-lock-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ep"],
                    enum_values: &["alphabetic", "alphanumeric", "any", "numeric"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enable-mobilesafe",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ep"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enforce-device-lock",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ep"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("true"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enforce-logon-mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ep"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "logon-mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ep"],
                    enum_values: &["native", "web"],
                    default: Some("native"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "max-inactivity-time",
                    value_type: ValueKind::Integer,
                    in_sections: &["client-policy", "android-ep"],
                    default: Some("5"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "min-passcode-length",
                    value_type: ValueKind::Integer,
                    in_sections: &["client-policy", "android-ep"],
                    required: true,
                    default: Some("4"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ep"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "android-ep"],
                    enum_values: &["disk", "memory"],
                    default: Some("disk"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password-timeout",
                    value_type: ValueKind::Integer,
                    in_sections: &["client-policy", "android-ep"],
                    default: Some("240"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "device-lock-method",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ep"],
            enum_values: &["alphabetic", "alphanumeric", "any", "numeric"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enable-mobilesafe",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ep"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforce-device-lock",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ep"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("true"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforce-logon-mode",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ep"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "logon-mode",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ep"],
            enum_values: &["native", "web"],
            default: Some("native"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-inactivity-time",
            value_type: ValueKind::Integer,
            in_sections: &["client-policy", "android-ep"],
            default: Some("5"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "min-passcode-length",
            value_type: ValueKind::Integer,
            in_sections: &["client-policy", "android-ep"],
            required: true,
            default: Some("4"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ep"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password-method",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "android-ep"],
            enum_values: &["disk", "memory"],
            default: Some("disk"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password-timeout",
            value_type: ValueKind::Integer,
            in_sections: &["client-policy", "android-ep"],
            default: Some("240"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "chromeos-ec",
            value_type: ValueKind::Unknown,
            in_sections: &["client-policy"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "enforce-logon-mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "chromeos-ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "logon-mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "chromeos-ec"],
                    enum_values: &["native", "web"],
                    default: Some("native"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "chromeos-ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "chromeos-ec"],
                    enum_values: &["disk", "memory"],
                    default: Some("disk"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password-timeout",
                    value_type: ValueKind::Integer,
                    in_sections: &["client-policy", "chromeos-ec"],
                    default: Some("240"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforce-logon-mode",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "chromeos-ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "logon-mode",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "chromeos-ec"],
            enum_values: &["native", "web"],
            default: Some("native"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "chromeos-ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password-method",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "chromeos-ec"],
            enum_values: &["disk", "memory"],
            default: Some("disk"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password-timeout",
            value_type: ValueKind::Integer,
            in_sections: &["client-policy", "chromeos-ec"],
            default: Some("240"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ec",
            value_type: ValueKind::Unknown,
            in_sections: &["client-policy"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "component-update",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ec"],
                    enum_values: &["no", "prompt", "yes"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "location-dns",
                    value_type: ValueKind::List,
                    in_sections: &["client-policy", "ec"],
                    default: Some("none"),
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "reuse-winlogon-creds",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "reuse-winlogon-session",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ec"],
                    enum_values: &["disk", "memory"],
                    default: Some("disk"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password-timeout",
                    value_type: ValueKind::Integer,
                    in_sections: &["client-policy", "ec"],
                    default: Some("240"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-servers-on-exit",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("true"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "component-update",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ec"],
            enum_values: &["no", "prompt", "yes"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "location-dns",
            value_type: ValueKind::List,
            in_sections: &["client-policy", "ec"],
            default: Some("none"),
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "reuse-winlogon-creds",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "reuse-winlogon-session",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password-method",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ec"],
            enum_values: &["disk", "memory"],
            default: Some("disk"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password-timeout",
            value_type: ValueKind::Integer,
            in_sections: &["client-policy", "ec"],
            default: Some("240"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-servers-on-exit",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("true"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ios-ec",
            value_type: ValueKind::Unknown,
            in_sections: &["client-policy"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "enable-mobilesafe",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ios-ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enforce-logon-mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ios-ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "logon-mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ios-ec"],
                    enum_values: &["native", "web"],
                    default: Some("native"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "require-device-auth",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ios-ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ios-ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ios-ec"],
                    enum_values: &["disk", "memory"],
                    default: Some("disk"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password-timeout",
                    value_type: ValueKind::Integer,
                    in_sections: &["client-policy", "ios-ec"],
                    default: Some("240"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "vod-disconnect-timeout",
                    value_type: ValueKind::Integer,
                    in_sections: &["client-policy", "ios-ec"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enable-mobilesafe",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ios-ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforce-logon-mode",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ios-ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "logon-mode",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ios-ec"],
            enum_values: &["native", "web"],
            default: Some("native"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "require-device-auth",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ios-ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ios-ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password-method",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ios-ec"],
            enum_values: &["disk", "memory"],
            default: Some("disk"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password-timeout",
            value_type: ValueKind::Integer,
            in_sections: &["client-policy", "ios-ec"],
            default: Some("240"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "vod-disconnect-timeout",
            value_type: ValueKind::Integer,
            in_sections: &["client-policy", "ios-ec"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ios-ep",
            value_type: ValueKind::Unknown,
            in_sections: &["client-policy"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "enable-mobilesafe",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ios-ep"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enforce-logon-mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ios-ep"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enforce-pin-lock",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ios-ep"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("true"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "logon-mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ios-ep"],
                    enum_values: &["native", "web"],
                    default: Some("native"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "max-grace-period",
                    value_type: ValueKind::Integer,
                    in_sections: &["client-policy", "ios-ep"],
                    default: Some("2"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ios-ep"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "ios-ep"],
                    enum_values: &["disk", "memory"],
                    default: Some("disk"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password-timeout",
                    value_type: ValueKind::Integer,
                    in_sections: &["client-policy", "ios-ep"],
                    default: Some("240"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enable-mobilesafe",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ios-ep"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforce-logon-mode",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ios-ep"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforce-pin-lock",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ios-ep"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("true"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "logon-mode",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ios-ep"],
            enum_values: &["native", "web"],
            default: Some("native"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-grace-period",
            value_type: ValueKind::Integer,
            in_sections: &["client-policy", "ios-ep"],
            default: Some("2"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ios-ep"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password-method",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "ios-ep"],
            enum_values: &["disk", "memory"],
            default: Some("disk"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password-timeout",
            value_type: ValueKind::Integer,
            in_sections: &["client-policy", "ios-ep"],
            default: Some("240"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "macos-ec",
            value_type: ValueKind::Unknown,
            in_sections: &["client-policy"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "component-update",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "macos-ec"],
                    enum_values: &["no", "prompt", "yes"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "enforce-logon-mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "macos-ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "location-dns",
                    value_type: ValueKind::List,
                    in_sections: &["client-policy", "macos-ec"],
                    default: Some("none"),
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "logon-mode",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "macos-ec"],
                    enum_values: &["native", "web"],
                    default: Some("native"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "macos-ec"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password-method",
                    value_type: ValueKind::Enum,
                    in_sections: &["client-policy", "macos-ec"],
                    enum_values: &["disk", "memory"],
                    default: Some("disk"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "save-password-timeout",
                    value_type: ValueKind::Integer,
                    in_sections: &["client-policy", "macos-ec"],
                    default: Some("240"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "component-update",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "macos-ec"],
            enum_values: &["no", "prompt", "yes"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enforce-logon-mode",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "macos-ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "location-dns",
            value_type: ValueKind::List,
            in_sections: &["client-policy", "macos-ec"],
            default: Some("none"),
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "logon-mode",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "macos-ec"],
            enum_values: &["native", "web"],
            default: Some("native"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "macos-ec"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password-method",
            value_type: ValueKind::Enum,
            in_sections: &["client-policy", "macos-ec"],
            enum_values: &["disk", "memory"],
            default: Some("disk"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "save-password-timeout",
            value_type: ValueKind::Integer,
            in_sections: &["client-policy", "macos-ec"],
            default: Some("240"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "oauth",
            value_type: ValueKind::Unknown,
            in_sections: &["client-policy"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "client-id",
                    value_type: ValueKind::String,
                    in_sections: &["client-policy", "oauth"],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "client-secret",
                    value_type: ValueKind::String,
                    in_sections: &["client-policy", "oauth"],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "done-uri",
                    value_type: ValueKind::String,
                    in_sections: &["client-policy", "oauth"],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "provider-name",
                    value_type: ValueKind::Reference,
                    in_sections: &["client-policy", "oauth"],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scopes",
                    value_type: ValueKind::String,
                    in_sections: &["client-policy", "oauth"],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-id",
            value_type: ValueKind::String,
            in_sections: &["client-policy", "oauth"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-secret",
            value_type: ValueKind::String,
            in_sections: &["client-policy", "oauth"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "done-uri",
            value_type: ValueKind::String,
            in_sections: &["client-policy", "oauth"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "provider-name",
            value_type: ValueKind::Reference,
            in_sections: &["client-policy", "oauth"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scopes",
            value_type: ValueKind::String,
            in_sections: &["client-policy", "oauth"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "servers",
            value_type: ValueKind::List,
            in_sections: &["client-policy"],
            block: &[
                BigipPropertySpec {
                    name: "alias",
                    value_type: ValueKind::String,
                    in_sections: &["client-policy", "servers"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "host",
                    value_type: ValueKind::String,
                    in_sections: &["client-policy", "servers"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "alias",
            value_type: ValueKind::String,
            in_sections: &["client-policy", "servers"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "host",
            value_type: ValueKind::String,
            in_sections: &["client-policy", "servers"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "compress-buffer-size",
            value_type: ValueKind::Integer,
            default: Some("4096"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "compress-cpu-saver",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("true"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "compress-cpu-saver-high",
            value_type: ValueKind::Integer,
            default: Some("90 percent"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "compress-cpu-saver-low",
            value_type: ValueKind::Integer,
            default: Some("75 percent"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "compress-gzip-level",
            value_type: ValueKind::Integer,
            default: Some(
                "6, which provides a higher amount of compression at the expense of more CPU processing time",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "compress-gzip-memlevel",
            value_type: ValueKind::Integer,
            default: Some("8192"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "compress-gzip-window-size",
            value_type: ValueKind::Integer,
            default: Some("16384"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "compress-ingress",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "compress-preferred-method",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("zlib"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "compression",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "compression-codecs",
            value_type: ValueKind::String,
            allow_none: true,
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
            name: "defaults",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &[
                "ltm_default_node_monitor",
                "security_firewall_uuid_default_autogenerate",
                "security_packet_filter_default_rules",
                "sys_default_config",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "deflate-compression-level",
            value_type: ValueKind::Integer,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
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
            name: "tunnel-name",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
