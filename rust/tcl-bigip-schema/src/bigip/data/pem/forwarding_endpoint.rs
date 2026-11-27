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

//! BIG-IP object specs for `pem forwarding-endpoint`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "pem_forwarding_endpoint",
        table_name: None,
        resolver_name: None,
        module: Some("pem"),
        object_types: &["forwarding-endpoint"],
    },
    header_types: &[("pem", "forwarding-endpoint")],
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
            name: "persistence",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "fallback",
                    value_type: ValueKind::Enum,
                    in_sections: &["persistence"],
                    enum_values: &["destination-ip", "disabled", "source-ip"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "hash-settings",
                    value_type: ValueKind::List,
                    in_sections: &["persistence"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "type",
                    value_type: ValueKind::Enum,
                    in_sections: &["persistence"],
                    enum_values: &["destination-ip", "disabled", "hash", "source-ip"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fallback",
            value_type: ValueKind::Enum,
            in_sections: &["persistence"],
            enum_values: &["destination-ip", "disabled", "source-ip"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hash-settings",
            value_type: ValueKind::List,
            in_sections: &["persistence"],
            block: &[
                BigipPropertySpec {
                    name: "algorithm",
                    value_type: ValueKind::Unknown,
                    in_sections: &["persistence", "hash-settings"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "length",
                    value_type: ValueKind::Integer,
                    in_sections: &["persistence", "hash-settings"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "offset",
                    value_type: ValueKind::Integer,
                    in_sections: &["persistence", "hash-settings"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "source",
                    value_type: ValueKind::Enum,
                    in_sections: &["persistence", "hash-settings"],
                    enum_values: &["tcl-snippet", "uri"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "tcl-value",
                    value_type: ValueKind::String,
                    in_sections: &["persistence", "hash-settings"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "algorithm",
            value_type: ValueKind::Unknown,
            in_sections: &["persistence", "hash-settings"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "length",
            value_type: ValueKind::Integer,
            in_sections: &["persistence", "hash-settings"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "offset",
            value_type: ValueKind::Integer,
            in_sections: &["persistence", "hash-settings"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source",
            value_type: ValueKind::Enum,
            in_sections: &["persistence", "hash-settings"],
            enum_values: &["tcl-snippet", "uri"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcl-value",
            value_type: ValueKind::String,
            in_sections: &["persistence", "hash-settings"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Enum,
            in_sections: &["persistence"],
            enum_values: &["destination-ip", "disabled", "hash", "source-ip"],
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
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "snat-pool",
            value_type: ValueKind::Reference,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source-port",
            value_type: ValueKind::Enum,
            enum_values: &["change", "preserve", "preserve-strict"],
            default: Some("preserve"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "translate-address",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "translate-service",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
