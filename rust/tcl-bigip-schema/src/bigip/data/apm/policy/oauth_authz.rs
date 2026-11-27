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

//! BIG-IP object specs for `apm policy oauth-authz`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_policy_agent_oauth_authz",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["policy agent oauth-authz"],
    },
    header_types: &[("apm", "policy agent oauth-authz")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "audience",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "customization-group",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "entries",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["entries"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "expression",
                    value_type: ValueKind::String,
                    in_sections: &["entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "id-token-claim-entries",
                    value_type: ValueKind::List,
                    in_sections: &["entries"],
                    allow_none: true,
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "jwt-access-token-claim-entries",
                    value_type: ValueKind::List,
                    in_sections: &["entries"],
                    allow_none: true,
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scope-entries",
                    value_type: ValueKind::List,
                    in_sections: &["entries"],
                    allow_none: true,
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "userinfo-claim-entries",
                    value_type: ValueKind::List,
                    in_sections: &["entries"],
                    allow_none: true,
                    list_operators: &["add", "delete", "modify", "replace-all-with"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["entries"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "expression",
            value_type: ValueKind::String,
            in_sections: &["entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "id-token-claim-entries",
            value_type: ValueKind::List,
            in_sections: &["entries"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["entries", "id-token-claim-entries"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "claim-name",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries", "id-token-claim-entries"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "claim-value",
                    value_type: ValueKind::String,
                    in_sections: &["entries", "id-token-claim-entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["entries", "id-token-claim-entries"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "claim-name",
            value_type: ValueKind::Unknown,
            in_sections: &["entries", "id-token-claim-entries"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "claim-value",
            value_type: ValueKind::String,
            in_sections: &["entries", "id-token-claim-entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "jwt-access-token-claim-entries",
            value_type: ValueKind::List,
            in_sections: &["entries"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["entries", "jwt-access-token-claim-entries"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "claim-name",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries", "jwt-access-token-claim-entries"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "claim-value",
                    value_type: ValueKind::String,
                    in_sections: &["entries", "jwt-access-token-claim-entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["entries", "jwt-access-token-claim-entries"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "claim-name",
            value_type: ValueKind::Unknown,
            in_sections: &["entries", "jwt-access-token-claim-entries"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "claim-value",
            value_type: ValueKind::String,
            in_sections: &["entries", "jwt-access-token-claim-entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scope-entries",
            value_type: ValueKind::List,
            in_sections: &["entries"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["entries", "scope-entries"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scope-name",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries", "scope-entries"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scope-value",
                    value_type: ValueKind::String,
                    in_sections: &["entries", "scope-entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["entries", "scope-entries"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scope-name",
            value_type: ValueKind::Unknown,
            in_sections: &["entries", "scope-entries"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scope-value",
            value_type: ValueKind::String,
            in_sections: &["entries", "scope-entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "userinfo-claim-entries",
            value_type: ValueKind::List,
            in_sections: &["entries"],
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "app-service",
                    value_type: ValueKind::String,
                    in_sections: &["entries", "userinfo-claim-entries"],
                    allow_none: true,
                    default: Some("none"),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "claim-name",
                    value_type: ValueKind::Unknown,
                    in_sections: &["entries", "userinfo-claim-entries"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "claim-value",
                    value_type: ValueKind::String,
                    in_sections: &["entries", "userinfo-claim-entries"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            in_sections: &["entries", "userinfo-claim-entries"],
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "claim-name",
            value_type: ValueKind::Unknown,
            in_sections: &["entries", "userinfo-claim-entries"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "claim-value",
            value_type: ValueKind::String,
            in_sections: &["entries", "userinfo-claim-entries"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "options",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "prompt-for-authorization",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "subject",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
