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

//! BIG-IP object specs for `sys global-settings`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "sys_global_settings",
        table_name: None,
        resolver_name: None,
        module: Some("sys"),
        object_types: &["global-settings"],
    },
    header_types: &[("sys", "global-settings")],
    properties: &[
        BigipPropertySpec {
            name: "aws-access-key",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "aws-api-max-concurrency",
            value_type: ValueKind::Integer,
            default: Some("1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "aws-secret-key",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "console-inactivity-timeout",
            value_type: ValueKind::Integer,
            default: Some("0 (zero), which means that no timeout is set"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "custom-addr",
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::IpAddress),
            default: Some("::"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            default: Some("no description"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "failsafe-action",
            value_type: ValueKind::Enum,
            enum_values: &[
                "failover-restart-tm",
                "go-offline",
                "go-offline-restart-tm",
                "reboot",
                "restart-all",
            ],
            default: Some("go-offline-restart-tm"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "file-blacklist-path-prefix",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "file-blacklist-read-only-path-prefix",
            value_type: ValueKind::String,
            usage_flags: &["read_only"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "file-local-path-prefix",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "file-whitelist-path-prefix",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "gui-audit",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "gui-expired-cert-alert",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "gui-security-banner",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "gui-security-banner-text",
            value_type: ValueKind::String,
            default: Some("Welcome to the BIG-IP Configuration Utility"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "gui-setup",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "host-addr-mode",
            value_type: ValueKind::Enum,
            enum_values: &["custom", "management", "state-mirror"],
            default: Some("management"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hostname",
            value_type: ValueKind::String,
            default: Some("bigip1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hosts-allow-include",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "lcd-display",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mgmt-dhcp",
            value_type: ValueKind::Enum,
            enum_values: &["dhcpv4", "dhcpv6", "disabled", "enabled"],
            default: Some("enabled for VE and disabled for all other platforms"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "net-reboot",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "password-prompt",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "quiet-boot",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "remote-host",
            value_type: ValueKind::List,
            allow_none: true,
            default: Some("none"),
            list_operators: &["add", "delete", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "addr",
                    value_type: ValueKind::String,
                    in_sections: &["remote-host"],
                    shape_kind: Some(ValueKind::IpAddress),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "hostname",
                    value_type: ValueKind::String,
                    in_sections: &["remote-host"],
                    default: Some("bigip1"),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "addr",
            value_type: ValueKind::String,
            in_sections: &["remote-host"],
            shape_kind: Some(ValueKind::IpAddress),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hostname",
            value_type: ValueKind::String,
            in_sections: &["remote-host"],
            default: Some("bigip1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssh-max-session-limit",
            value_type: ValueKind::Integer,
            default: Some("10 and the range is 1 to 65535"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssh-max-session-limit-per-user",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssh-root-session-limit",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssh-session-limit",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "username-prompt",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
