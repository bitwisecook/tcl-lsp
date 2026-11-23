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

//! BIG-IP object specs for `sys syslog`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "sys_syslog",
        table_name: None,
        resolver_name: None,
        module: Some("sys"),
        object_types: &["syslog"],
    },
    header_types: &[("sys", "syslog")],
    properties: &[
        BigipPropertySpec {
            name: "auth-priv-from",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("notice"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auth-priv-to",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("emerg"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "clustered-host-name",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "clustered-message-slot",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "console-log",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cron-from",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("warning"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cron-to",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("emerg"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "daemon-from",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("notice"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "daemon-to",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("emerg"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "include",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "iso-date",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "kern-from",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("debug"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "kern-to",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("emerg"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "local6-from",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("notice"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "local6-to",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("emerg"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mail-from",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("notice"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mail-to",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("emerg"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "messages-from",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("notice"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "messages-to",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("warning"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "remote-servers",
            value_type: ValueKind::List,
            allow_none: true,
            default: Some("none"),
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "host",
                    value_type: ValueKind::Unknown,
                    in_sections: &["remote-servers"],
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "local-ip",
                    value_type: ValueKind::String,
                    in_sections: &["remote-servers"],
                    shape_kind: Some(ValueKind::IpAddress),
                    address_grammar: Some(crate::bigip::AddressGrammar::Address),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "remote-port",
                    value_type: ValueKind::Unknown,
                    in_sections: &["remote-servers"],
                    default: Some("514"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "host",
            value_type: ValueKind::Unknown,
            in_sections: &["remote-servers"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "local-ip",
            value_type: ValueKind::String,
            in_sections: &["remote-servers"],
            shape_kind: Some(ValueKind::IpAddress),
            address_grammar: Some(crate::bigip::AddressGrammar::Address),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "remote-port",
            value_type: ValueKind::Unknown,
            in_sections: &["remote-servers"],
            default: Some("514"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "user-log-from",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("notice"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "user-log-to",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warning",
            ],
            default: Some("emerg"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
