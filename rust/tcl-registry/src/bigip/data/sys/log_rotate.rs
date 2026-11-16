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

//! BIG-IP object specs for `sys log-rotate`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "sys_log_rotate",
        table_name: None,
        resolver_name: None,
        module: Some("sys"),
        object_types: &["log-rotate"],
    },
    header_types: &[("sys", "log-rotate")],
    properties: &[
        BigipPropertySpec {
            name: "common-backlogs",
            value_type: ValueKind::Integer,
            default: Some("24"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "common-include",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ilx-include",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ilx-rotations",
            value_type: ValueKind::String,
            default: Some("10"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ilx-schedule",
            value_type: ValueKind::String,
            default: Some("daily"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ilx-size",
            value_type: ValueKind::String,
            default: Some("10240 kilobytes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "include",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-file-size",
            value_type: ValueKind::Integer,
            default: Some("1024000"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mysql-include",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "syslog-include",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tomcat-include",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "wa-include",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
