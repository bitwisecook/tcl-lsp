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

//! BIG-IP object specs for `sys daemon-ha`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "sys_daemon_ha",
        table_name: None,
        resolver_name: None,
        module: Some("sys"),
        object_types: &["daemon-ha"],
    },
    header_types: &[("sys", "daemon-ha")],
    properties: &[
        BigipPropertySpec {
            name: "heartbeat",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some(
                "enabled for all daemons, except the named daemon, which is disabled by default",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "heartbeat-action",
            value_type: ValueKind::Enum,
            enum_values: &[
                "go-offline",
                "go-offline-downlinks-restart",
                "go-offline-restart",
                "reboot",
                "restart",
                "restart-all",
            ],
            default: Some(
                "dependent on the specified daemon, the most common default value is restart",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "running",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some(
                "dependent on the specified daemon, the most common default value is enabled",
            ),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
