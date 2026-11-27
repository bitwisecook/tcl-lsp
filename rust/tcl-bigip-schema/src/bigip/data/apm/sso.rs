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

//! BIG-IP object specs for `apm sso`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_basic",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso basic"],
        },
        header_types: &[("apm", "sso basic")],
        properties: &[
            BigipPropertySpec {
                name: "apm-log-config",
                value_type: ValueKind::String,
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
                name: "headers",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hname",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hvalue",
                value_type: ValueKind::Integer,
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
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.sso.token.last.password"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-conversion",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_form_based",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso form-based"],
        },
        header_types: &[("apm", "sso form-based")],
        properties: &[
            BigipPropertySpec {
                name: "apm-log-config",
                value_type: ValueKind::String,
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
                name: "external-access-management",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "oam"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-action",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-field",
                value_type: ValueKind::String,
                required: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-method",
                value_type: ValueKind::Enum,
                enum_values: &["get", "post"],
                default: Some("post"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-password",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-username",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "headers",
                value_type: ValueKind::List,
                default: Some("none"),
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
                        value_type: ValueKind::Unknown,
                        in_sections: &["headers"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hvalue",
                        value_type: ValueKind::Integer,
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
                value_type: ValueKind::Unknown,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hvalue",
                value_type: ValueKind::Integer,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.sso.token.last.password"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "start-uri",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "success-match-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["cookie", "none", "url"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "success-match-value",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_form_basedv2",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso form-basedv2"],
        },
        header_types: &[("apm", "sso form-basedv2")],
        properties: &[
            BigipPropertySpec {
                name: "apm-log-config",
                value_type: ValueKind::String,
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
                name: "forms",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "attribute-value",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "controls",
                        value_type: ValueKind::List,
                        in_sections: &["forms"],
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "form-order",
                        value_type: ValueKind::Integer,
                        in_sections: &["forms"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "id-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["action", "id", "inputs", "order"],
                        default: Some("inputs"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-method",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["get", "post"],
                        default: Some("get"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-name",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-negative",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-prefix",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("true and specifies a partial match"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["cookie", "header", "uri"],
                        default: Some("uri"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-value",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-autodetect",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("true"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-javascript",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-javascript-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["auto", "custom", "extra"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-method",
                        value_type: ValueKind::Unknown,
                        in_sections: &["forms"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-name",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-negative",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-prefix",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("true and specifies partial match"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        enum_values: &["cookie", "header", "uri"],
                        default: Some("uri"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "submit-value",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "success-match-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms"],
                        allow_none: true,
                        enum_values: &["cookie", "none", "url"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "success-match-value",
                        value_type: ValueKind::String,
                        in_sections: &["forms"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attribute-value",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "controls",
                value_type: ValueKind::List,
                in_sections: &["forms"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["forms", "controls"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "secure",
                        value_type: ValueKind::Enum,
                        in_sections: &["forms", "controls"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("false"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "value",
                        value_type: ValueKind::String,
                        in_sections: &["forms", "controls"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["forms", "controls"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secure",
                value_type: ValueKind::Enum,
                in_sections: &["forms", "controls"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                in_sections: &["forms", "controls"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "form-order",
                value_type: ValueKind::Integer,
                in_sections: &["forms"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "id-type",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["action", "id", "inputs", "order"],
                default: Some("inputs"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-method",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["get", "post"],
                default: Some("get"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-name",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-negative",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-prefix",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true and specifies a partial match"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-type",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["cookie", "header", "uri"],
                default: Some("uri"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-value",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-autodetect",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-javascript",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-javascript-type",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["auto", "custom", "extra"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-method",
                value_type: ValueKind::Unknown,
                in_sections: &["forms"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-name",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-negative",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-prefix",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true and specifies partial match"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-type",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                enum_values: &["cookie", "header", "uri"],
                default: Some("uri"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "submit-value",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "success-match-type",
                value_type: ValueKind::Enum,
                in_sections: &["forms"],
                allow_none: true,
                enum_values: &["cookie", "none", "url"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "success-match-value",
                value_type: ValueKind::String,
                in_sections: &["forms"],
                allow_none: true,
                default: Some("none"),
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
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "value",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
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
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_kerberos",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso kerberos"],
        },
        header_types: &[("apm", "sso kerberos")],
        properties: &[
            BigipPropertySpec {
                name: "account-name",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "account-password",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "apm-log-config",
                value_type: ValueKind::String,
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
                name: "headers",
                value_type: ValueKind::List,
                default: Some("none"),
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
                        value_type: ValueKind::Integer,
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
                value_type: ValueKind::Integer,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "kdc",
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
                name: "realm",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send-authorization",
                value_type: ValueKind::Enum,
                enum_values: &["401", "always"],
                default: Some("always"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "spn-pattern",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ticket-lifetime",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("600 minutes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "upn-support",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-realm-source",
                value_type: ValueKind::String,
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::String,
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_ntlmv1",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso ntlmv1"],
        },
        header_types: &[("apm", "sso ntlmv1")],
        properties: &[
            BigipPropertySpec {
                name: "apm-log-config",
                value_type: ValueKind::String,
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
                name: "domain-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.domain"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "headers",
                value_type: ValueKind::List,
                default: Some("none"),
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
                        value_type: ValueKind::Integer,
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
                value_type: ValueKind::Integer,
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
                name: "ntlm-domain",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.sso.token.last.password"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-conversion",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_ntlmv2",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso ntlmv2"],
        },
        header_types: &[("apm", "sso ntlmv2")],
        properties: &[
            BigipPropertySpec {
                name: "apm-log-config",
                value_type: ValueKind::String,
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
                name: "domain-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.domain"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "headers",
                value_type: ValueKind::List,
                default: Some("none"),
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
                        value_type: ValueKind::Integer,
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
                value_type: ValueKind::Integer,
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
                name: "ntlm-domain",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.sso.token.last.password"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-conversion",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_oauth_bearer",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso oauth-bearer"],
        },
        header_types: &[("apm", "sso oauth-bearer")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "headers",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hname",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hvalue",
                value_type: ValueKind::Integer,
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
                name: "oauth-server",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_saml",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso saml"],
        },
        header_types: &[("apm", "sso saml")],
        properties: &[
            BigipPropertySpec {
                name: "apm-log-config",
                value_type: ValueKind::String,
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
                name: "artifact-resolution-service-name",
                value_type: ValueKind::Reference,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assertion-validity",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "attributes",
                value_type: ValueKind::List,
                allow_none: true,
                usage_flags: &["deprecated", "optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-context-method",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
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
                name: "encrypt",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encrypt-subject",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encryption-type",
                value_type: ValueKind::Enum,
                enum_values: &["aes128", "aes192", "aes256"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encryption-type-subject",
                value_type: ValueKind::Enum,
                enum_values: &["aes128", "aes192", "aes256"],
                default: Some("aes128"),
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
                name: "idp-certificate",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-certificate-session-var",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-host",
                value_type: ValueKind::Enum,
                required: true,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-scheme",
                value_type: ValueKind::Enum,
                enum_values: &["http", "https"],
                default: Some("https"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-signkey",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "idp-signkey-session-var",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "key-transport-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &["rsa-oaep", "rsa-v1.5"],
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
                name: "log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                usage_flags: &["deprecated"],
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
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-values",
                value_type: ValueKind::List,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "name-qualifier",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "saml-profiles",
                value_type: ValueKind::List,
                allow_none: true,
                default: Some("web-browser-sso"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sp-connectors",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subject-type",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "entity",
                    "kerberos",
                    "persistent",
                    "transient",
                    "unspecified",
                    "x509-subject",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subject-value",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_saml_resource",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso saml-resource"],
        },
        header_types: &[("apm", "sso saml-resource")],
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
                name: "publish-on-webtop",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sso-config-saml",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_saml_sp_automation",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso saml-sp-automation"],
        },
        header_types: &[("apm", "sso saml-sp-automation")],
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-resolver-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Integer,
                default: Some("60"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metadata-urls",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[BigipPropertySpec {
                    name: "url-value",
                    value_type: ValueKind::String,
                    in_sections: &["metadata-urls"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url-value",
                value_type: ValueKind::String,
                in_sections: &["metadata-urls"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serverssl-profile-name",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sp-obj-name-tag",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sso-config-saml",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_sso_saml_sp_connector",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["sso saml-sp-connector"],
        },
        header_types: &[("apm", "sso saml-sp-connector")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assertion-consumer-services",
                value_type: ValueKind::List,
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
                name: "encryption-type",
                value_type: ValueKind::Enum,
                enum_values: &["aes128", "aes192", "aes256"],
                default: Some("aes128"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "entity-id",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "import-metadata",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "index",
                value_type: ValueKind::Unknown,
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
                name: "is-default",
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
                name: "multi-domain-location",
                value_type: ValueKind::String,
                required: true,
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
                name: "signature-type",
                value_type: ValueKind::Enum,
                enum_values: &["rsa-sha1", "rsa-sha256", "rsa-sha384", "rsa-sha512"],
                default: Some("rsa-sha1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "single-logout-binding",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "single-logout-response-uri",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "single-logout-uri",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sp-certificate",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sp-location",
                value_type: ValueKind::Enum,
                enum_values: &["external", "internal", "internal-multi-domain"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sp-name-qualifier",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "uri",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "want-assertion-encrypted",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "want-assertion-signed",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "want-response-signed",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
