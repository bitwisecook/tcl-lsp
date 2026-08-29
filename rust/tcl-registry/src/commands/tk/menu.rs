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

//! `menu` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

/// Options accepted by menu entries (shared by `add`, `insert`, and
/// `entryconfigure`). Not every option applies to every entry type, but the
/// registry lists the full set so option values are typed correctly.
const MENU_ENTRY_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-command",
        value: OptionValue::deferred_script(),
        detail: "Tcl command to invoke when the menu entry is invoked.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-variable",
        value: OptionValue::global_var_name(),
        detail: "Global variable tied to a checkbutton or radiobutton entry.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-label",
        value: OptionValue::value("string"),
        detail: "Text displayed in the menu entry.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value("state"),
        detail: "State of the entry: normal, active, or disabled.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-value",
        value: OptionValue::value("value"),
        detail: "Value stored in the variable when a radiobutton entry is selected.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-onvalue",
        value: OptionValue::value("value"),
        detail: "Value stored in the variable when a checkbutton entry is on.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-offvalue",
        value: OptionValue::value("value"),
        detail: "Value stored in the variable when a checkbutton entry is off.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-accelerator",
        value: OptionValue::value("string"),
        detail: "Accelerator key text displayed at the right of the entry.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-menu",
        value: OptionValue::value("menu"),
        detail: "Submenu posted by a cascade entry.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

/// The command's subcommands.
static SUBCOMMANDS: [SubCommand; 15] = [
    SubCommand {
        name: "activate",
        arity: Arity::exact(1),
        detail: "Activate (highlight) the menu entry at the given index.",
        synopsis: "pathName activate index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "add",
        arity: Arity::at_least(1),
        detail: "Add a new entry to the bottom of the menu.",
        synopsis: "pathName add type ?option value ...?",
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "cascade",
                    detail: "A cascade entry that posts another menu.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "checkbutton",
                    detail: "A checkbutton entry with an on/off indicator.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "command",
                    detail: "A command entry that invokes a Tcl command.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "radiobutton",
                    detail: "A radiobutton entry with a mutual-exclusion indicator.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "separator",
                    detail: "A separator line between groups of entries.",
                    ..ArgValue::DEFAULT
                },
            ],
        )],
        options: MENU_ENTRY_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "clone",
        arity: Arity::new(1, 2),
        detail: "Create a clone of this menu.",
        synopsis: "pathName clone newPathname ?cloneType?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::new(1, 2),
        detail: "Delete menu entries between index1 and index2 inclusive.",
        synopsis: "pathName delete index1 ?index2?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "entrycget",
        arity: Arity::exact(2),
        detail: "Return the value of a configuration option for a menu entry.",
        synopsis: "pathName entrycget index option",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "entryconfigure",
        arity: Arity::at_least(1),
        detail: "Query or modify options of a menu entry.",
        synopsis: "pathName entryconfigure index ?option value ...?",
        options: MENU_ENTRY_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "index",
        arity: Arity::exact(1),
        detail: "Return the numerical index corresponding to the given index.",
        synopsis: "pathName index index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::at_least(2),
        detail: "Insert a new entry before the entry at the given index.",
        synopsis: "pathName insert index type ?option value ...?",
        arg_values: &[(
            1,
            &[
                ArgValue {
                    value: "cascade",
                    detail: "A cascade entry that posts another menu.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "checkbutton",
                    detail: "A checkbutton entry with an on/off indicator.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "command",
                    detail: "A command entry that invokes a Tcl command.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "radiobutton",
                    detail: "A radiobutton entry with a mutual-exclusion indicator.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "separator",
                    detail: "A separator line between groups of entries.",
                    ..ArgValue::DEFAULT
                },
            ],
        )],
        options: MENU_ENTRY_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "invoke",
        arity: Arity::exact(1),
        detail: "Invoke the action of the menu entry at the given index.",
        synopsis: "pathName invoke index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "post",
        arity: Arity::exact(2),
        detail: "Display the menu at the given screen coordinates.",
        synopsis: "pathName post x y",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "postcascade",
        arity: Arity::exact(1),
        detail: "Post the submenu associated with the cascade entry at the given index.",
        synopsis: "pathName postcascade index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "type",
        arity: Arity::exact(1),
        detail: "Return the type of the menu entry at the given index.",
        synopsis: "pathName type index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unpost",
        arity: Arity::exact(0),
        detail: "Unmap the menu so it is no longer displayed.",
        synopsis: "pathName unpost",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "xposition",
        arity: Arity::exact(1),
        detail: "Return the x-coordinate of the leftmost pixel of the entry at index.",
        synopsis: "pathName xposition index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "yposition",
        arity: Arity::exact(1),
        detail: "Return the y-coordinate of the topmost pixel of the entry at the given index.",
        synopsis: "pathName yposition index",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-tearoff",
        value: OptionValue::value(""),
        detail: "Whether the menu should include a tear-off entry at the top.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-title",
        value: OptionValue::value(""),
        detail: "Title string for the tear-off menu window.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-type",
        value: OptionValue::value(""),
        detail: "Type of the menu: menubar, tearoff, or normal.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-bg",
        value: OptionValue::value(""),
        detail: "Shorthand for -background.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-background",
        value: OptionValue::value(""),
        detail: "Background colour of the menu.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-fg",
        value: OptionValue::value(""),
        detail: "Shorthand for -foreground.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-foreground",
        value: OptionValue::value(""),
        detail: "Foreground colour for menu entries.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-font",
        value: OptionValue::value(""),
        detail: "Font to use for text in the menu.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-activebackground",
        value: OptionValue::value(""),
        detail: "Background colour for the active menu entry.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-activeforeground",
        value: OptionValue::value(""),
        detail: "Foreground colour for the active menu entry.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-activeborderwidth",
        value: OptionValue::value(""),
        detail: "Width of the border drawn around active entries.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-disabledforeground",
        value: OptionValue::value(""),
        detail: "Foreground colour for disabled menu entries.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-selectcolor",
        value: OptionValue::value(""),
        detail: "Colour of the indicator for checkbutton and radiobutton entries.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-relief",
        value: OptionValue::enumerated(super::common::RELIEF, true, "relief"),
        detail: "3-D effect: flat, groove, raised, ridge, solid, or sunken.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-borderwidth",
        value: OptionValue::value(""),
        detail: "Width of the border around the menu.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-postcommand",
        value: OptionValue::deferred_script(),
        detail: "Tcl command to invoke just before the menu is posted.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-tearoffcommand",
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix invoked when the menu is torn off (the parent menu path and the torn-off menu path are appended).",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value(""),
        detail: "Cursor to display when the mouse is over the menu.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value(""),
        detail: "Whether the menu accepts focus during keyboard traversal.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "menu pathName ?option value ...?",
    ..FormSpec::DEFAULT
}];

/// `menu`'s instance command dispatches through the same subcommand
/// table as its own constructor spec (see
/// `docs/design/tk-widget-instance-typing.md`).
static MENU_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "menu",
    instance_methods: &SUBCOMMANDS,
    superclasses: &[],
    allow_unknown_methods: false,
    method_prefix_matching: PrefixMatching::Enabled,
};

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "menu",
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a menu widget.",
            synopsis: &["menu pathName ?option value ...?"],
            snippet: "Displays a menu of commands, each of which may be a cascade, checkbutton, command, radiobutton, or separator entry.",
            source: "Tk man page menu.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: &SUBCOMMANDS,
        object_class: Some(&MENU_CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}
