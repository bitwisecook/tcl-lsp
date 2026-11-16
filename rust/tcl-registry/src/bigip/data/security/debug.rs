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

//! BIG-IP object specs for `security debug`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_debug_matcher",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["debug matcher"],
        },
        header_types: &[("security", "debug matcher")],
        properties: &[
            BigipPropertySpec {
                name: "matcher",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[BigipPropertySpec {
                    name: "drop-redirect",
                    value_type: ValueKind::Unknown,
                    in_sections: &["matcher"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "drop-redirect",
                value_type: ValueKind::Unknown,
                in_sections: &["matcher"],
                shape_kind: Some(ValueKind::Object),
                block: &[BigipPropertySpec {
                    name: "drop-redirect-mode",
                    value_type: ValueKind::Unknown,
                    in_sections: &["matcher", "drop-redirect"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "drop-redirect-mode",
                value_type: ValueKind::Unknown,
                in_sections: &["matcher", "drop-redirect"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "disable",
                        value_type: ValueKind::Unknown,
                        in_sections: &["matcher", "drop-redirect", "drop-redirect-mode"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "redirect-all",
                        value_type: ValueKind::Unknown,
                        in_sections: &["matcher", "drop-redirect", "drop-redirect-mode"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "redirect-hw-only",
                        value_type: ValueKind::Unknown,
                        in_sections: &["matcher", "drop-redirect", "drop-redirect-mode"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "redirect-sw-only",
                        value_type: ValueKind::Unknown,
                        in_sections: &["matcher", "drop-redirect", "drop-redirect-mode"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "disable",
                value_type: ValueKind::Unknown,
                in_sections: &["matcher", "drop-redirect", "drop-redirect-mode"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirect-all",
                value_type: ValueKind::Unknown,
                in_sections: &["matcher", "drop-redirect", "drop-redirect-mode"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirect-hw-only",
                value_type: ValueKind::Unknown,
                in_sections: &["matcher", "drop-redirect", "drop-redirect-mode"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "redirect-sw-only",
                value_type: ValueKind::Unknown,
                in_sections: &["matcher", "drop-redirect", "drop-redirect-mode"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_debug_register",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["debug register"],
        },
        header_types: &[("security", "debug register")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "address",
                        value_type: ValueKind::Unknown,
                        in_sections: &["destination"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "port",
                        value_type: ValueKind::Unknown,
                        in_sections: &["destination"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "address",
                value_type: ValueKind::Unknown,
                in_sections: &["destination"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Unknown,
                in_sections: &["destination"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-ip-version",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("false"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Unknown,
                default: Some("any"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "address",
                        value_type: ValueKind::Unknown,
                        in_sections: &["source"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "port",
                        value_type: ValueKind::Unknown,
                        in_sections: &["source"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "vlan",
                        value_type: ValueKind::Reference,
                        in_sections: &["source"],
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
                in_sections: &["source"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Unknown,
                in_sections: &["source"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "vlan",
                value_type: ValueKind::Reference,
                in_sections: &["source"],
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
    },
];
