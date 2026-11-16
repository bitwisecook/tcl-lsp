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

//! BIG-IP object specs for `apm resource address-space`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_resource_address_space",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["resource address-space"],
    },
    header_types: &[("apm", "resource address-space")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cfg-uri",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "discovery-interval",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dns",
            value_type: ValueKind::List,
            repeated: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ipv4",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ipv6",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "last-discovery-time",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-response-size",
            value_type: ValueKind::Unknown,
            default: Some("128 kB"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "options",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "trusted-ca-bundle",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
