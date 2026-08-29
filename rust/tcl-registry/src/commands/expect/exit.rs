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

//! `exit` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
// `-onexit`/`-noexit` are Expect's own extensions to `exit`; core Tcl
// `exit` takes only `?returnCode?` (exit(n)). They carry an explicit
// `Some(EXPECT)` gate rather than inheriting the command's universal
// `surface: None` (see below) — otherwise the inherited `None` would offer
// these Expect-only flags as valid `exit` options under *every* dialect
// (plain Tcl, iRules, the EDA vendors, …).
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-onexit",
        value: OptionValue::command_prefix("command"),
        detail: "Register a handler to run at exit.",
        surface: Some(SpecSurface::EXPECT),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-noexit",
        value: OptionValue::flag(),
        detail: "Prepare for exit without exiting.",
        surface: Some(SpecSurface::EXPECT),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const FORMS: &[FormSpec] = &[
    // The universal core-Tcl form (exit(n)): available in every dialect.
    FormSpec {
        synopsis: "exit ?returnCode?",
        ..FormSpec::DEFAULT
    },
    // The Expect-extended form — `-onexit`/`-noexit` only exist under
    // Expect, so this synopsis is gated to the EXPECT dialect (mirrors the
    // per-option `Some(EXPECT)` gates above) rather than being advertised
    // as a plain-Tcl `exit` form.
    FormSpec {
        synopsis: "exit ?-onexit command | -noexit? ?status?",
        surface: Some(SpecSurface::EXPECT),
        ..FormSpec::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exit",
        surface: Some(SpecSurface::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Exit Expect, optionally running an onexit handler.",
            synopsis: &["exit ?-onexit command? ?status?", "exit ?-noexit? ?status?"],
            snippet: "With ``-onexit``, registers a handler to run at exit. With ``-noexit``, prepares for exit but does not actually exit (useful for cleaning up in libraries).",
            source: "Expect exit(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
