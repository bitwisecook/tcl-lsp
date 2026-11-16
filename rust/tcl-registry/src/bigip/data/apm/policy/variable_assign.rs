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

//! BIG-IP object specs for `apm policy variable-assign`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_policy_agent_variable_assign",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["policy agent variable-assign"],
    },
    header_types: &[("apm", "policy agent variable-assign")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Enum,
            enum_values: &[
                "citrix-smart-access",
                "general",
                "intranet-webtop",
                "sso-cred-mapping",
                "virtual-keyboard",
            ],
            default: Some("general"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "variables",
            value_type: ValueKind::List,
            references: &["apm_policy_agent_variable_assign"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
