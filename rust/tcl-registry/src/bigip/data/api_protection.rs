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

//! BIG-IP object specs for the `api-protection` tmsh module.
//!
//! **Hand-maintained.** These files were originally produced by a
//! one-time port of the pre-rewrite Python registry
//! (`dialects/f5/bigip/registry/specs/`, still present on `main`) via a
//! `scripts/registry-audit/gen_bigip_rust.py` generator that no longer
//! exists — deleted along with the rest of the retired Python tooling on
//! this branch, and the commits that ran it were squashed away in the
//! `rust` branch's rebase-onto-main history (see issue #1404). Originally
//! organised by the first letter of `kind`; reorganised by tmsh module
//! name (this file's own module field, `Some("api-protection")`) per
//! maintainer review on the PR that introduced the consistency gate below.
//! There is no live generator to regenerate these from; edit them by hand,
//! in the same shape the other module files already use.
//!
//! `cargo xtask bigip-data-schema --check` enforces the internal
//! invariants a generator would otherwise have guaranteed: every `kind`
//! is globally unique, filed under the module file matching its own
//! `module` field, and every `references` target either names a real
//! kind or is on the documented known-gap list.
// Some modules hold property-less kinds, so not every imported type
// is used in every file; large tmsh bounds appear as bare f64 literals.
#![allow(unused_imports, clippy::unreadable_literal)]
use super::super::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "api_protection_profile_apiprotection",
            table_name: None,
            resolver_name: None,
            module: Some("api-protection"),
            object_types: &["profile apiprotection"],
        },
        header_types: &[("api-protection", "profile apiprotection")],
        properties: &[
            BigipPropertySpec {
                name: "access-profile",
                value_type: ValueKind::Unknown,
                default: Some("none if created using TMSH"),
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
                name: "default-response",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-server",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["api_protection_profile_apiprotection"],
                default: Some("apiprotection"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-mode",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("ipv4-only"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dns-resolver",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "last-generated-path-id",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-concurrent-subsessions",
                value_type: ValueKind::Integer,
                default: Some(
                    "0, which sets the maximum number of concurrent subsessions to 5 times the licensed access session limit",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "openapi-version",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "paths",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "active",
                        value_type: ValueKind::Enum,
                        in_sections: &["paths"],
                        enum_values: &["false", "true"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("true"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["paths"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["paths"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "method",
                        value_type: ValueKind::String,
                        in_sections: &["paths"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "path-id",
                        value_type: ValueKind::Integer,
                        in_sections: &["paths"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "server",
                        value_type: ValueKind::Unknown,
                        in_sections: &["paths"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "uri",
                        value_type: ValueKind::String,
                        in_sections: &["paths"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "active",
                value_type: ValueKind::Enum,
                in_sections: &["paths"],
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("true"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["paths"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["paths"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "method",
                value_type: ValueKind::String,
                in_sections: &["paths"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "path-id",
                value_type: ValueKind::Integer,
                in_sections: &["paths"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::Unknown,
                in_sections: &["paths"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "uri",
                value_type: ValueKind::String,
                in_sections: &["paths"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "per-request-policy",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "responses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "servers",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-pool",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "api_protection_response",
            table_name: None,
            resolver_name: None,
            module: Some("api-protection"),
            object_types: &["response"],
        },
        header_types: &[("api-protection", "response")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "body",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
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
                        name: "header-name",
                        value_type: ValueKind::String,
                        in_sections: &["headers"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "header-value",
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
                name: "header-name",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "header-value",
                value_type: ValueKind::String,
                in_sections: &["headers"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "status-code",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "status-string",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "api_protection_server",
            table_name: None,
            resolver_name: None,
            module: Some("api-protection"),
            object_types: &["server"],
        },
        header_types: &[("api-protection", "server")],
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
                name: "options",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serverssl-profile",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
