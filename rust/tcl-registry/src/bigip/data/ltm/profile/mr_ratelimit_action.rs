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

//! BIG-IP object specs for `ltm profile mr-ratelimit-action`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_mr_ratelimit_action",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile mr-ratelimit-action"],
    },
    header_types: &[("ltm", "profile mr-ratelimit-action")],
    properties: &[
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_profile_mr_ratelimit_action"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "priority-1",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &[
                "delay-100",
                "delay-25",
                "delay-50",
                "drop-100",
                "drop-25",
                "drop-50",
                "none",
                "return-100",
                "return-25",
                "return-50",
            ],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "priority-2",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &[
                "delay-100",
                "delay-25",
                "delay-50",
                "drop-100",
                "drop-25",
                "drop-50",
                "none",
                "return-100",
                "return-25",
                "return-50",
            ],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "priority-3",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &[
                "delay-100",
                "delay-25",
                "delay-50",
                "drop-100",
                "drop-25",
                "drop-50",
                "none",
                "return-100",
                "return-25",
                "return-50",
            ],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "priority-4",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &[
                "delay-100",
                "delay-25",
                "delay-50",
                "drop-100",
                "drop-25",
                "drop-50",
                "none",
                "return-100",
                "return-25",
                "return-50",
            ],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
