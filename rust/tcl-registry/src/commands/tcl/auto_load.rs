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

//! `auto_load` — load the definition of an auto-loadable command from a
//! `tclIndex` entry on `auto_path`.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "auto_load cmd ?namespace?",
    dialects: None,
}];

/// Command spec for `auto_load`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_load",
        // Universal Tcl data (`dialects: None`) is deliberate, not an
        // oversight: F5 iRules bans this proc, but the ban is modelled on
        // the *profile's* subtractive `IRULES_DISABLED_COMMANDS` list
        // (tcl-dialect/src/profile.rs), which explicitly names
        // `"auto_load"` alongside the rest of the K36322151
        // filesystem/process bans — never as a `dialects:` restriction
        // here (folding it into a `TCL84|IRULES`-style union on the spec
        // would re-admit exactly the command that list exists to ban; see
        // the comment on `IRULES_DISABLED_COMMANDS`). No other dialect
        // profile (Expect, the EDA vendor shells, tmsh, iApps, BPF) lists
        // `auto_load` in its `disabled_commands`, so it stays reachable
        // there.
        dialects: None,
        // A Tcl-level library proc (`library/init.tcl`), not a
        // `Tcl_CreateObjCommand`-registered builtin, so it carries no
        // `CmdInfo` row and is absent from the exact C Tcl
        // safe-interpreter hidden-command table documented on
        // `Traits::SAFE_INTERP_HIDDEN` — that trait does not apply here.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        // `auto_load cmd ?namespace?`: library.n (8.4 through 9.1) only
        // documents `cmd`, but every shipped `library/init.tcl` defines
        // `proc auto_load {cmd {namespace {}}} …` — confirmed against the
        // vendored Tcl 9.0.3 `init.tcl` in this repo
        // (runtime/rust/vendor/tcl_library/init.tcl) — where an omitted
        // or empty `namespace` defaults to the caller's current namespace
        // via `uplevel 1 [list ::namespace current]`. The real arity is
        // therefore 1-2, not the manpage-only 1; this second argument has
        // been part of the proc since namespaces were introduced and is
        // unchanged across every version examined.
        arity: Arity::new(1, 2),
        return_type: Some(TclType::Boolean),
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
            SideEffect {
                target: SideEffectTarget::ProcDefinition,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
        ],
        hover: Some(HoverSnippet {
            summary: "Load the definition of an auto-loadable command.",
            synopsis: &["auto_load cmd ?namespace?"],
            snippet: "Searches the tclIndex files across auto_path (or, if auto_path is unset, the TCLLIBPATH environment variable, falling back to just the Tcl library directory) for an entry matching cmd, qualified against namespace the same way namespace import patterns are qualified — namespace defaults to the caller's current namespace when omitted. When a matching entry is found, evaluates its indexed script to define the command and returns 1. Returns 0 if no index entry matches cmd, or if the indexed script ran without error but did not actually define the command; an error during script evaluation propagates as an error from auto_load itself. Index information is read from disk once per auto_path value and cached in the global array auto_index; auto_reset clears the cache to force a fresh read on the next call. Invoked automatically by the default unknown command handler when a script calls an undefined command, so calling it directly is rarely necessary.",
            source: "Tcl library(n)",
            examples: "if {![auto_load someCmd]} {\n    error \"someCmd is not available via auto-loading\"\n}\nsomeCmd arg1 arg2",
            return_value: "1 if cmd was successfully located and defined; 0 if no matching tclIndex entry was found, or the indexed script did not define cmd.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
