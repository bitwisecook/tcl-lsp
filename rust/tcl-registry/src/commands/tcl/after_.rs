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

//! `after` — execute a command after a time delay.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "after ms",
}];

/// Mark the first script word of `after ms ?script script ...?` as
/// [`ArgRole::Body`], so a bareword callback (`after 1000 myProc`) is
/// recursed as a real command invocation — same-file arity checking then
/// sees the `myProc` call exactly as it would inside any other script,
/// rather than treating it as an opaque, unchecked value (matching
/// `fileevent` / `chan event`'s static `(2, ArgRole::Body)` marking, and
/// `uplevel`'s dynamic resolver for the same "trailing words concatenate
/// into one script" shape). Only reached for the *default* numeric-delay
/// form: [`CommandRegistry::arg_indices_for_role`] tries subcommand
/// resolution first, so `after cancel …` / `after idle …` / `after
/// info …` never call this resolver at all.
///
/// `args[0]` is the delay (`after`'s own arity requires it); a script is
/// present only when a second word follows.
fn after_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() >= 2 {
        vec![(1, ArgRole::Body)]
    } else {
        Vec::new()
    }
}

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "cancel",
        arity: Arity::at_least(1),
        detail: "Cancel a previously scheduled delayed command.",
        synopsis: "after cancel id",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "idle",
        arity: Arity::at_least(1),
        // Only the first script word is marked, mirroring `uplevel`'s
        // resolver: idiomatic Tcl passes one braced script, and the
        // rarely-used "several words concatenate into one script" form
        // is a pre-existing convention shared with `uplevel` (see its
        // `arg_role_resolver` doc comment), not a gap introduced here.
        arg_roles: &[(0, ArgRole::Body)],
        detail: "Arrange for a script to be evaluated later as an idle callback.",
        synopsis: "after idle script ?script script ...?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::new(0, 1),
        detail: "Returns information about existing event handlers.",
        synopsis: "after info ?id?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `after`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "after",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        arg_role_resolver: Some(after_arg_roles),
        subcommands: SUBCOMMANDS,
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Execute a command after a time delay",
            synopsis: &[
                "after ms",
                "after ms ?script script script ...?",
                "after cancel id",
                "after cancel script script script ...",
            ],
            snippet: "This command is used to delay execution of the program or to execute a command in background sometime in the future.",
            source: "Tcl man page after.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
