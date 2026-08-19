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

//! BIG-IP object specs for the `wam` tmsh module.
//!
//! **Hand-maintained.** These files were originally produced by a
//! one-time port of the pre-rewrite Python registry
//! (`dialects/f5/bigip/registry/specs/`, still present on `main`) via a
//! `scripts/registry-audit/gen_bigip_rust.py` generator that no longer
//! exists — deleted along with the rest of the retired Python tooling on
//! this branch, and the commits that ran it were squashed away in the
//! `rust` branch's rebase-onto-main history (see issue #1404). Originally
//! organised by the first letter of `kind`; reorganised by tmsh module
//! name (this file's own module field, `Some("wam")`) per
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
            kind: "wam_ad_policy",
            table_name: None,
            resolver_name: None,
            module: Some("wam"),
            object_types: &["ad-policy"],
        },
        header_types: &[("wam", "ad-policy")],
        properties: &[
            BigipPropertySpec {
                name: "ad-insertion-order",
                value_type: ValueKind::Enum,
                enum_values: &["random", "sequential"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ads",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify"],
                block: &[
                    BigipPropertySpec {
                        name: "preroll",
                        value_type: ValueKind::Enum,
                        in_sections: &["ads"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "url",
                        value_type: ValueKind::Unknown,
                        in_sections: &["ads"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "preroll",
                value_type: ValueKind::Enum,
                in_sections: &["ads"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::Unknown,
                in_sections: &["ads"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "wam_application",
            table_name: None,
            resolver_name: None,
            module: Some("wam"),
            object_types: &["application"],
        },
        header_types: &[("wam", "application")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "code",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "collect-roi-statistics",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "content-expiration-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hosts",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["hosts"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "code",
                        value_type: ValueKind::Unknown,
                        in_sections: &["hosts"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "subdomain-number-of-http",
                        value_type: ValueKind::Unknown,
                        in_sections: &["hosts"],
                        default: Some("0"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "subdomain-number-of-https",
                        value_type: ValueKind::Unknown,
                        in_sections: &["hosts"],
                        default: Some("0"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "subdomain-prefix",
                        value_type: ValueKind::String,
                        in_sections: &["hosts"],
                        default: Some("wa"),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["hosts"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "code",
                value_type: ValueKind::Unknown,
                in_sections: &["hosts"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subdomain-number-of-http",
                value_type: ValueKind::Unknown,
                in_sections: &["hosts"],
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subdomain-number-of-https",
                value_type: ValueKind::Unknown,
                in_sections: &["hosts"],
                default: Some("0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "subdomain-prefix",
                value_type: ValueKind::String,
                in_sections: &["hosts"],
                default: Some("wa"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ibr-adaptive-lifetime",
                value_type: ValueKind::Unknown,
                default: Some("864000 (10 days)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ibr-default-lifetime",
                value_type: ValueKind::Unknown,
                default: Some("15724800 (6 months)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ibr-prefix",
                value_type: ValueKind::String,
                default: Some(""),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "info-header",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["debug", "none", "standard"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multibox",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "farm", "symmetric"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "perf-monitor",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "perf-monitor-data-retention-period",
                value_type: ValueKind::Unknown,
                default: Some("30 days"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "policy",
                value_type: ValueKind::Reference,
                references: &["wam_policy"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "roi-report-collect-statistics",
                value_type: ValueKind::List,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "roi-report-email-addresses",
                value_type: ValueKind::List,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "roi-report-frequency",
                value_type: ValueKind::Enum,
                enum_values: &["every-month", "every-week"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "roi-report-name",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "roi-report-next-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "roi-report-smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send-metadata",
                value_type: ValueKind::Enum,
                enum_values: &["always", "never", "uncompressed"],
                default: Some("always"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "wam_domain_list",
            table_name: None,
            resolver_name: None,
            module: Some("wam"),
            object_types: &["domain list"],
        },
        header_types: &[("wam", "domain list")],
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
                name: "domains",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "wam_object_type",
            table_name: None,
            resolver_name: None,
            module: Some("wam"),
            object_types: &["object-type"],
        },
        header_types: &[("wam", "object-type")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "code",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "compression",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "policy-controlled"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "extensions",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mime-types",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "symmetric-compression",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "wam_policy",
            table_name: None,
            resolver_name: None,
            module: Some("wam"),
            object_types: &["policy"],
        },
        header_types: &[("wam", "policy")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "code",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "copy-from",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "invalidations",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "active",
                        value_type: ValueKind::Enum,
                        in_sections: &["invalidations"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["invalidations"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "broadcast",
                        value_type: ValueKind::Enum,
                        in_sections: &["invalidations"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "cache-content",
                        value_type: ValueKind::List,
                        in_sections: &["invalidations"],
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["invalidations"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "active",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["invalidations"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "broadcast",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-content",
                value_type: ValueKind::List,
                in_sections: &["invalidations"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["invalidations", "cache-content"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-alias",
                        value_type: ValueKind::String,
                        in_sections: &["invalidations", "cache-content"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-direction",
                        value_type: ValueKind::Enum,
                        in_sections: &["invalidations", "cache-content"],
                        enum_values: &["left-to-right", "right-to-left"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-name",
                        value_type: ValueKind::String,
                        in_sections: &["invalidations", "cache-content"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-ordinal",
                        value_type: ValueKind::Unknown,
                        in_sections: &["invalidations", "cache-content"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["invalidations", "cache-content"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-data-alias",
                        value_type: ValueKind::String,
                        in_sections: &["invalidations", "cache-content"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-data-direction",
                        value_type: ValueKind::Enum,
                        in_sections: &["invalidations", "cache-content"],
                        enum_values: &["left-to-right", "right-to-left"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-data-name",
                        value_type: ValueKind::String,
                        in_sections: &["invalidations", "cache-content"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-data-ordinal",
                        value_type: ValueKind::Unknown,
                        in_sections: &["invalidations", "cache-content"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-data-type",
                        value_type: ValueKind::Unknown,
                        in_sections: &["invalidations", "cache-content"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "value-case-sensitive",
                        value_type: ValueKind::Enum,
                        in_sections: &["invalidations", "cache-content"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "values",
                        value_type: ValueKind::List,
                        in_sections: &["invalidations", "cache-content"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["invalidations", "cache-content"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-alias",
                value_type: ValueKind::String,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-direction",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations", "cache-content"],
                enum_values: &["left-to-right", "right-to-left"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-name",
                value_type: ValueKind::String,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-ordinal",
                value_type: ValueKind::Unknown,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-data-alias",
                value_type: ValueKind::String,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-data-direction",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations", "cache-content"],
                enum_values: &["left-to-right", "right-to-left"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-data-name",
                value_type: ValueKind::String,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-data-ordinal",
                value_type: ValueKind::Unknown,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-data-type",
                value_type: ValueKind::Unknown,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value-case-sensitive",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations", "cache-content"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "values",
                value_type: ValueKind::List,
                in_sections: &["invalidations", "cache-content"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["invalidations", "cache-content", "values"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "can-be-empty",
                        value_type: ValueKind::Enum,
                        in_sections: &["invalidations", "cache-content", "values"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "can-be-missing",
                        value_type: ValueKind::Enum,
                        in_sections: &["invalidations", "cache-content", "values"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "invert-match",
                        value_type: ValueKind::Enum,
                        in_sections: &["invalidations", "cache-content", "values"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["invalidations", "cache-content", "values"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-empty",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations", "cache-content", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-missing",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations", "cache-content", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "invert-match",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations", "cache-content", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["invalidations"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nodes",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "always-proxy",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["nodes"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-alias",
                        value_type: ValueKind::String,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-direction",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["left-to-right", "right-to-left"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-name",
                        value_type: ValueKind::String,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-ordinal",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-compression",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-compression-ows",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-concatenation",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-concatenation-sets",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes"],
                        repeated: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-css-inlining",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-css-inlining-urls",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes"],
                        repeated: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-css-reorder",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-css-reorder-cache-size",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-css-reorder-urls",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes"],
                        repeated: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-dns-prefetch",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-dns-prefetch-domain-lists",
                        value_type: ValueKind::List,
                        in_sections: &["nodes"],
                        list_operators: &["add", "delete", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-dns-prefetch-https-automatic",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-dns-prefetch-https-enable",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-ibr",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-icc-css-inlining-max-size",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-icc-force",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-icc-image-max-size",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-icc-js-inlining-max-size",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-icc-max-num-urls",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-icc-min-client-expiry",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-image-inlining",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-image-inlining-max-size",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-image-inlining-urls",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes"],
                        repeated: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-intelligent-client-cache",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-js-inlining",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-js-inlining-urls",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes"],
                        repeated: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-js-reorder",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-js-reorder-cache-size",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-js-reorder-urls",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes"],
                        repeated: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-minification",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-multiconnect",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-on-proxies",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "assembly-pdf-linearization",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "cache-complete-only",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "cache-first-hit",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "cache-mode",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["memory-and-disk", "memory-only"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "cache-priority",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["high", "low", "medium"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "cache-stand-in-period",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "can-be-empty",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "can-be-missing",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "code",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "coherency",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["blade", "cluster"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "defaults-from",
                        value_type: ValueKind::Reference,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "invert-match",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "jpeg-progressive-encoding",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "jpeg-quality",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "jpeg-quality-is-relative",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "jpeg-sampling",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["1x1", "1x2", "2x1", "2x2", "preserve"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "jpeg-strip-exif",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["if-safe", "make-safe", "no", "yes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "jpeg-strip-keeps-copyright",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "jpegxr-quality",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "lifetime-cache-control-extensions",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "lifetime-cache-max-age",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "lifetime-honor-ows",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "lifetime-honor-ows-values",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "lifetime-honor-request",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "lifetime-honor-request-values",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "lifetime-http-heuristic",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "lifetime-insert-no-cache",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "lifetime-preserve-response",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "lifetime-preserve-response-values",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "lifetime-response-max-age",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "lifetime-response-s-maxage",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "lifetime-stand-in-codes",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "lifetime-use-heuristic",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "matching",
                        value_type: ValueKind::List,
                        in_sections: &["nodes"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "object-max-size",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["from-profile"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "object-min-size",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["from-profile"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "optimize-for-client",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "optimize-image",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        allow_none: true,
                        enum_values: &["none", "to-gif", "to-jpeg", "to-png", "to-tiff"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "options",
                        value_type: ValueKind::List,
                        in_sections: &["nodes"],
                        repeated: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "order",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "png-256-colors",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "request-queueing",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "response-codes-cached",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "value-case-sensitive",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "values",
                        value_type: ValueKind::List,
                        in_sections: &["nodes"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "variation",
                        value_type: ValueKind::List,
                        in_sections: &["nodes"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "video-acceleration-ad-policy",
                        value_type: ValueKind::String,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "video-optimization-ad-frequency",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "video-optimization-fast-start",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "video-optimization-insert-ad",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "video-optimization-max-bitrate",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "video-optimization-preroll-ad",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["disable", "enable"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "viewstate-cache",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "viewstate-cache-size",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "viewstate-tag",
                        value_type: ValueKind::String,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "webp-quality",
                        value_type: ValueKind::Integer,
                        in_sections: &["nodes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "always-proxy",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["nodes"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-alias",
                value_type: ValueKind::String,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-direction",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["left-to-right", "right-to-left"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-name",
                value_type: ValueKind::String,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-ordinal",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-compression",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-compression-ows",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-concatenation",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-concatenation-sets",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-css-inlining",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-css-inlining-urls",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-css-reorder",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-css-reorder-cache-size",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-css-reorder-urls",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-dns-prefetch",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-dns-prefetch-domain-lists",
                value_type: ValueKind::List,
                in_sections: &["nodes"],
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-dns-prefetch-https-automatic",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-dns-prefetch-https-enable",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-ibr",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-icc-css-inlining-max-size",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-icc-force",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-icc-image-max-size",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-icc-js-inlining-max-size",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-icc-max-num-urls",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-icc-min-client-expiry",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-image-inlining",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-image-inlining-max-size",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-image-inlining-urls",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-intelligent-client-cache",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-js-inlining",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-js-inlining-urls",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-js-reorder",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-js-reorder-cache-size",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-js-reorder-urls",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-minification",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-multiconnect",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-on-proxies",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "assembly-pdf-linearization",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-complete-only",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-first-hit",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-mode",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["memory-and-disk", "memory-only"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-priority",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["high", "low", "medium"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-stand-in-period",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-empty",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-missing",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "code",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "coherency",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["blade", "cluster"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                in_sections: &["nodes"],
                references: &["wam_policy"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "invert-match",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jpeg-progressive-encoding",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jpeg-quality",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jpeg-quality-is-relative",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jpeg-sampling",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["1x1", "1x2", "2x1", "2x2", "preserve"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jpeg-strip-exif",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["if-safe", "make-safe", "no", "yes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jpeg-strip-keeps-copyright",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "jpegxr-quality",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-cache-control-extensions",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-cache-max-age",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-honor-ows",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-honor-ows-values",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-honor-request",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-honor-request-values",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-http-heuristic",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-insert-no-cache",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-preserve-response",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-preserve-response-values",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-response-max-age",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-response-s-maxage",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-stand-in-codes",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime-use-heuristic",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "matching",
                value_type: ValueKind::List,
                in_sections: &["nodes"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["nodes", "matching"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-alias",
                        value_type: ValueKind::String,
                        in_sections: &["nodes", "matching"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-direction",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes", "matching"],
                        enum_values: &["left-to-right", "right-to-left"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-name",
                        value_type: ValueKind::String,
                        in_sections: &["nodes", "matching"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-ordinal",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes", "matching"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["nodes", "matching"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "value-case-sensitive",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes", "matching"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "values",
                        value_type: ValueKind::List,
                        in_sections: &["nodes", "matching"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["nodes", "matching"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-alias",
                value_type: ValueKind::String,
                in_sections: &["nodes", "matching"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-direction",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "matching"],
                enum_values: &["left-to-right", "right-to-left"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-name",
                value_type: ValueKind::String,
                in_sections: &["nodes", "matching"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-ordinal",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes", "matching"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["nodes", "matching"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value-case-sensitive",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "matching"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "values",
                value_type: ValueKind::List,
                in_sections: &["nodes", "matching"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["nodes", "matching", "values"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "can-be-empty",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes", "matching", "values"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "can-be-missing",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes", "matching", "values"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "invert-match",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes", "matching", "values"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["nodes", "matching", "values"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-empty",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "matching", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-missing",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "matching", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "invert-match",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "matching", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "object-max-size",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["from-profile"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "object-min-size",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["from-profile"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "optimize-for-client",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "optimize-image",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                allow_none: true,
                enum_values: &["none", "to-gif", "to-jpeg", "to-png", "to-tiff"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "options",
                value_type: ValueKind::List,
                in_sections: &["nodes"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "order",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "png-256-colors",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-queueing",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "response-codes-cached",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value-case-sensitive",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "values",
                value_type: ValueKind::List,
                in_sections: &["nodes"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "variation",
                value_type: ValueKind::List,
                in_sections: &["nodes"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["nodes", "variation"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-alias",
                        value_type: ValueKind::String,
                        in_sections: &["nodes", "variation"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-all",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes", "variation"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-ambiguous-as-unnamed",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes", "variation"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-direction",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes", "variation"],
                        enum_values: &["left-to-right", "right-to-left"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-name",
                        value_type: ValueKind::String,
                        in_sections: &["nodes", "variation"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "arg-ordinal",
                        value_type: ValueKind::Unknown,
                        in_sections: &["nodes", "variation"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["nodes", "variation"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "value-case-sensitive",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes", "variation"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "values",
                        value_type: ValueKind::List,
                        in_sections: &["nodes", "variation"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["nodes", "variation"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-alias",
                value_type: ValueKind::String,
                in_sections: &["nodes", "variation"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-all",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-ambiguous-as-unnamed",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-direction",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation"],
                enum_values: &["left-to-right", "right-to-left"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-name",
                value_type: ValueKind::String,
                in_sections: &["nodes", "variation"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-ordinal",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes", "variation"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["nodes", "variation"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value-case-sensitive",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "values",
                value_type: ValueKind::List,
                in_sections: &["nodes", "variation"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["nodes", "variation", "values"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "cache-as",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes", "variation", "values"],
                        enum_values: &["different", "same"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "can-be-empty",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes", "variation", "values"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "can-be-missing",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes", "variation", "values"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "invert-match",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes", "variation", "values"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "match-all",
                        value_type: ValueKind::Enum,
                        in_sections: &["nodes", "variation", "values"],
                        enum_values: &["no", "yes"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["nodes", "variation", "values"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-as",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation", "values"],
                enum_values: &["different", "same"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-empty",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-missing",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "invert-match",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-all",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "video-acceleration-ad-policy",
                value_type: ValueKind::String,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "video-optimization-ad-frequency",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "video-optimization-fast-start",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "video-optimization-insert-ad",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "video-optimization-max-bitrate",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "video-optimization-preroll-ad",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "viewstate-cache",
                value_type: ValueKind::Enum,
                in_sections: &["nodes"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "viewstate-cache-size",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "viewstate-tag",
                value_type: ValueKind::String,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "webp-quality",
                value_type: ValueKind::Integer,
                in_sections: &["nodes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "publish-build",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "publish-comment",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "published-on",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "substitutions",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["substitutions"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["substitutions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "dst-alias",
                        value_type: ValueKind::String,
                        in_sections: &["substitutions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "dst-direction",
                        value_type: ValueKind::Enum,
                        in_sections: &["substitutions"],
                        enum_values: &["left-to-right", "right-to-left"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "dst-name",
                        value_type: ValueKind::String,
                        in_sections: &["substitutions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "dst-ordinal",
                        value_type: ValueKind::Unknown,
                        in_sections: &["substitutions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "dst-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["substitutions"],
                        enum_values: &["path-segment", "query-param"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "dst-urls",
                        value_type: ValueKind::List,
                        in_sections: &["substitutions"],
                        allow_none: true,
                        list_operators: &["add", "delete", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "src-alias",
                        value_type: ValueKind::String,
                        in_sections: &["substitutions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "src-direction",
                        value_type: ValueKind::Enum,
                        in_sections: &["substitutions"],
                        enum_values: &["left-to-right", "right-to-left"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "src-name",
                        value_type: ValueKind::String,
                        in_sections: &["substitutions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "src-ordinal",
                        value_type: ValueKind::Unknown,
                        in_sections: &["substitutions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "src-type",
                        value_type: ValueKind::Unknown,
                        in_sections: &["substitutions"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "src-url",
                        value_type: ValueKind::Enum,
                        in_sections: &["substitutions"],
                        enum_values: &["absolute", "relative"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["substitutions"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-alias",
                value_type: ValueKind::String,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-direction",
                value_type: ValueKind::Enum,
                in_sections: &["substitutions"],
                enum_values: &["left-to-right", "right-to-left"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-name",
                value_type: ValueKind::String,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-ordinal",
                value_type: ValueKind::Unknown,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-type",
                value_type: ValueKind::Enum,
                in_sections: &["substitutions"],
                enum_values: &["path-segment", "query-param"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-urls",
                value_type: ValueKind::List,
                in_sections: &["substitutions"],
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-alias",
                value_type: ValueKind::String,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-direction",
                value_type: ValueKind::Enum,
                in_sections: &["substitutions"],
                enum_values: &["left-to-right", "right-to-left"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-name",
                value_type: ValueKind::String,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-ordinal",
                value_type: ValueKind::Unknown,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-type",
                value_type: ValueKind::Unknown,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-url",
                value_type: ValueKind::Enum,
                in_sections: &["substitutions"],
                enum_values: &["absolute", "relative"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "wam_resource_concat_set",
            table_name: None,
            resolver_name: None,
            module: Some("wam"),
            object_types: &["resource concat-set"],
        },
        header_types: &[("wam", "resource concat-set")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "members",
                value_type: ValueKind::Unknown,
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &["css", "js"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "wam_resource_domain_list",
            table_name: None,
            resolver_name: None,
            module: Some("wam"),
            object_types: &["resource domain-list"],
        },
        header_types: &[("wam", "resource domain-list")],
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
                name: "domains",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "wam_resource_url",
            table_name: None,
            resolver_name: None,
            module: Some("wam"),
            object_types: &["resource url"],
        },
        header_types: &[("wam", "resource url")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &["css", "js"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
