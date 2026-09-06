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
use crate::arity::ArityWindow;
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const USER_EVENT_INPUTS: &[CallbackTaintInput] = &[CallbackTaintInput::TK_EVENT_CHAR];
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const SELECT_MODES: &[ArgValue] = &[
    ArgValue {
        value: "none",
        detail: "Do not change selection through the built-in bindings.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "single",
        detail: "Select one item or cell at a time, with keyboard clearing allowed.",
        lifecycle: Lifecycle::introduced_in("9.1"),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "browse",
        detail: "Select one item or cell at a time and follow pointer traversal.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "extended",
        detail: "Select ranges and multiple items or cells with modifier keys.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "multiple",
        detail: "Toggle multiple independent item or cell selections.",
        lifecycle: Lifecycle::introduced_in("9.1"),
        ..ArgValue::DEFAULT
    },
];

const SELECT_TYPES: &[ArgValue] = &[
    ArgValue {
        value: "item",
        detail: "The built-in bindings focus and select complete items.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "cell",
        detail: "The built-in bindings focus and select individual cells.",
        ..ArgValue::DEFAULT
    },
];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-columns",
        value: OptionValue::value("columnList"),
        detail: "List of column identifiers.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-displaycolumns",
        value: OptionValue::value("columnList"),
        detail: "List of columns to display, or #all.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value("rows"),
        detail: "Number of rows to display.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-headingheight",
        value: OptionValue::value("pixels"),
        detail: "Override the automatically calculated heading height in pixels.",
        lifecycle: Lifecycle::introduced_in("9.1"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-padding",
        value: OptionValue::value("padSpec"),
        detail: "Internal padding around the widget content.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-selectmode",
        value: OptionValue::enumerated(SELECT_MODES, true, "mode"),
        detail: "How the built-in bindings manage selection.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-selecttype",
        value: OptionValue::enumerated(SELECT_TYPES, true, "item|cell"),
        detail: "Whether the built-in bindings select complete items or individual cells.",
        lifecycle: Lifecycle::introduced_in("8.7"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-show",
        value: OptionValue::value("components"),
        detail: "Which parts of the treeview to display (tree, headings, or both).",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-striped",
        value: OptionValue::boolean(),
        detail: "Use alternate-line colouring when the current theme supports it.",
        lifecycle: Lifecycle::introduced_in("8.7"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-titlecolumns",
        value: OptionValue::value("count"),
        detail: "Number of leftmost display columns kept fixed while horizontally scrolling.",
        lifecycle: Lifecycle::introduced_in("9.0"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-titleitems",
        value: OptionValue::value("count"),
        detail: "Number of top items kept fixed while vertically scrolling.",
        lifecycle: Lifecycle::introduced_in("9.0"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-rowheight",
        value: OptionValue::value("pixels"),
        detail: "Override the automatically calculated row height in pixels.",
        lifecycle: Lifecycle::introduced_in("9.1"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-xscrollcommand",
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix for horizontal scroll communication.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-yscrollcommand",
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix for vertical scroll communication.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-style",
        value: OptionValue::value("style"),
        detail: "Style to use for the widget.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-class",
        value: OptionValue::value("className"),
        detail: "Widget class name for option-database lookups.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value("cursor"),
        detail: "Cursor to display when the pointer is over the widget.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value("focusSpec"),
        detail: "Whether the widget accepts focus during keyboard traversal.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "ttk::treeview pathName ?options?",
    ..FormSpec::DEFAULT
}];

const COLUMN_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-id",
        value: OptionValue::value("columnName"),
        detail: "Return the column's read-only identifier.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-anchor",
        value: OptionValue::enumerated(super::common::ANCHOR, true, "anchor"),
        detail: "Alignment of cell contents in the column.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-minwidth",
        value: OptionValue::value("pixels"),
        detail: "Minimum permitted width of the column.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-separator",
        value: OptionValue::boolean(),
        detail: "Whether to draw a separator after the column.",
        lifecycle: Lifecycle::introduced_in("9.0"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-stretch",
        value: OptionValue::boolean(),
        detail: "Whether the column stretches when the widget is resized.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value("pixels"),
        detail: "Requested width of the column.",
        ..OptionSpec::DEFAULT
    },
];

const HEADING_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-anchor",
        value: OptionValue::enumerated(super::common::ANCHOR, true, "anchor"),
        detail: "Alignment of the heading contents.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::deferred_script(),
        detail: "Script evaluated when the heading is pressed.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-image",
        value: OptionValue::value("imageName"),
        detail: "Image displayed in the heading.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-text",
        value: OptionValue::value("text"),
        detail: "Text displayed in the heading.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "state",
        value: OptionValue::value("stateSpec"),
        detail: "Query or modify the heading state flags.",
        ..OptionSpec::DEFAULT
    },
];

const ITEM_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-id",
        value: OptionValue::value("itemId"),
        detail: "Return the item's read-only identifier.",
        lifecycle: Lifecycle::introduced_in("9.1"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value("rows"),
        detail: "Height of the item in row-height multiples.",
        lifecycle: Lifecycle::introduced_in("9.0"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-image",
        value: OptionValue::value("imageName"),
        detail: "Image displayed beside the tree-column label.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-imageanchor",
        value: OptionValue::enumerated(super::common::ANCHOR, true, "anchor"),
        detail: "Alignment of the item image relative to its text.",
        lifecycle: Lifecycle::introduced_in("9.0"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-hidden",
        value: OptionValue::boolean(),
        detail: "Whether the item is hidden from the current view.",
        lifecycle: Lifecycle::introduced_in("9.0"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-open",
        value: OptionValue::boolean(),
        detail: "Whether the item's children are displayed.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-tags",
        value: OptionValue::value("tagList"),
        detail: "Tags associated with the item.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-text",
        value: OptionValue::value("text"),
        detail: "Text displayed in the tree column.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-values",
        value: OptionValue::value("valueList"),
        detail: "Values displayed in the data columns.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "state",
        value: OptionValue::value("stateSpec"),
        detail: "Query or modify the item's state flags.",
        lifecycle: Lifecycle::introduced_in("9.1"),
        ..OptionSpec::DEFAULT
    },
];

const INSERT_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-id",
        value: OptionValue::value("itemId"),
        detail: "Explicit identifier for the new item.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value("rows"),
        detail: "Height of the item in row-height multiples.",
        lifecycle: Lifecycle::introduced_in("9.0"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-image",
        value: OptionValue::value("imageName"),
        detail: "Image displayed beside the tree-column label.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-imageanchor",
        value: OptionValue::enumerated(super::common::ANCHOR, true, "anchor"),
        detail: "Alignment of the item image relative to its text.",
        lifecycle: Lifecycle::introduced_in("9.0"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-hidden",
        value: OptionValue::boolean(),
        detail: "Whether the new item is hidden from the current view.",
        lifecycle: Lifecycle::introduced_in("9.0"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-open",
        value: OptionValue::boolean(),
        detail: "Whether the new item's children are displayed.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-tags",
        value: OptionValue::value("tagList"),
        detail: "Tags associated with the new item.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-text",
        value: OptionValue::value("text"),
        detail: "Text displayed in the tree column.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-values",
        value: OptionValue::value("valueList"),
        detail: "Values displayed in the data columns.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "state",
        value: OptionValue::value("stateSpec"),
        detail: "Initial item state flags.",
        lifecycle: Lifecycle::introduced_in("9.1"),
        ..OptionSpec::DEFAULT
    },
];

const SEARCH_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-all",
        value: OptionValue::flag(),
        detail: "Return every match instead of only the first.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-ascii",
        value: OptionValue::flag(),
        detail: "Compare values as Unicode strings (the default).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-backwards",
        value: OptionValue::flag(),
        detail: "Search backwards from the start item or cell.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-cell",
        value: OptionValue::flag(),
        detail: "Return cell identifiers and interpret start and stop as cells.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-columns",
        value: OptionValue::value("columnList"),
        detail: "Limit the search to the listed columns.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-dictionary",
        value: OptionValue::flag(),
        detail: "Use dictionary-style, case-insensitive string comparison.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-exact",
        value: OptionValue::flag(),
        detail: "Match the pattern as an exact string (the default).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-forwards",
        value: OptionValue::flag(),
        detail: "Search forwards from the start item or cell (the default).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-glob",
        value: OptionValue::flag(),
        detail: "Interpret pattern using string-match glob rules.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-hidden",
        value: OptionValue::flag(),
        detail: "Include hidden items and descendants of closed items.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-integer",
        value: OptionValue::flag(),
        detail: "Compare pattern and non-empty cell values as integers.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-nocase",
        value: OptionValue::flag(),
        detail: "Use case-insensitive string matching.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-not",
        value: OptionValue::flag(),
        detail: "Return values that do not match the pattern.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-real",
        value: OptionValue::flag(),
        detail: "Compare pattern and non-empty cell values as floating-point numbers.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-recurse",
        value: OptionValue::flag(),
        detail: "Search all descendants of parent.",
        aliases: &["-recursive"],
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-regexp",
        value: OptionValue::flag(),
        detail: "Interpret pattern as an advanced regular expression.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-start",
        value: OptionValue::value("itemOrCell"),
        detail: "Start at this descendant item or cell.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-stop",
        value: OptionValue::value("itemOrCell"),
        detail: "Stop after searching this descendant item or cell.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-unicode",
        value: OptionValue::flag(),
        detail: "Alias for -ascii.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-wraparound",
        value: OptionValue::flag(),
        detail: "Wrap from one end of the search range to the other when -start is used.",
        ..OptionSpec::DEFAULT
    },
];

const SORT_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-ascii",
        value: OptionValue::flag(),
        detail: "Sort in Unicode character-code order (the default).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-column",
        value: OptionValue::value("column"),
        detail: "Sort using values from the specified column.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Comparison command prefix invoked with the two values appended.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-decreasing",
        value: OptionValue::flag(),
        detail: "Sort from largest to smallest.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-dictionary",
        value: OptionValue::flag(),
        detail: "Use dictionary-style string ordering.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-ignoreempty",
        value: OptionValue::flag(),
        detail: "Permit empty values in integer or real sorts.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-increasing",
        value: OptionValue::flag(),
        detail: "Sort from smallest to largest (the default).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-integer",
        value: OptionValue::flag(),
        detail: "Sort values as integers.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-nocase",
        value: OptionValue::flag(),
        detail: "Ignore case for -ascii sorting.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-real",
        value: OptionValue::flag(),
        detail: "Sort values as floating-point numbers.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-recurse",
        value: OptionValue::flag(),
        detail: "Sort descendants recursively.",
        aliases: &["-recursive"],
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-unicode",
        value: OptionValue::flag(),
        detail: "Alias for -ascii.",
        ..OptionSpec::DEFAULT
    },
];

