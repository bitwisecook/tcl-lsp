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

//! BIG-IP object specs for the `saas` tmsh module.
//!
//! **Hand-maintained.** These files were originally produced by a
//! one-time port of the pre-rewrite Python registry
//! (`dialects/f5/bigip/registry/specs/`, still present on `main`) via a
//! `scripts/registry-audit/gen_bigip_rust.py` generator that no longer
//! exists — deleted along with the rest of the retired Python tooling on
//! this branch, and the commits that ran it were squashed away in the
//! `rust` branch's rebase-onto-main history (see issue #1404). Originally
//! organised by the first letter of `kind`; reorganised by tmsh module
//! name (this file's own module field, `Some("saas")`) per
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
            kind: "saas_ap_ai_profile",
            table_name: None,
            resolver_name: None,
            module: Some("saas"),
            object_types: &["ap-ai profile"],
        },
        header_types: &[("saas", "ap-ai profile")],
        properties: &[
            BigipPropertySpec {
                name: "account-protection",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "add-connecting-ip",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ai-header-name",
                value_type: ValueKind::String,
                default: Some("x-apg-sr"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ap-header-name",
                value_type: ValueKind::String,
                default: Some("x-safe-fr"),
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
                name: "authentication-intelligence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "block-response-body",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "block-response-code",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("200"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "block-response-content-type",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "connecting-ip-header",
                value_type: ValueKind::String,
                default: Some("x-iapp-real-ip"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customer-id",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "decrypt-cookie",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["saas_ap_ai_profile"],
                default: Some("ap-ai"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain-pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encryption-key",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hostname",
                value_type: ValueKind::String,
                default: Some("us"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "irules",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                references: &["apm_policy_agent_irule_event", "pem_irule"],
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ivs-ssl",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-exclude-paths",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-exclude-paths-enable",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-include-paths",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-include-paths-enable",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-location",
                value_type: ValueKind::Enum,
                enum_values: &["body", "head"],
                default: Some("after head"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-script-attribute",
                value_type: ValueKind::Enum,
                enum_values: &["async", "async-defer", "defer", "sync"],
                default: Some("async-defer"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-path",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protected-endpoints",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "ai-endpoint",
                        value_type: ValueKind::Enum,
                        in_sections: &["protected-endpoints"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("enabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ap-endpoint",
                        value_type: ValueKind::Enum,
                        in_sections: &["protected-endpoints"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["protected-endpoints"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "enforcement-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["protected-endpoints"],
                        enum_values: &["mitigate", "monitor"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "host",
                        value_type: ValueKind::String,
                        in_sections: &["protected-endpoints"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "max-cookie-age",
                        value_type: ValueKind::Integer,
                        in_sections: &["protected-endpoints"],
                        allow_none: true,
                        default: Some("7"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "mitigate-malformed-cookie",
                        value_type: ValueKind::Enum,
                        in_sections: &["protected-endpoints"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "mitigate-max-cookie-age",
                        value_type: ValueKind::Enum,
                        in_sections: &["protected-endpoints"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "mitigate-missing-cookie",
                        value_type: ValueKind::Enum,
                        in_sections: &["protected-endpoints"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "mitigation-action",
                        value_type: ValueKind::Enum,
                        in_sections: &["protected-endpoints"],
                        enum_values: &["block", "redirect"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "path",
                        value_type: ValueKind::String,
                        in_sections: &["protected-endpoints"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ai-endpoint",
                value_type: ValueKind::Enum,
                in_sections: &["protected-endpoints"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ap-endpoint",
                value_type: ValueKind::Enum,
                in_sections: &["protected-endpoints"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["protected-endpoints"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enforcement-mode",
                value_type: ValueKind::Enum,
                in_sections: &["protected-endpoints"],
                enum_values: &["mitigate", "monitor"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::String,
                in_sections: &["protected-endpoints"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-cookie-age",
                value_type: ValueKind::Integer,
                in_sections: &["protected-endpoints"],
                allow_none: true,
                default: Some("7"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mitigate-malformed-cookie",
                value_type: ValueKind::Enum,
                in_sections: &["protected-endpoints"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mitigate-max-cookie-age",
                value_type: ValueKind::Enum,
                in_sections: &["protected-endpoints"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mitigate-missing-cookie",
                value_type: ValueKind::Enum,
                in_sections: &["protected-endpoints"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mitigation-action",
                value_type: ValueKind::Enum,
                in_sections: &["protected-endpoints"],
                enum_values: &["block", "redirect"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "path",
                value_type: ValueKind::String,
                in_sections: &["protected-endpoints"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-destination",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("https://us"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-password",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-username",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recommendation-cookie-name",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("_imp_apg_r_"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirect-path",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirect-response-code",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("302"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "telemetry-path",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-proxy-server",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-sni",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "saas_ati_profile",
            table_name: None,
            resolver_name: None,
            module: Some("saas"),
            object_types: &["ati profile"],
        },
        header_types: &[("saas", "ati profile")],
        properties: &[
            BigipPropertySpec {
                name: "api-svc-add-connecting-ip",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-svc-connecting-ip-header",
                value_type: ValueKind::String,
                default: Some("x-iapp-real-ip"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-svc-domain-pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-svc-hostname",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-svc-ivs-ssl",
                value_type: ValueKind::Reference,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-svc-js-path",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-svc-telemetry-path",
                value_type: ValueKind::String,
                default: Some("/_|_imp_apg_|_/api/dip/v1/dip"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-svc-use-sni",
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
                name: "bas-domain-pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bas-enable",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bas-hostname",
                value_type: ValueKind::String,
                default: Some("us"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bas-ivs-ssl",
                value_type: ValueKind::Reference,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bas-proxy-destination",
                value_type: ValueKind::String,
                default: Some("https://us"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bas-telemetry-path",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customer-id",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["saas_ati_profile"],
                default: Some("ati"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "irules",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                references: &["apm_policy_agent_irule_event", "pem_irule"],
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-exclude-paths",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-exclude-paths-enable",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-include-paths",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-include-paths-enable",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-location",
                value_type: ValueKind::Enum,
                enum_values: &["body", "head"],
                default: Some("after head"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-script-attribute",
                value_type: ValueKind::Enum,
                enum_values: &["async", "async-defer", "defer", "sync"],
                default: Some("async-defer"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-destination",
                value_type: ValueKind::String,
                default: Some("https://us"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-password",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-username",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-proxy-server",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "saas_bd_profile",
            table_name: None,
            resolver_name: None,
            module: Some("saas"),
            object_types: &["bd profile"],
        },
        header_types: &[("saas", "bd profile")],
        properties: &[
            BigipPropertySpec {
                name: "allow-headers",
                value_type: ValueKind::List,
                repeated: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "allow-ip-addresses",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-auth-key",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-hostname",
                value_type: ValueKind::String,
                default: Some("ibd-web"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-key",
                value_type: ValueKind::String,
                default: Some("none"),
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
                name: "application-id",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bigip-handles-js-injections",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "block-response-body",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "block-response-code",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "block-response-content-type",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "block-responses",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["block-responses"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "block-response-body",
                        value_type: ValueKind::String,
                        in_sections: &["block-responses"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "block-response-code",
                        value_type: ValueKind::Integer,
                        in_sections: &["block-responses"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "block-response-content-type",
                        value_type: ValueKind::String,
                        in_sections: &["block-responses"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["block-responses"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "block-response-body",
                value_type: ValueKind::String,
                in_sections: &["block-responses"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "block-response-code",
                value_type: ValueKind::Integer,
                in_sections: &["block-responses"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "block-response-content-type",
                value_type: ValueKind::String,
                in_sections: &["block-responses"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cors-support",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dashboard-link",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["saas_bd_profile"],
                default: Some("bd"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "deployment-environment",
                value_type: ValueKind::Enum,
                enum_values: &["pre-production", "production"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "exclude-js-injection-from-specific-url",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-post-body",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "inject-js-in-specific-url",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "inject-telemetry-js-in-body-tag",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "irules",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                references: &["apm_policy_agent_irule_event", "pem_irule"],
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-exclude-paths",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-include-paths",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-mode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "async-and-caching-with-defer-XHR",
                    "async-no-caching",
                    "async-with-caching",
                    "sync",
                ],
                default: Some("async-no-caching"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "location-for-shape-js-injection",
                value_type: ValueKind::Enum,
                enum_values: &["after-head", "after-title", "before-script"],
                default: Some("after after-head"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert",
                    "crit",
                    "debug",
                    "default-value",
                    "emerg",
                    "err",
                    "info",
                    "notice",
                    "warn",
                ],
                default: Some("notice"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-publisher",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("local-syslog-publisher"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mitigation-handler",
                value_type: ValueKind::Enum,
                enum_values: &["bigip", "shape-policy"],
                default: Some("bigip"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-api-hostname",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("ibd-mobileus"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-applications-in-scope",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-block-response-body",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-block-response-code",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("200"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-block-response-content-type",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-identifier-body-keywords",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-identifier-request-headers",
                value_type: ValueKind::List,
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-include-post-body",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-mitigation-handler",
                value_type: ValueKind::Enum,
                enum_values: &["bigip", "shape-policy"],
                default: Some("bigip"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-protected-endpoints",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "any-method",
                        value_type: ValueKind::Enum,
                        in_sections: &["mobile-protected-endpoints"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "block-response-name",
                        value_type: ValueKind::Enum,
                        in_sections: &["mobile-protected-endpoints"],
                        allow_none: true,
                        enum_values: &["none"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "check-mobile-request-identifier",
                        value_type: ValueKind::Enum,
                        in_sections: &["mobile-protected-endpoints"],
                        enum_values: &["body", "header", "skip"],
                        default: Some("skip"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "endpoint",
                        value_type: ValueKind::String,
                        in_sections: &["mobile-protected-endpoints"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "get",
                        value_type: ValueKind::Enum,
                        in_sections: &["mobile-protected-endpoints"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "host",
                        value_type: ValueKind::String,
                        in_sections: &["mobile-protected-endpoints"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "mitigation-action",
                        value_type: ValueKind::Enum,
                        in_sections: &["mobile-protected-endpoints"],
                        enum_values: &["block", "continue"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "post",
                        value_type: ValueKind::Enum,
                        in_sections: &["mobile-protected-endpoints"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "any-method",
                value_type: ValueKind::Enum,
                in_sections: &["mobile-protected-endpoints"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "block-response-name",
                value_type: ValueKind::Enum,
                in_sections: &["mobile-protected-endpoints"],
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "check-mobile-request-identifier",
                value_type: ValueKind::Enum,
                in_sections: &["mobile-protected-endpoints"],
                enum_values: &["body", "header", "skip"],
                default: Some("skip"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "endpoint",
                value_type: ValueKind::String,
                in_sections: &["mobile-protected-endpoints"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "get",
                value_type: ValueKind::Enum,
                in_sections: &["mobile-protected-endpoints"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::String,
                in_sections: &["mobile-protected-endpoints"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mitigation-action",
                value_type: ValueKind::Enum,
                in_sections: &["mobile-protected-endpoints"],
                enum_values: &["block", "continue"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "post",
                value_type: ValueKind::Enum,
                in_sections: &["mobile-protected-endpoints"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-proxy-shape-endpoint-url",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-sdk-config-fetch-url-android",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("/v1/android/update"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-sdk-config-fetch-url-ios",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("/v1/ios/update"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-sdk-reload-header-name",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("ggaj1661"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-shape-protection-pool",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mobile-telemetry-header-prefix",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-cookie-persistence",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protected-endpoints",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "any-method",
                        value_type: ValueKind::Enum,
                        in_sections: &["protected-endpoints"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "block-response-name",
                        value_type: ValueKind::Enum,
                        in_sections: &["protected-endpoints"],
                        allow_none: true,
                        enum_values: &["none"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "endpoint",
                        value_type: ValueKind::String,
                        in_sections: &["protected-endpoints"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "get-document",
                        value_type: ValueKind::Enum,
                        in_sections: &["protected-endpoints"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "get-xhr-or-fetch",
                        value_type: ValueKind::Enum,
                        in_sections: &["protected-endpoints"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "host",
                        value_type: ValueKind::String,
                        in_sections: &["protected-endpoints"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "mitigation-action",
                        value_type: ValueKind::Enum,
                        in_sections: &["protected-endpoints"],
                        enum_values: &["block", "continue", "drop", "redirect"],
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "post",
                        value_type: ValueKind::Enum,
                        in_sections: &["protected-endpoints"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "put",
                        value_type: ValueKind::Enum,
                        in_sections: &["protected-endpoints"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        default: Some("disabled"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "redirect-response-name",
                        value_type: ValueKind::Enum,
                        in_sections: &["protected-endpoints"],
                        allow_none: true,
                        enum_values: &["none"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "any-method",
                value_type: ValueKind::Enum,
                in_sections: &["protected-endpoints"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "block-response-name",
                value_type: ValueKind::Enum,
                in_sections: &["protected-endpoints"],
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "endpoint",
                value_type: ValueKind::String,
                in_sections: &["protected-endpoints"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "get-document",
                value_type: ValueKind::Enum,
                in_sections: &["protected-endpoints"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "get-xhr-or-fetch",
                value_type: ValueKind::Enum,
                in_sections: &["protected-endpoints"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::String,
                in_sections: &["protected-endpoints"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mitigation-action",
                value_type: ValueKind::Enum,
                in_sections: &["protected-endpoints"],
                enum_values: &["block", "continue", "drop", "redirect"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "post",
                value_type: ValueKind::Enum,
                in_sections: &["protected-endpoints"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "put",
                value_type: ValueKind::Enum,
                in_sections: &["protected-endpoints"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirect-response-name",
                value_type: ValueKind::Enum,
                in_sections: &["protected-endpoints"],
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-password",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-pool",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-shape-endpoint-url",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-username",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirect-path",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirect-response-code",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("302"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirect-responses",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["redirect-responses"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "redirect-path",
                        value_type: ValueKind::String,
                        in_sections: &["redirect-responses"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "redirect-response-code",
                        value_type: ValueKind::Integer,
                        in_sections: &["redirect-responses"],
                        allow_none: true,
                        default: Some("302"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["redirect-responses"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirect-path",
                value_type: ValueKind::String,
                in_sections: &["redirect-responses"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirect-response-code",
                value_type: ValueKind::Integer,
                in_sections: &["redirect-responses"],
                allow_none: true,
                default: Some("302"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "report-transaction-result",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rewrite-xff-header-with-connecting-ip",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "service-level",
                value_type: ValueKind::Enum,
                enum_values: &["enterprise", "standard"],
                default: Some("standard"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "shape-api-response-timeout",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("300"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "shape-inference-header",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "shape-js-url-or-path",
                value_type: ValueKind::String,
                default: Some("/customer1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "shape-protection-pool",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-of-client-ip-address",
                value_type: ValueKind::Enum,
                enum_values: &["connecting-ip", "custom", "x-forwarded-for"],
                default: Some("Connecting IP"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-profile",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "telemetry-header-prefix",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "telemetry-request-body-size",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("65536"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tenant-id",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tls-fingerprint",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-proxy",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-sni",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "web-applications-in-scope",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "saas_csd_profile",
            table_name: None,
            resolver_name: None,
            module: Some("saas"),
            object_types: &["csd profile"],
        },
        header_types: &[("saas", "csd profile")],
        properties: &[
            BigipPropertySpec {
                name: "add-connecting-ip",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-domain-pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-hostname",
                value_type: ValueKind::String,
                default: Some("us"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-ivs-ssl",
                value_type: ValueKind::Reference,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-js-path",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "api-proxy-destination",
                value_type: ValueKind::String,
                default: Some("https://us"),
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
                name: "connecting-ip-header",
                value_type: ValueKind::String,
                default: Some("x-iapp-real-ip"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customer-id",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["saas_csd_profile"],
                default: Some("csd"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "irules",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                references: &["apm_policy_agent_irule_event", "pem_irule"],
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-exclude-paths",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-exclude-paths-enable",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-include-paths",
                value_type: ValueKind::List,
                repeated: true,
                allow_none: true,
                default: Some("none"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-include-paths-enable",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-location",
                value_type: ValueKind::Enum,
                enum_values: &["body", "head"],
                default: Some("after head"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "js-inject-script-attribute",
                value_type: ValueKind::Enum,
                enum_values: &["async", "async-defer", "defer", "sync"],
                default: Some("async-defer"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-password",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-username",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "telemetry-domain-pool",
                value_type: ValueKind::Reference,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "telemetry-hostname",
                value_type: ValueKind::String,
                default: Some("csd"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "telemetry-ivs-ssl",
                value_type: ValueKind::Reference,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "telemetry-path",
                value_type: ValueKind::String,
                default: Some("/csd/oob"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "telemetry-proxy-destination",
                value_type: ValueKind::String,
                default: Some("https://csd"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-proxy-server",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "use-sni",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
