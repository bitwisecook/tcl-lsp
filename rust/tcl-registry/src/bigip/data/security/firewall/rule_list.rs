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

//! BIG-IP object specs for `security firewall rule-list`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "security_firewall_rule_list",
        table_name: None,
        resolver_name: None,
        module: Some("security"),
        object_types: &["firewall rule-list"],
    },
    header_types: &[("security", "firewall rule-list")],
    properties: &[
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
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
                    name: "action",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules"],
                    enum_values: &["accept", "accept-decisively", "drop", "reject"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "classification-policy",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::String,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "destination",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "icmp",
                    value_type: ValueKind::List,
                    in_sections: &["rules"],
                    allow_none: true,
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ip-protocol",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ips-profile",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "irule",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    references: &["apm_policy_agent_irule_event", "pem_irule"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "irule-sample-rate",
                    value_type: ValueKind::Integer,
                    in_sections: &["rules"],
                    default: Some(
                        "1 and causes the iRule to be triggered for every packet that matches this firewall rule",
                    ),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "log",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules"],
                    enum_values: &["no", "yes"],
                    shape_kind: Some(ValueKind::Boolean),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "place-after",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "place-before",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "rule-list",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    references: &["security_firewall_rule_list"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "schedule",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    references: &[
                        "ltm_classification_signature_update_schedule",
                        "security_firewall_schedule",
                        "sys_url_db_download_schedule",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "service-policy",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    references: &["net_service_policy"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "source",
                    value_type: ValueKind::Unknown,
                    in_sections: &["rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "status",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules"],
                    enum_values: &["disabled", "enabled", "scheduled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "uuid",
                    value_type: ValueKind::Enum,
                    in_sections: &["rules"],
                    allow_none: true,
                    enum_values: &["auto-generate", "none"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "virtual-server",
                    value_type: ValueKind::Reference,
                    in_sections: &["rules"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "action",
            value_type: ValueKind::Enum,
            in_sections: &["rules"],
            enum_values: &["accept", "accept-decisively", "drop", "reject"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "classification-policy",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "destination",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "address-lists",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "destination"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "addresses",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "destination"],
                    allow_none: true,
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "fqdns",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "destination"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "geo",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "destination"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ipi-category",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "destination"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "port-lists",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "destination"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ports",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "destination"],
                    allow_none: true,
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "address-lists",
            value_type: ValueKind::List,
            in_sections: &["rules", "destination"],
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "addresses",
            value_type: ValueKind::List,
            in_sections: &["rules", "destination"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fqdns",
            value_type: ValueKind::List,
            in_sections: &["rules", "destination"],
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "geo",
            value_type: ValueKind::List,
            in_sections: &["rules", "destination"],
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ipi-category",
            value_type: ValueKind::List,
            in_sections: &["rules", "destination"],
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port-lists",
            value_type: ValueKind::List,
            in_sections: &["rules", "destination"],
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ports",
            value_type: ValueKind::List,
            in_sections: &["rules", "destination"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "icmp",
            value_type: ValueKind::List,
            in_sections: &["rules"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["rules", "icmp"],
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["rules", "icmp"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-protocol",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ips-profile",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "irule",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            references: &["apm_policy_agent_irule_event", "pem_irule"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "irule-sample-rate",
            value_type: ValueKind::Integer,
            in_sections: &["rules"],
            default: Some(
                "1 and causes the iRule to be triggered for every packet that matches this firewall rule",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log",
            value_type: ValueKind::Enum,
            in_sections: &["rules"],
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "place-after",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "place-before",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rule-list",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            references: &["security_firewall_rule_list"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "schedule",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            references: &[
                "ltm_classification_signature_update_schedule",
                "security_firewall_schedule",
                "sys_url_db_download_schedule",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "service-policy",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            references: &["net_service_policy"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source",
            value_type: ValueKind::Unknown,
            in_sections: &["rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "address-lists",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "source"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "addresses",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "source"],
                    allow_none: true,
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "fqdns",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "source"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "geo",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "source"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ipi-category",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "source"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "port-lists",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "source"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ports",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "source"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "vlans",
                    value_type: ValueKind::List,
                    in_sections: &["rules", "source"],
                    allow_none: true,
                    list_operators: &["add", "delete", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "address-lists",
            value_type: ValueKind::List,
            in_sections: &["rules", "source"],
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "addresses",
            value_type: ValueKind::List,
            in_sections: &["rules", "source"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fqdns",
            value_type: ValueKind::List,
            in_sections: &["rules", "source"],
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "geo",
            value_type: ValueKind::List,
            in_sections: &["rules", "source"],
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ipi-category",
            value_type: ValueKind::List,
            in_sections: &["rules", "source"],
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port-lists",
            value_type: ValueKind::List,
            in_sections: &["rules", "source"],
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ports",
            value_type: ValueKind::List,
            in_sections: &["rules", "source"],
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "vlans",
            value_type: ValueKind::List,
            in_sections: &["rules", "source"],
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "status",
            value_type: ValueKind::Enum,
            in_sections: &["rules"],
            enum_values: &["disabled", "enabled", "scheduled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "uuid",
            value_type: ValueKind::Enum,
            in_sections: &["rules"],
            allow_none: true,
            enum_values: &["auto-generate", "none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "virtual-server",
            value_type: ValueKind::Reference,
            in_sections: &["rules"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
