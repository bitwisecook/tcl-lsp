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

//! Properties `publish-build` to `substitutions` of `wam policy`, each with the properties of its sections.

use crate::bigip::{BigipPropertySpec, ValueKind};

pub(super) const PROPERTIES: &[BigipPropertySpec] = &[
    BigipPropertySpec {
        name: "publish-build",
        value_type: ValueKind::Integer,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "publish-comment",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "published-on",
        value_type: ValueKind::Unknown,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "substitutions",
        value_type: ValueKind::List,
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["substitutions"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-alias",
                value_type: ValueKind::String,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-direction",
                value_type: ValueKind::Enum,
                in_sections: &["substitutions"],
                enum_values: &["left-to-right", "right-to-left"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-name",
                value_type: ValueKind::String,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-ordinal",
                value_type: ValueKind::Unknown,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-type",
                value_type: ValueKind::Enum,
                in_sections: &["substitutions"],
                enum_values: &["path-segment", "query-param"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dst-urls",
                value_type: ValueKind::List,
                in_sections: &["substitutions"],
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-alias",
                value_type: ValueKind::String,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-direction",
                value_type: ValueKind::Enum,
                in_sections: &["substitutions"],
                enum_values: &["left-to-right", "right-to-left"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-name",
                value_type: ValueKind::String,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-ordinal",
                value_type: ValueKind::Unknown,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-type",
                value_type: ValueKind::Unknown,
                in_sections: &["substitutions"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "src-url",
                value_type: ValueKind::Enum,
                in_sections: &["substitutions"],
                enum_values: &["absolute", "relative"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "app-service",
        value_type: ValueKind::String,
        in_sections: &["substitutions"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "description",
        value_type: ValueKind::String,
        in_sections: &["substitutions"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dst-alias",
        value_type: ValueKind::String,
        in_sections: &["substitutions"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dst-direction",
        value_type: ValueKind::Enum,
        in_sections: &["substitutions"],
        enum_values: &["left-to-right", "right-to-left"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dst-name",
        value_type: ValueKind::String,
        in_sections: &["substitutions"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dst-ordinal",
        value_type: ValueKind::Unknown,
        in_sections: &["substitutions"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dst-type",
        value_type: ValueKind::Enum,
        in_sections: &["substitutions"],
        enum_values: &["path-segment", "query-param"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "dst-urls",
        value_type: ValueKind::List,
        in_sections: &["substitutions"],
        allow_none: true,
        list_operators: &["add", "delete", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "src-alias",
        value_type: ValueKind::String,
        in_sections: &["substitutions"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "src-direction",
        value_type: ValueKind::Enum,
        in_sections: &["substitutions"],
        enum_values: &["left-to-right", "right-to-left"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "src-name",
        value_type: ValueKind::String,
        in_sections: &["substitutions"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "src-ordinal",
        value_type: ValueKind::Unknown,
        in_sections: &["substitutions"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "src-type",
        value_type: ValueKind::Unknown,
        in_sections: &["substitutions"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "src-url",
        value_type: ValueKind::Enum,
        in_sections: &["substitutions"],
        enum_values: &["absolute", "relative"],
        ..BigipPropertySpec::DEFAULT
    },
];
