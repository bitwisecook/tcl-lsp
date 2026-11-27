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

//! Properties `request-queueing` to `webp-quality` of `wam policy`, each with the properties of its sections.

use crate::bigip::{BigipPropertySpec, ValueKind};

pub(super) const PROPERTIES: &[BigipPropertySpec] = &[
    BigipPropertySpec {
        name: "request-queueing",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "response-codes-cached",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "value-case-sensitive",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "values",
        value_type: ValueKind::List,
        in_sections: &["nodes"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "variation",
        value_type: ValueKind::List,
        in_sections: &["nodes"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["nodes", "variation"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-alias",
                value_type: ValueKind::String,
                in_sections: &["nodes", "variation"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-all",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-ambiguous-as-unnamed",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-direction",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation"],
                enum_values: &["left-to-right", "right-to-left"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-name",
                value_type: ValueKind::String,
                in_sections: &["nodes", "variation"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-ordinal",
                value_type: ValueKind::Unknown,
                in_sections: &["nodes", "variation"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["nodes", "variation"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value-case-sensitive",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "values",
                value_type: ValueKind::List,
                in_sections: &["nodes", "variation"],
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
        in_sections: &["nodes", "variation"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-alias",
        value_type: ValueKind::String,
        in_sections: &["nodes", "variation"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-all",
        value_type: ValueKind::Enum,
        in_sections: &["nodes", "variation"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-ambiguous-as-unnamed",
        value_type: ValueKind::Enum,
        in_sections: &["nodes", "variation"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-direction",
        value_type: ValueKind::Enum,
        in_sections: &["nodes", "variation"],
        enum_values: &["left-to-right", "right-to-left"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-name",
        value_type: ValueKind::String,
        in_sections: &["nodes", "variation"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-ordinal",
        value_type: ValueKind::Unknown,
        in_sections: &["nodes", "variation"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "description",
        value_type: ValueKind::String,
        in_sections: &["nodes", "variation"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "value-case-sensitive",
        value_type: ValueKind::Enum,
        in_sections: &["nodes", "variation"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "values",
        value_type: ValueKind::List,
        in_sections: &["nodes", "variation"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["nodes", "variation", "values"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-as",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation", "values"],
                enum_values: &["different", "same"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-empty",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-missing",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "invert-match",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-all",
                value_type: ValueKind::Enum,
                in_sections: &["nodes", "variation", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "app-service",
        value_type: ValueKind::String,
        in_sections: &["nodes", "variation", "values"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "cache-as",
        value_type: ValueKind::Enum,
        in_sections: &["nodes", "variation", "values"],
        enum_values: &["different", "same"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "can-be-empty",
        value_type: ValueKind::Enum,
        in_sections: &["nodes", "variation", "values"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "can-be-missing",
        value_type: ValueKind::Enum,
        in_sections: &["nodes", "variation", "values"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "invert-match",
        value_type: ValueKind::Enum,
        in_sections: &["nodes", "variation", "values"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "match-all",
        value_type: ValueKind::Enum,
        in_sections: &["nodes", "variation", "values"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "video-acceleration-ad-policy",
        value_type: ValueKind::String,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "video-optimization-ad-frequency",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "video-optimization-fast-start",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "video-optimization-insert-ad",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "video-optimization-max-bitrate",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "video-optimization-preroll-ad",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["disable", "enable"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "viewstate-cache",
        value_type: ValueKind::Enum,
        in_sections: &["nodes"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "viewstate-cache-size",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "viewstate-tag",
        value_type: ValueKind::String,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "webp-quality",
        value_type: ValueKind::Integer,
        in_sections: &["nodes"],
        ..BigipPropertySpec::DEFAULT
    },
];