const TAG_CONFIGURE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-foreground",
        value: OptionValue::value("color"),
        detail: "Text foreground colour for tagged items or cells.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-background",
        value: OptionValue::value("color"),
        detail: "Background colour for tagged items or cells.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-font",
        value: OptionValue::value("font"),
        detail: "Font used for tagged text.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-image",
        value: OptionValue::value("imageName"),
        detail: "Image used for tagged items or cells.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-imageanchor",
        value: OptionValue::enumerated(super::common::ANCHOR, true, "anchor"),
        detail: "Alignment of a tagged image.",
        lifecycle: Lifecycle::introduced_in("9.0"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-padding",
        value: OptionValue::value("amount"),
        detail: "Padding around tagged cell contents.",
        lifecycle: Lifecycle::introduced_in("9.0"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-stripedbackground",
        value: OptionValue::value("color"),
        detail: "Alternate-line background colour when -striped is enabled.",
        lifecycle: Lifecycle::introduced_in("9.0"),
        ..OptionSpec::DEFAULT
    },
];

const TAG_SUBCOMMANDS: &[SubSubCommand] = &[
    SubSubCommand {
        name: "add",
        detail: "Add a tag to a list of items.",
        synopsis: "pathName tag add tagName itemList",
        options: Some(&[]),
        ..SubSubCommand::DEFAULT
    },
    SubSubCommand {
        name: "bind",
        detail: "Query or install a deferred event binding script for a tag.",
        synopsis: "pathName tag bind tagName ?sequence? ?script?",
        options: Some(&[]),
        ..SubSubCommand::DEFAULT
    },
    SubSubCommand {
        name: "cell",
        detail: "Add, test, or remove tags on individual cells.",
        synopsis: "pathName tag cell add|has|remove tagName ?cellList?",
        lifecycle: Lifecycle::introduced_in("9.0"),
        options: Some(&[]),
        ..SubSubCommand::DEFAULT
    },
    SubSubCommand {
        name: "configure",
        detail: "Query or modify a tag's display options.",
        synopsis: "pathName tag configure tagName ?option? ?value option value ...?",
        options: Some(TAG_CONFIGURE_OPTIONS),
        ..SubSubCommand::DEFAULT
    },
    SubSubCommand {
        name: "delete",
        detail: "Delete a tag and its bindings and display information.",
        synopsis: "pathName tag delete tagName",
        options: Some(&[]),
        lifecycle: Lifecycle::introduced_in("9.0"),
        ..SubSubCommand::DEFAULT
    },
    SubSubCommand {
        name: "has",
        detail: "List tagged items or test whether an item has a tag.",
        synopsis: "pathName tag has tagName ?item?",
        options: Some(&[]),
        ..SubSubCommand::DEFAULT
    },
    SubSubCommand {
        name: "names",
        detail: "Return all tags used by the widget.",
        synopsis: "pathName tag names",
        options: Some(&[]),
        ..SubSubCommand::DEFAULT
    },
    SubSubCommand {
        name: "remove",
        detail: "Remove a tag from selected items or every item.",
        synopsis: "pathName tag remove tagName ?itemList?",
        options: Some(&[]),
        ..SubSubCommand::DEFAULT
    },
];

