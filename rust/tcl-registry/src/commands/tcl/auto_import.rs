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

//! `auto_import` — load auto-loadable commands matching a `namespace
//! import` pattern so the import can create links to them.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "auto_import pattern",
    dialects: None,
}];

/// Command spec for `auto_import`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_import",
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
        // core-version bit that `ALL_TCL` does intersect, so `auto_import`
        // stays reachable there.
        dialects: Some(DialectSet::ALL_TCL),
        // A Tcl-level library proc (`library/init.tcl`), not a
        // `Tcl_CreateObjCommand`-registered builtin, so it carries no
        // `CmdInfo` row and is absent from the exact C Tcl
        // safe-interpreter hidden-command table documented on
        // `Traits::SAFE_INTERP_HIDDEN` — that trait does not apply here.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC
            // Auto-loads (and imports) the commands matching `pattern`
            // from whichever unit the auto-index names, so another unit's
            // script runs in this interpreter.
            | Traits::LOADS_EXTERNAL_UNIT
            // Imports commands matched by their spelled names.
            | Traits::REFLECTS_COMMAND_NAMES,
        // `auto_import pattern` — exactly one argument, unchanged across
        // every documented release, Tcl library.n 8.4 through 9.1.
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
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
            summary: "Load auto-loadable commands matching a namespace-import pattern.",
            synopsis: &["auto_import pattern"],
            snippet: "Invoked internally by namespace import to see whether the commands matched by pattern (in the style of namespace import, e.g. foo::*) reside in an autoloaded library. If pattern contains no :: at all it is treated as unqualified and the call is a no-op. Otherwise the pattern is resolved against the caller's current namespace, the auto-load index is (re)built by scanning tclIndex files across auto_path, and every not-yet-defined command whose indexed name matches is loaded into the global namespace so that namespace import can create its import link right afterwards. Commands already defined, or with no matching index entry, are left untouched. It is not normally necessary to call this command directly.",
            source: "Tcl library(n)",
            examples: "namespace eval ::mylib {\n    namespace export foo bar\n}\nnamespace import ::mylib::*",
            return_value: "An empty string.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
