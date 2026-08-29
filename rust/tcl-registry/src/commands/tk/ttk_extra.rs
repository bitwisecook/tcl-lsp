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

//! Additional themed (Ttk) widgets — `ttk::checkbutton`, `ttk::menubutton`,
//! `ttk::panedwindow`, `ttk::radiobutton`, and `ttk::spinbox`.
//!
//! Widget-specific options and their descriptions are extracted from the
//! Tk 8.6 manual pages; the common widget options (`-class`, `-cursor`,
//! `-style`, `-takefocus`, `-state`) are shared.  Available in Tk 8.5+.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const VALIDATION_USER_INPUTS: &[CallbackTaintInput] = &[
    CallbackTaintInput::TK_PROPOSED_VALUE,
    CallbackTaintInput::TK_CURRENT_VALUE,
    CallbackTaintInput::TK_EDIT_TEXT,
];

const CHECKBUTTON_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-text",
        value: OptionValue::value("string"),
        detail: "Text displayed by the checkbutton.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-textvariable",
        value: OptionValue::global_var_name(),
        detail: "Global variable whose value supplies the displayed label; editing the checkbutton does not change it.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::deferred_script(),
        detail: "A Tcl script to execute whenever the widget is invoked.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-offvalue",
        value: OptionValue::value("value"),
        detail: "The value to store in the associated -variable when the widget is deselected. Defaults to 0.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-onvalue",
        value: OptionValue::value("value"),
        detail: "The value to store in the associated -variable when the widget is selected. Defaults to 1.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-variable",
        value: OptionValue::user_input_var(),
        detail: "Global variable linked to the widget; defaults to the widget pathname when omitted.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-class",
        value: OptionValue::value("className"),
        detail: "Widget class name for option-database lookups.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value("cursor"),
        detail: "Cursor to display when the pointer is over the widget.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-style",
        value: OptionValue::value("style"),
        detail: "Style to use for the widget.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value("focusSpec"),
        detail: "Whether the widget accepts focus during keyboard traversal.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value("stateSpec"),
        detail: "Widget state (normal or disabled).",
        ..OptionSpec::DEFAULT
    },
];

super::common::ttk_widget_class!(
    CHECKBUTTON_METHODS,
    CHECKBUTTON_CLASS,
    "ttk::checkbutton",
    SubCommand {
        name: "invoke",
        arity: Arity::exact(0),
        detail: "Toggle the selection and evaluate the associated command.",
        synopsis: "pathName invoke",
        traits: Traits::EVALUATES_CODE,
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_CALLBACK_EFFECTS,
        ..SubCommand::DEFAULT
    },
);