fn treeview_tag_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() == 4 && !args[0].is_empty() && "bind".starts_with(args[0]) {
        vec![(3, ArgRole::Body)]
    } else {
        Vec::new()
    }
}

fn treeview_tag_script_timing(args: &[&str]) -> Vec<(u8, ScriptTiming)> {
    if treeview_tag_arg_roles(args).is_empty() {
        Vec::new()
    } else {
        vec![(3, ScriptTiming::Deferred)]
    }
}

const INDEX_ARITY_WINDOWS: &[ArityWindow] = &[
    ArityWindow {
        lifecycle: Lifecycle::UNSPECIFIED.retired_from("9.1"),
        arity: Arity::exact(1),
    },
    ArityWindow {
        lifecycle: Lifecycle::introduced_in("9.1"),
        arity: Arity::new(1, 2),
    },
];

const SEE_ARITY_WINDOWS: &[ArityWindow] = INDEX_ARITY_WINDOWS;

const SELECTION_ARITY_WINDOWS: &[ArityWindow] = &[
    ArityWindow {
        lifecycle: Lifecycle::UNSPECIFIED.retired_from("9.1"),
        arity: Arity::new(0, 2),
    },
    ArityWindow {
        lifecycle: Lifecycle::introduced_in("9.1"),
        arity: Arity::new(0, 5),
    },
];

const CELL_SELECTION_ARITY_WINDOWS: &[ArityWindow] = &[
    ArityWindow {
        lifecycle: Lifecycle::introduced_in("9.0").retired_from("9.1"),
        arity: Arity::new(0, 3),
    },
    ArityWindow {
        lifecycle: Lifecycle::introduced_in("9.1"),
        arity: Arity::new(0, 5),
    },
];

