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

//! Terminal control: the `term::ansi::*` code tables and `term::send`.

use super::{Arity, Row};

/// The `term::ansi::code::attr` package.
pub(super) const TERM__ANSI__CODE__ATTR_CMDS: &[Row] = &[(
    "term::ansi::code::attr::names",
    Arity::exact(0),
    &["term::ansi::code::attr::names"],
    "This command is for introspection.",
)];

/// The `term::ansi::code::ctrl` package.
pub(super) const TERM__ANSI__CODE__CTRL_CMDS: &[Row] = &[
    (
        "term::ansi::code::ctrl::names",
        Arity::exact(0),
        &["term::ansi::code::ctrl::names"],
        "This command is for introspection.",
    ),
    (
        "term::ansi::code::ctrl::skd",
        Arity::exact(2),
        &["term::ansi::code::ctrl::skd code str"],
        "Set Key Definition.",
    ),
    (
        "term::ansi::code::ctrl::title",
        Arity::exact(1),
        &["term::ansi::code::ctrl::title str"],
        "Set the terminal title.",
    ),
    (
        "term::ansi::code::ctrl::gron",
        Arity::exact(0),
        &["term::ansi::code::ctrl::gron"],
        "Switch to character/box graphics.",
    ),
    (
        "term::ansi::code::ctrl::groff",
        Arity::exact(0),
        &["term::ansi::code::ctrl::groff"],
        "Switch to regular characters.",
    ),
    (
        "term::ansi::code::ctrl::groptim",
        Arity::exact(1),
        &["term::ansi::code::ctrl::groptim str"],
        "Optimize character graphics.",
    ),
    (
        "term::ansi::code::ctrl::clear",
        Arity::exact(0),
        &["term::ansi::code::ctrl::clear"],
        "Clear screen.",
    ),
    (
        "term::ansi::code::ctrl::init",
        Arity::exact(0),
        &["term::ansi::code::ctrl::init"],
        "Initialize default and alternate fonts to ASCII and box graphics.",
    ),
    (
        "term::ansi::code::ctrl::showat",
        Arity::exact(3),
        &["term::ansi::code::ctrl::showat row col text"],
        "Format the block of text for display at the specified location.",
    ),
];

/// The `term::ansi::code::macros` package.
pub(super) const TERM__ANSI__CODE__MACROS_CMDS: &[Row] = &[
    (
        "term::ansi::code::macros::names",
        Arity::exact(0),
        &["term::ansi::code::macros::names"],
        "This command is for introspection.",
    ),
    (
        "term::ansi::code::macros::menu",
        Arity::exact(1),
        &["term::ansi::code::macros::menu menu"],
        "The description of a menu is converted into a formatted rectangular block of text, with the menu command characte",
    ),
    (
        "term::ansi::code::macros::frame",
        Arity::exact(1),
        &["term::ansi::code::macros::frame string"],
        "The paragraph of text contained in the string is padded with spaces at the right margin, after normalizing intern",
    ),
];

/// The `term::ansi::ctrl::unix` package.
pub(super) const TERM__ANSI__CTRL__UNIX_CMDS: &[Row] = &[
    (
        "term::ansi::ctrl::unix::raw",
        Arity::exact(0),
        &["term::ansi::ctrl::unix::raw"],
        "This command switches the standard input of the current process to raw input mode.",
    ),
    (
        "term::ansi::ctrl::unix::cooked",
        Arity::exact(0),
        &["term::ansi::ctrl::unix::cooked"],
        "This command switches the standard input of the current process to cooked input mode.",
    ),
    (
        "term::ansi::ctrl::unix::columns",
        Arity::exact(0),
        &["term::ansi::ctrl::unix::columns"],
        "This command queries the terminal connected to the standard input for the number of columns available for display",
    ),
    (
        "term::ansi::ctrl::unix::rows",
        Arity::exact(0),
        &["term::ansi::ctrl::unix::rows"],
        "This command queries the terminal connected to the standard input for the number of rows (aka lines) available fo",
    ),
];

/// The `term::send` package.
pub(super) const TERM__SEND_CMDS: &[Row] = &[
    (
        "term::send::wrch",
        Arity::exact(2),
        &["term::send::wrch chan str"],
        "Send the text str to the channel specified by the handle chan.",
    ),
    (
        "term::send::wr",
        Arity::exact(1),
        &["term::send::wr str"],
        "This convenience command is like ::term::send::wrch, except that the destination channel is fixed to stdout.",
    ),
];
