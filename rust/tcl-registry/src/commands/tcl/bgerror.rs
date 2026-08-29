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

//! `bgerror` — application-defined handler for background errors.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "bgerror message",
    ..FormSpec::DEFAULT
}];

/// Command spec for `bgerror`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "bgerror",
        // `surface: Some(SpecSurface::ALL_TCL)` is deliberate, not an
        // oversight: F5 iRules bans this proc — one of the K36322151
        // event-loop/filesystem/process bans — and that ban is carried by this
        // very surface. `ALL_TCL` spans every core Tcl release but names no
        // iRules row, so it never admits the iRules point and the command is
        // simply unreachable there — no disable list is involved (folding it
        // into a `TCL84|IRULES`-style union would re-admit exactly the command
        // the ban exists to exclude). Every other dialect profile (Expect, the
        // EDA vendor shells, tmsh, iApps, BPF) runs a Tcl core that `ALL_TCL`
        // does admit, so `bgerror` stays reachable there.
        surface: Some(SpecSurface::ALL_TCL),
        // A Tcl-level convention the application/script defines, not a
        // `Tcl_CreateObjCommand`-registered builtin, so it carries no
        // `CmdInfo` row and is absent from the exact C Tcl
        // safe-interpreter hidden-command table documented on
        // `Traits::SAFE_INTERP_HIDDEN` — that trait does not apply here.
        // Unlike `auto_execok`/`unknown` (which core `library/init.tcl`
        // does define), `bgerror` ships with no default implementation of
        // its own anywhere in core Tcl 8.4 through 9.1 — only a
        // `package require Tk` interpreter supplies one, to pop up an
        // error dialog — so it is squarely a user-replaceable overlay and
        // redefining it must not fire the W113 "overrides a built-in"
        // warning.
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        // `bgerror message` — exactly one argument; unchanged across
        // every documented release, Tcl bgerror.n 8.4 through 9.1.
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Application-defined handler invoked for background errors.",
            synopsis: &["bgerror message"],
            snippet: "bgerror is not a built-in Tcl command: define a proc of this name to receive errors that happen outside any script's normal call chain, such as an error thrown inside an after callback, a fileevent handler, or (in Tk) a widget binding. When a background error occurs, Tcl restores the errorInfo and errorCode variables to their values at the time of the error, then invokes bgerror as an idle event with the error message as its only argument. Any value bgerror returns is ignored; if the bgerror call itself raises an error, Tcl writes that error to stderr instead of retrying. Several background errors accumulated before the event loop is idle each invoke bgerror once, in the order they occurred — returning from bgerror with a break exception discards any errors still queued. A bare tclsh has no default bgerror, so an error with none defined is reported to stderr; a Tk-linked interpreter supplies a default that pops up an error dialog. From Tcl 8.5 onward, interp bgerror registers a handler command programmatically and is the preferred mechanism for code that only needs to run on 8.5 and later.",
            source: "Tcl bgerror(n)",
            examples: "proc bgerror {message} {\n    global errorInfo\n    puts stderr \"background error: $message\\n$errorInfo\"\n}\nafter idle {error \"boom\"}",
            return_value: "Ignored by Tcl, unless invoking bgerror itself raises an error, in which case Tcl reports that error to stderr instead.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
