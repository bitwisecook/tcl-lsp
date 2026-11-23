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

//! BIG-IP object specs for `net fdb`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_fdb_tunnel",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["fdb tunnel"],
        },
        header_types: &[("net", "fdb tunnel")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "records",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["records"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["records"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "endpoint",
                        value_type: ValueKind::String,
                        in_sections: &["records"],
                        shape_kind: Some(ValueKind::IpAddress),
                        address_grammar: Some(crate::bigip::AddressGrammar::Endpoint),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "endpoints",
                        value_type: ValueKind::List,
                        in_sections: &["records"],
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "replicators",
                        value_type: ValueKind::List,
                        in_sections: &["records"],
                        list_operators: &["add", "delete", "modify", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["records"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["records"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "endpoint",
                value_type: ValueKind::String,
                in_sections: &["records"],
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Endpoint),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "endpoints",
                value_type: ValueKind::List,
                in_sections: &["records"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "replicators",
                value_type: ValueKind::List,
                in_sections: &["records"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_fdb_vlan",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["fdb vlan"],
        },
        header_types: &[("net", "fdb vlan")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interface",
                value_type: ValueKind::Reference,
                references: &[
                    "net_interface",
                    "net_interface_cos",
                    "net_interface_ddm",
                    "sys_sflow_data_source_interface",
                    "sys_sflow_global_settings_interface",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "records",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "trunk",
                value_type: ValueKind::Reference,
                references: &["net_trunk"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
