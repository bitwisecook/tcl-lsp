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

//! BIG-IP object specs for `apm aaa oauth-server`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_aaa_oauth_server",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["aaa oauth-server"],
    },
    header_types: &[("apm", "aaa oauth-server")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-id",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-jwe-key",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-secret",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-serverssl-profile-name",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dns-resolver-name",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mode",
            value_type: ValueKind::Enum,
            enum_values: &["client", "client-rs", "rs"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "provider-name",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "resource-server-id",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "resource-server-secret",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "resource-serverssl-profile-name",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rules",
            value_type: ValueKind::String,
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
        BigipPropertySpec {
            name: "token-validation-interval",
            value_type: ValueKind::Integer,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
