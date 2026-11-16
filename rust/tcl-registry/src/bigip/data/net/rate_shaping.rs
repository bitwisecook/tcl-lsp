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

//! BIG-IP object specs for `net rate-shaping`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_rate_shaping_class",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["rate-shaping class"],
        },
        header_types: &[("net", "rate-shaping class")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ceiling",
                value_type: ValueKind::Integer,
                default: Some("the value of the rate option"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ceiling-percentage",
                value_type: ValueKind::Integer,
                default: Some(
                    "0 (zero), which indicates that the class uses the value of the ceiling option",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "direction",
                value_type: ValueKind::Enum,
                enum_values: &["any", "to-client", "to-server"],
                default: Some("any"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "drop-policy",
                value_type: ValueKind::Reference,
                references: &["net_rate_shaping_drop_policy"],
                default: Some("tail, which is the simplest drop policy"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-burst",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "parent",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "queue",
                value_type: ValueKind::Enum,
                enum_values: &["pfifp", "sfq"],
                references: &[
                    "net_rate_shaping_queue",
                    "sys_nethsm_async_queue_stat",
                    "sys_nethsm_sync_queue_stat",
                ],
                default: Some("sfq"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-percentage",
                value_type: ValueKind::Integer,
                default: Some(
                    "0 (zero), which specifies that the system uses the value of the rate option",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "shaping-policy",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_rate_shaping_shaping_policy"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_rate_shaping_color_policer",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["rate-shaping color-policer"],
        },
        header_types: &[("net", "rate-shaping color-policer")],
        properties: &[
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Unknown,
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
                name: "committed-burst-size",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "committed-information-rate",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "excess-burst-size",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_rate_shaping_drop_policy",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["rate-shaping drop-policy"],
        },
        header_types: &[("net", "rate-shaping drop-policy")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "average-packet-size",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fred-max-active",
                value_type: ValueKind::Integer,
                default: Some("0 (zero),which disables active flow limitation"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fred-max-drop",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "fred-min-drop",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "inverse-weight",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-probability",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-threshold",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-threshold",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "red-hard-limit",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &["fred", "red", "tail"],
                default: Some("tail"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_rate_shaping_queue",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["rate-shaping queue"],
        },
        header_types: &[("net", "rate-shaping queue")],
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
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pfifo-max-size",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pfifo-min-size",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sfq-bucket-count",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sfq-bucket-size",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sfq-perturbation",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                enum_values: &["pfifo", "sfq"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_rate_shaping_shaping_policy",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["rate-shaping shaping-policy"],
        },
        header_types: &[("net", "rate-shaping shaping-policy")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ceiling-percentage",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "drop-policy",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &["net_rate_shaping_drop_policy"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-burst",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "queue",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "net_rate_shaping_queue",
                    "sys_nethsm_async_queue_stat",
                    "sys_nethsm_sync_queue_stat",
                ],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-percentage",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
