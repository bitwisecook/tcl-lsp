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

//! `report::report` — create a report object.
//
// VERIFIED: tcllib report(n) (report 0.3.1 / 0.4 / 0.5).  Synopsis:
//
//     ::report::report reportName columns ?style "style arg..."?
//
// `package require Tcl 8.5 9` — so the whole package (and this command) is
// gated out of the tcl8.4 dialect by `tcllib_package_dialect_floor`.
//
// The command creates an object command named `reportName` supporting the
// report configuration + rendering methods.  Those methods (`printmatrix`,
// `size`, `pad`, `justify`, the `top`/`data`/… line codes, …) are modelled as
// the [`ObjectClassSpec`] below, so a later `reportName printmatrix …` /
// `$r size 0 10` dispatch resolves its subcommands, option values, arity, and
// hover.  The general `creates_instance_at` factory hook binds `reportName` as
// an instance command of this class.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "report::report reportName columns ?style \"style arg...\"?",
    ..FormSpec::DEFAULT
}];

// ---------------------------------------------------------------------------
// Report object methods — the instance command's subcommands.
// ---------------------------------------------------------------------------

/// The ordered `left|right|both` padding-side values (`pad` method).
const PAD_WHERE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "left",
        detail: "pad on the left of the cell",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "right",
        detail: "pad on the right of the cell",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "both",
        detail: "pad on both sides of the cell",
        ..ArgValue::DEFAULT
    },
];

/// The `left|right|center` justification values (`justify` method).
const JUSTIFY_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "left",
        detail: "left-justify the column",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "right",
        detail: "right-justify the column",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "center",
        detail: "centre the column",
        ..ArgValue::DEFAULT
    },
];

/// The `dyn` size keyword (`size` method) — a dynamically-sized column.
const SIZE_VALUES: &[ArgValue] = &[ArgValue {
    value: "dyn",
    detail: "size the column dynamically to its widest cell",
    ..ArgValue::DEFAULT
}];

/// Operations of a separator line code (`enable` / `disable` / `enabled`
/// additionally to the shared `set` / `get`).
const SEPARATOR_OPS: &[SubSubCommand] = &[
    SubSubCommand {
        name: "set",
        detail: "install the line template",
        synopsis: "<code> set templatedata",
        ..SubSubCommand::DEFAULT
    },
    SubSubCommand {
        name: "get",
        detail: "return the line template",
        synopsis: "<code> get",
        ..SubSubCommand::DEFAULT
    },
    SubSubCommand {
        name: "enable",
        detail: "draw this separator line",
        synopsis: "<code> enable",
        ..SubSubCommand::DEFAULT
    },
    SubSubCommand {
        name: "disable",
        detail: "do not draw this separator line",
        synopsis: "<code> disable",
        ..SubSubCommand::DEFAULT
    },
    SubSubCommand {
        name: "enabled",
        detail: "query whether the separator is drawn",
        synopsis: "<code> enabled",
        ..SubSubCommand::DEFAULT
    },
];

/// Operations of a data / template line code (`set` / `get` only).
const DATA_OPS: &[SubSubCommand] = &[
    SubSubCommand {
        name: "set",
        detail: "install the line template",
        synopsis: "<code> set templatedata",
        ..SubSubCommand::DEFAULT
    },
    SubSubCommand {
        name: "get",
        detail: "return the line template",
        synopsis: "<code> get",
        ..SubSubCommand::DEFAULT
    },
];

/// A separator line-code method (`top`, `topdatasep`, …).
const fn separator_method(name: &'static str, detail: &'static str) -> SubCommand {
    SubCommand {
        name,
        arity: Arity::new(1, 2),
        detail,
        synopsis: "<code> set data | get | enable | disable | enabled",
        sub_subcommands: SEPARATOR_OPS,
        ..SubCommand::DEFAULT
    }
}

/// A data / template line-code method (`topdata`, `data`, `botdata`).
const fn data_method(name: &'static str, detail: &'static str) -> SubCommand {
    SubCommand {
        name,
        arity: Arity::new(1, 2),
        detail,
        synopsis: "<code> set data | get",
        sub_subcommands: DATA_OPS,
        ..SubCommand::DEFAULT
    }
}

/// A plain report method (`columns`, `printmatrix`, …).
const fn plain_method(
    name: &'static str,
    arity: Arity,
    detail: &'static str,
    synopsis: &'static str,
) -> SubCommand {
    SubCommand {
        name,
        arity,
        detail,
        synopsis,
        ..SubCommand::DEFAULT
    }
}