/// `ttk::checkbutton` widget spec.
fn checkbutton() -> CommandSpec {
    CommandSpec {
        name: "ttk::checkbutton",
        traits: Traits::TAINTS_VAR_WRITES,
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed checkbutton widget.",
            synopsis: &["ttk::checkbutton pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_checkbutton.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        lifecycle: Lifecycle::introduced_in("8.5"),
        warn_missing_import: false,
        options: CHECKBUTTON_OPTS,
        side_effects: SIDE_EFFECTS,
        subcommands: CHECKBUTTON_METHODS,
        object_class: Some(&CHECKBUTTON_CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}

const MENUBUTTON_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-direction",
        value: OptionValue::value("value"),
        detail: "Menu placement: above, below, left, right, or flush.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-menu",
        value: OptionValue::value("value"),
        detail: "Path of the associated menu, preferably a direct child of the menubutton.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-class",
        value: OptionValue::value("className"),
        detail: "Widget class name for option-database lookups.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value("cursor"),
        detail: "Cursor to display when the pointer is over the widget.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-style",
        value: OptionValue::value("style"),
        detail: "Style to use for the widget.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value("focusSpec"),
        detail: "Whether the widget accepts focus during keyboard traversal.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value("stateSpec"),
        detail: "Widget state (normal or disabled).",
        ..OptionSpec::DEFAULT
    },
];

super::common::ttk_widget_class!(MENUBUTTON_METHODS, MENUBUTTON_CLASS, "ttk::menubutton",);

/// `ttk::menubutton` widget spec.
fn menubutton() -> CommandSpec {
    CommandSpec {
        name: "ttk::menubutton",
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed menubutton widget.",
            synopsis: &["ttk::menubutton pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_menubutton.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        lifecycle: Lifecycle::introduced_in("8.5"),
        warn_missing_import: false,
        options: MENUBUTTON_OPTS,
        side_effects: SIDE_EFFECTS,
        subcommands: MENUBUTTON_METHODS,
        object_class: Some(&MENUBUTTON_CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}

const PANEDWINDOW_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-orient",
        value: OptionValue::value("value"),
        detail: "Pane stacking direction: vertical (top-to-bottom) or horizontal (left-to-right).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value("value"),
        detail: "Requested width in pixels; managed windows determine non-positive values.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value("value"),
        detail: "Requested height in pixels; managed windows determine non-positive values.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-weight",
        value: OptionValue::value("value"),
        detail: "Pane's relative share of space added or removed when the widget is resized.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-class",
        value: OptionValue::value("className"),
        detail: "Widget class name for option-database lookups.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value("cursor"),
        detail: "Cursor to display when the pointer is over the widget.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-style",
        value: OptionValue::value("style"),
        detail: "Style to use for the widget.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value("focusSpec"),
        detail: "Whether the widget accepts focus during keyboard traversal.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value("stateSpec"),
        detail: "Widget state (normal or disabled).",
        ..OptionSpec::DEFAULT
    },
];

super::common::ttk_widget_class!(
    PANEDWINDOW_METHODS,
    PANEDWINDOW_CLASS,
    "ttk::panedwindow",
    SubCommand {
        name: "add",
        arity: Arity::at_least(1),
        detail: "Add a child window as a pane.",
        synopsis: "pathName add subwindow ?options?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::exact(1),
        detail: "Remove a pane from the widget.",
        synopsis: "pathName forget pane",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::at_least(2),
        detail: "Insert or move a pane at a position.",
        synopsis: "pathName insert pos subwindow ?options?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pane",
        arity: Arity::at_least(1),
        detail: "Query or modify options of a managed pane.",
        synopsis: "pathName pane pane ?-option ?value ...??",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "panes",
        arity: Arity::exact(0),
        detail: "Return the managed pane windows in order.",
        synopsis: "pathName panes",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sashpos",
        arity: Arity::new(1, 2),
        detail: "Query or set a sash position.",
        synopsis: "pathName sashpos index ?newpos?",
        ..SubCommand::DEFAULT
    },
);

/// `ttk::panedwindow` widget spec.
fn panedwindow() -> CommandSpec {
    CommandSpec {
        name: "ttk::panedwindow",
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed paned-window widget.",
            synopsis: &["ttk::panedwindow pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_panedwindow.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        lifecycle: Lifecycle::introduced_in("8.5"),
        warn_missing_import: false,
        options: PANEDWINDOW_OPTS,
        side_effects: SIDE_EFFECTS,
        subcommands: PANEDWINDOW_METHODS,
        object_class: Some(&PANEDWINDOW_CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}

const RADIOBUTTON_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-text",
        value: OptionValue::value("string"),
        detail: "Text displayed by the radiobutton.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-textvariable",
        value: OptionValue::global_var_name(),
        detail: "Global variable whose value supplies the displayed label; selecting the radiobutton does not change it.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::deferred_script(),
        detail: "A Tcl script to evaluate whenever the widget is invoked.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-value",
        value: OptionValue::value("value"),
        detail: "The value to store in the associated -variable when the widget is selected.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-variable",
        value: OptionValue::user_input_var(),
        detail: "The name of a global variable whose value is linked to the widget. Default value is ::selectedButton.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-class",
        value: OptionValue::value("className"),
        detail: "Widget class name for option-database lookups.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value("cursor"),
        detail: "Cursor to display when the pointer is over the widget.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-style",
        value: OptionValue::value("style"),
        detail: "Style to use for the widget.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value("focusSpec"),
        detail: "Whether the widget accepts focus during keyboard traversal.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value("stateSpec"),
        detail: "Widget state (normal or disabled).",
        ..OptionSpec::DEFAULT
    },
];

super::common::ttk_widget_class!(
    RADIOBUTTON_METHODS,
    RADIOBUTTON_CLASS,
    "ttk::radiobutton",
    SubCommand {
        name: "invoke",
        arity: Arity::exact(0),
        detail: "Select the widget and evaluate the associated command.",
        synopsis: "pathName invoke",
        traits: Traits::EVALUATES_CODE,
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_CALLBACK_EFFECTS,
        ..SubCommand::DEFAULT
    },
);

/// `ttk::radiobutton` widget spec.
fn radiobutton() -> CommandSpec {
    CommandSpec {
        name: "ttk::radiobutton",
        traits: Traits::TAINTS_VAR_WRITES,
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed radiobutton widget.",
            synopsis: &["ttk::radiobutton pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_radiobutton.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        lifecycle: Lifecycle::introduced_in("8.5"),
        warn_missing_import: false,
        options: RADIOBUTTON_OPTS,
        side_effects: SIDE_EFFECTS,
        subcommands: RADIOBUTTON_METHODS,
        object_class: Some(&RADIOBUTTON_CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}

const SPINBOX_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-background",
        value: OptionValue::value("color"),
        detail: "Background colour for the entry field.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-textvariable",
        value: OptionValue::user_input_var(),
        detail: "Variable linked to the spinbox's editable value.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::deferred_script(),
        detail: "Specifies a Tcl command to be invoked whenever a spinbutton is invoked.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-validate",
        value: OptionValue::value("validateMode"),
        detail: "When to run entry validation (none, focus, focusin, focusout, key, all).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-validatecommand",
        value: OptionValue::deferred_tainted_script(VALIDATION_USER_INPUTS),
        detail: "Script to evaluate for entry validation.",
        ..OptionSpec::DEFAULT
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
        name: "-xscrollcommand",
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix for horizontal scrolling.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-format",
        value: OptionValue::value("value"),
        detail: "Floating-point format applied to values from the numeric range.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-from",
        value: OptionValue::value("value"),
        detail: "Lowest numeric value; used with -to and -increment.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-increment",
        value: OptionValue::value("value"),
        detail: "Numeric step added by the up button and subtracted by the down button.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-to",
        value: OptionValue::value("value"),
        detail: "Highest numeric value; used with -from and -increment.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-values",
        value: OptionValue::value("value"),
        detail: "Explicit value list; overrides -from, -to, and -increment.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-wrap",
        value: OptionValue::value("value"),
        detail: "Whether navigation wraps between the first and last values.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-class",
        value: OptionValue::value("className"),
        detail: "Widget class name for option-database lookups.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value("cursor"),
        detail: "Cursor to display when the pointer is over the widget.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-style",
        value: OptionValue::value("style"),
        detail: "Style to use for the widget.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value("focusSpec"),
        detail: "Whether the widget accepts focus during keyboard traversal.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value("stateSpec"),
        detail: "Widget state (normal or disabled).",
        ..OptionSpec::DEFAULT
    },
];

super::common::ttk_widget_class!(
    SPINBOX_METHODS,
    SPINBOX_CLASS,
    "ttk::spinbox",
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
        detail: "Return the spinbox's current value.",
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
        detail: "Set the spinbox string value.",
        synopsis: "pathName set value",
        traits: Traits::EVALUATES_CODE,
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
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

/// `ttk::spinbox` widget spec.
fn spinbox() -> CommandSpec {
    CommandSpec {
        name: "ttk::spinbox",
        traits: Traits::TAINTS_VAR_WRITES,
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed spinbox widget.",
            synopsis: &["ttk::spinbox pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_spinbox.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        lifecycle: Lifecycle::introduced_in("8.5"),
        warn_missing_import: false,
        options: SPINBOX_OPTS,
        side_effects: SIDE_EFFECTS,
        subcommands: SPINBOX_METHODS,
        object_class: Some(&SPINBOX_CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}

/// All additional Ttk widget specs.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        checkbutton(),
        menubutton(),
        panedwindow(),
        radiobutton(),
        spinbox(),
    ]
}
