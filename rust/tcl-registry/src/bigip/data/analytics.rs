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

//! BIG-IP object specs for the `analytics` tmsh module.
//!
//! **Hand-maintained.** These files were originally produced by a
//! one-time port of the pre-rewrite Python registry
//! (`dialects/f5/bigip/registry/specs/`, still present on `main`) via a
//! `scripts/registry-audit/gen_bigip_rust.py` generator that no longer
//! exists — deleted along with the rest of the retired Python tooling on
//! this branch, and the commits that ran it were squashed away in the
//! `rust` branch's rebase-onto-main history (see issue #1404). Originally
//! organised by the first letter of `kind`; reorganised by tmsh module
//! name (this file's own module field, `Some("analytics")`) per
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
            kind: "analytics_afm_sweeper_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["afm-sweeper scheduled-report"],
        },
        header_types: &[("analytics", "afm-sweeper scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_application_security_anomalies_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["application-security-anomalies scheduled-report"],
        },
        header_types: &[(
            "analytics",
            "application-security-anomalies scheduled-report",
        )],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_application_security_network_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["application-security-network scheduled-report"],
        },
        header_types: &[("analytics", "application-security-network scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_application_security_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["application-security scheduled-report"],
        },
        header_types: &[("analytics", "application-security scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_asm_bypass_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["asm-bypass scheduled-report"],
        },
        header_types: &[("analytics", "asm-bypass scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_asm_cpu_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["asm-cpu scheduled-report"],
        },
        header_types: &[("analytics", "asm-cpu scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_asm_memory_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["asm-memory scheduled-report"],
        },
        header_types: &[("analytics", "asm-memory scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_asm_violation_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["asm-violation scheduled-report"],
        },
        header_types: &[("analytics", "asm-violation scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_cpu_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["cpu scheduled-report"],
        },
        header_types: &[("analytics", "cpu scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_device_traffic_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["device-traffic scheduled-report"],
        },
        header_types: &[("analytics", "device-traffic scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_disk_info_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["disk-info scheduled-report"],
        },
        header_types: &[("analytics", "disk-info scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_dns_protocol_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["dns-protocol scheduled-report"],
        },
        header_types: &[("analytics", "dns-protocol scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_dns_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["dns scheduled-report"],
        },
        header_types: &[("analytics", "dns scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_dos_l3_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["dos-l3 scheduled-report"],
        },
        header_types: &[("analytics", "dos-l3 scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_fw_nat_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["fw-nat scheduled-report"],
        },
        header_types: &[("analytics", "fw-nat scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_global_settings",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["global-settings"],
        },
        header_types: &[("analytics", "global-settings")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_http_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["http scheduled-report"],
        },
        header_types: &[("analytics", "http scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_ip_intelligence_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["ip-intelligence scheduled-report"],
        },
        header_types: &[("analytics", "ip-intelligence scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_ip_layer_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["ip-layer scheduled-report"],
        },
        header_types: &[("analytics", "ip-layer scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_lsn_pool_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["lsn-pool scheduled-report"],
        },
        header_types: &[("analytics", "lsn-pool scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_memory_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["memory scheduled-report"],
        },
        header_types: &[("analytics", "memory scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_network_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["network scheduled-report"],
        },
        header_types: &[("analytics", "network scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_pem_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["pem scheduled-report"],
        },
        header_types: &[("analytics", "pem scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_pool_traffic_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["pool-traffic scheduled-report"],
        },
        header_types: &[("analytics", "pool-traffic scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_proc_cpu_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["proc-cpu scheduled-report"],
        },
        header_types: &[("analytics", "proc-cpu scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_protocol_security_http_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["protocol-security-http scheduled-report"],
        },
        header_types: &[("analytics", "protocol-security-http scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_protocol_security_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["protocol-security scheduled-report"],
        },
        header_types: &[("analytics", "protocol-security scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_sip_dos_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["sip-dos scheduled-report"],
        },
        header_types: &[("analytics", "sip-dos scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_sip_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["sip scheduled-report"],
        },
        header_types: &[("analytics", "sip scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_ssl_orchestrator_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["ssl-orchestrator scheduled-report"],
        },
        header_types: &[("analytics", "ssl-orchestrator scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_ssl_orchestrator_service_virtual_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["ssl-orchestrator-service-virtual scheduled-report"],
        },
        header_types: &[(
            "analytics",
            "ssl-orchestrator-service-virtual scheduled-report",
        )],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_swg_blocked_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["swg-blocked scheduled-report"],
        },
        header_types: &[("analytics", "swg-blocked scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_swg_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["swg scheduled-report"],
        },
        header_types: &[("analytics", "swg scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_tcp_analytics_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["tcp-analytics scheduled-report"],
        },
        header_types: &[("analytics", "tcp-analytics scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_tcp_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["tcp scheduled-report"],
        },
        header_types: &[("analytics", "tcp scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_traffic_classification_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["traffic-classification scheduled-report"],
        },
        header_types: &[("analytics", "traffic-classification scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_udp_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["udp scheduled-report"],
        },
        header_types: &[("analytics", "udp scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_uri_type",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["uri-type"],
        },
        header_types: &[("analytics", "uri-type")],
        properties: &[BigipPropertySpec {
            name: "file-extensions",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_vcmp_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["vcmp scheduled-report"],
        },
        header_types: &[("analytics", "vcmp scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "analytics_virtual_scheduled_report",
            table_name: None,
            resolver_name: None,
            module: Some("analytics"),
            object_types: &["virtual scheduled-report"],
        },
        header_types: &[("analytics", "virtual scheduled-report")],
        properties: &[
            BigipPropertySpec {
                name: "device-group",
                value_type: ValueKind::Reference,
                references: &["cm_device_group"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "email-addresses",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "first-time",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "every-12-hours",
                    "every-24-hours",
                    "every-6-hours",
                    "every-month",
                    "every-week",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "include-total",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "multi-leveled-report",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "chart-path",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "limit",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "measures",
                        value_type: ValueKind::List,
                        in_sections: &["multi-leveled-report"],
                        allow_none: true,
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "time-diff",
                        value_type: ValueKind::Enum,
                        in_sections: &["multi-leveled-report"],
                        enum_values: &[
                            "last-day",
                            "last-hour",
                            "last-month",
                            "last-week",
                            "last-year",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "view-by",
                        value_type: ValueKind::Unknown,
                        in_sections: &["multi-leveled-report"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "chart-path",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "limit",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "measures",
                value_type: ValueKind::List,
                in_sections: &["multi-leveled-report"],
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "time-diff",
                value_type: ValueKind::Enum,
                in_sections: &["multi-leveled-report"],
                enum_values: &[
                    "last-day",
                    "last-hour",
                    "last-month",
                    "last-week",
                    "last-year",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "view-by",
                value_type: ValueKind::Unknown,
                in_sections: &["multi-leveled-report"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-report-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "smtp-config",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
