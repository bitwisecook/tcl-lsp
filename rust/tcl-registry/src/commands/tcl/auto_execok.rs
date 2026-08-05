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

//! `auto_execok` — probe whether an external program or shell builtin is
//! runnable, and return the `exec` argument list for it.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "auto_execok cmd",
    dialects: None,
}];

/// Command spec for `auto_execok`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_execok",
        // `dialects: Some(DialectSet::ALL_TCL)` is deliberate, not an
        // oversight: F5 iRules bans this proc (it is one of the K36322151
        // filesystem/process bans), and that ban is now carried by this
        // very `dialects` group. `ALL_TCL` spans every core Tcl version
        // but not the `IRULES` bit, so the spec never intersects iRules'
        // bare `IRULES` availability mask and the command is simply
        // unreachable there — no disable list is involved. Widening this
        // to a `TCL84|IRULES`-style union would re-admit exactly the
        // command the ban exists to exclude. Every other dialect profile
        // (Expect, the EDA vendor shells, tmsh, iApps, BPF) carries a
        // core-version bit that `ALL_TCL` does intersect, so `auto_execok`
        // stays reachable there.
        dialects: Some(DialectSet::ALL_TCL),
        // A Tcl-level library proc (`library/init.tcl`), not a
        // `Tcl_CreateObjCommand`-registered builtin, so it carries no
        // `CmdInfo` row and is absent from the exact C Tcl
        // safe-interpreter hidden-command table documented on
        // `Traits::SAFE_INTERP_HIDDEN` — that trait does not apply here.
        // Probes `cmd` as a command / executable identity by name.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC | Traits::REFLECTS_COMMAND_NAMES,
        // `auto_execok cmd` — exactly one argument; the synopsis is
        // unchanged across every documented release, Tcl library.n
        // 8.4 through 9.1.
        arity: Arity::exact(1),
        return_type: Some(TclType::List),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Resolve a command name to its exec argument list, or return an empty string.",
            synopsis: &["auto_execok cmd"],
            snippet: "Searches the directories on PATH (and, on Windows, several additional locations) for an executable file or shell builtin named cmd, and returns the argument list to pass to exec to run it. Returns an empty string when nothing matching is found. Used internally by the unknown command handler to auto-exec external programs invoked as bare Tcl commands. Through Tcl 9.0, results are cached in the global array auto_execs, cleared by auto_reset. Tcl 9.1 removed that cache and reworked the Windows search to match exec's own algorithm: cmd is first matched against cmd.exe's own builtins (excluding batch-file-only ones); failing that, the directory of the process executable, the current working directory (conditionally), the Windows system32 and Windows directories, and then PATH are searched in that order, matching cmd against the PATHEXT extensions and the registry's file-association templates; finally the App Paths registry key is checked. From 9.1 onward, callers must not assume a returned path is normalized, absolute, native, or canonical.",
            source: "Tcl library(n)",
            examples: "set found [auto_execok grep]\nif {[llength $found]} {\n    exec {*}$found -c foo bar.txt\n}",
            return_value: "A list of arguments for exec naming the resolved executable or shell builtin, or an empty string if cmd cannot be found.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
