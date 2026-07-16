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

//! tcllib functional / scoping packages — `lambda`, `defer`, `tie`, and
//! `base32::core`.
//!
//! Public command surface from tcllib `modules/{lambda/lambda,defer/defer,
//! tie/tie,base32/base32core}.man`.  `lambda` gets first-class parameter-list
//! and body argument roles so its anonymous-proc bodies highlight and analyse
//! like `proc` / `apply`.  Requires Tcl 8.5+.

use crate::prelude::*;

/// The `tie::info` methods (a closed subcommand set).
const TIE_INFO_METHODS: &[ArgValue] = &[
    ArgValue {
        value: "ties",
        detail: "List the tokens tied to an array variable.",
        min_tcl: None,
    },
    ArgValue {
        value: "types",
        detail: "List the registered data-source types.",
        min_tcl: None,
    },
    ArgValue {
        value: "type",
        detail: "Return the class command for a data-source type.",
        min_tcl: None,
    },
];

/// The `lambda` package — anonymous-proc constructors.
fn lambda_specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            name: "lambda",
            arity: Arity::at_least(2),
            arg_roles: &[(0, ArgRole::ParamList), (1, ArgRole::Body)],
            body_kind: BodyKind::Structural,
            hover: Some(HoverSnippet {
                summary: "Construct an anonymous procedure (a lambda term).",
                synopsis: &["lambda arguments body ?arg ...?"],
                snippet: "",
                source: "tcllib lambda package",
                examples: "",
                return_value: "A command prefix that applies the lambda.",
            }),
            tcllib_package: Some("lambda"),
            required_package: Some("lambda"),
            ..CommandSpec::DEFAULT
        },
        CommandSpec {
            name: "lambda@",
            arity: Arity::at_least(3),
            arg_roles: &[
                (0, ArgRole::Name),
                (1, ArgRole::ParamList),
                (2, ArgRole::Body),
            ],
            body_kind: BodyKind::Structural,
            hover: Some(HoverSnippet {
                summary: "Construct an anonymous procedure that runs in a namespace.",
                synopsis: &["lambda@ namespace arguments body ?arg ...?"],
                snippet: "",
                source: "tcllib lambda package",
                examples: "",
                return_value: "A command prefix that applies the lambda.",
            }),
            tcllib_package: Some("lambda"),
            required_package: Some("lambda"),
            ..CommandSpec::DEFAULT
        },
    ]
}

/// A `defer::*` command whose trailing argument is a deferred script body.
fn defer_cmd(
    name: &'static str,
    arity: Arity,
    body_index: u8,
    synopsis: &'static [&'static str],
    summary: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        arity,
        arg_roles: match body_index {
            0 => &[(0, ArgRole::Body)],
            1 => &[(1, ArgRole::Body)],
            _ => &[],
        },
        hover: Some(HoverSnippet::brief(
            summary,
            synopsis,
            "tcllib defer package",
        )),
        tcllib_package: Some("defer"),
        required_package: Some("defer"),
        ..CommandSpec::DEFAULT
    }
}

/// The `defer` package — run scripts on scope exit.
fn defer_specs() -> Vec<CommandSpec> {
    vec![
        defer_cmd(
            "defer::defer",
            Arity::at_least(1),
            0,
            &["defer::defer script ?script ...?"],
            "Schedule a script to run when the current scope exits.",
        ),
        defer_cmd(
            "defer::with",
            Arity::at_least(2),
            1,
            &["defer::with vars script"],
            "Schedule a script capturing the named variables at scope exit.",
        ),
        defer_cmd(
            "defer::autowith",
            Arity::exact(1),
            0,
            &["defer::autowith script"],
            "Schedule a script capturing all local variables at scope exit.",
        ),
        CommandSpec {
            name: "defer::cancel",
            arity: Arity::at_least(1),
            hover: Some(HoverSnippet::brief(
                "Cancel one or more previously deferred scripts by id.",
                &["defer::cancel id ?id ...?"],
                "tcllib defer package",
            )),
            tcllib_package: Some("defer"),
            required_package: Some("defer"),
            ..CommandSpec::DEFAULT
        },
    ]
}

/// The `tie` package — tie an array variable to a persistent data source.
fn tie_specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            name: "tie::tie",
            arity: Arity::at_least(2),
            arg_roles: &[(0, ArgRole::VarWrite)],
            hover: Some(HoverSnippet {
                summary: "Tie an array variable to a persistent data source.",
                synopsis: &["tie::tie arrayvarname options... dstype dsname..."],
                snippet: "",
                source: "tcllib tie package",
                examples: "",
                return_value: "A token identifying the tie.",
            }),
            tcllib_package: Some("tie"),
            required_package: Some("tie"),
            ..CommandSpec::DEFAULT
        },
        CommandSpec {
            name: "tie::untie",
            arity: Arity::new(1, 2),
            arg_roles: &[(0, ArgRole::VarWrite)],
            hover: Some(HoverSnippet::brief(
                "Break the tie between an array variable and its data source.",
                &["tie::untie arrayvarname ?token?"],
                "tcllib tie package",
            )),
            tcllib_package: Some("tie"),
            required_package: Some("tie"),
            ..CommandSpec::DEFAULT
        },
        CommandSpec {
            name: "tie::info",
            arity: Arity::at_least(1),
            arg_values: &[(0, TIE_INFO_METHODS)],
            closed_value_args: &[0],
            hover: Some(HoverSnippet::brief(
                "Query the tie subsystem (ties / types / type).",
                &["tie::info ties arrayvarname | tie::info types | tie::info type dstype"],
                "tcllib tie package",
            )),
            tcllib_package: Some("tie"),
            required_package: Some("tie"),
            ..CommandSpec::DEFAULT
        },
        CommandSpec {
            name: "tie::register",
            arity: Arity::exact(3),
            hover: Some(HoverSnippet::brief(
                "Register a data-source class command under a type name.",
                &["tie::register dsclasscmd as dstype"],
                "tcllib tie package",
            )),
            tcllib_package: Some("tie"),
            required_package: Some("tie"),
            ..CommandSpec::DEFAULT
        },
    ]
}

/// The `base32::core` package — building blocks for base32 codecs.
fn base32_core_specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            name: "base32::core::define",
            arity: Arity::exact(4),
            arg_roles: &[
                (1, ArgRole::VarWrite),
                (2, ArgRole::VarWrite),
                (3, ArgRole::VarWrite),
            ],
            hover: Some(HoverSnippet::brief(
                "Build the forward / backward maps for a base32 variant.",
                &["base32::core::define map forwvar backwvar ivar"],
                "tcllib base32::core package",
            )),
            tcllib_package: Some("base32::core"),
            required_package: Some("base32::core"),
            ..CommandSpec::DEFAULT
        },
        CommandSpec {
            name: "base32::core::valid",
            arity: Arity::exact(3),
            arg_roles: &[(2, ArgRole::VarWrite)],
            hover: Some(HoverSnippet::brief(
                "Test whether a string is a valid base32 encoding.",
                &["base32::core::valid string pattern mvar"],
                "tcllib base32::core package",
            )),
            tcllib_package: Some("base32::core"),
            required_package: Some("base32::core"),
            ..CommandSpec::DEFAULT
        },
    ]
}

/// All `lambda` / `defer` / `tie` / `base32::core` command specs.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = lambda_specs();
    specs.extend(defer_specs());
    specs.extend(tie_specs());
    specs.extend(base32_core_specs());
    specs
}
