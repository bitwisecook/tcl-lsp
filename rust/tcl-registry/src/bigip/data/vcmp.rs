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

//! BIG-IP object specs for the `vcmp` tmsh module.
//!
//! **Hand-maintained.** These files were originally produced by a
//! one-time port of the pre-rewrite Python registry
//! (`dialects/f5/bigip/registry/specs/`, still present on `main`) via a
//! `scripts/registry-audit/gen_bigip_rust.py` generator that no longer
//! exists — deleted along with the rest of the retired Python tooling on
//! this branch, and the commits that ran it were squashed away in the
//! `rust` branch's rebase-onto-main history (see issue #1404). Originally
//! organised by the first letter of `kind`; reorganised by tmsh module
//! name (this file's own module field, `Some("vcmp")`) per
//! maintainer review on the PR that introduced the consistency gate below.
//! There is no live generator to regenerate these from; edit them by hand,
//! in the same shape the other module files already use.
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
            kind: "vcmp_guest",
            table_name: None,
            resolver_name: None,
            module: Some("vcmp"),
            object_types: &["guest"],
        },
        header_types: &[("vcmp", "guest")],
        properties: &[
            BigipPropertySpec {
                name: "allowed-slots",
                value_type: ValueKind::List,
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
                name: "boot-priority",
                value_type: ValueKind::Integer,
                default: Some("65535"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "capabilities",
                value_type: ValueKind::List,
                usage_flags: &["optional"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cores-per-slot",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hostname",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "initial-hotfix",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "initial-image",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "management-gw",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "management-ip",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "management-network",
                value_type: ValueKind::Enum,
                enum_values: &["bridged", "isolated"],
                default: Some("Bridged"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-slots",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "slots",
                value_type: ValueKind::Integer,
                default: Some("1"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Enum,
                enum_values: &["configured", "deployed", "provisioned"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-profile",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "virtual-disk",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlans",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "vcmp_traffic_profile",
            table_name: None,
            resolver_name: None,
            module: Some("vcmp"),
            object_types: &["traffic-profile"],
        },
        header_types: &[("vcmp", "traffic-profile")],
        properties: &[BigipPropertySpec {
            name: "color-policer",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "vcmp_virtual_disk",
            table_name: None,
            resolver_name: None,
            module: Some("vcmp"),
            object_types: &["virtual-disk"],
        },
        header_types: &[("vcmp", "virtual-disk")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "vcmp_virtual_disk_template",
            table_name: None,
            resolver_name: None,
            module: Some("vcmp"),
            object_types: &["virtual-disk-template"],
        },
        header_types: &[("vcmp", "virtual-disk-template")],
        properties: &[],
    },
];
