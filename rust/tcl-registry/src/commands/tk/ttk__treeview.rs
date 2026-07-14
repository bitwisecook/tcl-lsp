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

//! `ttk::treeview` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-columns",
        value: OptionValue::value("columnList"),
        detail: "List of column identifiers.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-displaycolumns",
        value: OptionValue::value("columnList"),
        detail: "List of columns to display, or #all.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value("rows"),
        detail: "Number of rows to display.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-padding",
        value: OptionValue::value("padSpec"),
        detail: "Internal padding around the widget content.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-selectmode",
        value: OptionValue::value("mode"),
        detail: "Selection mode (extended, browse, or none).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-show",
        value: OptionValue::value("components"),
        detail: "Which parts of the treeview to display (tree, headings, or both).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-xscrollcommand",
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix for horizontal scroll communication.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-yscrollcommand",
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix for vertical scroll communication.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-style",
        value: OptionValue::value("style"),
        detail: "Style to use for the widget.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-class",
        value: OptionValue::value("className"),
        detail: "Widget class name for option-database lookups.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value("cursor"),
        detail: "Cursor to display when the pointer is over the widget.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value("focusSpec"),
        detail: "Whether the widget accepts focus during keyboard traversal.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ttk::treeview pathName ?options?",
}];

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "bbox",
        arity: Arity::at_least(1),
        detail: "Return the bounding box of the item, optionally restricted to a column.",
        synopsis: "pathName bbox item ?column?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "children",
        arity: Arity::new(1, 2),
        detail: "Query or replace the list of children of the given item.",
        synopsis: "pathName children item ?newchildren?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "column",
        arity: Arity::at_least(1),
        detail: "Query or modify the options of the specified column.",
        synopsis: "pathName column column ?-option ?value ...??",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(1),
        detail: "Delete the given items and all of their descendants.",
        synopsis: "pathName delete itemList",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "detach",
        arity: Arity::exact(1),
        detail: "Unlink the given items from the tree without deleting them.",
        synopsis: "pathName detach itemList",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Return whether the specified item is present in the tree.",
        synopsis: "pathName exists item",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "focus",
        arity: Arity::new(0, 1),
        detail: "Set the focus item, or return the current focus item.",
        synopsis: "pathName focus ?item?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "heading",
        arity: Arity::at_least(1),
        detail: "Query or modify the heading options for the specified column.",
        synopsis: "pathName heading column ?-option ?value ...??",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "identify",
        arity: Arity::at_least(1),
        detail: "Identify the tree component at the given coordinates.",
        synopsis: "pathName identify ?component? x y",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "index",
        arity: Arity::exact(1),
        detail: "Return the integer index of the item within its parent's children.",
        synopsis: "pathName index item",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::at_least(2),
        detail: "Create a new item as a child of parent at the given index.",
        synopsis: "pathName insert parent index ?-id id? ?options?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "item",
        arity: Arity::at_least(1),
        detail: "Query or modify the options of the specified item.",
        synopsis: "pathName item item ?-option ?value ...??",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "move",
        arity: Arity::exact(3),
        detail: "Move the item to the given position among the children of parent.",
        synopsis: "pathName move item parent index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "next",
        arity: Arity::exact(1),
        detail: "Return the identifier of the item's next sibling.",
        synopsis: "pathName next item",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "parent",
        arity: Arity::exact(1),
        detail: "Return the identifier of the item's parent.",
        synopsis: "pathName parent item",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "prev",
        arity: Arity::exact(1),
        detail: "Return the identifier of the item's previous sibling.",
        synopsis: "pathName prev item",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "see",
        arity: Arity::exact(1),
        detail: "Ensure that the specified item is visible, scrolling if necessary.",
        synopsis: "pathName see item",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "selection",
        arity: Arity::new(0, 2),
        detail: "Query or modify the set of selected items.",
        synopsis: "pathName selection ?selop itemList?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::at_least(1),
        detail: "Query or set the value of a column for the specified item.",
        synopsis: "pathName set item ?column? ?value?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tag",
        arity: Arity::at_least(1),
        detail: "Query or manipulate tags and their bindings and options.",
        synopsis: "pathName tag args",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "instate",
        arity: Arity::new(1, 2),
        detail: "Test whether the widget state matches statespec, optionally running a script.",
        synopsis: "pathName instate statespec ?script?",
        // `script`, when given, runs as `if {[pathName instate statespec]} script`
        // — a real Tcl body. (The LSP has no widget-path → widget-type
        // tracking yet, so this only takes effect for a literal
        // `ttk::treeview instate ...` head, not realistic `$w instate ...`
        // usage; declared correctly regardless, per the registry contract.)
        arg_roles: &[(1, ArgRole::Body)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "state",
        arity: Arity::new(0, 1),
        detail: "Modify or query the widget state.",
        synopsis: "pathName state ?stateSpec?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "xview",
        arity: Arity::at_least(0),
        detail: "Query or change the horizontal position of the view.",
        synopsis: "pathName xview ?args?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "yview",
        arity: Arity::at_least(0),
        detail: "Query or change the vertical position of the view.",
        synopsis: "pathName yview ?args?",
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::treeview",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed hierarchical multicolumn data display widget.",
            synopsis: &["ttk::treeview pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_treeview.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        min_version: Some("8.5"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
