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

//! BIG-IP object specs for `ltm cipher`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_cipher_group",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["cipher group"],
        },
        header_types: &[("ltm", "cipher group")],
        properties: &[
            BigipPropertySpec {
                name: "allow",
                value_type: ValueKind::List,
                repeated: true,
                list_operators: &["add"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "exclude",
                value_type: ValueKind::List,
                repeated: true,
                list_operators: &["add"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "require",
                value_type: ValueKind::List,
                required: true,
                repeated: true,
                list_operators: &["add"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_cipher_rule",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["cipher rule"],
        },
        header_types: &[("ltm", "cipher rule")],
        properties: &[
            BigipPropertySpec {
                name: "cipher",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dh-groups",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "signature-algorithms",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
