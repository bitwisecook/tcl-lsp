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

//! BIG-IP object specs for `security dos network-whitelist`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "security_dos_network_whitelist",
        table_name: None,
        resolver_name: None,
        module: Some("security"),
        object_types: &["dos network-whitelist"],
    },
    header_types: &[("security", "dos network-whitelist")],
    properties: &[
        BigipPropertySpec {
            name: "address-list",
            value_type: ValueKind::Reference,
            references: &["net_address_list", "security_firewall_address_list"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "entries",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::String,
                    in_sections: &["entries"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "destination",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ip-protocol",
                    value_type: ValueKind::Enum,
                    in_sections: &["entries"],
                    enum_values: &["any", "icmp", "igmp", "tcp", "udp"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "match-ip-version",
                    value_type: ValueKind::Enum,
                    in_sections: &["entries"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "source",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["entries"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "destination",
            value_type: ValueKind::Unknown,
            in_sections: &["entries"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "address",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries", "destination"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "port",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries", "destination"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "address",
            value_type: ValueKind::Unknown,
            in_sections: &["entries", "destination"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port",
            value_type: ValueKind::Unknown,
            in_sections: &["entries", "destination"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-protocol",
            value_type: ValueKind::Enum,
            in_sections: &["entries"],
            enum_values: &["any", "icmp", "igmp", "tcp", "udp"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "match-ip-version",
            value_type: ValueKind::Enum,
            in_sections: &["entries"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source",
            value_type: ValueKind::Unknown,
            in_sections: &["entries"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "address",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries", "source"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "vlans",
                    value_type: ValueKind::Enum,
                    in_sections: &["entries", "source"],
                    enum_values: &["vlanid/mask"],
                    references: &[
                        "net_fdb_vlan",
                        "net_vlan",
                        "net_vlan_allowed",
                        "net_vlan_group",
                        "sys_sflow_data_source_vlan",
                        "sys_sflow_global_settings_vlan",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "address",
            value_type: ValueKind::Unknown,
            in_sections: &["entries", "source"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "vlans",
            value_type: ValueKind::Enum,
            in_sections: &["entries", "source"],
            enum_values: &["vlanid/mask"],
            references: &[
                "net_fdb_vlan",
                "net_vlan",
                "net_vlan_allowed",
                "net_vlan_group",
                "sys_sflow_data_source_vlan",
                "sys_sflow_global_settings_vlan",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "extended-entries",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "description",
                    value_type: ValueKind::String,
                    in_sections: &["extended-entries"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "destination",
                    value_type: ValueKind::Unknown,
                    in_sections: &["extended-entries"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "ip-protocol",
                    value_type: ValueKind::Enum,
                    in_sections: &["extended-entries"],
                    enum_values: &["any", "icmp", "igmp", "tcp", "udp"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "match-ip-version",
                    value_type: ValueKind::Enum,
                    in_sections: &["extended-entries"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Boolean),
                    default: Some("false"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "source",
                    value_type: ValueKind::Unknown,
                    in_sections: &["extended-entries"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            in_sections: &["extended-entries"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "destination",
            value_type: ValueKind::Unknown,
            in_sections: &["extended-entries"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "address",
                    value_type: ValueKind::Unknown,
                    in_sections: &["extended-entries", "destination"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "port",
                    value_type: ValueKind::Unknown,
                    in_sections: &["extended-entries", "destination"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "address",
            value_type: ValueKind::Unknown,
            in_sections: &["extended-entries", "destination"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port",
            value_type: ValueKind::Unknown,
            in_sections: &["extended-entries", "destination"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ip-protocol",
            value_type: ValueKind::Enum,
            in_sections: &["extended-entries"],
            enum_values: &["any", "icmp", "igmp", "tcp", "udp"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "match-ip-version",
            value_type: ValueKind::Enum,
            in_sections: &["extended-entries"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "source",
            value_type: ValueKind::Unknown,
            in_sections: &["extended-entries"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "address",
                    value_type: ValueKind::Unknown,
                    in_sections: &["extended-entries", "source"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "vlans",
                    value_type: ValueKind::Enum,
                    in_sections: &["extended-entries", "source"],
                    enum_values: &["vlanid/mask"],
                    references: &[
                        "net_fdb_vlan",
                        "net_vlan",
                        "net_vlan_allowed",
                        "net_vlan_group",
                        "sys_sflow_data_source_vlan",
                        "sys_sflow_global_settings_vlan",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "address",
            value_type: ValueKind::Unknown,
            in_sections: &["extended-entries", "source"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "vlans",
            value_type: ValueKind::Enum,
            in_sections: &["extended-entries", "source"],
            enum_values: &["vlanid/mask"],
            references: &[
                "net_fdb_vlan",
                "net_vlan",
                "net_vlan_allowed",
                "net_vlan_group",
                "sys_sflow_data_source_vlan",
                "sys_sflow_global_settings_vlan",
            ],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
