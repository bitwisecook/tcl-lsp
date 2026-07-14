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

/// Mark the sole script word of `after ms script` as [`ArgRole::Body`], so
/// a bareword callback (`after 1000 myProc`) is recursed as a real command
/// invocation — same-file arity checking then sees the `myProc` call
/// exactly as it would inside any other script, rather than treating it as
/// an opaque, unchecked value (matching `fileevent` / `chan event`'s static
/// `(2, ArgRole::Body)` marking). Only reached for the *default*
/// numeric-delay form: [`CommandRegistry::arg_indices_for_role`] tries
/// subcommand resolution first, so `after cancel …` / `after idle …` /
/// `after info …` never call this resolver at all.
///
/// `args[0]` is the delay (`after`'s own arity requires it). Marks a script
/// word only when it is the **sole** trailing word (`args.len() == 2`):
/// `after ms script script script ...?` concatenates every trailing word
/// together (like `concat`) before evaluating the result as one script, so
/// `after 1000 {cb} 1 2` really runs `cb 1 2`, not `cb` alone — with more
/// than one trailing word, marking just the first as `Body` would recurse
/// into a truncated, wrongly-arity-checked fragment of the real script
/// (confirmed against tclsh 9.0.4: `after info` shows the registered script
/// as the space-joined concatenation). Abstain in that case rather than
/// model the concatenation — a single braced script is by far the
/// idiomatic form.
fn after_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() == 2 {
        vec![(1, ArgRole::Body)]
    } else {
        Vec::new()
    }
}

/// Same concatenation-aware guard as [`after_arg_roles`], for `after idle
/// script ?script script ...?`. `args` here is already the subcommand's
/// own slice (the word after `idle`), so the sole script word is at index
/// 0 — marked as `Body` only when it is the only trailing word.
fn after_idle_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() == 1 {
        vec![(0, ArgRole::Body)]
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
        // `Tcl_AfterObjCmd` (tclTimer.c, `AFTER_CANCEL` arm) removes the
        // scheduled timer/idle handler — the destroyed handler cannot be
        // re-cancelled, which is why `catch {after cancel $id}` is the
        // documented fire-and-forget idiom the W302 suppression keys off.
        destructive: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "idle",
        arity: Arity::at_least(1),
        // Same concatenation shape as the default `after ms script` form
        // (`after_arg_roles`) — mark the sole script word only when it is
        // the only one present, abstaining rather than mis-recursing a
        // truncated fragment when several words concatenate together.
        arg_role_resolver: Some(after_idle_arg_roles),
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
        // `HAS_DESTRUCTIVE_OPS`: the `cancel` subform destroys a scheduled
        // handler (`Tcl_AfterObjCmd`, tclTimer.c) — see the `destructive`
        // flag on the `cancel` subcommand.
        traits: Traits::BYTE_COMPILED | Traits::HAS_DESTRUCTIVE_OPS,
        arity: Arity::at_least(1),
        arg_role_resolver: Some(after_arg_roles),
        subcommands: SUBCOMMANDS,
        // `after 200 …` — an integer first word selects the default
        // delayed-execution form rather than dispatching on a subcommand.
        default_form_first_word: Some(DefaultFormFirstWord::Integer),
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
