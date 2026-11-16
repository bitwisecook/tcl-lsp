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

//! BIG-IP object specs for `wam application`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
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
}];
