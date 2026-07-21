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

//! `grid` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "anchor",
        arity: Arity::new(1, 2),
        detail: "Set the anchor point for the grid within the master window.",
        synopsis: "grid anchor master ?anchor?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "bbox",
        arity: Arity::new(1, 5),
        detail: "Return the bounding box of a cell or group of cells.",
        synopsis: "grid bbox master ?column row? ?column2 row2?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "columnconfigure",
        arity: Arity::at_least(2),
        detail: "Query or set column properties of the grid.",
        synopsis: "grid columnconfigure master index ?-option value ...?",
        options: ROWCOLUMN_CONFIGURE_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "configure",
        arity: Arity::at_least(1),
        detail: "Set or query the grid options for one or more slaves.",
        synopsis: "grid configure slave ?slave ...? ?option value ...?",
        options: CONFIGURE_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "content",
        arity: Arity::at_least(1),
        detail: "Return a list of all slaves in the grid for the master (9.0+ name for `slaves`).",
        synopsis: "grid content master ?-option value?",
        dialects: Some(DialectSet::TCL90_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::at_least(1),
        detail: "Remove each slave from the grid for its master.",
        synopsis: "grid forget slave ?slave ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::exact(1),
        detail: "Return a list of the current grid configuration for the slave.",
        synopsis: "grid info slave",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "location",
        arity: Arity::exact(3),
        detail: "Return the column and row containing the screen point x, y.",
        synopsis: "grid location master x y",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "propagate",
        arity: Arity::new(1, 2),
        detail: "Control whether the master computes its geometry from slaves.",
        synopsis: "grid propagate master ?boolean?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        arity: Arity::at_least(1),
        detail: "Remove each slave from the grid, but remember its configuration.",
        synopsis: "grid remove slave ?slave ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "rowconfigure",
        arity: Arity::at_least(2),
        detail: "Query or set row properties of the grid.",
        synopsis: "grid rowconfigure master index ?-option value ...?",
        options: ROWCOLUMN_CONFIGURE_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "size",
        arity: Arity::exact(1),
        detail: "Return the size of the grid as a list of two elements (columns, rows).",
        synopsis: "grid size master",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "slaves",
        arity: Arity::at_least(1),
        detail: "Return a list of all slaves in the grid for the master.",
        synopsis: "grid slaves master ?-option value?",
        options: SLAVES_OPTIONS,
        ..SubCommand::DEFAULT
    },
];

/// `configure` (and the default `grid slave ?slave ...? ?option value ...?`
/// form): the widget-placement options. Distinct from the
/// `columnconfigure`/`rowconfigure` layout options below. Per
/// <https://www.tcl-lang.org/man/tcl8.6/TkCmd/grid.htm>.
const CONFIGURE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-row",
        value: OptionValue::value("n"),
        detail: "Insert the slave so that it occupies the nth row in the grid.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-column",
        value: OptionValue::value("n"),
        detail: "Insert the slave so that it occupies the nth column in the grid.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-rowspan",
        value: OptionValue::value("n"),
        detail: "Insert the slave so that it occupies n rows in the grid.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-columnspan",
        value: OptionValue::value("n"),
        detail: "Insert the slave so that it occupies n columns in the grid.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-sticky",
        value: OptionValue::value("nsew"),
        detail: "Specifies which edges of the cell the slave sticks to (combination of n, s, e, w).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-padx",
        value: OptionValue::value("amount"),
        detail: "Specifies external horizontal padding for the slave.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-pady",
        value: OptionValue::value("amount"),
        detail: "Specifies external vertical padding for the slave.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-ipadx",
        value: OptionValue::value("amount"),
        detail: "Specifies internal horizontal padding for the slave.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-ipady",
        value: OptionValue::value("amount"),
        detail: "Specifies internal vertical padding for the slave.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-in",
        value: OptionValue::value("master"),
        detail: "Insert the slave into the specified master window.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

/// `columnconfigure` and `rowconfigure`: the per-column/per-row layout
/// options (relative weight, minimum size, extra pad, and uniform-group
/// membership). These are a distinct option set from `configure`'s
/// widget-placement options above — do not conflate the two. Per
/// <https://www.tcl-lang.org/man/tcl8.6/TkCmd/grid.htm>.
const ROWCOLUMN_CONFIGURE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-weight",
        value: OptionValue::value("int"),
        detail: "Relative weight for apportioning extra space (columnconfigure/rowconfigure).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-minsize",
        value: OptionValue::value("amount"),
        detail: "Minimum size of the column or row (columnconfigure/rowconfigure).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-pad",
        value: OptionValue::value("amount"),
        detail: "Extra padding for the largest slave (columnconfigure/rowconfigure).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-uniform",
        value: OptionValue::value("group"),
        detail: "Group columns/rows for uniform sizing (columnconfigure/rowconfigure).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

/// `slaves` (aka `content`): filters restricting the returned list to a
/// single row or column — not the full placement option set used by
/// `configure`. Per <https://www.tcl-lang.org/man/tcl8.6/TkCmd/grid.htm>.
const SLAVES_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-row",
        value: OptionValue::value("n"),
        detail: "Insert the slave so that it occupies the nth row in the grid.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-column",
        value: OptionValue::value("n"),
        detail: "Insert the slave so that it occupies the nth column in the grid.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-row",
        value: OptionValue::value("n"),
        detail: "Insert the slave so that it occupies the nth row in the grid.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-column",
        value: OptionValue::value("n"),
        detail: "Insert the slave so that it occupies the nth column in the grid.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-rowspan",
        value: OptionValue::value("n"),
        detail: "Insert the slave so that it occupies n rows in the grid.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-columnspan",
        value: OptionValue::value("n"),
        detail: "Insert the slave so that it occupies n columns in the grid.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-sticky",
        value: OptionValue::value("nsew"),
        detail: "Specifies which edges of the cell the slave sticks to (combination of n, s, e, w).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-padx",
        value: OptionValue::value("amount"),
        detail: "Specifies external horizontal padding for the slave.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-pady",
        value: OptionValue::value("amount"),
        detail: "Specifies external vertical padding for the slave.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-ipadx",
        value: OptionValue::value("amount"),
        detail: "Specifies internal horizontal padding for the slave.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-ipady",
        value: OptionValue::value("amount"),
        detail: "Specifies internal vertical padding for the slave.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-in",
        value: OptionValue::value("master"),
        detail: "Insert the slave into the specified master window.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-weight",
        value: OptionValue::value("int"),
        detail: "Relative weight for apportioning extra space (columnconfigure/rowconfigure).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-minsize",
        value: OptionValue::value("amount"),
        detail: "Minimum size of the column or row (columnconfigure/rowconfigure).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-pad",
        value: OptionValue::value("amount"),
        detail: "Extra padding for the largest slave (columnconfigure/rowconfigure).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-uniform",
        value: OptionValue::value("group"),
        detail: "Group columns/rows for uniform sizing (columnconfigure/rowconfigure).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "grid option arg ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "grid",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Geometry manager that arranges widgets in a grid.",
            synopsis: &[
                "grid slave ?slave ...? ?option value ...?",
                "grid configure slave ?slave ...? ?option value ...?",
                "grid columnconfigure master index ?-option value ...?",
                "grid rowconfigure master index ?-option value ...?",
                "grid bbox master ?column row? ?column2 row2?",
                "grid forget slave ?slave ...?",
                "grid info slave",
                "grid location master x y",
                "grid propagate master ?boolean?",
                "grid size master",
                "grid slaves master ?-option value?",
            ],
            snippet: "",
            source: "Tk man page grid.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
