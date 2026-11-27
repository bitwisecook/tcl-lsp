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

//! BIG-IP object specs for `sys software`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_software_hotfix",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["software hotfix"],
        },
        header_types: &[("sys", "software hotfix")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_software_image",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["software image"],
        },
        header_types: &[("sys", "software image")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_software_signature",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["software signature"],
        },
        header_types: &[("sys", "software signature")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_software_update",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["software update"],
        },
        header_types: &[("sys", "software update")],
        properties: &[
            BigipPropertySpec {
                name: "auto-check",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-phonehome",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "frequency",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_software_volume",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["software volume"],
        },
        header_types: &[("sys", "software volume")],
        properties: &[],
    },
];
