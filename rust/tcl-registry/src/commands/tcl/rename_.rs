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

//! `rename` — rename or delete a command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::ProcDefinition,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "rename oldName newName",
    dialects: None,
}];

/// Command spec for `rename`.
///
/// Manpage comparison across Tcl 8.4, 8.5, 8.6, 9.0, and 9.1
/// (tcl-lang.org's TclCmd/rename.html, `.htm` for the 8.6 tree): the NAME,
/// SYNOPSIS, DESCRIPTION, EXAMPLE, SEE ALSO, and KEYWORDS sections are
/// byte-for-byte identical across all five — `rename oldName newName` has
/// never gained an option, a third argument, or a changed default in any
/// release; the only cross-version differences in the raw HTML are cosmetic
/// (navigation chrome, doctype, copyright-block ordering).
///
/// `generic/tclCmdMZ.c`'s `Tcl_RenameObjCmd` (dispatching to
/// `TclRenameCommand` in `generic/tclBasic.c`) is likewise unchanged in
/// substance across the five fetched source trees (`core-8-4-20`,
/// `core-8-5-19`, `core-8-6-16`, `core-9-0-4`, `core-9-1-b0`): `objc != 3`
/// is a hard "wrong # args" in every release (arity is exactly 2), and the
/// three error strings are word-for-word identical in all five — `can't
/// {rename|delete} "oldName": command doesn't exist` when `oldName` isn't a
/// real command, `can't rename to "newName": bad command name` when
/// `newName`'s namespace can't be resolved, and `can't rename to "newName":
/// command already exists` when `newName` already denotes one (8.6, 9.0,
/// and 9.1 additionally attach a structured `-errorcode` to each of the
/// three — `TCL LOOKUP COMMAND oldName`, `TCL VALUE COMMAND`, and `TCL
/// OPERATION RENAME TARGET_EXISTS` respectively. `TclRenameCommand`'s
/// three `Tcl_SetErrorCode` calls first appear in the 8.6 source and are
/// unchanged in 9.0/9.1; 8.4's and 8.5's `TclRenameCommand` call
/// `Tcl_SetErrorCode` on none of the three paths, so `errorCode` stays
/// `NONE` there. The message text a user sees is the same in all five
/// regardless). A rename also
/// fires any `trace add command … rename` handler on `oldName`, and a
/// deleting `rename oldName {}` fires its `… delete` handler instead —
/// command traces (TIP 110) have been present since 8.4, so this is not
/// version-gated either.
///
/// `dialects: ALL_TCL` (no `IRULES` bit) here is deliberate, not an
/// oversight: F5 iRules is the one modelled dialect that drops `rename` —
/// it is one of the K36322151 commands F5 bans from direct command-table
/// surgery in the TMM event sandbox alongside its `namespace`/`interp`
/// siblings — and that exclusion is enforced simply by this group's
/// omission of the `IRULES` bit: an `ALL_TCL` group never intersects the
/// bare `IRULES` availability mask, so `rename` falls out by plain
/// intersection with no separate disable list, the same treatment
/// `pwd`/`cd`/`open`/`glob`/`exec` get in their own spec files. Every
/// other modelled dialect (Expect, Tk, the EDA vendor consoles, F5 iApps,
/// F5 tmsh, incr Tcl) hosts a real Tcl core whose availability mask
/// intersects this `ALL_TCL` group and adds no dedicated `rename` override
/// of its own, so the command resolves there identically to plain Tcl.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "rename",
        dialects: Some(DialectSet::ALL_TCL),
        // `FIRE_AND_FORGET_TEARDOWN`: `Tcl_RenameObjCmd` → `TclRenameCommand`
        // (tclCmdMZ.c / tclBasic.c) deletes `oldName` (an empty `newName`
        // deletes the command outright) and errors when `oldName` doesn't
        // exist — the property the W302 fire-and-forget suppression
        // (`catch {rename foo ""}`) keys off.
        traits: Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::INSTALLS_NAMED_DEFINITION
            | Traits::FIRE_AND_FORGET_TEARDOWN,
        arity: Arity::exact(2),
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Name)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Rename or delete a command",
            synopsis: &["rename oldName newName"],
            snippet: "If newName is an empty string, oldName is deleted instead of renamed. Both oldName and newName may include namespace qualifiers; renaming a command into a different namespace relocates it there, and future invocations run in that namespace's context. Raises an error if oldName does not name an existing command, or if newName is non-empty and already names one — rename never silently overwrites an existing command. Any `trace add command` handler registered on the command fires as part of the operation: a rename trace when moved, a delete trace when removed. A common idiom wraps a built-in with custom logic: move the original out of the way, then define a same-named replacement that delegates to the saved name.",
            source: "Tcl rename(n)",
            examples: "rename ::source ::theRealSource\nset sourceCount 0\nproc ::source args {\n    global sourceCount\n    puts \"called source for the [incr sourceCount]'th time\"\n    uplevel 1 ::theRealSource $args\n}",
            return_value: "An empty string.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Rename),
        command_table_effect: Some(crate::command_table::CommandTableEffect::RenamesCommands),
        ..CommandSpec::DEFAULT
    }
}