const SELECTION_FORMS: &[SubCommandForm] = &[
    SubCommandForm {
        name: "query",
        arity: Arity::exact(0),
        traits: Some(Traits::PURE.union(Traits::TAINT_SOURCE_ZERO_ARGS)),
        mutator: Some(false),
        side_effects: Some(super::common::TTK_WIDGET_READS),
        ..SubCommandForm::DEFAULT
    },
    SubCommandForm {
        name: "modify",
        arity: Arity::new(1, 5),
        traits: Some(Traits::empty()),
        mutator: Some(true),
        side_effects: Some(super::common::TTK_WIDGET_READS_WRITES),
        ..SubCommandForm::DEFAULT
    },
];

/// Query-all, query-one, and option/value setter shapes shared by `column`,
/// `heading`, and `item`. The first argument identifies the row being
/// configured; it is not itself a mutation.
const ROW_CONFIGURE_FORMS: &[SubCommandForm] = &[
    SubCommandForm {
        name: "query-all",
        arity: Arity::exact(1),
        traits: Some(Traits::PURE),
        mutator: Some(false),
        side_effects: Some(super::common::TTK_WIDGET_READS),
        ..SubCommandForm::DEFAULT
    },
    SubCommandForm {
        name: "query-one",
        arity: Arity::exact(2),
        traits: Some(Traits::PURE),
        mutator: Some(false),
        side_effects: Some(super::common::TTK_WIDGET_READS),
        ..SubCommandForm::DEFAULT
    },
    SubCommandForm {
        name: "set",
        arity: Arity::stepped(3, Arity::UNLIMITED, 2),
        traits: Some(Traits::CONFIGURES_INSTANCE_OPTIONS),
        mutator: Some(true),
        side_effects: Some(super::common::TTK_WIDGET_READS_WRITES),
        ..SubCommandForm::DEFAULT
    },
];

macro_rules! tag_query_form {
    ($name:literal, $arity:expr, $($word:literal),+ $(,)?) => {
        SubCommandForm {
            name: $name,
            arity: $arity,
            literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&[$($word),+])),
            traits: Some(Traits::PURE),
            mutator: Some(false),
            side_effects: Some(super::common::TTK_WIDGET_READS),
            ..SubCommandForm::DEFAULT
        }
    };
}

macro_rules! tag_mutation_form {
    ($name:literal, $arity:expr, $($word:literal),+ $(,)?) => {
        SubCommandForm {
            name: $name,
            arity: $arity,
            literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&[$($word),+])),
            traits: Some(Traits::empty()),
            mutator: Some(true),
            side_effects: Some(super::common::TTK_WIDGET_READS_WRITES),
            ..SubCommandForm::DEFAULT
        }
    };
}

const TAG_FORMS: &[SubCommandForm] = &[
    tag_mutation_form!("add", Arity::exact(3), "add"),
    tag_query_form!("bind-all", Arity::exact(2), "bind"),
    tag_query_form!("bind-one", Arity::exact(3), "bind"),
    SubCommandForm {
        name: "bind-set",
        arity: Arity::exact(4),
        literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&["bind"])),
        traits: Some(Traits::DEFERS_BODY),
        mutator: Some(true),
        side_effects: Some(super::common::TTK_WIDGET_READS_WRITES),
        ..SubCommandForm::DEFAULT
    },
    tag_mutation_form!("cell-add", Arity::exact(4), "cell", "add"),
    tag_query_form!("cell-has-all", Arity::exact(3), "cell", "has"),
    tag_query_form!("cell-has-one", Arity::exact(4), "cell", "has"),
    tag_mutation_form!("cell-remove-all", Arity::exact(3), "cell", "remove"),
    tag_mutation_form!("cell-remove", Arity::exact(4), "cell", "remove"),
    tag_query_form!("configure-all", Arity::exact(2), "configure"),
    tag_query_form!("configure-one", Arity::exact(3), "configure"),
    tag_mutation_form!(
        "configure-set",
        Arity::stepped(4, Arity::UNLIMITED, 2),
        "configure"
    ),
    tag_mutation_form!("delete", Arity::exact(2), "delete"),
    tag_query_form!("has-all", Arity::exact(2), "has"),
    tag_query_form!("has-one", Arity::exact(3), "has"),
    tag_query_form!("names", Arity::exact(1), "names"),
    tag_mutation_form!("remove-all", Arity::exact(2), "remove"),
    tag_mutation_form!("remove", Arity::exact(3), "remove"),
];

