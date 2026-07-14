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

//! `ttk::notebook` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-width",
        value: OptionValue::value("width"),
        detail: "Desired width of the notebook.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value("height"),
        detail: "Desired height of the notebook.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-padding",
        value: OptionValue::value("padSpec"),
        detail: "Internal padding around the notebook content.",
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
    synopsis: "ttk::notebook pathName ?options?",
}];

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::at_least(1),
        detail: "Add a new tab displaying the given window as a pane.",
        synopsis: "pathName add window ?options?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::exact(1),
        detail: "Remove the tab specified by tabid and unmanage its window.",
        synopsis: "pathName forget tabid",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "hide",
        arity: Arity::exact(1),
        detail: "Hide the tab specified by tabid without removing it.",
        synopsis: "pathName hide tabid",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "identify",
        arity: Arity::at_least(1),
        detail: "Identify the element or tab at the given coordinates.",
        synopsis: "pathName identify ?component? x y",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "index",
        arity: Arity::exact(1),
        detail: "Return the numeric index of the tab specified by tabid.",
        synopsis: "pathName index tabid",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::at_least(2),
        detail: "Insert a tab at the specified position, adding or moving its window.",
        synopsis: "pathName insert pos window ?options?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "select",
        arity: Arity::new(0, 1),
        detail: "Select the given tab, or return the currently selected tab.",
        synopsis: "pathName select ?tabid?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tab",
        arity: Arity::at_least(1),
        detail: "Query or modify the options of the tab specified by tabid.",
        synopsis: "pathName tab tabid ?option? ?value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tabs",
        arity: Arity::exact(0),
        detail: "Return the list of windows managed by the notebook.",
        synopsis: "pathName tabs",
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
        // `ttk::notebook instate ...` head, not realistic `$w instate ...`
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
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::notebook",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed tabbed notebook widget.",
            synopsis: &["ttk::notebook pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_notebook.n",
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
