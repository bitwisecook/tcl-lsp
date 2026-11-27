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

//! BIG-IP object specs for `apm log-setting`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_log_setting",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["log-setting"],
    },
    header_types: &[("apm", "log-setting")],
    properties: &[
        BigipPropertySpec {
            name: "access",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "log-level",
                    value_type: ValueKind::Unknown,
                    in_sections: &["access"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "publisher",
                    value_type: ValueKind::String,
                    in_sections: &["access"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-level",
            value_type: ValueKind::Unknown,
            in_sections: &["access"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "access-control",
                    value_type: ValueKind::Enum,
                    in_sections: &["access", "log-level"],
                    enum_values: &[
                        "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "access-per-request",
                    value_type: ValueKind::Enum,
                    in_sections: &["access", "log-level"],
                    enum_values: &[
                        "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "apm-acl",
                    value_type: ValueKind::Enum,
                    in_sections: &["access", "log-level"],
                    enum_values: &[
                        "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "eca",
                    value_type: ValueKind::Enum,
                    in_sections: &["access", "log-level"],
                    enum_values: &[
                        "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "paa",
                    value_type: ValueKind::Enum,
                    in_sections: &["access", "log-level"],
                    enum_values: &[
                        "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "sso",
                    value_type: ValueKind::Enum,
                    in_sections: &["access", "log-level"],
                    enum_values: &[
                        "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "swg",
                    value_type: ValueKind::Enum,
                    in_sections: &["access", "log-level"],
                    enum_values: &[
                        "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                    ],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "access-control",
            value_type: ValueKind::Enum,
            in_sections: &["access", "log-level"],
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "access-per-request",
            value_type: ValueKind::Enum,
            in_sections: &["access", "log-level"],
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "apm-acl",
            value_type: ValueKind::Enum,
            in_sections: &["access", "log-level"],
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "eca",
            value_type: ValueKind::Enum,
            in_sections: &["access", "log-level"],
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "paa",
            value_type: ValueKind::Enum,
            in_sections: &["access", "log-level"],
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sso",
            value_type: ValueKind::Enum,
            in_sections: &["access", "log-level"],
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "swg",
            value_type: ValueKind::Enum,
            in_sections: &["access", "log-level"],
            enum_values: &[
                "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "publisher",
            value_type: ValueKind::String,
            in_sections: &["access"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "url-filters",
            value_type: ValueKind::List,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "filter",
                    value_type: ValueKind::Unknown,
                    in_sections: &["url-filters"],
                    enum_values: &["false", "true"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "publisher",
                    value_type: ValueKind::String,
                    in_sections: &["url-filters"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "filter",
            value_type: ValueKind::Unknown,
            in_sections: &["url-filters"],
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Object),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "publisher",
            value_type: ValueKind::String,
            in_sections: &["url-filters"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
