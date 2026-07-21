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

//! `auto_reset` — discard the auto-loading/auto-exec caches and delete any
//! commands that were previously loaded through them.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "auto_reset",
    dialects: None,
}];

/// Command spec for `auto_reset`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_reset",
        // Universal Tcl data (`dialects: None`) is deliberate, not an
        // oversight: F5 iRules bans this proc, but the ban is modelled on
        // the *profile's* subtractive `IRULES_DISABLED_COMMANDS` list
        // (tcl-dialect/src/profile.rs), which explicitly names
        // `"auto_reset"` alongside the rest of the K36322151
        // filesystem/process bans — never as a `dialects:` restriction
        // here (folding it into a `TCL84|IRULES`-style union on the spec
        // would re-admit exactly the command that list exists to ban; see
        // the comment on `IRULES_DISABLED_COMMANDS`). No other dialect
        // profile (Expect, the EDA vendor shells, tmsh, iApps, BPF) lists
        // `auto_reset` in its `disabled_commands`, so it stays reachable
        // there.
        dialects: None,
        // A Tcl-level library proc (`library/auto.tcl`), not a
        // `Tcl_CreateObjCommand`-registered builtin, so it carries no
        // `CmdInfo` row and is absent from the exact C Tcl
        // safe-interpreter hidden-command table documented on
        // `Traits::SAFE_INTERP_HIDDEN` — that trait does not apply here.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        // `proc auto_reset {}` in every shipped `library/auto.tcl` from
        // 8.4 through 9.1 (the 9.1 beta tag `core-9-1-b0` still declares
        // it with an empty formal-argument list), matching `library.n`'s
        // bare `auto_reset` synopsis — unchanged across the same range.
        // No arguments, ever.
        arity: Arity::exact(0),
        return_type: Some(TclType::String),
        side_effects: &[
            // Unsets its own cache variables and re-reads/repairs
            // `auto_path`. The exact variable set moved over time: 8.4
            // unsets `auto_execs`, `auto_index`, and `auto_oldpath`, and
            // never touches `auto_path`; 8.5 through 9.0 unset the same
            // caches (renamed to `::tcl::auto_oldpath`) and additionally
            // read `auto_path`, initialising it to `[info library]` when
            // it is missing or not a valid list, or appending
            // `[info library]` when it is a valid list that lacks it; 9.1
            // dropped the `auto_execs` cache entirely (see `auto_execok`'s
            // 9.1 rework) but keeps unsetting `auto_index` /
            // `::tcl::auto_oldpath` and the `auto_path` repair. The union
            // across every version is a read/write of interpreter-global
            // Tcl variables.
            SideEffect {
                target: SideEffectTarget::Variable,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
            // Deletes (renames away) every command still resolvable from
            // the auto-load index at call time, so a fresh copy loads on
            // next use. 8.4 walks `info procs` in the global namespace and
            // skips a fixed list of other library procs (`unknown`,
            // `pkg_mkIndex`, `tcl_findLibrary`, …); 8.5 onward instead
            // walks every `auto_index` key, resolves it with `namespace
            // which` (so a command autoloaded into any namespace is
            // found, not just the global one), and renames whatever
            // currently resolves.
            SideEffect {
                target: SideEffectTarget::ProcDefinition,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
        ],
        hover: Some(HoverSnippet {
            summary: "Discard the auto-loading and auto-exec caches, deleting any commands they previously loaded.",
            synopsis: &["auto_reset"],
            snippet: "Destroys all information cached by auto_load and auto_execok so it gets recomputed from disk the next time it is needed, and deletes any command still resolvable from the auto-load index so a fresh copy loads on next use. Unsets the global array auto_index (auto_load's index cache); through Tcl 9.0 also unsets auto_execs (auto_execok's search cache) and the private ::tcl::auto_oldpath bookkeeping variable — Tcl 9.1 removed the auto_execs cache along with the rest of auto_execok's old caching behaviour, so auto_reset no longer touches it there. From Tcl 8.5 onward, auto_reset also checks auto_path: if it is missing or not a valid list it is reinitialised to just the Tcl library directory, and if it is a valid list that lacks the Tcl library directory that directory is appended, so a corrupted auto_path cannot permanently break auto-loading. Typically run during interactive development after editing or regenerating a tclIndex-indexed script, so the next call to the affected command re-scans auto_path and picks up the change instead of reusing an already-loaded copy.",
            source: "Tcl library(n)",
            examples: "# After regenerating a library's tclIndex on disk:\nauto_reset\nsomeAutoloadedCmd arg1 arg2  ;# re-scans auto_path and loads the fresh definition",
            return_value: "An empty string in the common case. Because auto_reset has no explicit return statement, its result is whatever its last statement evaluates to; when the auto_path repair fires (Tcl 8.5+), that last statement is the set or lappend that updates auto_path, so the result is the new auto_path list instead — an implementation detail, not part of the documented contract.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
