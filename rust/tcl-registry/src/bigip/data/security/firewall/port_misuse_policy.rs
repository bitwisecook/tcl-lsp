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

//! BIG-IP object specs for `security firewall port-misuse-policy`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "security_firewall_port_misuse_policy",
        table_name: None,
        resolver_name: None,
        module: Some("security"),
        object_types: &["firewall port-misuse-policy"],
    },
    header_types: &[("security", "firewall port-misuse-policy")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "drop-on-l7-mismatch",
            value_type: ValueKind::Enum,
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("yes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-on-l7-mismatch",
            value_type: ValueKind::Enum,
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("no"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rules",
            value_type: ValueKind::List,
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
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::String,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "drop-on-l7-mismatch",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules"],
                    enum_values: &["no", "use-policy-setting", "yes"],
                    default: Some("yes"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ip-protocol",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules"],
                    enum_values: &["sctp", "tcp", "udp"],
                    default: Some("tcp"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "l7-protocol",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "log-on-l7-mismatch",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules"],
                    enum_values: &["no", "use-policy-setting", "yes"],
                    default: Some("no"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "port",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "drop-on-l7-mismatch",
            value_type: ValueKind::Enum,
            in_sections: &["rules"],
            enum_values: &["no", "use-policy-setting", "yes"],
            default: Some("yes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-protocol",
            value_type: ValueKind::Enum,
            in_sections: &["rules"],
            enum_values: &["sctp", "tcp", "udp"],
            default: Some("tcp"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "l7-protocol",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-on-l7-mismatch",
            value_type: ValueKind::Enum,
            in_sections: &["rules"],
            enum_values: &["no", "use-policy-setting", "yes"],
            default: Some("no"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