/// The command's subcommands.
static SUBCOMMANDS: [SubCommand; 48] = [
    SubCommand {
        name: "cget",
        arity: Arity::exact(1),
        detail: "Return the current value of a widget option.",
        synopsis: "pathName cget option",
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "configure",
        arity: Arity::at_least(0),
        detail: "Query or change widget options.",
        synopsis: "pathName configure ?option? ?value option value ...?",
        return_type: Some(TclType::String),
        subcommand_forms: super::common::CONFIGURE_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "after",
        arity: Arity::new(1, 3),
        detail: "Return the item after an item in the current view.",
        synopsis: "pathName after ?-hidden? ?-norecurse? item",
        lifecycle: Lifecycle::introduced_in("9.1"),
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "bbox",
        arity: Arity::new(1, 2),
        detail: "Return the bounding box of the item, optionally restricted to a column.",
        synopsis: "pathName bbox item ?column?",
        pure: true,
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "before",
        arity: Arity::new(1, 3),
        detail: "Return the item before an item in the current view.",
        synopsis: "pathName before ?-hidden? ?-norecurse? item",
        lifecycle: Lifecycle::introduced_in("9.1"),
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cellfocus",
        arity: Arity::new(0, 1),
        detail: "Query, set, or clear the focused cell.",
        synopsis: "pathName cellfocus ?cell?",
        lifecycle: Lifecycle::introduced_in("9.1"),
        return_type: Some(TclType::String),
        subcommand_forms: super::common::TAINTED_QUERY_OR_SET_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cellselection",
        arity: Arity::new(0, 5),
        arity_windows: CELL_SELECTION_ARITY_WINDOWS,
        detail: "Query or modify the independent cell selection.",
        synopsis: "pathName cellselection ?selop ?-nohidden? ?-norecurse? arg ...?",
        lifecycle: Lifecycle::introduced_in("9.0"),
        return_type: Some(TclType::String),
        subcommand_forms: SELECTION_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "children",
        arity: Arity::new(1, 2),
        detail: "Query or replace the list of children of the given item.",
        synopsis: "pathName children item ?newchildren?",
        mutator: true,
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "collapse",
        arity: Arity::new(1, 2),
        detail: "Close items, optionally including descendants.",
        synopsis: "pathName collapse ?-recurse? itemList",
        lifecycle: Lifecycle::introduced_in("9.1"),
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "column",
        arity: Arity::at_least(1),
        detail: "Query or modify the options of the specified column.",
        synopsis: "pathName column column ?-option ?value ...??",
        options: COLUMN_OPTIONS,
        return_type: Some(TclType::String),
        subcommand_forms: ROW_CONFIGURE_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "current",
        traits: Traits::TAINT_SOURCE_ZERO_ARGS,
        arity: Arity::exact(0),
        detail: "Return the item and column currently under the pointer.",
        synopsis: "pathName current",
        lifecycle: Lifecycle::introduced_in("9.1"),
        pure: true,
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::exact(1),
        detail: "Delete the given items and all of their descendants.",
        synopsis: "pathName delete itemList",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "depth",
        arity: Arity::exact(1),
        detail: "Return an item's depth below the root.",
        synopsis: "pathName depth item",
        lifecycle: Lifecycle::introduced_in("9.1"),
        pure: true,
        return_type: Some(TclType::Int),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "detach",
        arity: Arity::exact(1),
        detail: "Unlink the given items from the tree without deleting them.",
        synopsis: "pathName detach itemList",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "detached",
        arity: Arity::new(0, 1),
        detail: "List detached items or test whether an item is detached.",
        synopsis: "pathName detached ?-all|item?",
        lifecycle: Lifecycle::introduced_in("9.0"),
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "drag",
        arity: Arity::exact(2),
        detail: "Move a displayed column's right edge to the given x coordinate while resizing.",
        synopsis: "pathName drag column xposition",
        lifecycle: Lifecycle::introduced_in("9.0"),
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "drop",
        arity: Arity::exact(0),
        detail: "Finish an interactive column resize and redistribute column widths.",
        synopsis: "pathName drop",
        lifecycle: Lifecycle::introduced_in("9.0"),
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Return whether the specified item is present in the tree.",
        synopsis: "pathName exists item",
        pure: true,
        return_type: Some(TclType::Boolean),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "expand",
        arity: Arity::new(1, 2),
        detail: "Open items, optionally including descendants.",
        synopsis: "pathName expand ?-recurse? itemList",
        lifecycle: Lifecycle::introduced_in("9.1"),
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "focus",
        arity: Arity::new(0, 1),
        detail: "Set the focus item, or return the current focus item.",
        synopsis: "pathName focus ?item?",
        return_type: Some(TclType::String),
        subcommand_forms: super::common::TAINTED_QUERY_OR_SET_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "haschildren",
        arity: Arity::exact(1),
        detail: "Return whether an item has children.",
        synopsis: "pathName haschildren item",
        lifecycle: Lifecycle::introduced_in("9.1"),
        pure: true,
        return_type: Some(TclType::Boolean),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "heading",
        arity: Arity::at_least(1),
        detail: "Query or modify the heading options for the specified column.",
        synopsis: "pathName heading column ?-option ?value ...??",
        options: HEADING_OPTIONS,
        traits: Traits::DEFERS_BODY,
        return_type: Some(TclType::String),
        subcommand_forms: ROW_CONFIGURE_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "hide",
        arity: Arity::new(1, 2),
        detail: "Hide items, optionally including descendants.",
        synopsis: "pathName hide ?-recurse? itemList",
        lifecycle: Lifecycle::introduced_in("9.1"),
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "id",
        arity: Arity::exact(2),
        detail: "Return the identifier of the child at an index.",
        synopsis: "pathName id item index",
        lifecycle: Lifecycle::introduced_in("9.1"),
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "identify",
        arity: Arity::new(2, 3),
        detail: "Identify the tree component at the given coordinates.",
        synopsis: "pathName identify ?component? x y",
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "identifier",
        arity: Arity::exact(2),
        detail: "Return the child identifier at an index.",
        synopsis: "pathName identifier item index",
        lifecycle: Lifecycle::introduced_in("9.1"),
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "index",
        arity: Arity::new(1, 2),
        arity_windows: INDEX_ARITY_WINDOWS,
        detail: "Return an item's sibling index, or resolve a child index below it.",
        synopsis: "pathName index item ?index?",
        pure: true,
        return_type: Some(TclType::Int),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::at_least(2),
        detail: "Create a new item as a child of parent at the given index.",
        synopsis: "pathName insert parent index ?-id id? ?options?",
        options: INSERT_OPTIONS,
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "item",
        arity: Arity::at_least(1),
        detail: "Query or modify the options of the specified item.",
        synopsis: "pathName item item ?-option ?value ...??",
        options: ITEM_OPTIONS,
        return_type: Some(TclType::String),
        subcommand_forms: ROW_CONFIGURE_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "move",
        arity: Arity::exact(3),
        detail: "Move an item below a parent at an index, or before or after another item.",
        synopsis: "pathName move item parent index | pathName move item before|after otherItem",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "next",
        arity: Arity::exact(1),
        detail: "Return the identifier of the item's next sibling.",
        synopsis: "pathName next item",
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "parent",
        arity: Arity::exact(1),
        detail: "Return the identifier of the item's parent.",
        synopsis: "pathName parent item",
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "prev",
        arity: Arity::exact(1),
        detail: "Return the identifier of the item's previous sibling.",
        synopsis: "pathName prev item",
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "range",
        arity: Arity::new(2, 4),
        detail: "Return the inclusive view range between two items.",
        synopsis: "pathName range ?-hidden? ?-norecurse? first last",
        lifecycle: Lifecycle::introduced_in("9.1"),
        pure: true,
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "search",
        arity: Arity::new(2, 23),
        detail: "Search an item's children using comparison and traversal options.",
        synopsis: "pathName search parent ?options? pattern",
        options: SEARCH_OPTIONS,
        lifecycle: Lifecycle::introduced_in("9.1"),
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "see",
        arity: Arity::new(1, 2),
        arity_windows: SEE_ARITY_WINDOWS,
        detail: "Open and scroll to an item and, in Tk 9.1+, optionally a column.",
        synopsis: "pathName see item ?column?",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "selection",
        arity: Arity::new(0, 5),
        arity_windows: SELECTION_ARITY_WINDOWS,
        detail: "Query or modify the set of selected items.",
        synopsis: "pathName selection ?selop ?-nohidden? ?-norecurse? arg ...?",
        return_type: Some(TclType::String),
        subcommand_forms: SELECTION_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::stepped(1, Arity::UNLIMITED, 2).with_also_exact(2),
        detail: "Query all values or one column, or set one or more column/value pairs.",
        synopsis: "pathName set item ?column? ?value? ?column value ...?",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "size",
        arity: Arity::new(1, 3),
        detail: "Count an item's children with optional traversal controls.",
        synopsis: "pathName size ?-hidden? ?-recurse? item",
        lifecycle: Lifecycle::introduced_in("9.1"),
        pure: true,
        return_type: Some(TclType::Int),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sort",
        arity: Arity::new(1, 20),
        detail: "Sort an item's children using comparison and traversal options.",
        synopsis: "pathName sort parent ?options?",
        traits: Traits::EVALUATES_CODE,
        options: SORT_OPTIONS,
        lifecycle: Lifecycle::introduced_in("9.1"),
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_CALLBACK_EFFECTS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tag",
        arity: Arity::at_least(1),
        detail: "Query or manipulate tags and their bindings and options.",
        synopsis: "pathName tag add|bind|cell|configure|delete|has|names|remove ?arg ...?",
        traits: Traits::DEFERS_BODY,
        arg_role_resolver: Some(treeview_tag_arg_roles),
        arg_role_resolver_roles: &[ArgRole::Body],
        script_timing_resolver: Some(treeview_tag_script_timing),
        callback_taint_inputs: &[(3, USER_EVENT_INPUTS)],
        body_kind: BodyKind::Structural,
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        sub_subcommands: TAG_SUBCOMMANDS,
        subcommand_forms: TAG_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "instate",
        arity: Arity::new(1, 2),
        detail: "Test whether the widget state matches statespec, optionally running a script.",
        synopsis: "pathName instate statespec ?script?",
        arg_roles: &[(1, ArgRole::Body)],
        traits: Traits::EVALUATES_CODE,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_INSTATE_EFFECTS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "state",
        arity: Arity::new(0, 1),
        detail: "Modify or query the widget state.",
        synopsis: "pathName state ?stateSpec?",
        return_type: Some(TclType::List),
        subcommand_forms: super::common::QUERY_OR_SET_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "style",
        arity: Arity::exact(0),
        detail: "Return the widget's current style.",
        synopsis: "pathName style",
        lifecycle: Lifecycle::introduced_in("9.0"),
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unhide",
        arity: Arity::new(1, 2),
        detail: "Unhide items, optionally including descendants.",
        synopsis: "pathName unhide ?-recurse? itemList",
        lifecycle: Lifecycle::introduced_in("9.1"),
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "visible",
        arity: Arity::exact(1),
        detail: "Return whether an item is visible in the current view.",
        synopsis: "pathName visible item",
        lifecycle: Lifecycle::introduced_in("9.1"),
        pure: true,
        return_type: Some(TclType::Boolean),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "xview",
        arity: Arity::new(0, 3),
        detail: "Query or change the horizontal position of the view.",
        synopsis: "pathName xview | pathName xview moveto fraction | pathName xview scroll number units|pages",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        subcommand_forms: super::common::VIEW_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "yview",
        arity: Arity::new(0, 3),
        detail: "Query or change the vertical position of the view.",
        synopsis: "pathName yview | pathName yview moveto fraction | pathName yview scroll number units|pages",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        subcommand_forms: super::common::VIEW_FORMS,
        ..SubCommand::DEFAULT
    },
];