/// The report object's method table.
const REPORT_METHODS: &[SubCommand] = &[
    plain_method(
        "destroy",
        Arity::exact(0),
        "destroy the report object",
        "reportName destroy",
    ),
    plain_method(
        "columns",
        Arity::exact(0),
        "number of columns in the report",
        "reportName columns",
    ),
    // Configuration methods carrying option values.
    SubCommand {
        name: "size",
        arity: Arity::new(1, 2),
        detail: "get/set a column's width (a number or `dyn`)",
        synopsis: "reportName size column ?number|dyn?",
        arg_values: &[(1, SIZE_VALUES)],
        ..SubCommand::DEFAULT
    },
    plain_method(
        "sizes",
        Arity::new(0, 1),
        "get/set all column widths",
        "reportName sizes ?size-list?",
    ),
    SubCommand {
        name: "pad",
        arity: Arity::new(1, 3),
        detail: "get/set a column's padding",
        synopsis: "reportName pad column ?left|right|both ?padstring??",
        arg_values: &[(1, PAD_WHERE_VALUES)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "justify",
        arity: Arity::new(1, 2),
        detail: "get/set a column's justification",
        synopsis: "reportName justify column ?left|right|center?",
        arg_values: &[(1, JUSTIFY_VALUES)],
        ..SubCommand::DEFAULT
    },
    plain_method(
        "tcaption",
        Arity::new(0, 1),
        "get/set the top caption region size",
        "reportName tcaption ?size?",
    ),
    plain_method(
        "bcaption",
        Arity::new(0, 1),
        "get/set the bottom caption region size",
        "reportName bcaption ?size?",
    ),
    plain_method(
        "printmatrix",
        Arity::exact(1),
        "format a matrix using this report",
        "reportName printmatrix matrix",
    ),
    plain_method(
        "printmatrix2channel",
        Arity::exact(2),
        "format a matrix, writing to a channel",
        "reportName printmatrix2channel matrix chan",
    ),
    // Separator line codes.
    separator_method("top", "topmost separator line of the report"),
    separator_method(
        "topdatasep",
        "separator between rows of the top caption region",
    ),
    separator_method(
        "topcapsep",
        "separator between the top caption and data regions",
    ),
    separator_method("datasep", "separator between rows of the data region"),
    separator_method(
        "botcapsep",
        "separator between the data and bottom caption regions",
    ),
    separator_method(
        "botdatasep",
        "separator between rows of the bottom caption region",
    ),
    separator_method("bottom", "bottommost separator line of the report"),
    // Data / template line codes.
    data_method(
        "topdata",
        "template for data lines in the top caption region",
    ),
    data_method("data", "template for data lines in the data region"),
    data_method(
        "botdata",
        "template for data lines in the bottom caption region",
    ),
];

/// The report object class — its instance is dispatched as `reportName
/// <method> …`.  `class_name` equals the factory command name so
/// [`crate::CommandRegistry::object_class`] resolves it directly.
static REPORT_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "report::report",
    instance_methods: REPORT_METHODS,
    superclasses: &[],
    allow_unknown_methods: false,
};

/// Command spec for `report::report`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report::report",
        // `reportName columns ?style "style arg..."?` — at least the name and
        // the column count.
        arity: Arity::at_least(2),
        // arg 0 names the created object command; arg 1 is the column count.
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Value)],
        return_type: Some(TclType::String),
        // The factory binds `reportName` (arg 0) as an object command of
        // `REPORT_CLASS`, so later `reportName <method>` dispatch resolves.
        object_class: Some(&REPORT_CLASS),
        creates_instance_at: Some(0),
        hover: Some(HoverSnippet {
            summary: "Creates a new report object with the given number of columns.",
            synopsis: &["report::report reportName columns ?style \"style arg...\"?"],
            snippet: "`reportName` becomes a command supporting the report configuration and \
                      rendering methods (`printmatrix`, `size`, `pad`, `justify`, the line codes, …). \
                      An optional `style` applies a previously-defined style at creation.",
            source: "tcllib report package",
            examples: "::report::report r 3 style captionedtable 1 \"Header\"",
            return_value: "The name of the new report object (`reportName`).",
        }),
        forms: FORMS,
        tcllib_package: Some("report"),
        required_package: Some("report"),
        ..CommandSpec::DEFAULT
    }
}
