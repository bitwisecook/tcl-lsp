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

//! BIG-IP object specs for `ltm monitor module-score`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_monitor_module_score",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["monitor module-score"],
    },
    header_types: &[("ltm", "monitor module-score")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "debug",
            value_type: ValueKind::Enum,
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("no"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            references: &["ltm_monitor_module_score"],
            default: Some("module_score"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "interval",
            value_type: ValueKind::Integer,
            default: Some("10 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "pool",
            value_type: ValueKind::Reference,
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
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "snmp-community",
            value_type: ValueKind::String,
            default: Some("public"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "snmp-ip-address",
            value_type: ValueKind::String,
            allow_none: true,
            shape_kind: Some(ValueKind::IpAddress),
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "snmp-port",
            value_type: ValueKind::Unknown,
            default: Some("161"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "snmp-version",
            value_type: ValueKind::String,
            default: Some("v2c"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "time-until-up",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "timeout",
            value_type: ValueKind::Integer,
            default: Some("30 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "up-interval",
            value_type: ValueKind::Integer,
            default: Some(
                "0 (zero), which specifies that the system uses the value of the interval option whether the resource is up or down",
            ),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