/// `ttk::treeview`'s instance command (`.t instate …`, `.t tag …`)
/// dispatches through the same subcommand table as its own constructor
/// spec, so `object_class` is self-referential rather than naming a
/// separate class (see `docs/design/tk-widget-instance-typing.md`).
static TTK_TREEVIEW_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "ttk::treeview",
    instance_methods: &SUBCOMMANDS,
    superclasses: &[],
    allow_unknown_methods: false,
    method_prefix_matching: PrefixMatching::Enabled,
};

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::treeview",
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed hierarchical multicolumn data display widget.",
            synopsis: &["ttk::treeview pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_treeview.n",
            examples: "ttk::treeview .files -columns {size modified} -selecttype cell",
            return_value: "Returns pathName, the command name of the new treeview widget.",
        }),
        required_package: Some("Tk"),
        lifecycle: Lifecycle::introduced_in("8.5"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: &SUBCOMMANDS,
        object_class: Some(&TTK_TREEVIEW_CLASS),
        creates_instance_at: Some(0),
        return_type: Some(TclType::String),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub(name: &str) -> &'static SubCommand {
        SUBCOMMANDS
            .iter()
            .find(|sub| sub.name == name)
            .unwrap_or_else(|| panic!("missing treeview subcommand {name}"))
    }

    fn option<'a>(options: &'a [OptionSpec], name: &str) -> &'a OptionSpec {
        options
            .iter()
            .find(|option| option.name == name)
            .unwrap_or_else(|| panic!("missing treeview option {name}"))
    }

    fn arity_at(sub: &SubCommand, version: &str) -> Arity {
        ArityWindow::select(sub.arity_windows, Some(version)).map_or(sub.arity, |w| w.arity)
    }

    #[test]
    fn treeview_options_follow_the_tk_90_and_91_surface() {
        for name in ["-selecttype", "-striped", "-titlecolumns", "-titleitems"] {
            let option = option(OPTIONS, name);
            assert!(!option.available_for_version(Some("8.6")), "{name}");
            assert!(option.available_for_version(Some("9.0")), "{name}");
        }
        for name in ["-headingheight", "-rowheight"] {
            let option = option(OPTIONS, name);
            assert!(!option.available_for_version(Some("9.0")), "{name}");
            assert!(option.available_for_version(Some("9.1")), "{name}");
        }

        let OptionValue::Takes(select_mode) = option(OPTIONS, "-selectmode").value else {
            panic!("-selectmode takes an enumerated value")
        };
        assert!(select_mode.closed);
        let values = select_mode.values;
        assert_eq!(
            values.iter().map(|value| value.value).collect::<Vec<_>>(),
            ["none", "single", "browse", "extended", "multiple"]
        );
        for name in ["single", "multiple"] {
            let value = values.iter().find(|value| value.value == name).unwrap();
            assert!(!value.available_for_version(Some("9.0")), "{name}");
            assert!(value.available_for_version(Some("9.1")), "{name}");
        }

        assert!(option(COLUMN_OPTIONS, "-id").available_for_version(Some("8.6")));
        let separator = option(COLUMN_OPTIONS, "-separator");
        assert!(!separator.available_for_version(Some("8.6")));
        assert!(separator.available_for_version(Some("9.0")));

        for name in ["-height", "-hidden", "-imageanchor"] {
            let option = option(ITEM_OPTIONS, name);
            assert!(!option.available_for_version(Some("8.6")), "{name}");
            assert!(option.available_for_version(Some("9.0")), "{name}");
        }
        for name in ["-id", "state"] {
            let option = option(ITEM_OPTIONS, name);
            assert!(!option.available_for_version(Some("9.0")), "{name}");
            assert!(option.available_for_version(Some("9.1")), "{name}");
        }
        assert!(option(HEADING_OPTIONS, "state").available_for_version(Some("8.6")));
    }

    #[test]
    fn treeview_subcommands_and_versioned_signatures_match_tk() {
        assert_eq!(SUBCOMMANDS.len(), 48);
        for name in ["drag", "drop"] {
            let sub = sub(name);
            assert!(!sub.available_for_version(Some("8.6")), "{name}");
            assert!(sub.available_for_version(Some("9.0")), "{name}");
        }
        for name in ["id", "identifier", "search", "sort"] {
            let sub = sub(name);
            assert!(!sub.available_for_version(Some("9.0")), "{name}");
            assert!(sub.available_for_version(Some("9.1")), "{name}");
        }

        let index = sub("index");
        assert!(arity_at(index, "9.0").accepts(1));
        assert!(!arity_at(index, "9.0").accepts(2));
        assert!(arity_at(index, "9.1").accepts(2));

        let see = sub("see");
        assert!(!arity_at(see, "9.0").accepts(2));
        assert!(arity_at(see, "9.1").accepts(2));

        let selection = sub("selection");
        assert!(!arity_at(selection, "9.0").accepts(5));
        assert!(arity_at(selection, "9.1").accepts(5));
        let cellselection = sub("cellselection");
        assert!(arity_at(cellselection, "9.0").accepts(3));
        assert!(!arity_at(cellselection, "9.0").accepts(4));
        assert!(arity_at(cellselection, "9.1").accepts(5));

        assert!(
            sub("set").arity.accepts(7),
            "set accepts repeated column/value pairs"
        );
        assert!(sub("set").arity.accepts(1), "set queries all values");
        assert!(sub("set").arity.accepts(2), "set queries one column");
        assert!(sub("set").arity.accepts(5), "set accepts two pairs");
        assert!(
            !sub("set").arity.accepts(4),
            "set rejects an incomplete second column/value pair"
        );
        assert!(
            !sub("delete").arity.accepts(2),
            "delete takes one item-list word"
        );
        assert_eq!(sub("drag").arity, Arity::exact(2));
        assert_eq!(sub("drop").arity, Arity::exact(0));
    }

    #[test]
    fn treeview_search_sort_and_tag_callbacks_are_structured() {
        let search = sub("search");
        assert_eq!(
            search
                .options
                .iter()
                .map(|option| option.name)
                .collect::<Vec<_>>(),
            [
                "-all",
                "-ascii",
                "-backwards",
                "-cell",
                "-columns",
                "-dictionary",
                "-exact",
                "-forwards",
                "-glob",
                "-hidden",
                "-integer",
                "-nocase",
                "-not",
                "-real",
                "-recurse",
                "-regexp",
                "-start",
                "-stop",
                "-unicode",
                "-wraparound",
            ]
        );

        let sort = sub("sort");
        let command = option(sort.options, "-command");
        let OptionValue::Takes(command) = command.value else {
            panic!("sort -command takes a command prefix")
        };
        assert_eq!(command.role, ArgRole::CommandPrefix);
        assert_eq!(command.appended_arity, AppendedArity::Exactly(2));
        assert!(sort.traits.contains(Traits::EVALUATES_CODE));
        assert!(sort.side_effects.iter().any(|effect| {
            effect.target == SideEffectTarget::Unknown && effect.reads && effect.writes
        }));
        let configure = sub("configure");
        assert!(configure.traits.is_empty());
        assert!(configure.subcommand_forms.iter().any(|form| {
            form.name == "set"
                && form
                    .traits
                    .is_some_and(|traits| traits.contains(Traits::CONFIGURES_INSTANCE_OPTIONS))
        }));

        let tag = sub("tag");
        assert_eq!(tag.body_kind, BodyKind::Structural);
        assert!(tag.traits.contains(Traits::DEFERS_BODY));
        assert_eq!(
            tag.sub_subcommands
                .iter()
                .map(|sub| sub.name)
                .collect::<Vec<_>>(),
            [
                "add",
                "bind",
                "cell",
                "configure",
                "delete",
                "has",
                "names",
                "remove"
            ]
        );
        let roles = tag.arg_role_resolver.unwrap()(&["bind", "warning", "<Button-1>", "puts %x"]);
        assert_eq!(roles, vec![(3, ArgRole::Body)]);
        assert_eq!(
            tag.arg_role_resolver.unwrap()(&["b", "warning", "<Button-1>", "puts %x"]),
            vec![(3, ArgRole::Body)],
            "unique-prefix tag bind is executable too"
        );
        assert!(tag.arg_role_resolver.unwrap()(&["bind", "warning", "<Button-1>"]).is_empty());
        let scope = tag.option_scope(Some("configure"), None, Some("9.1"), spec().surface);
        assert_eq!(scope.sub_subcommand, Some("configure"));
        assert!(
            scope
                .options
                .iter()
                .any(|option| option.name == "-foreground")
        );
    }

    #[test]
    fn treeview_reads_and_mutations_have_explicit_semantics() {
        for name in [
            "cget", "bbox", "exists", "index", "search", "size", "visible",
        ] {
            let sub = sub(name);
            assert!(sub.pure, "{name}");
            assert!(!sub.side_effects.is_empty(), "{name}");
            assert!(sub.side_effects.iter().any(|effect| effect.reads), "{name}");
            assert!(
                !sub.side_effects.iter().any(|effect| effect.writes),
                "{name}"
            );
            assert!(sub.return_type.is_some(), "{name}");
        }
        for name in ["insert", "move", "sort", "tag", "xview"] {
            let sub = sub(name);
            assert!(sub.mutator, "{name}");
            assert!(
                sub.side_effects.iter().any(|effect| effect.writes),
                "{name}"
            );
            assert!(sub.return_type.is_some(), "{name}");
        }
        for name in [
            "configure",
            "selection",
            "cellselection",
            "focus",
            "cellfocus",
        ] {
            let sub = sub(name);
            assert!(!sub.mutator, "neutral parent row: {name}");
            assert!(sub.side_effects.is_empty(), "neutral parent row: {name}");
            assert!(!sub.subcommand_forms.is_empty(), "form refinement: {name}");
        }
    }
}
