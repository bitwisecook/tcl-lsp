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

//! tcllib data / utility packages — `inifile`, `units`, and `counter`.
//!
//! Public command surface from tcllib `modules/{inifile/ini,units/units,
//! counter/counter}.man`.  Requires Tcl 8.5+.

use crate::prelude::*;

/// Commands that touch persistent state (file handles, counters).
const STATEFUL: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

/// One command in a flat tcllib package.  `stateful` marks a state-mutating
/// command (file / counter side effects); otherwise the command is pure.
fn cmd(
    name: &'static str,
    pkg: &'static str,
    arity: Arity,
    synopsis: &'static [&'static str],
    summary: &'static str,
    stateful: bool,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: if stateful {
            Traits::empty()
        } else {
            Traits::PURE
        },
        arity,
        hover: Some(HoverSnippet::brief(summary, synopsis, "tcllib package")),
        side_effects: if stateful { STATEFUL } else { &[] },
        tcllib_package: Some(pkg),
        required_package: Some(pkg),
        ..CommandSpec::DEFAULT
    }
}

/// A row in a package's command table: name, arity, synopsis, summary, and
/// whether the command mutates state.
type Row = (
    &'static str,
    Arity,
    &'static [&'static str],
    &'static str,
    bool,
);

/// Build specs for a package from its command table.
fn rows(pkg: &'static str, table: &'static [Row]) -> Vec<CommandSpec> {
    table
        .iter()
        .map(|&(name, arity, synopsis, summary, stateful)| {
            cmd(name, pkg, arity, synopsis, summary, stateful)
        })
        .collect()
}

/// The flat `ini::*` commands (all but the option-bearing `ini::open`).
const INI_CMDS: &[Row] = &[
    (
        "ini::close",
        Arity::exact(1),
        &["ini::close ini"],
        "Close an INI handle without saving.",
        true,
    ),
    (
        "ini::commit",
        Arity::exact(1),
        &["ini::commit ini"],
        "Write pending changes back to the INI file.",
        true,
    ),
    (
        "ini::revert",
        Arity::exact(1),
        &["ini::revert ini"],
        "Discard pending changes to an INI handle.",
        true,
    ),
    (
        "ini::filename",
        Arity::exact(1),
        &["ini::filename ini"],
        "Return the filename backing an INI handle.",
        false,
    ),
    (
        "ini::sections",
        Arity::exact(1),
        &["ini::sections ini"],
        "List the sections in an INI file.",
        false,
    ),
    (
        "ini::keys",
        Arity::exact(2),
        &["ini::keys ini section"],
        "List the keys in an INI section.",
        false,
    ),
    (
        "ini::get",
        Arity::exact(2),
        &["ini::get ini section"],
        "Return a section as a key/value list.",
        false,
    ),
    (
        "ini::exists",
        Arity::new(2, 3),
        &["ini::exists ini section ?key?"],
        "Test whether a section or key exists.",
        false,
    ),
    (
        "ini::value",
        Arity::new(3, 4),
        &["ini::value ini section key ?default?"],
        "Return the value of an INI key.",
        false,
    ),
    (
        "ini::set",
        Arity::exact(4),
        &["ini::set ini section key value"],
        "Set the value of an INI key.",
        true,
    ),
    (
        "ini::delete",
        Arity::new(2, 3),
        &["ini::delete ini section ?key?"],
        "Delete an INI section or key.",
        true,
    ),
    (
        "ini::comment",
        Arity::new(2, 4),
        &["ini::comment ini section ?key? ?text?"],
        "Get or set the comment for a section or key.",
        true,
    ),
    (
        "ini::commentchar",
        Arity::new(0, 1),
        &["ini::commentchar ?char?"],
        "Get or set the comment character.",
        false,
    ),
];

/// The `units::*` commands.
const UNITS_CMDS: &[Row] = &[
    (
        "units::convert",
        Arity::exact(2),
        &["units::convert value targetUnits"],
        "Convert a value string into a number scaled to the target units.",
        false,
    ),
    (
        "units::reduce",
        Arity::exact(1),
        &["units::reduce unitString"],
        "Reduce a unit string to a scale factor and its base units.",
        false,
    ),
    (
        "units::new",
        Arity::exact(2),
        &["units::new name baseUnits"],
        "Define a new named unit in terms of existing units.",
        true,
    ),
];

/// The `counter::*` commands.
const COUNTER_CMDS: &[Row] = &[
    (
        "counter::init",
        Arity::at_least(1),
        &["counter::init tag args"],
        "Initialise a counter with the given options.",
        true,
    ),
    (
        "counter::count",
        Arity::new(1, 3),
        &["counter::count tag ?delta? ?instance?"],
        "Increment a counter by a delta (default 1).",
        true,
    ),
    (
        "counter::start",
        Arity::exact(2),
        &["counter::start tag instance"],
        "Start a timing interval for a counter instance.",
        true,
    ),
    (
        "counter::stop",
        Arity::exact(2),
        &["counter::stop tag instance"],
        "Stop a timing interval and record the elapsed time.",
        true,
    ),
    (
        "counter::get",
        Arity::at_least(1),
        &["counter::get tag args"],
        "Return a value or statistic held by a counter.",
        false,
    ),
    (
        "counter::exists",
        Arity::exact(1),
        &["counter::exists tag"],
        "Test whether a counter exists.",
        false,
    ),
    (
        "counter::names",
        Arity::exact(0),
        &["counter::names"],
        "List the names of all defined counters.",
        false,
    ),
    (
        "counter::histHtmlDisplay",
        Arity::at_least(1),
        &["counter::histHtmlDisplay tag args"],
        "Render a counter's histogram as HTML.",
        false,
    ),
    (
        "counter::reset",
        Arity::at_least(1),
        &["counter::reset tag args"],
        "Reset a counter to its initial state.",
        true,
    ),
];

/// Options for `ini::open`.
const INI_OPEN_OPTS: &[OptionSpec] = &[OptionSpec {
    name: "-encoding",
    value: OptionValue::value("encoding"),
    detail: "The character encoding used to read and write the file.",
    ..OptionSpec::DEFAULT
}];

/// The `inifile` package — the option-bearing `ini::open` plus the flat table.
fn inifile_specs() -> Vec<CommandSpec> {
    let mut specs = vec![CommandSpec {
        name: "ini::open",
        traits: Traits::empty(),
        arity: Arity::at_least(1),
        options: INI_OPEN_OPTS,
        hover: Some(HoverSnippet {
            summary: "Open an INI file and return a handle for the other ini commands.",
            synopsis: &["ini::open file ?-encoding encoding? ?access?"],
            snippet: "",
            source: "tcllib inifile package",
            examples: "",
            return_value: "A handle identifying the open INI file.",
        }),
        side_effects: STATEFUL,
        tcllib_package: Some("inifile"),
        required_package: Some("inifile"),
        ..CommandSpec::DEFAULT
    }];
    specs.extend(rows("inifile", INI_CMDS));
    specs
}

/// All `inifile` / `units` / `counter` command specs.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = inifile_specs();
    specs.extend(rows("units", UNITS_CMDS));
    specs.extend(rows("counter", COUNTER_CMDS));
    specs
}
