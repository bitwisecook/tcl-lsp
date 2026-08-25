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

//! `ttk::entry` command.
use crate::prelude::*;

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
        detail: "Variable linked to the entry value.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value("width"),
        detail: "Desired width of the entry in characters.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value("stateSpec"),
        detail: "Widget state (normal, disabled, or readonly).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-show",
        value: OptionValue::value("char"),
        detail: "Character to display instead of actual contents (e.g. for passwords).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-validate",
        value: OptionValue::value("validateMode"),
        detail: "When to run validation (none, focus, focusin, focusout, key, all).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-validatecommand",
        value: OptionValue::deferred_tainted_script(VALIDATION_USER_INPUTS),
        detail: "Script to evaluate for input validation.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-invalidcommand",
        value: OptionValue::deferred_tainted_script(VALIDATION_USER_INPUTS),
        detail: "Script to evaluate when validation fails.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-locale",
        value: OptionValue::value("locale name"),
        detail: "Locale used to determine word and character boundaries (Tk 9.1+).",
        lifecycle: Lifecycle::introduced_in("9.1"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-xscrollcommand",
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix for horizontal scroll communication.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-exportselection",
        value: OptionValue::boolean(),
        detail: "Whether the selection is exported to the X selection.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-font",
        value: OptionValue::value("font"),
        detail: "Font to use for the entry text.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-foreground",
        value: OptionValue::value("colour"),
        detail: "Foreground colour for the entry text.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-justify",
        value: OptionValue::enumerated(super::common::JUSTIFY, true, "justify"),
        detail: "How to justify the text within the entry.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-style",
        value: OptionValue::value("style"),
        detail: "Style to use for the widget.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-class",
        value: OptionValue::value("className"),
        detail: "Widget class name for option-database lookups.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value("cursor"),
        detail: "Cursor to display when the pointer is over the widget.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value("focusSpec"),
        detail: "Whether the widget accepts focus during keyboard traversal.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-placeholder",
        value: OptionValue::value("text"),
        detail: "Help text shown when the entry is empty (Tk 8.7+).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::introduced_in("8.7"),
        min_abbrev: None,
    },
    OptionSpec {
        name: "-placeholderforeground",
        value: OptionValue::value("color"),
        detail: "Foreground colour of the placeholder text (Tk 8.7+).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::introduced_in("8.7"),
        min_abbrev: None,
    },
];

super::common::ttk_widget_class!(
    SUBCOMMANDS,
    CLASS,
    "ttk::entry",
    SubCommand {
        name: "bbox",
        arity: Arity::exact(1),
        detail: "Return the bounding box of the indexed character.",
        synopsis: "pathName bbox index",
        pure: true,
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::new(1, 2),
        detail: "Delete one or more characters.",
        synopsis: "pathName delete first ?last?",
        // With `-validate key`, editing invokes validation (and possibly the
        // invalid callback), so this is a callback-capable mutation.
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
        detail: "Return the entry's current string.",
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
        name: "validate",
        arity: Arity::exact(0),
        detail: "Force revalidation of the current value.",
        synopsis: "pathName validate",
        traits: Traits::EVALUATES_CODE,
        mutator: true,
        return_type: Some(TclType::Boolean),
        side_effects: super::common::TTK_CALLBACK_EFFECTS,
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
    synopsis: "ttk::entry pathName ?options?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::entry",
        traits: Traits::TAINTS_VAR_WRITES,
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed text entry widget.",
            synopsis: &["ttk::entry pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_entry.n",
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
