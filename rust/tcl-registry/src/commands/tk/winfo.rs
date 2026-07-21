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

//! `winfo` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "atom",
        arity: Arity::new(1, 3),
        detail: "Return the integer identifier for the atom given by name.",
        synopsis: "winfo atom ?-displayof window? name",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "atomname",
        arity: Arity::new(1, 3),
        detail: "Return the textual name for the atom given by integer id.",
        synopsis: "winfo atomname ?-displayof window? id",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cells",
        arity: Arity::exact(1),
        detail: "Return the number of cells in the colour map for the window.",
        synopsis: "winfo cells window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "children",
        arity: Arity::exact(1),
        detail: "Return a list of the path names of the window's children.",
        synopsis: "winfo children window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "class",
        arity: Arity::exact(1),
        detail: "Return the class name of the window.",
        synopsis: "winfo class window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "colormapfull",
        arity: Arity::exact(1),
        detail: "Return 1 if the colour map for the window is full, 0 otherwise.",
        synopsis: "winfo colormapfull window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "containing",
        arity: Arity::new(2, 4),
        detail: "Return the path name of the window containing the point.",
        synopsis: "winfo containing ?-displayof window? rootX rootY",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "depth",
        arity: Arity::exact(1),
        detail: "Return the number of bits per pixel in the window.",
        synopsis: "winfo depth window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Return 1 if the window exists, 0 otherwise.",
        synopsis: "winfo exists window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "fpixels",
        arity: Arity::exact(2),
        detail: "Return the number of pixels corresponding to the distance as a float.",
        synopsis: "winfo fpixels window number",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "geometry",
        arity: Arity::exact(1),
        detail: "Return the geometry of the window in WxH+X+Y format.",
        synopsis: "winfo geometry window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "height",
        arity: Arity::exact(1),
        detail: "Return the height of the window in pixels.",
        synopsis: "winfo height window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "id",
        arity: Arity::exact(1),
        detail: "Return the platform-specific window identifier.",
        synopsis: "winfo id window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "interps",
        arity: Arity::new(0, 2),
        detail: "Return a list of all Tcl interpreters on the display.",
        synopsis: "winfo interps ?-displayof window?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "ismapped",
        arity: Arity::exact(1),
        detail: "Return 1 if the window is currently mapped, 0 otherwise.",
        synopsis: "winfo ismapped window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "manager",
        arity: Arity::exact(1),
        detail: "Return the name of the geometry manager for the window.",
        synopsis: "winfo manager window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "name",
        arity: Arity::exact(1),
        detail: "Return the window's name (last component of its path).",
        synopsis: "winfo name window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "parent",
        arity: Arity::exact(1),
        detail: "Return the path name of the window's parent.",
        synopsis: "winfo parent window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pathname",
        arity: Arity::new(1, 3),
        detail: "Return the path name of the window whose identifier is id.",
        synopsis: "winfo pathname ?-displayof window? id",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pixels",
        arity: Arity::exact(2),
        detail: "Return the number of pixels corresponding to the distance.",
        synopsis: "winfo pixels window number",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pointerx",
        arity: Arity::exact(1),
        detail: "Return the x-coordinate of the mouse pointer on the screen.",
        synopsis: "winfo pointerx window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pointerxy",
        arity: Arity::exact(1),
        detail: "Return the x and y coordinates of the mouse pointer.",
        synopsis: "winfo pointerxy window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pointery",
        arity: Arity::exact(1),
        detail: "Return the y-coordinate of the mouse pointer on the screen.",
        synopsis: "winfo pointery window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "reqheight",
        arity: Arity::exact(1),
        detail: "Return the requested height of the window in pixels.",
        synopsis: "winfo reqheight window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "reqwidth",
        arity: Arity::exact(1),
        detail: "Return the requested width of the window in pixels.",
        synopsis: "winfo reqwidth window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "rgb",
        arity: Arity::exact(2),
        detail: "Return the RGB values for a colour in the window.",
        synopsis: "winfo rgb window color",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "rootx",
        arity: Arity::exact(1),
        detail: "Return the x-coordinate of the upper-left corner in the root window.",
        synopsis: "winfo rootx window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "rooty",
        arity: Arity::exact(1),
        detail: "Return the y-coordinate of the upper-left corner in the root window.",
        synopsis: "winfo rooty window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "screen",
        arity: Arity::exact(1),
        detail: "Return the display and screen of the window.",
        synopsis: "winfo screen window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "screencells",
        arity: Arity::exact(1),
        detail: "Return the number of cells in the default colour map for the screen.",
        synopsis: "winfo screencells window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "screendepth",
        arity: Arity::exact(1),
        detail: "Return the number of bits per pixel on the screen.",
        synopsis: "winfo screendepth window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "screenheight",
        arity: Arity::exact(1),
        detail: "Return the height of the screen in pixels.",
        synopsis: "winfo screenheight window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "screenmmheight",
        arity: Arity::exact(1),
        detail: "Return the height of the screen in millimetres.",
        synopsis: "winfo screenmmheight window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "screenmmwidth",
        arity: Arity::exact(1),
        detail: "Return the width of the screen in millimetres.",
        synopsis: "winfo screenmmwidth window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "screenvisual",
        arity: Arity::exact(1),
        detail: "Return the visual class of the screen.",
        synopsis: "winfo screenvisual window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "screenwidth",
        arity: Arity::exact(1),
        detail: "Return the width of the screen in pixels.",
        synopsis: "winfo screenwidth window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "server",
        arity: Arity::exact(1),
        detail: "Return information about the display server.",
        synopsis: "winfo server window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "toplevel",
        dialects: None,
        arity: Arity::exact(1),
        detail: "Return the path name of the top-level window containing the window.",
        synopsis: "winfo toplevel window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "viewable",
        arity: Arity::exact(1),
        detail: "Return 1 if the window and all ancestors are mapped.",
        synopsis: "winfo viewable window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "visual",
        arity: Arity::exact(1),
        detail: "Return the visual class of the window.",
        synopsis: "winfo visual window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "visualid",
        arity: Arity::exact(1),
        detail: "Return the X identifier for the visual of the window.",
        synopsis: "winfo visualid window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "visualsavailable",
        arity: Arity::new(1, 2),
        detail: "Return a list of available visual classes for the screen.",
        synopsis: "winfo visualsavailable window ?includeids?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vrootheight",
        arity: Arity::exact(1),
        detail: "Return the height of the virtual root window.",
        synopsis: "winfo vrootheight window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vrootwidth",
        arity: Arity::exact(1),
        detail: "Return the width of the virtual root window.",
        synopsis: "winfo vrootwidth window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vrootx",
        arity: Arity::exact(1),
        detail: "Return the x offset of the virtual root window.",
        synopsis: "winfo vrootx window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vrooty",
        arity: Arity::exact(1),
        detail: "Return the y offset of the virtual root window.",
        synopsis: "winfo vrooty window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "width",
        arity: Arity::exact(1),
        detail: "Return the width of the window in pixels.",
        synopsis: "winfo width window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "x",
        arity: Arity::exact(1),
        detail: "Return the x-coordinate of the upper-left corner of the window.",
        synopsis: "winfo x window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "y",
        arity: Arity::exact(1),
        detail: "Return the y-coordinate of the upper-left corner of the window.",
        synopsis: "winfo y window",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "winfo option ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "winfo",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Return window-related information.",
            synopsis: &[
                "winfo atom ?-displayof window? name",
                "winfo atomname ?-displayof window? id",
                "winfo cells window",
                "winfo children window",
                "winfo class window",
                "winfo colormapfull window",
                "winfo containing ?-displayof window? rootX rootY",
                "winfo depth window",
                "winfo exists window",
                "winfo geometry window",
                "winfo height window",
                "winfo id window",
                "winfo ismapped window",
                "winfo manager window",
                "winfo name window",
                "winfo parent window",
                "winfo pathname ?-displayof window? id",
                "winfo pixels window number",
                "winfo reqheight window",
                "winfo reqwidth window",
                "winfo rootx window",
                "winfo rooty window",
                "winfo screen window",
                "winfo screenheight window",
                "winfo screenwidth window",
                "winfo toplevel window",
                "winfo viewable window",
                "winfo visual window",
                "winfo width window",
                "winfo x window",
                "winfo y window",
            ],
            snippet: "",
            source: "Tk man page winfo.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
