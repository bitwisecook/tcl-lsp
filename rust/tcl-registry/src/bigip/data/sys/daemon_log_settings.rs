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

//! BIG-IP object specs for `sys daemon-log-settings`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_daemon_log_settings_clusterd",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["daemon-log-settings clusterd"],
        },
        header_types: &[("sys", "daemon-log-settings clusterd")],
        properties: &[BigipPropertySpec {
            name: "log-level",
            value_type: ValueKind::Enum,
            enum_values: &[
                "critical",
                "debug",
                "error",
                "informational",
                "notice",
                "warning",
            ],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_daemon_log_settings_csyncd",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["daemon-log-settings csyncd"],
        },
        header_types: &[("sys", "daemon-log-settings csyncd")],
        properties: &[BigipPropertySpec {
            name: "log-level",
            value_type: ValueKind::Enum,
            enum_values: &[
                "critical",
                "debug",
                "error",
                "informational",
                "notice",
                "warning",
            ],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_daemon_log_settings_icr_eventd",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["daemon-log-settings icr-eventd"],
        },
        header_types: &[("sys", "daemon-log-settings icr-eventd")],
        properties: &[BigipPropertySpec {
            name: "log-level",
            value_type: ValueKind::Enum,
            enum_values: &[
                "critical",
                "debug",
                "error",
                "informational",
                "notice",
                "warning",
            ],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_daemon_log_settings_icrd",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["daemon-log-settings icrd"],
        },
        header_types: &[("sys", "daemon-log-settings icrd")],
        properties: &[BigipPropertySpec {
            name: "audit",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["all", "modifications", "none"],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_daemon_log_settings_lind",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["daemon-log-settings lind"],
        },
        header_types: &[("sys", "daemon-log-settings lind")],
        properties: &[BigipPropertySpec {
            name: "log-level",
            value_type: ValueKind::Enum,
            enum_values: &[
                "critical",
                "debug",
                "error",
                "informational",
                "notice",
                "warning",
            ],
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_daemon_log_settings_mcpd",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["daemon-log-settings mcpd"],
        },
        header_types: &[("sys", "daemon-log-settings mcpd")],
        properties: &[
            BigipPropertySpec {
                name: "audit",
                value_type: ValueKind::Enum,
                enum_values: &["all", "disabled", "enabled", "verbose"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert",
                    "critical",
                    "debug",
                    "emergency",
                    "error",
                    "informational",
                    "notice",
                    "panic",
                    "warning",
                ],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_daemon_log_settings_tmm",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["daemon-log-settings tmm"],
        },
        header_types: &[("sys", "daemon-log-settings tmm")],
        properties: &[
            BigipPropertySpec {
                name: "arp-log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "error", "informational", "notice", "warning"],
                default: Some("warning"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "http-compression-log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "error", "informational", "notice", "warning"],
                default: Some("error"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "http-log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "error", "informational", "notice", "warning"],
                default: Some("error"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "informational", "notice", "warning"],
                default: Some("warning"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "irule-log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "error", "informational", "notice", "warning"],
                default: Some("warning"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "layer4-log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "informational", "notice"],
                default: Some("notice"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "net-log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "critical",
                    "debug",
                    "error",
                    "informational",
                    "notice",
                    "warning",
                ],
                default: Some("warning"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "os-log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert",
                    "critical",
                    "debug",
                    "emergency",
                    "error",
                    "informational",
                    "notice",
                    "warning",
                ],
                default: Some("notice"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pva-log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "informational", "notice"],
                default: Some("informational"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ssl-log-level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert",
                    "critical",
                    "debug",
                    "emergency",
                    "error",
                    "informational",
                    "notice",
                    "warning",
                ],
                default: Some("warning"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
