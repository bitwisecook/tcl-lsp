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

//! BIG-IP object specs for `ltm profile traffic-acceleration`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_traffic_acceleration",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile traffic-acceleration"],
    },
    header_types: &[("ltm", "profile traffic-acceleration")],
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
            references: &["ltm_profile_traffic_acceleration"],
            default: Some("traffic-acceleration"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "idle-timeout",
            value_type: ValueKind::Integer,
            default: Some("five minutes or 300000 milliseconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tcp-handshake-timeout",
            value_type: ValueKind::Integer,
            default: Some("5000 milliseconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "time-wait-timeout",
            value_type: ValueKind::Integer,
            default: Some("2000 milliseconds"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
