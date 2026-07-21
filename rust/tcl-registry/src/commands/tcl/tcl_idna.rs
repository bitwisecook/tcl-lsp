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

//! `tcl::idna` — Internationalised Domain Name (IDNA/Punycode) helpers
//! (Tcl 9.0+, bundled package).
//!
//! Two name forms are registered because Tcl callers may use the
//! namespace-relative spelling `tcl::idna` (inside `namespace eval
//! ::tcl`) or the fully-qualified `::tcl::idna`.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tcl::idna subcommand ?arg ...?",
    dialects: None,
}];

fn make_spec(name: &'static str) -> CommandSpec {
    CommandSpec {
        name,
        dialects: Some(DialectSet::TCL90_PLUS),
        required_package: Some("tcl::idna"),
        arity: Arity::at_least(1),
        subcommands: &SUBCOMMANDS,
        return_type: Some(TclType::String),
        forms: FORMS,
        hover: Some(HoverSnippet::brief(
            "Internationalised Domain Name (IDNA/Punycode) helpers.",
            &["tcl::idna subcommand ?arg ...?"],
            "Tcl man page idna.n",
        )),
        ..CommandSpec::DEFAULT
    }
}

static SUBCOMMANDS: [SubCommand; 4] = [
    SubCommand {
        name: "decode",
        arity: Arity::new(1, 1),
        detail: "Decode punycode in a hostname for display.",
        synopsis: "tcl::idna decode hostname",
        return_type: Some(TclType::String),
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "encode",
        arity: Arity::new(1, 1),
        detail: "Encode a hostname with punycode where needed.",
        synopsis: "tcl::idna encode hostname",
        return_type: Some(TclType::String),
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "puny",
        arity: Arity::new(2, 3),
        detail: "Direct access to the punycode encoder/decoder.",
        synopsis: "tcl::idna puny decode|encode string ?case?",
        return_type: Some(TclType::String),
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "version",
        arity: Arity::new(0, 0),
        detail: "Return the tcl::idna package version.",
        synopsis: "tcl::idna version",
        return_type: Some(TclType::String),
        pure: true,
        ..SubCommand::DEFAULT
    },
];

/// Command spec for the namespace-relative form `tcl::idna`.
pub fn spec() -> CommandSpec {
    make_spec("tcl::idna")
}

/// Command spec for the fully-qualified form `::tcl::idna`.
pub fn spec_qualified() -> CommandSpec {
    make_spec("::tcl::idna")
}
