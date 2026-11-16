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

//! BIG-IP object specs for `ltm profile request-log`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_request_log",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile request-log"],
    },
    header_types: &[("ltm", "profile request-log")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_profile_request_log"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-request-logging-errors",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-response-by-default",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-response-logging-error",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "proxy-close-on-error",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "proxy-respond-on-logging-error",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "proxy-response",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-log-error-pool",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &[
                "analytics_lsn_pool_report",
                "analytics_lsn_pool_scheduled_report",
                "analytics_pool_traffic_report",
                "analytics_pool_traffic_scheduled_report",
                "gtm_pool_a",
                "gtm_pool_aaaa",
                "gtm_pool_cname",
                "gtm_pool_https",
                "gtm_pool_mx",
                "gtm_pool_naptr",
                "gtm_pool_srv",
                "gtm_pool_svcb",
                "gtm_prober_pool",
                "ltm_lsn_pool",
                "ltm_pool",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-log-error-protocol",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["TCP", "UDP", "none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-log-error-template",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-log-pool",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &[
                "analytics_lsn_pool_report",
                "analytics_lsn_pool_scheduled_report",
                "analytics_pool_traffic_report",
                "analytics_pool_traffic_scheduled_report",
                "gtm_pool_a",
                "gtm_pool_aaaa",
                "gtm_pool_cname",
                "gtm_pool_https",
                "gtm_pool_mx",
                "gtm_pool_naptr",
                "gtm_pool_srv",
                "gtm_pool_svcb",
                "gtm_prober_pool",
                "ltm_lsn_pool",
                "ltm_pool",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-log-protocol",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["TCP", "UDP", "none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-log-template",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-logging",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "response-log-error-pool",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &[
                "analytics_lsn_pool_report",
                "analytics_lsn_pool_scheduled_report",
                "analytics_pool_traffic_report",
                "analytics_pool_traffic_scheduled_report",
                "gtm_pool_a",
                "gtm_pool_aaaa",
                "gtm_pool_cname",
                "gtm_pool_https",
                "gtm_pool_mx",
                "gtm_pool_naptr",
                "gtm_pool_srv",
                "gtm_pool_svcb",
                "gtm_prober_pool",
                "ltm_lsn_pool",
                "ltm_pool",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "response-log-error-protocol",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["TCP", "UDP", "none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "response-log-error-template",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "response-log-pool",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &[
                "analytics_lsn_pool_report",
                "analytics_lsn_pool_scheduled_report",
                "analytics_pool_traffic_report",
                "analytics_pool_traffic_scheduled_report",
                "gtm_pool_a",
                "gtm_pool_aaaa",
                "gtm_pool_cname",
                "gtm_pool_https",
                "gtm_pool_mx",
                "gtm_pool_naptr",
                "gtm_pool_srv",
                "gtm_pool_svcb",
                "gtm_prober_pool",
                "ltm_lsn_pool",
                "ltm_pool",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "response-log-protocol",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["TCP", "UDP", "none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "response-log-template",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "response-logging",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
