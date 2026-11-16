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

//! Properties `nat` of `security log`, each with the properties of its sections.

use crate::bigip::{BigipPropertySpec, ValueKind};

pub(super) const PROPERTIES: &[BigipPropertySpec] = &[
    BigipPropertySpec {
        name: "nat",
        value_type: ValueKind::Unknown,
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "end-inbound-session",
                value_type: ValueKind::Enum,
                in_sections: &["nat"],
                enum_values: &["backup-allocation-only", "disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "end-outbound-session",
                value_type: ValueKind::Unknown,
                in_sections: &["nat"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "errors",
                value_type: ValueKind::Enum,
                in_sections: &["nat"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "format",
                value_type: ValueKind::Unknown,
                in_sections: &["nat"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-publisher",
                value_type: ValueKind::Reference,
                in_sections: &["nat"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-subscriber-id",
                value_type: ValueKind::Enum,
                in_sections: &["nat"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lsn-legacy-mode",
                value_type: ValueKind::Enum,
                in_sections: &["nat"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "quota-exceeded",
                value_type: ValueKind::Enum,
                in_sections: &["nat"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rate-limit",
                value_type: ValueKind::Unknown,
                in_sections: &["nat"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "start-inbound-session",
                value_type: ValueKind::Enum,
                in_sections: &["nat"],
                enum_values: &["backup-allocation-only", "disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "start-outbound-session",
                value_type: ValueKind::Unknown,
                in_sections: &["nat"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "end-inbound-session",
        value_type: ValueKind::Enum,
        in_sections: &["nat"],
        enum_values: &["backup-allocation-only", "disabled", "enabled"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "end-outbound-session",
        value_type: ValueKind::Unknown,
        in_sections: &["nat"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["nat", "end-outbound-session"],
                enum_values: &["backup-allocation-only", "disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "elements",
                value_type: ValueKind::List,
                in_sections: &["nat", "end-outbound-session"],
                allow_none: true,
                usage_flags: &["optional"],
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "action",
        value_type: ValueKind::Enum,
        in_sections: &["nat", "end-outbound-session"],
        enum_values: &["backup-allocation-only", "disabled", "enabled"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "elements",
        value_type: ValueKind::List,
        in_sections: &["nat", "end-outbound-session"],
        allow_none: true,
        usage_flags: &["optional"],
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "errors",
        value_type: ValueKind::Enum,
        in_sections: &["nat"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "format",
        value_type: ValueKind::Unknown,
        in_sections: &["nat"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "end-inbound-session",
                value_type: ValueKind::Unknown,
                in_sections: &["nat", "format"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "end-outbound-session",
                value_type: ValueKind::Unknown,
                in_sections: &["nat", "format"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "errors",
                value_type: ValueKind::List,
                in_sections: &["nat", "format"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "quota-exceeded",
                value_type: ValueKind::Unknown,
                in_sections: &["nat", "format"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "start-inbound-session",
                value_type: ValueKind::Unknown,
                in_sections: &["nat", "format"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "start-outbound-session",
                value_type: ValueKind::Unknown,
                in_sections: &["nat", "format"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "end-inbound-session",
        value_type: ValueKind::Unknown,
        in_sections: &["nat", "format"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "field-list",
                value_type: ValueKind::Unknown,
                in_sections: &["nat", "format", "end-inbound-session"],
                allow_none: true,
                enum_values: &[
                    "dest_ip",
                    "duration",
                    "none",
                    "protocol",
                    "route_domain",
                    "src_ip",
                    "sub_id",
                    "timestamp",
                    "translated_dest_ip",
                    "translated_route_domain",
                    "translated_src_ip",
                ],
                references: &[
                    "analytics_bot_defense_event_report",
                    "apm_policy_agent_irule_event",
                    "apm_saml_auth_context_class_list",
                    "security_device_device_context",
                    "security_firewall_context_stat",
                    "sys_icall_event",
                ],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-list-delimiter",
                value_type: ValueKind::String,
                in_sections: &["nat", "format", "end-inbound-session"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["nat", "format", "end-inbound-session"],
                allow_none: true,
                enum_values: &["field-list", "none", "user-defined"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-defined",
                value_type: ValueKind::String,
                in_sections: &["nat", "format", "end-inbound-session"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list",
        value_type: ValueKind::Unknown,
        in_sections: &["nat", "format", "end-inbound-session"],
        allow_none: true,
        enum_values: &[
            "dest_ip",
            "duration",
            "none",
            "protocol",
            "route_domain",
            "src_ip",
            "sub_id",
            "timestamp",
            "translated_dest_ip",
            "translated_route_domain",
            "translated_src_ip",
        ],
        references: &[
            "analytics_bot_defense_event_report",
            "apm_policy_agent_irule_event",
            "apm_saml_auth_context_class_list",
            "security_device_device_context",
            "security_firewall_context_stat",
            "sys_icall_event",
        ],
        shape_kind: Some(ValueKind::Object),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list-delimiter",
        value_type: ValueKind::String,
        in_sections: &["nat", "format", "end-inbound-session"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        in_sections: &["nat", "format", "end-inbound-session"],
        allow_none: true,
        enum_values: &["field-list", "none", "user-defined"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-defined",
        value_type: ValueKind::String,
        in_sections: &["nat", "format", "end-inbound-session"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "end-outbound-session",
        value_type: ValueKind::Unknown,
        in_sections: &["nat", "format"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "field-list",
                value_type: ValueKind::Unknown,
                in_sections: &["nat", "format", "end-outbound-session"],
                allow_none: true,
                enum_values: &[
                    "dest_ip",
                    "duration",
                    "none",
                    "protocol",
                    "route_domain",
                    "src_ip",
                    "sub_id",
                    "timestamp",
                    "translated_dest_ip",
                    "translated_route_domain",
                    "translated_src_ip",
                ],
                references: &[
                    "analytics_bot_defense_event_report",
                    "apm_policy_agent_irule_event",
                    "apm_saml_auth_context_class_list",
                    "security_device_device_context",
                    "security_firewall_context_stat",
                    "sys_icall_event",
                ],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-list-delimiter",
                value_type: ValueKind::String,
                in_sections: &["nat", "format", "end-outbound-session"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["nat", "format", "end-outbound-session"],
                allow_none: true,
                enum_values: &["field-list", "none", "user-defined"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-defined",
                value_type: ValueKind::String,
                in_sections: &["nat", "format", "end-outbound-session"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list",
        value_type: ValueKind::Unknown,
        in_sections: &["nat", "format", "end-outbound-session"],
        allow_none: true,
        enum_values: &[
            "dest_ip",
            "duration",
            "none",
            "protocol",
            "route_domain",
            "src_ip",
            "sub_id",
            "timestamp",
            "translated_dest_ip",
            "translated_route_domain",
            "translated_src_ip",
        ],
        references: &[
            "analytics_bot_defense_event_report",
            "apm_policy_agent_irule_event",
            "apm_saml_auth_context_class_list",
            "security_device_device_context",
            "security_firewall_context_stat",
            "sys_icall_event",
        ],
        shape_kind: Some(ValueKind::Object),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list-delimiter",
        value_type: ValueKind::String,
        in_sections: &["nat", "format", "end-outbound-session"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        in_sections: &["nat", "format", "end-outbound-session"],
        allow_none: true,
        enum_values: &["field-list", "none", "user-defined"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-defined",
        value_type: ValueKind::String,
        in_sections: &["nat", "format", "end-outbound-session"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "errors",
        value_type: ValueKind::List,
        in_sections: &["nat", "format"],
        block: &[
            BigipPropertySpec {
                name: "field-list",
                value_type: ValueKind::Unknown,
                in_sections: &["nat", "format", "errors"],
                allow_none: true,
                enum_values: &[
                    "dest_ip",
                    "duration",
                    "none",
                    "protocol",
                    "route_domain",
                    "src_ip",
                    "sub_id",
                    "timestamp",
                    "translated_dest_ip",
                    "translated_route_domain",
                    "translated_src_ip",
                ],
                references: &[
                    "analytics_bot_defense_event_report",
                    "apm_policy_agent_irule_event",
                    "apm_saml_auth_context_class_list",
                    "security_device_device_context",
                    "security_firewall_context_stat",
                    "sys_icall_event",
                ],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-list-delimiter",
                value_type: ValueKind::String,
                in_sections: &["nat", "format", "errors"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["nat", "format", "errors"],
                allow_none: true,
                enum_values: &["field-list", "none", "user-defined"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-defined",
                value_type: ValueKind::String,
                in_sections: &["nat", "format", "errors"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list",
        value_type: ValueKind::Unknown,
        in_sections: &["nat", "format", "errors"],
        allow_none: true,
        enum_values: &[
            "dest_ip",
            "duration",
            "none",
            "protocol",
            "route_domain",
            "src_ip",
            "sub_id",
            "timestamp",
            "translated_dest_ip",
            "translated_route_domain",
            "translated_src_ip",
        ],
        references: &[
            "analytics_bot_defense_event_report",
            "apm_policy_agent_irule_event",
            "apm_saml_auth_context_class_list",
            "security_device_device_context",
            "security_firewall_context_stat",
            "sys_icall_event",
        ],
        shape_kind: Some(ValueKind::Object),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list-delimiter",
        value_type: ValueKind::String,
        in_sections: &["nat", "format", "errors"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        in_sections: &["nat", "format", "errors"],
        allow_none: true,
        enum_values: &["field-list", "none", "user-defined"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-defined",
        value_type: ValueKind::String,
        in_sections: &["nat", "format", "errors"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "quota-exceeded",
        value_type: ValueKind::Unknown,
        in_sections: &["nat", "format"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "field-list",
                value_type: ValueKind::Unknown,
                in_sections: &["nat", "format", "quota-exceeded"],
                allow_none: true,
                enum_values: &[
                    "dest_ip",
                    "duration",
                    "none",
                    "protocol",
                    "route_domain",
                    "src_ip",
                    "sub_id",
                    "timestamp",
                    "translated_dest_ip",
                    "translated_route_domain",
                    "translated_src_ip",
                ],
                references: &[
                    "analytics_bot_defense_event_report",
                    "apm_policy_agent_irule_event",
                    "apm_saml_auth_context_class_list",
                    "security_device_device_context",
                    "security_firewall_context_stat",
                    "sys_icall_event",
                ],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-list-delimiter",
                value_type: ValueKind::String,
                in_sections: &["nat", "format", "quota-exceeded"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["nat", "format", "quota-exceeded"],
                allow_none: true,
                enum_values: &["field-list", "none", "user-defined"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-defined",
                value_type: ValueKind::String,
                in_sections: &["nat", "format", "quota-exceeded"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list",
        value_type: ValueKind::Unknown,
        in_sections: &["nat", "format", "quota-exceeded"],
        allow_none: true,
        enum_values: &[
            "dest_ip",
            "duration",
            "none",
            "protocol",
            "route_domain",
            "src_ip",
            "sub_id",
            "timestamp",
            "translated_dest_ip",
            "translated_route_domain",
            "translated_src_ip",
        ],
        references: &[
            "analytics_bot_defense_event_report",
            "apm_policy_agent_irule_event",
            "apm_saml_auth_context_class_list",
            "security_device_device_context",
            "security_firewall_context_stat",
            "sys_icall_event",
        ],
        shape_kind: Some(ValueKind::Object),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list-delimiter",
        value_type: ValueKind::String,
        in_sections: &["nat", "format", "quota-exceeded"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        in_sections: &["nat", "format", "quota-exceeded"],
        allow_none: true,
        enum_values: &["field-list", "none", "user-defined"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-defined",
        value_type: ValueKind::String,
        in_sections: &["nat", "format", "quota-exceeded"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "start-inbound-session",
        value_type: ValueKind::Unknown,
        in_sections: &["nat", "format"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "field-list",
                value_type: ValueKind::Unknown,
                in_sections: &["nat", "format", "start-inbound-session"],
                allow_none: true,
                enum_values: &[
                    "dest_ip",
                    "duration",
                    "none",
                    "protocol",
                    "route_domain",
                    "src_ip",
                    "sub_id",
                    "timestamp",
                    "translated_dest_ip",
                    "translated_route_domain",
                    "translated_src_ip",
                ],
                references: &[
                    "analytics_bot_defense_event_report",
                    "apm_policy_agent_irule_event",
                    "apm_saml_auth_context_class_list",
                    "security_device_device_context",
                    "security_firewall_context_stat",
                    "sys_icall_event",
                ],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-list-delimiter",
                value_type: ValueKind::String,
                in_sections: &["nat", "format", "start-inbound-session"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["nat", "format", "start-inbound-session"],
                allow_none: true,
                enum_values: &["field-list", "none", "user-defined"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-defined",
                value_type: ValueKind::String,
                in_sections: &["nat", "format", "start-inbound-session"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list",
        value_type: ValueKind::Unknown,
        in_sections: &["nat", "format", "start-inbound-session"],
        allow_none: true,
        enum_values: &[
            "dest_ip",
            "duration",
            "none",
            "protocol",
            "route_domain",
            "src_ip",
            "sub_id",
            "timestamp",
            "translated_dest_ip",
            "translated_route_domain",
            "translated_src_ip",
        ],
        references: &[
            "analytics_bot_defense_event_report",
            "apm_policy_agent_irule_event",
            "apm_saml_auth_context_class_list",
            "security_device_device_context",
            "security_firewall_context_stat",
            "sys_icall_event",
        ],
        shape_kind: Some(ValueKind::Object),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list-delimiter",
        value_type: ValueKind::String,
        in_sections: &["nat", "format", "start-inbound-session"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        in_sections: &["nat", "format", "start-inbound-session"],
        allow_none: true,
        enum_values: &["field-list", "none", "user-defined"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-defined",
        value_type: ValueKind::String,
        in_sections: &["nat", "format", "start-inbound-session"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "start-outbound-session",
        value_type: ValueKind::Unknown,
        in_sections: &["nat", "format"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "field-list",
                value_type: ValueKind::Unknown,
                in_sections: &["nat", "format", "start-outbound-session"],
                allow_none: true,
                enum_values: &[
                    "dest_ip",
                    "duration",
                    "none",
                    "protocol",
                    "route_domain",
                    "src_ip",
                    "sub_id",
                    "timestamp",
                    "translated_dest_ip",
                    "translated_route_domain",
                    "translated_src_ip",
                ],
                references: &[
                    "analytics_bot_defense_event_report",
                    "apm_policy_agent_irule_event",
                    "apm_saml_auth_context_class_list",
                    "security_device_device_context",
                    "security_firewall_context_stat",
                    "sys_icall_event",
                ],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "field-list-delimiter",
                value_type: ValueKind::String,
                in_sections: &["nat", "format", "start-outbound-session"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Enum,
                in_sections: &["nat", "format", "start-outbound-session"],
                allow_none: true,
                enum_values: &["field-list", "none", "user-defined"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user-defined",
                value_type: ValueKind::String,
                in_sections: &["nat", "format", "start-outbound-session"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list",
        value_type: ValueKind::Unknown,
        in_sections: &["nat", "format", "start-outbound-session"],
        allow_none: true,
        enum_values: &[
            "dest_ip",
            "duration",
            "none",
            "protocol",
            "route_domain",
            "src_ip",
            "sub_id",
            "timestamp",
            "translated_dest_ip",
            "translated_route_domain",
            "translated_src_ip",
        ],
        references: &[
            "analytics_bot_defense_event_report",
            "apm_policy_agent_irule_event",
            "apm_saml_auth_context_class_list",
            "security_device_device_context",
            "security_firewall_context_stat",
            "sys_icall_event",
        ],
        shape_kind: Some(ValueKind::Object),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "field-list-delimiter",
        value_type: ValueKind::String,
        in_sections: &["nat", "format", "start-outbound-session"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "type",
        value_type: ValueKind::Enum,
        in_sections: &["nat", "format", "start-outbound-session"],
        allow_none: true,
        enum_values: &["field-list", "none", "user-defined"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "user-defined",
        value_type: ValueKind::String,
        in_sections: &["nat", "format", "start-outbound-session"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-publisher",
        value_type: ValueKind::Reference,
        in_sections: &["nat"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "log-subscriber-id",
        value_type: ValueKind::Enum,
        in_sections: &["nat"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "lsn-legacy-mode",
        value_type: ValueKind::Enum,
        in_sections: &["nat"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "quota-exceeded",
        value_type: ValueKind::Enum,
        in_sections: &["nat"],
        enum_values: &["disabled", "enabled"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "rate-limit",
        value_type: ValueKind::Unknown,
        in_sections: &["nat"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "aggregate-rate",
                value_type: ValueKind::Integer,
                in_sections: &["nat", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "end-inbound-session",
                value_type: ValueKind::Integer,
                in_sections: &["nat", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "end-outbound-session",
                value_type: ValueKind::Integer,
                in_sections: &["nat", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "errors",
                value_type: ValueKind::Integer,
                in_sections: &["nat", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "quota-exceeded",
                value_type: ValueKind::Integer,
                in_sections: &["nat", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "start-inbound-session",
                value_type: ValueKind::Integer,
                in_sections: &["nat", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "start-outbound-session",
                value_type: ValueKind::Integer,
                in_sections: &["nat", "rate-limit"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "aggregate-rate",
        value_type: ValueKind::Integer,
        in_sections: &["nat", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "end-inbound-session",
        value_type: ValueKind::Integer,
        in_sections: &["nat", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "end-outbound-session",
        value_type: ValueKind::Integer,
        in_sections: &["nat", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "errors",
        value_type: ValueKind::Integer,
        in_sections: &["nat", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "quota-exceeded",
        value_type: ValueKind::Integer,
        in_sections: &["nat", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "start-inbound-session",
        value_type: ValueKind::Integer,
        in_sections: &["nat", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "start-outbound-session",
        value_type: ValueKind::Integer,
        in_sections: &["nat", "rate-limit"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "start-inbound-session",
        value_type: ValueKind::Enum,
        in_sections: &["nat"],
        enum_values: &["backup-allocation-only", "disabled", "enabled"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "start-outbound-session",
        value_type: ValueKind::Unknown,
        in_sections: &["nat"],
        shape_kind: Some(ValueKind::Object),
        block: &[
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["nat", "start-outbound-session"],
                enum_values: &["backup-allocation-only", "disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "elements",
                value_type: ValueKind::List,
                in_sections: &["nat", "start-outbound-session"],
                allow_none: true,
                usage_flags: &["optional"],
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "action",
        value_type: ValueKind::Enum,
        in_sections: &["nat", "start-outbound-session"],
        enum_values: &["backup-allocation-only", "disabled", "enabled"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "elements",
        value_type: ValueKind::List,
        in_sections: &["nat", "start-outbound-session"],
        allow_none: true,
        usage_flags: &["optional"],
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
];
