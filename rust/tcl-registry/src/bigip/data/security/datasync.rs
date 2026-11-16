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

//! BIG-IP object specs for `security datasync`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_datasync_background_tasks",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["datasync background-tasks"],
        },
        header_types: &[("security", "datasync background-tasks")],
        properties: &[BigipPropertySpec {
            name: "daily-start-time",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_datasync_global_profile",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["datasync global-profile"],
        },
        header_types: &[("security", "datasync global-profile")],
        properties: &[
            BigipPropertySpec {
                name: "activation-epoch",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "deactivation-epoch",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "grace-time",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hash-alg",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mac-alg",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "master-key",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-rows",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-rows",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode-of-op",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "params",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "regen-interval",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "regen-time-offset",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rsa-bits",
                value_type: ValueKind::Integer,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rsa-exp",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["default", "none", "rsa-3", "rsa-f4"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scramble-alg",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "table",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_datasync_local_profile",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["datasync local-profile"],
        },
        header_types: &[("security", "datasync local-profile")],
        properties: &[
            BigipPropertySpec {
                name: "buf-size",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ds-area",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["asm", "fps", "none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gen-pause-sec",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gen-timeout-sec",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "keep-conf-files",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-gen-rows",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-iowait-percent",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-cpu-percent",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "min-mem-mb",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "offline-until-gen",
                value_type: ValueKind::Enum,
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rows-bulk",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
