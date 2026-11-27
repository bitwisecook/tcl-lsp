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

//! BIG-IP object specs for `wam ad-policy`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "wam_ad_policy",
        table_name: None,
        resolver_name: None,
        module: Some("wam"),
        object_types: &["ad-policy"],
    },
    header_types: &[("wam", "ad-policy")],
    properties: &[
        BigipPropertySpec {
            name: "ad-insertion-order",
            value_type: ValueKind::Enum,
            enum_values: &["random", "sequential"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ads",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify"],
            block: &[
                BigipPropertySpec {
                    name: "preroll",
                    value_type: ValueKind::Enum,
                    in_sections: &["ads"],
                    enum_values: &["no", "yes"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "url",
                    value_type: ValueKind::Unknown,
                    in_sections: &["ads"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "preroll",
            value_type: ValueKind::Enum,
            in_sections: &["ads"],
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "url",
            value_type: ValueKind::Unknown,
            in_sections: &["ads"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
