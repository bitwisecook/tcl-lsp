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

//! `ttk::frame` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-width",
        value: OptionValue::value("width"),
        detail: "Desired width of the frame.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value("height"),
        detail: "Desired height of the frame.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-relief",
        value: OptionValue::enumerated(super::common::RELIEF, true, "relief"),
        detail: "Border relief style for the frame.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-borderwidth",
        value: OptionValue::value("width"),
        detail: "Width of the frame border.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-padding",
        value: OptionValue::value("padSpec"),
        detail: "Internal padding around the frame content.",
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
    synopsis: "ttk::frame pathName ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::frame",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed frame container widget.",
            synopsis: &["ttk::frame pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_frame.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        min_version: Some("8.5"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}
