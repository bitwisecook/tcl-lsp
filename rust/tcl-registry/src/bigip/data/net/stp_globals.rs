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

//! BIG-IP object specs for `net stp-globals`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "net_stp_globals",
        table_name: None,
        resolver_name: None,
        module: Some("net"),
        object_types: &["stp-globals"],
    },
    header_types: &[("net", "stp-globals")],
    properties: &[
        BigipPropertySpec {
            name: "config-name",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "config-revision",
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
            name: "fwd-delay",
            value_type: ValueKind::Integer,
            default: Some("15 seconds, and the valid range is 4 to 30"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hello-time",
            value_type: ValueKind::Integer,
            default: Some("2 seconds, and the valid range is 1 - 10"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-age",
            value_type: ValueKind::Integer,
            default: Some("20 seconds, and the valid range is 6-40 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-hops",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mode",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "mstp", "passthru", "rstp", "stp"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "transmit-hold",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
