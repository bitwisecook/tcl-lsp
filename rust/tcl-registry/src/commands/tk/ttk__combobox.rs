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

//! `ttk::combobox` command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

const VALIDATION_USER_INPUTS: &[CallbackTaintInput] = &[
    CallbackTaintInput::TK_PROPOSED_VALUE,
    CallbackTaintInput::TK_CURRENT_VALUE,
    CallbackTaintInput::TK_EDIT_TEXT,
];
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-background",
        value: OptionValue::value("color"),
        detail: "Background colour for the entry field.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-textvariable",
        value: OptionValue::user_input_var(),
        detail: "Variable linked to the current combobox value.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-values",
        value: OptionValue::value("valueList"),
        detail: "List of values to display in the drop-down list.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value("width"),
        detail: "Desired width of the combobox in characters.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value("height"),
        detail: "Maximum number of rows in the drop-down listbox.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value("stateSpec"),
        detail: "Widget state (normal, readonly, or disabled).",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-postcommand",
        value: OptionValue::deferred_script(),
        detail: "Script to evaluate just before displaying the drop-down list.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-invalidcommand",
        value: OptionValue::deferred_tainted_script(VALIDATION_USER_INPUTS),
        detail: "Script to evaluate when entry validation fails.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-locale",
        value: OptionValue::value("locale name"),
        detail: "Locale used to determine word and character boundaries (Tk 9.1+).",
        lifecycle: Lifecycle::introduced_in("9.1"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-placeholder",
        value: OptionValue::value("text"),
        detail: "Help text shown while the entry is empty (Tk 8.7+).",
        lifecycle: Lifecycle::introduced_in("8.7"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-placeholderforeground",
        value: OptionValue::value("color"),
        detail: "Foreground colour of placeholder text (Tk 8.7+).",
        lifecycle: Lifecycle::introduced_in("8.7"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-show",
        value: OptionValue::value("char"),
        detail: "Character displayed in place of entry contents.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-validate",
        value: OptionValue::value("validateMode"),
        detail: "When to run entry validation.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-validatecommand",
        value: OptionValue::deferred_tainted_script(VALIDATION_USER_INPUTS),
        detail: "Script to evaluate for entry validation.",
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
        name: "-exportselection",
        value: OptionValue::boolean(),
        detail: "Whether the selection is exported to the X selection.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-justify",
        value: OptionValue::enumerated(super::common::JUSTIFY, true, "justify"),
        detail: "How to justify the text within the combobox.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-font",
        value: OptionValue::value("font"),
        detail: "Font to use for the combobox text.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-foreground",
        value: OptionValue::value("colour"),
        detail: "Foreground colour for the combobox text.",
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

super::common::ttk_widget_class!(
    SUBCOMMANDS,
    CLASS,
    "ttk::combobox",
    SubCommand {
        name: "bbox",
        arity: Arity::exact(1),
        detail: "Return the bounding box of the indexed character.",
        synopsis: "pathName bbox index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "current",
        arity: Arity::new(0, 1),
        detail: "Query or set the index of the current value.",
        synopsis: "pathName current ?newIndex?",
        return_type: Some(TclType::String),
        subcommand_forms: super::common::TAINTED_QUERY_OR_SET_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::new(1, 2),
        detail: "Delete one or more characters.",
        synopsis: "pathName delete first ?last?",
        traits: Traits::EVALUATES_CODE,
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_CALLBACK_EFFECTS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        traits: Traits::TAINT_SOURCE,
        arity: Arity::exact(0),
        detail: "Return the current combobox value.",
        synopsis: "pathName get",
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "icursor",
        arity: Arity::exact(1),
        detail: "Set the insertion cursor position.",
        synopsis: "pathName icursor index",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "index",
        arity: Arity::exact(1),
        detail: "Resolve an entry index to its numeric position.",
        synopsis: "pathName index index",
        pure: true,
        return_type: Some(TclType::Int),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::exact(2),
        detail: "Insert text at the indexed position.",
        synopsis: "pathName insert index string",
        traits: Traits::EVALUATES_CODE,
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_CALLBACK_EFFECTS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "selection",
        arity: Arity::new(1, 3),
        detail: "Query or change the entry selection.",
        synopsis: "pathName selection option ?arg ...?",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        subcommand_forms: super::common::ENTRY_SELECTION_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::exact(1),
        detail: "Set the combobox value.",
        synopsis: "pathName set value",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "xview",
        arity: Arity::at_least(0),
        detail: "Query or change the horizontal view.",
        synopsis: "pathName xview ?args?",
        mutator: true,
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        subcommand_forms: super::common::VIEW_FORMS,
        ..SubCommand::DEFAULT
    },
);

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "ttk::combobox pathName ?options?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::combobox",
        traits: Traits::TAINTS_VAR_WRITES,
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed combobox widget.",
            synopsis: &["ttk::combobox pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_combobox.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        lifecycle: Lifecycle::introduced_in("8.5"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        object_class: Some(&CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}
