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

//! `wm` command.
use crate::prelude::*;

/// Dynamic arg-role resolver for `wm protocol`.
///
/// `wm protocol window` and `wm protocol window name` are *query* forms
/// (they return the current handler, or the registered protocols) and
/// carry no script.  Only the full `wm protocol window name command`
/// form registers a handler, supplied as the trailing (third) argument
/// — a deferred script run later from the window manager's event loop
/// (e.g. the `WM_DELETE_WINDOW` callback), so `body_kind` is
/// `Structural`.  Args here are those *after* the `protocol` subcommand
/// word: `window`(0) `name`(1) `command`(2).
fn wm_protocol_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() == 3 {
        vec![(2, ArgRole::Body)]
    } else {
        Vec::new()
    }
}

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "aspect",
        arity: Arity::new(1, 5),
        detail: "Set or query the desired aspect ratio for the window.",
        synopsis: "wm aspect window ?minNumer minDenom maxNumer maxDenom?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "attributes",
        arity: Arity::at_least(1),
        detail: "Set or query platform-specific window attributes.",
        synopsis: "wm attributes window ?option? ?value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "client",
        arity: Arity::new(1, 2),
        detail: "Set or query the WM_CLIENT_MACHINE property.",
        synopsis: "wm client window ?name?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "colormapwindows",
        arity: Arity::new(1, 2),
        detail: "Set or query the WM_COLORMAP_WINDOWS property.",
        synopsis: "wm colormapwindows window ?windowList?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "command",
        arity: Arity::new(1, 2),
        detail: "Set or query the WM_COMMAND property.",
        synopsis: "wm command window ?value?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "deiconify",
        arity: Arity::exact(1),
        detail: "Arrange for the window to be displayed in normal form.",
        synopsis: "wm deiconify window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "focusmodel",
        arity: Arity::new(1, 2),
        detail: "Set or query the focus model for the window.",
        synopsis: "wm focusmodel window ?active|passive?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::exact(1),
        detail: "Make the window no longer managed by wm.",
        synopsis: "wm forget window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "frame",
        dialects: None,
        arity: Arity::exact(1),
        detail: "Return the platform-specific window identifier of the outermost frame.",
        synopsis: "wm frame window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "geometry",
        arity: Arity::new(1, 2),
        detail: "Set or query the geometry of the window.",
        synopsis: "wm geometry window ?newGeometry?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "grid",
        dialects: None,
        arity: Arity::new(1, 5),
        detail: "Set or query the gridding information for the window.",
        synopsis: "wm grid window ?baseWidth baseHeight widthInc heightInc?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "group",
        arity: Arity::new(1, 2),
        detail: "Set or query the group leader for the window.",
        synopsis: "wm group window ?pathName?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "iconbadge",
        arity: Arity::exact(2),
        detail: "Set a badge (a short overlay label) on the window's taskbar/dock icon.",
        synopsis: "wm iconbadge window badge",
        dialects: Some(DialectSet::TCL90_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "iconbitmap",
        arity: Arity::new(1, 2),
        detail: "Set or query the bitmap for when the window is iconified.",
        synopsis: "wm iconbitmap window ?bitmap?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "iconify",
        arity: Arity::exact(1),
        detail: "Arrange for the window to be iconified.",
        synopsis: "wm iconify window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "iconmask",
        arity: Arity::new(1, 2),
        detail: "Set or query the icon mask bitmap.",
        synopsis: "wm iconmask window ?bitmap?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "iconname",
        arity: Arity::new(1, 2),
        detail: "Set or query the icon name for the window.",
        synopsis: "wm iconname window ?newName?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "iconphoto",
        arity: Arity::at_least(2),
        detail: "Set the window icon from one or more photo images.",
        synopsis: "wm iconphoto window ?-default? image1 ?image2 ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "iconposition",
        arity: Arity::new(1, 3),
        detail: "Set or query the icon position hint.",
        synopsis: "wm iconposition window ?x y?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "iconwindow",
        arity: Arity::new(1, 2),
        detail: "Set or query the icon window for the window.",
        synopsis: "wm iconwindow window ?pathName?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "manage",
        arity: Arity::exact(1),
        detail: "Make the window managed by wm.",
        synopsis: "wm manage window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "maxsize",
        arity: Arity::new(1, 3),
        detail: "Set or query the maximum size for the window.",
        synopsis: "wm maxsize window ?width height?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "minsize",
        arity: Arity::new(1, 3),
        detail: "Set or query the minimum size for the window.",
        synopsis: "wm minsize window ?width height?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "overrideredirect",
        arity: Arity::new(1, 2),
        detail: "Set or query the override-redirect flag.",
        synopsis: "wm overrideredirect window ?boolean?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "positionfrom",
        arity: Arity::new(1, 2),
        detail: "Set or query who specified the window position.",
        synopsis: "wm positionfrom window ?who?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "protocol",
        arity: Arity::new(1, 3),
        detail: "Register a handler for a window manager protocol.",
        synopsis: "wm protocol window ?name? ?command?",
        arg_role_resolver: Some(wm_protocol_arg_roles),
        body_kind: BodyKind::Structural,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "resizable",
        arity: Arity::new(1, 3),
        detail: "Set or query whether the window is resizable.",
        synopsis: "wm resizable window ?width height?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sizefrom",
        arity: Arity::new(1, 2),
        detail: "Set or query who specified the window size.",
        synopsis: "wm sizefrom window ?who?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "stackorder",
        arity: Arity::new(1, 3),
        detail: "Return the stacking order or compare two windows.",
        synopsis: "wm stackorder window ?isabove|isbelow window?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "state",
        arity: Arity::new(1, 2),
        detail: "Set or query the current state of the window.",
        synopsis: "wm state window ?newstate?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "title",
        arity: Arity::new(1, 2),
        detail: "Set or query the title of the window.",
        synopsis: "wm title window ?string?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "transient",
        arity: Arity::new(1, 2),
        detail: "Set or query the transient master for the window.",
        synopsis: "wm transient window ?master?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "withdraw",
        arity: Arity::exact(1),
        detail: "Withdraw the window so it is no longer mapped.",
        synopsis: "wm withdraw window",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "wm option window ?arg ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "wm",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet {
            summary: "Communicate with the window manager.",
            synopsis: &[
                "wm aspect window ?minNumer minDenom maxNumer maxDenom?",
                "wm attributes window ?option? ?value ...?",
                "wm client window ?name?",
                "wm colormapwindows window ?windowList?",
                "wm command window ?value?",
                "wm deiconify window",
                "wm focusmodel window ?active|passive?",
                "wm forget window",
                "wm frame window",
                "wm geometry window ?newGeometry?",
                "wm grid window ?baseWidth baseHeight widthInc heightInc?",
                "wm group window ?pathName?",
                "wm iconbitmap window ?bitmap?",
                "wm iconify window",
                "wm iconmask window ?bitmap?",
                "wm iconname window ?newName?",
                "wm iconphoto window ?-default? image1 ?image2 ...?",
                "wm iconposition window ?x y?",
                "wm iconwindow window ?pathName?",
                "wm manage window",
                "wm maxsize window ?width height?",
                "wm minsize window ?width height?",
                "wm overrideredirect window ?boolean?",
                "wm positionfrom window ?who?",
                "wm protocol window ?name? ?command?",
                "wm resizable window ?width height?",
                "wm sizefrom window ?who?",
                "wm stackorder window ?isabove|isbelow window?",
                "wm state window ?newstate?",
                "wm title window ?string?",
                "wm transient window ?master?",
                "wm withdraw window",
            ],
            snippet: "",
            source: "Tk man page wm.n",
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
