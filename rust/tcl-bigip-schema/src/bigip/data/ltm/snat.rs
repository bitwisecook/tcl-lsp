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

//! BIG-IP object specs for `ltm snat`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_snat",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["snat"],
    },
    header_types: &[("ltm", "snat")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auto-lasthop",
            value_type: ValueKind::Enum,
            enum_values: &["default", "disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "metadata",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mirror",
            value_type: ValueKind::Unknown,
            allow_none: true,
            enum_values: &["disabled", "enabled", "none"],
            shape_kind: Some(ValueKind::Object),
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "origins",
            value_type: ValueKind::Unknown,
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "persist",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "snatpool",
            value_type: ValueKind::Reference,
            references: &["ltm_snatpool"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source-port",
            value_type: ValueKind::Enum,
            enum_values: &["change", "preserve", "preserve-strict"],
            default: Some("preserve"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "translation",
            value_type: ValueKind::Reference,
            repeated: true,
            references: &[
                "ltm_snat_translation",
                "security_nat_destination_translation",
                "security_nat_source_translation",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "value",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "vlans",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["default", "none"],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
