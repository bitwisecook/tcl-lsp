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

//! `console` — control the Tk debugging console window — and
//! `consoleinterp`, the command the console interpreter uses to reach back
//! into the interpreter it is attached to.  Both are missing from
//! `Tcl_EvalObjEx`-style single-body forms without a `SubCommand` table, so
//! `console eval script` / `consoleinterp eval script` /
//! `consoleinterp record script` previously had no declared `ArgRole::Body`
//! argument and the semantic-token walker left `script` as an opaque string
//! instead of recursing into it (issue #925).
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

static CONSOLE_SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "eval",
        arity: Arity::exact(1),
        detail: "Evaluate a script in the console's own interpreter.",
        synopsis: "console eval script",
        arg_roles: &[(0, ArgRole::Body)],
        // Runs an arbitrary script in the console interpreter — dynamic-dispatch
        // consumers (memory-SSA clobber classification, side-effect analysis)
        // key off this, as with `interp eval`.
        traits: Traits::EVALUATES_CODE,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "hide",
        arity: Arity::exact(0),
        detail: "Hide (unmap) the console window.",
        synopsis: "console hide",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "show",
        arity: Arity::exact(0),
        detail: "Show (map and raise) the console window, creating it if it does not yet exist.",
        synopsis: "console show",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "title",
        arity: Arity::new(0, 1),
        detail: "Query or set the console window's title.",
        synopsis: "console title ?newTitle?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
];

const CONSOLE_FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "console subcommand ?arg ...?",
    ..FormSpec::DEFAULT
}];

/// `console` — control the Tk debugging console window.
pub fn console_spec() -> CommandSpec {
    CommandSpec {
        name: "console",
        surface: Some(SpecSurface::TK_AND_TCL),
        traits: Traits::DYNAMIC_EVAL_BODY,
        arity: Arity::at_least(1),
        subcommands: CONSOLE_SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Control the debugging console window.",
            synopsis: &[
                "console eval script",
                "console hide",
                "console show",
                "console title ?newTitle?",
            ],
            snippet: "Manages the Tk debugging console, a text-widget UI backed by its own Tcl interpreter distinct from the caller's. `console eval` runs script in that console interpreter, not the caller's.",
            source: "Tk man page console.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        // `console eval` crosses into the separate console interpreter —
        // cross-interpreter code injection (T105), the same category as
        // `interp eval`.
        taint_interp_eval_subcommands: &["eval"],
        forms: CONSOLE_FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}

static CONSOLEINTERP_SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "eval",
        arity: Arity::exact(1),
        detail: "Evaluate a script in the interpreter the console is attached to.",
        synopsis: "consoleinterp eval script",
        arg_roles: &[(0, ArgRole::Body)],
        traits: Traits::EVALUATES_CODE,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "record",
        arity: Arity::exact(1),
        detail: "Evaluate a script in the attached interpreter and record it in the console's command history.",
        synopsis: "consoleinterp record script",
        arg_roles: &[(0, ArgRole::Body)],
        traits: Traits::EVALUATES_CODE,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
];

const CONSOLEINTERP_FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "consoleinterp subcommand script",
    ..FormSpec::DEFAULT
}];

/// `consoleinterp` — available inside the console interpreter created by
/// `console eval`/`console show`; evaluates or records a script in the
/// interpreter the console is attached to (typically the main application
/// interpreter). See `library/console.tcl` (`EvalAttached`, `ConsoleInvoke`)
/// and `generic/tkConsole.c` (`InterpreterObjCmd`) in the Tk source.
pub fn consoleinterp_spec() -> CommandSpec {
    CommandSpec {
        name: "consoleinterp",
        surface: Some(SpecSurface::TK_AND_TCL),
        traits: Traits::DYNAMIC_EVAL_BODY,
        arity: Arity::at_least(1),
        subcommands: CONSOLEINTERP_SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Evaluate or record a script in the console's attached interpreter.",
            synopsis: &["consoleinterp eval script", "consoleinterp record script"],
            snippet: "Available inside the console interpreter. Both subcommands run script in the interpreter the console is attached to; `record` additionally adds it to the console's command history.",
            source: "Tk source tkConsole.c (InterpreterObjCmd)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        // Both subcommands cross back into the attached interpreter —
        // cross-interpreter code injection (T105).
        taint_interp_eval_subcommands: &["eval", "record"],
        forms: CONSOLEINTERP_FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}

/// All `console`/`consoleinterp` command specs.
pub fn specs() -> Vec<CommandSpec> {
    vec![console_spec(), consoleinterp_spec()]
}
