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

//! BIG-IP object specs for `ltm lsn-log-profile`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_lsn_log_profile",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["lsn-log-profile"],
    },
    header_types: &[("ltm", "lsn-log-profile")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "csv-format",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "end-inbound-session",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["end-inbound-session"],
                enum_values: &["backup-allocation-only", "disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "action",
            value_type: ValueKind::Enum,
            in_sections: &["end-inbound-session"],
            enum_values: &["backup-allocation-only", "disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "end-outbound-session",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "action",
                    value_type: ValueKind::Enum,
                    in_sections: &["end-outbound-session"],
                    enum_values: &["backup-allocation-only", "disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "elements",
                    value_type: ValueKind::List,
                    in_sections: &["end-outbound-session"],
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
            in_sections: &["end-outbound-session"],
            enum_values: &["backup-allocation-only", "disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "elements",
            value_type: ValueKind::List,
            in_sections: &["end-outbound-session"],
            usage_flags: &["optional"],
            list_operators: &["add", "delete", "replace-all-with"],
            block: &[BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::Unknown,
                in_sections: &["end-outbound-session", "elements"],
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "destination",
            value_type: ValueKind::Unknown,
            in_sections: &["end-outbound-session", "elements"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "errors",
            value_type: ValueKind::List,
            block: &[BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["errors"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "action",
            value_type: ValueKind::Enum,
            in_sections: &["errors"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "quota-exceeded",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["quota-exceeded"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "action",
            value_type: ValueKind::Enum,
            in_sections: &["quota-exceeded"],
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "start-inbound-session",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["start-inbound-session"],
                enum_values: &["backup-allocation-only", "disabled", "enabled"],
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "action",
            value_type: ValueKind::Enum,
            in_sections: &["start-inbound-session"],
            enum_values: &["backup-allocation-only", "disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "start-outbound-session",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "action",
                    value_type: ValueKind::Enum,
                    in_sections: &["start-outbound-session"],
                    enum_values: &["backup-allocation-only", "disabled", "enabled"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "elements",
                    value_type: ValueKind::List,
                    in_sections: &["start-outbound-session"],
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
            in_sections: &["start-outbound-session"],
            enum_values: &["backup-allocation-only", "disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "elements",
            value_type: ValueKind::List,
            in_sections: &["start-outbound-session"],
            usage_flags: &["optional"],
            list_operators: &["add", "delete", "replace-all-with"],
            block: &[BigipPropertySpec {
                name: "destination",
                value_type: ValueKind::Unknown,
                in_sections: &["start-outbound-session", "elements"],
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "destination",
            value_type: ValueKind::Unknown,
            in_sections: &["start-outbound-session", "elements"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "subscriber-id",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
