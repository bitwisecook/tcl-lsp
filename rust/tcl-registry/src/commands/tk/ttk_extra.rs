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

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const CHECKBUTTON_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-command",
        value: OptionValue::script(),
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
        value: OptionValue::var_name(),
        detail: "The name of a global variable whose value is linked to the widget. Defaults to the widget pathname if not.",
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

/// `ttk::checkbutton` widget spec.
fn checkbutton() -> CommandSpec {
    CommandSpec {
        name: "ttk::checkbutton",
        dialects: Some(DialectSet::TK_AND_TCL),
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
        min_version: Some("8.5"),
        warn_missing_import: false,
        options: CHECKBUTTON_OPTS,
        side_effects: SIDE_EFFECTS,
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}

const MENUBUTTON_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-direction",
        value: OptionValue::value("value"),
        detail: "Specifies where the menu is to be popped up relative to the menubutton. One of: above, below, left, right, or.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-menu",
        value: OptionValue::value("value"),
        detail: "Specifies the path name of the menu associated with the menubutton. To be on the safe side, the menu ought to.",
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

/// `ttk::menubutton` widget spec.
fn menubutton() -> CommandSpec {
    CommandSpec {
        name: "ttk::menubutton",
        dialects: Some(DialectSet::TK_AND_TCL),
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
        min_version: Some("8.5"),
        warn_missing_import: false,
        options: MENUBUTTON_OPTS,
        side_effects: SIDE_EFFECTS,
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}

const PANEDWINDOW_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-orient",
        value: OptionValue::value("value"),
        detail: "Specifies the orientation of the window. If vertical, subpanes are stacked top-to-bottom; if horizontal,.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value("value"),
        detail: "If present and greater than zero, specifies the desired width of the widget in pixels. Otherwise, the.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value("value"),
        detail: "If present and greater than zero, specifies the desired height of the widget in pixels. Otherwise, the.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-weight",
        value: OptionValue::value("value"),
        detail: "An integer specifying the relative stretchability of the pane. When the paned window is resized, the extra.",
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

/// `ttk::panedwindow` widget spec.
fn panedwindow() -> CommandSpec {
    CommandSpec {
        name: "ttk::panedwindow",
        dialects: Some(DialectSet::TK_AND_TCL),
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
        min_version: Some("8.5"),
        warn_missing_import: false,
        options: PANEDWINDOW_OPTS,
        side_effects: SIDE_EFFECTS,
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}

const RADIOBUTTON_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-command",
        value: OptionValue::script(),
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
        value: OptionValue::var_name(),
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

/// `ttk::radiobutton` widget spec.
fn radiobutton() -> CommandSpec {
    CommandSpec {
        name: "ttk::radiobutton",
        dialects: Some(DialectSet::TK_AND_TCL),
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
        min_version: Some("8.5"),
        warn_missing_import: false,
        options: RADIOBUTTON_OPTS,
        side_effects: SIDE_EFFECTS,
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}

const SPINBOX_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-command",
        value: OptionValue::script(),
        detail: "Specifies a Tcl command to be invoked whenever a spinbutton is invoked.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-format",
        value: OptionValue::value("value"),
        detail: "Specifies an alternate format to use when setting the string value when using the -from and -to range. This.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-from",
        value: OptionValue::value("value"),
        detail: "A floating-point value specifying the lowest value for the spinbox. This is used in conjunction with -to and.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-increment",
        value: OptionValue::value("value"),
        detail: "A floating-point value specifying the change in value to be applied each time one of the widget spin buttons.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-to",
        value: OptionValue::value("value"),
        detail: "A floating-point value specifying the highest permissible value for the widget. See also -from and.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-values",
        value: OptionValue::value("value"),
        detail: "This must be a Tcl list of values. If this option is set then this will override any range set using the.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-wrap",
        value: OptionValue::value("value"),
        detail: "Must be a proper boolean value. If on, the spinbox will wrap around the values of data in the widget.",
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

/// `ttk::spinbox` widget spec.
fn spinbox() -> CommandSpec {
    CommandSpec {
        name: "ttk::spinbox",
        dialects: Some(DialectSet::TK_AND_TCL),
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
        min_version: Some("8.5"),
        warn_missing_import: false,
        options: SPINBOX_OPTS,
        side_effects: SIDE_EFFECTS,
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
