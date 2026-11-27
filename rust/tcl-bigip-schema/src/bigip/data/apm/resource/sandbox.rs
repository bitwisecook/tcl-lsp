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

//! BIG-IP object specs for `apm resource sandbox`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_resource_sandbox",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["resource sandbox"],
    },
    header_types: &[("apm", "resource sandbox")],
    properties: &[
        BigipPropertySpec {
            name: "base-uri",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "files",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "content-type",
                    value_type: ValueKind::String,
                    in_sections: &["files"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "file-type",
                    value_type: ValueKind::Enum,
                    in_sections: &["files"],
                    required: true,
                    enum_values: &["citrix-bundle", "customization", "unknown"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "filename",
                    value_type: ValueKind::String,
                    in_sections: &["files"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "folder",
                    value_type: ValueKind::String,
                    in_sections: &["files"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "local-path",
                    value_type: ValueKind::String,
                    in_sections: &["files"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "content-type",
            value_type: ValueKind::String,
            in_sections: &["files"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "file-type",
            value_type: ValueKind::Enum,
            in_sections: &["files"],
            required: true,
            enum_values: &["citrix-bundle", "customization", "unknown"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "filename",
            value_type: ValueKind::String,
            in_sections: &["files"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "folder",
            value_type: ValueKind::String,
            in_sections: &["files"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "local-path",
            value_type: ValueKind::String,
            in_sections: &["files"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "options",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
