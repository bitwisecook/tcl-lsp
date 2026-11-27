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

//! BIG-IP object specs for `apm policy http-header-modify`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_policy_agent_http_header_modify",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["policy agent http-header-modify"],
    },
    header_types: &[("apm", "policy agent http-header-modify")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cookie-entries",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["cookie-entries"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "cookie-name",
                    value_type: ValueKind::String,
                    in_sections: &["cookie-entries"],
                    required: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "cookie-operation",
                    value_type: ValueKind::Enum,
                    in_sections: &["cookie-entries"],
                    enum_values: &["cookie-delete", "cookie-update"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "cookie-value",
                    value_type: ValueKind::String,
                    in_sections: &["cookie-entries"],
                    required: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["cookie-entries"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cookie-name",
            value_type: ValueKind::String,
            in_sections: &["cookie-entries"],
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cookie-operation",
            value_type: ValueKind::Enum,
            in_sections: &["cookie-entries"],
            enum_values: &["cookie-delete", "cookie-update"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "cookie-value",
            value_type: ValueKind::String,
            in_sections: &["cookie-entries"],
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "header-entries",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["header-entries"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "header-delimiter",
                    value_type: ValueKind::String,
                    in_sections: &["header-entries"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "header-name",
                    value_type: ValueKind::String,
                    in_sections: &["header-entries"],
                    required: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "header-operation",
                    value_type: ValueKind::Enum,
                    in_sections: &["header-entries"],
                    enum_values: &[
                        "header-append",
                        "header-insert",
                        "header-remove",
                        "header-replace",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "header-value",
                    value_type: ValueKind::String,
                    in_sections: &["header-entries"],
                    required: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["header-entries"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "header-delimiter",
            value_type: ValueKind::String,
            in_sections: &["header-entries"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "header-name",
            value_type: ValueKind::String,
            in_sections: &["header-entries"],
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "header-operation",
            value_type: ValueKind::Enum,
            in_sections: &["header-entries"],
            enum_values: &[
                "header-append",
                "header-insert",
                "header-remove",
                "header-replace",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "header-value",
            value_type: ValueKind::String,
            in_sections: &["header-entries"],
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
