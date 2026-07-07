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

//! tcllib terminal / text packages — `stooop`, the `textutil::{repeat,split,
//! wcswidth}` sub-packages, and `term::ansi::{code,send}`.
//!
//! Public command surface from the upstream tcllib 2.0 manual pages.  Command
//! names, arity bounds, synopses, and summaries are derived from those pages.
//! Requires Tcl 8.5+.

use crate::prelude::*;

/// A row in a package command table: name, arity, synopsis, and summary.
type Row = (&'static str, Arity, &'static [&'static str], &'static str);

/// Build command specs for a package from its table.
fn rows(pkg: &'static str, table: &'static [Row]) -> Vec<CommandSpec> {
    table
        .iter()
        .map(|&(name, arity, synopsis, summary)| CommandSpec {
            name,
            arity,
            hover: Some(HoverSnippet::brief(summary, synopsis, "tcllib package")),
            tcllib_package: Some(pkg),
            required_package: Some(pkg),
            ..CommandSpec::DEFAULT
        })
        .collect()
}

/// The `stooop` package.
//
// `stooop::class` is not here — it carries a structural definition body
// (member `proc` definitions), so it has a dedicated spec in
// `stooop__class.rs` (the flat `Row` table can't express arg roles).
const STOOOP_CMDS: &[Row] = &[
    (
        "stooop::new",
        Arity::at_least(1),
        &["stooop::new class"],
        "This command is used to create an object.",
    ),
    (
        "stooop::delete",
        Arity::at_least(1),
        &["stooop::delete object"],
        "This command is used to delete one or several objects.",
    ),
    (
        "stooop::virtual",
        Arity::at_least(1),
        &["stooop::virtual proc name {this }"],
        "The virtual specifier may be used on member procedures to achieve dynamic binding.",
    ),
    (
        "stooop::classof",
        Arity::exact(1),
        &["stooop::classof object"],
        "This command returns the class of the existing object passed as single parameter.",
    ),
    (
        "stooop::printObjects",
        Arity::at_least(0),
        &["stooop::printObjects"],
        "Prints an ordered list of existing objects, in creation order, oldest first.",
    ),
    (
        "stooop::record",
        Arity::exact(0),
        &["stooop::record"],
        "When invoked, a snapshot of all existing stooop objects is taken.",
    ),
    (
        "stooop::report",
        Arity::at_least(0),
        &["stooop::report"],
        "Prints the created and deleted objects since the ::stooop::record procedure was invoked last.",
    ),
];

/// The `textutil::repeat` package.
const TU_REPEAT_CMDS: &[Row] = &[
    (
        "textutil::repeat::strRepeat",
        Arity::exact(2),
        &["textutil::repeat::strRepeat text num"],
        "This command returns a string containing the text repeated num times.",
    ),
    (
        "textutil::repeat::blank",
        Arity::exact(1),
        &["textutil::repeat::blank num"],
        "A convenience command.",
    ),
];

/// The `textutil::split` package.
const TU_SPLIT_CMDS: &[Row] = &[
    (
        "textutil::split::splitn",
        Arity::at_least(1),
        &["textutil::split::splitn string"],
        "This command splits the given string into chunks of len characters and returns a list containing these chunks.",
    ),
    (
        "textutil::split::splitx",
        Arity::at_least(1),
        &["textutil::split::splitx string"],
        "This command splits the string and return a list.",
    ),
];

/// The `textutil::wcswidth` package.
const TU_WCS_CMDS: &[Row] = &[
    (
        "textutil::wcswidth",
        Arity::exact(1),
        &["textutil::wcswidth string"],
        "Returns the number of character cells taken by the string when printed to the terminal.",
    ),
    (
        "textutil::wcswidth_char",
        Arity::exact(1),
        &["textutil::wcswidth_char char"],
        "Returns the number of character cells taken by the character when printed to the terminal.",
    ),
    (
        "textutil::wcswidth_type",
        Arity::exact(1),
        &["textutil::wcswidth_type char"],
        "Returns the character type of the specified character.",
    ),
];

/// The `term::ansi::code` package.
const ANSI_CODE_CMDS: &[Row] = &[
    (
        "term::ansi::code::esc",
        Arity::exact(1),
        &["term::ansi::code::esc str"],
        "This command returns the argument string, prefixed with the ANSI escape character, 033.",
    ),
    (
        "term::ansi::code::escb",
        Arity::exact(1),
        &["term::ansi::code::escb str"],
        "This command returns the argument string, prefixed with a common ANSI escape sequence, 033[.",
    ),
    (
        "term::ansi::code::define",
        Arity::exact(3),
        &["term::ansi::code::define name escape code"],
        "This command defines a procedure name which returns the control sequence code, beginning with the specified.",
    ),
    (
        "term::ansi::code::const",
        Arity::exact(2),
        &["term::ansi::code::const name code"],
        "This command defines a procedure name which returns the control sequence code.",
    ),
];

/// The `term::ansi::send` package.
const ANSI_SEND_CMDS: &[Row] = &[
    (
        "term::ansi::send::import",
        Arity::at_least(1),
        &["term::ansi::send::import ..."],
        "Imports the commands of this package into the namespace ns.",
    ),
    (
        "term::ansi::send::eeol",
        Arity::exact(0),
        &["term::ansi::send::eeol"],
        "Erase (to) End Of Line.",
    ),
    (
        "term::ansi::send::esol",
        Arity::exact(0),
        &["term::ansi::send::esol"],
        "Erase (to) Start Of Line.",
    ),
    (
        "term::ansi::send::el",
        Arity::exact(0),
        &["term::ansi::send::el"],
        "Erase (current) Line.",
    ),
    (
        "term::ansi::send::ed",
        Arity::exact(0),
        &["term::ansi::send::ed"],
        "Erase Down (to bottom).",
    ),
    (
        "term::ansi::send::eu",
        Arity::exact(0),
        &["term::ansi::send::eu"],
        "Erase Up (to top).",
    ),
    (
        "term::ansi::send::es",
        Arity::exact(0),
        &["term::ansi::send::es"],
        "Erase Screen.",
    ),
    (
        "term::ansi::send::sd",
        Arity::exact(0),
        &["term::ansi::send::sd"],
        "Scroll Down.",
    ),
    (
        "term::ansi::send::su",
        Arity::exact(0),
        &["term::ansi::send::su"],
        "Scroll Up.",
    ),
    (
        "term::ansi::send::ch",
        Arity::exact(0),
        &["term::ansi::send::ch"],
        "Cursor Home.",
    ),
    (
        "term::ansi::send::sc",
        Arity::exact(0),
        &["term::ansi::send::sc"],
        "Save Cursor.",
    ),
    (
        "term::ansi::send::rc",
        Arity::exact(0),
        &["term::ansi::send::rc"],
        "Restore Cursor (Unsave).",
    ),
    (
        "term::ansi::send::sca",
        Arity::exact(0),
        &["term::ansi::send::sca"],
        "Save Cursor + Attributes.",
    ),
    (
        "term::ansi::send::rca",
        Arity::exact(0),
        &["term::ansi::send::rca"],
        "Restore Cursor + Attributes.",
    ),
    (
        "term::ansi::send::st",
        Arity::exact(0),
        &["term::ansi::send::st"],
        "Set Tab (@ current position).",
    ),
    (
        "term::ansi::send::ct",
        Arity::exact(0),
        &["term::ansi::send::ct"],
        "Clear Tab (@ current position).",
    ),
    (
        "term::ansi::send::cat",
        Arity::exact(0),
        &["term::ansi::send::cat"],
        "Clear All Tabs.",
    ),
    (
        "term::ansi::send::qdc",
        Arity::exact(0),
        &["term::ansi::send::qdc"],
        "Query Device Code.",
    ),
    (
        "term::ansi::send::qds",
        Arity::exact(0),
        &["term::ansi::send::qds"],
        "Query Device Status.",
    ),
    (
        "term::ansi::send::qcp",
        Arity::exact(0),
        &["term::ansi::send::qcp"],
        "Query Cursor Position.",
    ),
    (
        "term::ansi::send::rd",
        Arity::exact(0),
        &["term::ansi::send::rd"],
        "Reset Device.",
    ),
    (
        "term::ansi::send::elw",
        Arity::exact(0),
        &["term::ansi::send::elw"],
        "Enable Line Wrap.",
    ),
    (
        "term::ansi::send::dlw",
        Arity::exact(0),
        &["term::ansi::send::dlw"],
        "Disable Line Wrap.",
    ),
    (
        "term::ansi::send::eg",
        Arity::exact(0),
        &["term::ansi::send::eg"],
        "Enter Graphics Mode.",
    ),
    (
        "term::ansi::send::lg",
        Arity::exact(0),
        &["term::ansi::send::lg"],
        "Exit Graphics Mode.",
    ),
    (
        "term::ansi::send::scs0",
        Arity::exact(1),
        &["term::ansi::send::scs0 tag"],
        "Choose the character set used for the default font (G0).",
    ),
    (
        "term::ansi::send::scs1",
        Arity::exact(1),
        &["term::ansi::send::scs1 tag"],
        "Select Character Set.",
    ),
    (
        "term::ansi::send::sda",
        Arity::at_least(1),
        &["term::ansi::send::sda arg..."],
        "Set Display Attributes.",
    ),
    (
        "term::ansi::send::fcp",
        Arity::exact(2),
        &["term::ansi::send::fcp row col"],
        "Force Cursor Position (aka Go To).",
    ),
    (
        "term::ansi::send::ss",
        Arity::at_least(0),
        &["term::ansi::send::ss"],
        "Scroll Screen (entire display, or between rows start end, inclusive).",
    ),
    (
        "term::ansi::send::skd",
        Arity::exact(2),
        &["term::ansi::send::skd code str"],
        "Set Key Definition.",
    ),
    (
        "term::ansi::send::title",
        Arity::exact(1),
        &["term::ansi::send::title str"],
        "Set the terminal title.",
    ),
    (
        "term::ansi::send::gron",
        Arity::exact(0),
        &["term::ansi::send::gron"],
        "Switch to character/box graphics.",
    ),
    (
        "term::ansi::send::groff",
        Arity::exact(0),
        &["term::ansi::send::groff"],
        "Switch to regular characters.",
    ),
    (
        "term::ansi::send::groptim",
        Arity::exact(1),
        &["term::ansi::send::groptim str"],
        "Optimize character graphics.",
    ),
    (
        "term::ansi::send::clear",
        Arity::exact(0),
        &["term::ansi::send::clear"],
        "Clear screen.",
    ),
    (
        "term::ansi::send::init",
        Arity::exact(0),
        &["term::ansi::send::init"],
        "Initialize default and alternate fonts to ASCII and box graphics.",
    ),
    (
        "term::ansi::send::showat",
        Arity::exact(3),
        &["term::ansi::send::showat row col text"],
        "Show the block of text at the specified location.",
    ),
];

/// All terminal / text package command specs.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = Vec::new();
    specs.extend(rows("stooop", STOOOP_CMDS));
    specs.extend(rows("textutil::repeat", TU_REPEAT_CMDS));
    specs.extend(rows("textutil::split", TU_SPLIT_CMDS));
    specs.extend(rows("textutil::wcswidth", TU_WCS_CMDS));
    specs.extend(rows("term::ansi::code", ANSI_CODE_CMDS));
    specs.extend(rows("term::ansi::send", ANSI_SEND_CMDS));
    specs
}
