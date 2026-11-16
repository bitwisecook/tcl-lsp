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

//! BIG-IP object specs for `analytics application-security-network`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
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
}];
