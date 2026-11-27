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

//! BIG-IP object specs for `security analytics`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "security_analytics_settings",
        table_name: None,
        resolver_name: None,
        module: Some("security"),
        object_types: &["analytics settings"],
    },
    header_types: &[("security", "analytics settings")],
    properties: &[
        BigipPropertySpec {
            name: "acl-rules",
            value_type: ValueKind::List,
            block: &[
                BigipPropertySpec {
                    name: "collect-client-ip",
                    value_type: ValueKind::Enum,
                    in_sections: &["acl-rules"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "collect-client-port",
                    value_type: ValueKind::Enum,
                    in_sections: &["acl-rules"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "collect-dest-ip",
                    value_type: ValueKind::Enum,
                    in_sections: &["acl-rules"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "collect-dest-port",
                    value_type: ValueKind::Enum,
                    in_sections: &["acl-rules"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "collect-server-side-stats",
                    value_type: ValueKind::Enum,
                    in_sections: &["acl-rules"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-client-ip",
            value_type: ValueKind::Enum,
            in_sections: &["acl-rules"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-client-port",
            value_type: ValueKind::Enum,
            in_sections: &["acl-rules"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-dest-ip",
            value_type: ValueKind::Enum,
            in_sections: &["acl-rules"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-dest-port",
            value_type: ValueKind::Enum,
            in_sections: &["acl-rules"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-server-side-stats",
            value_type: ValueKind::Enum,
            in_sections: &["acl-rules"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collected-stats-external-logging",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collected-stats-internal-logging",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dns",
            value_type: ValueKind::List,
            block: &[BigipPropertySpec {
                name: "collect-client-ip",
                value_type: ValueKind::Enum,
                in_sections: &["dns"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-client-ip",
            value_type: ValueKind::Enum,
            in_sections: &["dns"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dos-l2-l4",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[BigipPropertySpec {
                name: "collect-client-ip",
                value_type: ValueKind::Enum,
                in_sections: &["dos-l2-l4"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-client-ip",
            value_type: ValueKind::Enum,
            in_sections: &["dos-l2-l4"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "l3-l4-errors",
            value_type: ValueKind::List,
            block: &[
                BigipPropertySpec {
                    name: "collect-client-ip",
                    value_type: ValueKind::Enum,
                    in_sections: &["l3-l4-errors"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "collect-dest-ip",
                    value_type: ValueKind::Enum,
                    in_sections: &["l3-l4-errors"],
                    enum_values: &["disabled", "enabled"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-client-ip",
            value_type: ValueKind::Enum,
            in_sections: &["l3-l4-errors"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect-dest-ip",
            value_type: ValueKind::Enum,
            in_sections: &["l3-l4-errors"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "publisher",
            value_type: ValueKind::Reference,
            references: &[
                "security_blacklist_publisher_all_blacklist_publisher",
                "security_blacklist_publisher_blacklist_publisher_stats",
                "security_blacklist_publisher_by_addr",
                "security_blacklist_publisher_by_category",
                "security_blacklist_publisher_category",
                "security_blacklist_publisher_profile",
                "sys_icall_publisher",
                "sys_log_config_publisher",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "smtp-config",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "stale-rules",
            value_type: ValueKind::List,
            block: &[BigipPropertySpec {
                name: "collect",
                value_type: ValueKind::Enum,
                in_sections: &["stale-rules"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "collect",
            value_type: ValueKind::Enum,
            in_sections: &["stale-rules"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
