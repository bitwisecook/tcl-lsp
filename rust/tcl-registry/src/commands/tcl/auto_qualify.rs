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

//! `auto_qualify` — compute the fully qualified candidate name(s) for a
//! command in a namespace, for use by the auto-loading facilities.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "auto_qualify cmd namespace",
    dialects: None,
}];

/// Command spec for `auto_qualify`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_qualify",
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
        // core-version bit that `ALL_TCL` does intersect, so `auto_qualify`
        // stays reachable there.
        dialects: Some(DialectSet::ALL_TCL),
        // A Tcl-level library proc (`library/init.tcl`), not a
        // `Tcl_CreateObjCommand`-registered builtin, so it carries no
        // `CmdInfo` row and is absent from the exact C Tcl
        // safe-interpreter hidden-command table documented on
        // `Traits::SAFE_INTERP_HIDDEN` — that trait does not apply here.
        // `PURE`: unlike its `auto_execok` / `auto_import` / `auto_load`
        // siblings, the proc body touches only its two parameters — no
        // `global`/`variable` statement, no filesystem access, no read of
        // `auto_index`/`auto_path` — so its result depends solely on
        // `cmd` and `namespace` and it commits no side effect.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC | Traits::PURE,
        // `proc auto_qualify {cmd namespace}` in every shipped
        // `library/init.tcl` from 8.4 through 9.1, matching `library.n`'s
        // `auto_qualify command namespace` synopsis (unchanged across the
        // same range) — both arguments are mandatory, no default.
        arity: Arity::exact(2),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Compute the fully qualified candidate name(s) for a command in a namespace.",
            synopsis: &["auto_qualify cmd namespace"],
            snippet: "Returns a list of the fully qualified names a standard Tcl interpreter would try, in lookup order, to resolve cmd starting from namespace: first the current namespace, then the global namespace. namespace must be a canonical namespace name, such as the result of [namespace current]. Repeated colons within cmd are first collapsed (foo:::::bar becomes foo::bar). If cmd is relative (no leading ::): when namespace is the global namespace :: the result is the single element cmd (or ::cmd if cmd itself contains ::); otherwise the result has two elements, cmd qualified by namespace first, then cmd's global-namespace form. If cmd is already absolute (starts with ::) and contains further :: separators, the result is the single element cmd unchanged, and namespace is ignored. If cmd is absolute with no further separators (e.g. ::global), namespace is likewise ignored and the result is the single element cmd with its leading :: stripped (global, not ::global) — a historical quirk of the auto_index key format, where commands defined in the global namespace are indexed without a leading ::. auto_qualify is used internally by auto_load, auto_import, and the default unknown handler to build and probe the auto_index array; scripts rarely need to call it directly.",
            source: "Tcl library(n)",
            examples: "set names [auto_qualify foo [namespace current]]\nforeach name $names {\n    if {[namespace which -command $name] ne \"\"} {\n        return $name\n    }\n}",
            return_value: "A list of one or two fully qualified candidate names for cmd, in the order a command lookup would try them.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
