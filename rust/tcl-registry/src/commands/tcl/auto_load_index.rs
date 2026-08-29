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

//! `auto_load_index` — (re)build the `auto_index` lookup table from the
//! `tclIndex` files on `auto_path`.

use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "auto_load_index",
    ..FormSpec::DEFAULT
}];

/// Command spec for `auto_load_index`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_load_index",
        // `surface: Some(SpecSurface::ALL_TCL)` matches every sibling
        // autoloading proc (`auto_execok`, `auto_import`, `auto_load`,
        // `auto_mkindex`, `auto_mkindex_old`, `auto_qualify`, `auto_reset`):
        // F5 iRules bans all seven, and that ban is now carried by each
        // spec's own `dialects` group rather than any list. `ALL_TCL`
        // spans every core Tcl version but not the `IRULES` bit, so none
        // of these specs intersect iRules' bare `IRULES` availability mask
        // — narrowing one to a `DialectSet` that *did* carry `IRULES`
        // would re-admit exactly what the ban excludes (same rationale
        // documented on `auto_load`'s spec). Under the old subtractive
        // disable list `auto_load_index` was conspicuously absent even
        // though it is the proc every other listed one either calls or
        // exists alongside — a gap that left it erroneously reachable in
        // iRules; making availability explicit per spec closes that gap,
        // since this `ALL_TCL` group now excludes iRules directly, just
        // like its siblings.
        surface: Some(SpecSurface::ALL_TCL),
        // A Tcl-level library proc (`library/init.tcl`), not a
        // `Tcl_CreateObjCommand`-registered builtin, so it carries no
        // `CmdInfo` row and is absent from the exact C Tcl safe-interpreter
        // hidden-command table documented on `Traits::SAFE_INTERP_HIDDEN`
        // — that trait does not apply here. Redefinable by user code and
        // invoked automatically by `auto_load` (see
        // `Traits::OVERRIDABLE_LIBRARY_PROC`), so redefining it must not
        // fire the W113 "overrides a built-in" warning.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        // `proc auto_load_index {}` in every shipped `library/init.tcl`
        // from 8.4 through 9.1 — no arguments in any version.
        arity: Arity::exact(0),
        return_type: Some(TclType::Boolean),
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                ..SideEffect::DEFAULT
            },
            SideEffect {
                target: SideEffectTarget::Variable,
                reads: true,
                writes: true,
                ..SideEffect::DEFAULT
            },
        ],
        hover: Some(HoverSnippet {
            summary: "Rebuild the auto-load index from the tclIndex files on auto_path.",
            synopsis: &["auto_load_index"],
            snippet: "Walks auto_path from last directory to first, reading each directory's tclIndex file and recording, for every command name it lists, a script to source in order to define that command later. Results accumulate in the global array auto_index; because the walk runs back-to-front and later assignments overwrite earlier ones, a directory that appears earlier in auto_path wins over one that appears later when both index the same command name. This is the proc auto_load calls to populate auto_index before looking up a command, and the one unknown reaches transitively when a script invokes an undefined command; calling it directly forces an immediate rebuild instead of waiting for that lazy path. The up-to-date check compares only the current value of auto_path against the value recorded during the previous scan — it returns 0 and leaves auto_index untouched whenever auto_path itself has not changed, even if a tclIndex file on disk was regenerated in place, so regenerating an index without changing auto_path requires auto_reset first to force a real rescan. In a safe interpreter only the newer \"version 2.0\" tclIndex format is supported, each directory's tclIndex is sourced directly and wrapped in catch, and a directory whose tclIndex errors is silently skipped; in a normal interpreter a malformed or unreadable tclIndex file aborts the scan immediately and its error (with its full return -options dictionary) propagates out of auto_load_index instead of being caught, so later directories in the walk are never examined. auto_load_index has no dedicated Tcl manual page of its own — library.n documents auto_load, auto_execok, auto_import, auto_mkindex, auto_qualify, and auto_reset, but not this proc — its only documentation is the header comment above its definition in library/init.tcl.",
            source: "Tcl library (library/init.tcl)",
            examples: "lappend auto_path /opt/mylib/tcl\nauto_load_index\nsomeCommandFromMylib arg1",
            return_value: "1 if auto_path had changed since the last scan and the tclIndex files were re-read; 0 if auto_path is unchanged and the existing auto_index cache was left as-is.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
