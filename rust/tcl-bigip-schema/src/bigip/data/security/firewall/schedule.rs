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

//! BIG-IP object specs for `security firewall schedule`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "security_firewall_schedule",
        table_name: None,
        resolver_name: None,
        module: Some("security"),
        object_types: &["firewall schedule"],
    },
    header_types: &[("security", "firewall schedule")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "daily-hour-end",
            value_type: ValueKind::Unknown,
            default: Some("24:00 (midnight)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "daily-hour-start",
            value_type: ValueKind::Unknown,
            default: Some("0:00 (midnight at the start of the day)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "date-valid-end",
            value_type: ValueKind::Integer,
            default: Some("19:14 1/18/2038 (the latest date expressible with a 32-bit integer)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "date-valid-start",
            value_type: ValueKind::Unknown,
            default: Some("midnight 1/1/1970 (Unix epoch)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "days-of-week",
            value_type: ValueKind::Enum,
            repeated: true,
            enum_values: &[
                "friday",
                "monday",
                "saturday",
                "sunday",
                "thursday",
                "tuesday",
                "wednesday",
            ],
            default: Some("all seven days"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
