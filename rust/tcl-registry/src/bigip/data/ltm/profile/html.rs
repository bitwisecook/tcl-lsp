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

//! BIG-IP object specs for `ltm profile html`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_html",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile html"],
    },
    header_types: &[("ltm", "profile html")],
    properties: &[
        BigipPropertySpec {
            name: "content-detection",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "content-selection",
            value_type: ValueKind::Unknown,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_profile_html"],
            default: Some("html"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rules",
            value_type: ValueKind::Unknown,
            allow_none: true,
            references: &[
                "gtm_rule",
                "ltm_cipher_rule",
                "ltm_global_settings_rule",
                "ltm_rule",
                "ltm_rule_profiler",
                "security_firewall_matching_rule",
                "security_firewall_on_demand_rule_deploy",
                "security_firewall_rule_list",
                "security_firewall_rule_stat",
                "security_packet_filter_rule_stat",
                "sys_file_rewrite_rule",
            ],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
