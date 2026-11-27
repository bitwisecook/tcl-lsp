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

//! Properties `app-service` to `invalidations` of `wam policy`, each with the properties of its sections.

use crate::bigip::{BigipPropertySpec, ValueKind};

pub(super) const PROPERTIES: &[BigipPropertySpec] = &[
    BigipPropertySpec {
        name: "app-service",
        value_type: ValueKind::String,
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "code",
        value_type: ValueKind::Integer,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "copy-from",
        value_type: ValueKind::Reference,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "description",
        value_type: ValueKind::String,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "invalidations",
        value_type: ValueKind::List,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "active",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["invalidations"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "broadcast",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-content",
                value_type: ValueKind::List,
                in_sections: &["invalidations"],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["invalidations"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "active",
        value_type: ValueKind::Enum,
        in_sections: &["invalidations"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "app-service",
        value_type: ValueKind::String,
        in_sections: &["invalidations"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "broadcast",
        value_type: ValueKind::Enum,
        in_sections: &["invalidations"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "cache-content",
        value_type: ValueKind::List,
        in_sections: &["invalidations"],
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["invalidations", "cache-content"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-alias",
                value_type: ValueKind::String,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-direction",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations", "cache-content"],
                enum_values: &["left-to-right", "right-to-left"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-name",
                value_type: ValueKind::String,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "arg-ordinal",
                value_type: ValueKind::Unknown,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-data-alias",
                value_type: ValueKind::String,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-data-direction",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations", "cache-content"],
                enum_values: &["left-to-right", "right-to-left"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-data-name",
                value_type: ValueKind::String,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-data-ordinal",
                value_type: ValueKind::Unknown,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "request-data-type",
                value_type: ValueKind::Unknown,
                in_sections: &["invalidations", "cache-content"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "value-case-sensitive",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations", "cache-content"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "values",
                value_type: ValueKind::List,
                in_sections: &["invalidations", "cache-content"],
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
        in_sections: &["invalidations", "cache-content"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-alias",
        value_type: ValueKind::String,
        in_sections: &["invalidations", "cache-content"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-direction",
        value_type: ValueKind::Enum,
        in_sections: &["invalidations", "cache-content"],
        enum_values: &["left-to-right", "right-to-left"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-name",
        value_type: ValueKind::String,
        in_sections: &["invalidations", "cache-content"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "arg-ordinal",
        value_type: ValueKind::Unknown,
        in_sections: &["invalidations", "cache-content"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "description",
        value_type: ValueKind::String,
        in_sections: &["invalidations", "cache-content"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "request-data-alias",
        value_type: ValueKind::String,
        in_sections: &["invalidations", "cache-content"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "request-data-direction",
        value_type: ValueKind::Enum,
        in_sections: &["invalidations", "cache-content"],
        enum_values: &["left-to-right", "right-to-left"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "request-data-name",
        value_type: ValueKind::String,
        in_sections: &["invalidations", "cache-content"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "request-data-ordinal",
        value_type: ValueKind::Unknown,
        in_sections: &["invalidations", "cache-content"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "request-data-type",
        value_type: ValueKind::Unknown,
        in_sections: &["invalidations", "cache-content"],
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "value-case-sensitive",
        value_type: ValueKind::Enum,
        in_sections: &["invalidations", "cache-content"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "values",
        value_type: ValueKind::List,
        in_sections: &["invalidations", "cache-content"],
        allow_none: true,
        list_operators: &["add", "delete", "modify", "replace-all-with"],
        block: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["invalidations", "cache-content", "values"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-empty",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations", "cache-content", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "can-be-missing",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations", "cache-content", "values"],
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "invert-match",
                value_type: ValueKind::Enum,
                in_sections: &["invalidations", "cache-content", "values"],
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
        in_sections: &["invalidations", "cache-content", "values"],
        allow_none: true,
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "can-be-empty",
        value_type: ValueKind::Enum,
        in_sections: &["invalidations", "cache-content", "values"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "can-be-missing",
        value_type: ValueKind::Enum,
        in_sections: &["invalidations", "cache-content", "values"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "invert-match",
        value_type: ValueKind::Enum,
        in_sections: &["invalidations", "cache-content", "values"],
        enum_values: &["no", "yes"],
        shape_kind: Some(ValueKind::Boolean),
        ..BigipPropertySpec::DEFAULT
    },
    BigipPropertySpec {
        name: "description",
        value_type: ValueKind::String,
        in_sections: &["invalidations"],
        ..BigipPropertySpec::DEFAULT
    },
];
