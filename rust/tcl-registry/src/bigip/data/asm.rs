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

//! BIG-IP object specs for the `asm` tmsh module.
//!
//! **Hand-maintained**, with no generator to produce these from — edit
//! them directly, in the same shape the other module files use.  Organised
//! by tmsh module name (this file's own module field, `Some("asm")`).
//!
//! `cargo xtask bigip-data-schema --check` enforces the internal
//! invariants a generator would otherwise have guaranteed: every `kind`
//! is globally unique, filed under the module file matching its own
//! `module` field, and every `references` target either names a real
//! kind or is on the documented known-gap list.
// Some modules hold property-less kinds, so not every imported type
// is used in every file; large tmsh bounds appear as bare f64 literals.
#![allow(unused_imports, clippy::unreadable_literal)]
use super::super::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "asm_httpclass_asm",
            table_name: None,
            resolver_name: None,
            module: Some("asm"),
            object_types: &["httpclass-asm"],
        },
        header_types: &[("asm", "httpclass-asm")],
        properties: &[
            BigipPropertySpec {
                name: "active-policy-name",
                value_type: ValueKind::String,
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "language",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "predefined-policy",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "asm_policy",
            table_name: None,
            resolver_name: None,
            module: Some("asm"),
            object_types: &["policy"],
        },
        header_types: &[("asm", "policy")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blocking-mode",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encoding",
                value_type: ValueKind::Reference,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "parent-policy",
                value_type: ValueKind::Reference,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "policy-builder",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "policy-template",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "policy-type",
                value_type: ValueKind::Enum,
                enum_values: &["parent", "security"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
