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

//! Additional standalone Tk commands not previously registered — `bindtags`,
//! `tk_optionMenu`, `tk_dialog`, `tk_setPalette`, `tk_bisque`,
//! `tk_focusNext`, `tk_focusPrev`, and `tk_focusFollowsMouse`.
//!
//! Synopses are taken from the Tk 8.6 manual pages.  (`tk busy` and
//! `tk fontchooser` are already modelled as sub-commands of the `tk`
//! command; `console` and `consoleinterp` have their own file, `console.rs`,
//! since they need a `SubCommand` table.)  Available in a `wish` /
//! `package require Tk` interpreter.

use crate::prelude::*;

const WRITES: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

/// One additional Tk command: name, arity, synopsis, summary, and the argument
/// roles (used only for the `tk_optionMenu` variable link).
fn cmd(
    name: &'static str,
    arity: Arity,
    synopsis: &'static [&'static str],
    summary: &'static str,
    arg_roles: &'static [(u8, ArgRole)],
) -> CommandSpec {
    CommandSpec {
        name,
        dialects: Some(DialectSet::TK_AND_TCL),
        arity,
        arg_roles,
        hover: Some(HoverSnippet::brief(summary, synopsis, "Tk man page")),
        required_package: Some("Tk"),
        warn_missing_import: false,
        side_effects: WRITES,
        ..CommandSpec::DEFAULT
    }
}

/// All additional standalone Tk command specs.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd(
            "bindtags",
            Arity::new(1, 2),
            &["bindtags window ?tagList?"],
            "Get or set the binding tags used for a window.",
            &[],
        ),
        cmd(
            "tk_optionMenu",
            Arity::at_least(3),
            &["tk_optionMenu pathName varName value ?value ...?"],
            "Create an option-menubutton whose value is linked to a variable.",
            &[(1, ArgRole::VarWrite)],
        ),
        cmd(
            "tk_dialog",
            Arity::at_least(5),
            &["tk_dialog window title text bitmap default string ?string ...?"],
            "Create a modal dialog box with a bitmap and a row of buttons.",
            &[],
        ),
        cmd(
            "tk_setPalette",
            Arity::at_least(1),
            &["tk_setPalette background | ?name value ...?"],
            "Change the colour scheme of the whole application.",
            &[],
        ),
        cmd(
            "tk_bisque",
            Arity::exact(0),
            &["tk_bisque"],
            "Restore the classic Tk bisque colour scheme.",
            &[],
        ),
        cmd(
            "tk_focusNext",
            Arity::exact(1),
            &["tk_focusNext window"],
            "Return the next window in keyboard-traversal order.",
            &[],
        ),
        cmd(
            "tk_focusPrev",
            Arity::exact(1),
            &["tk_focusPrev window"],
            "Return the previous window in keyboard-traversal order.",
            &[],
        ),
        cmd(
            "tk_focusFollowsMouse",
            Arity::exact(0),
            &["tk_focusFollowsMouse"],
            "Change focus behaviour so focus follows the mouse pointer.",
            &[],
        ),
    ]
}
