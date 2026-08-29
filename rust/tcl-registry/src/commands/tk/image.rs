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

//! `image` command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "create",
        arity: Arity::at_least(1),
        detail: "Create a new image of the given type (photo or bitmap).",
        synopsis: "image create type ?name? ?option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(0),
        detail: "Delete one or more images by name.",
        synopsis: "image delete ?name name ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "height",
        arity: Arity::exact(1),
        detail: "Return the height of the image in pixels.",
        synopsis: "image height name",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "inuse",
        arity: Arity::exact(1),
        detail: "Return whether the image is in use by any widgets.",
        synopsis: "image inuse name",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "Return a list of the names of all existing images.",
        synopsis: "image names",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "type",
        arity: Arity::exact(1),
        detail: "Return the type of the image (e.g. photo or bitmap).",
        synopsis: "image type name",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "types",
        arity: Arity::exact(0),
        detail: "Return a list of the valid image types.",
        synopsis: "image types",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "width",
        arity: Arity::exact(1),
        detail: "Return the width of the image in pixels.",
        synopsis: "image width name",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "image option ?arg ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "image",
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate images.",
            synopsis: &[
                "image create type ?name? ?option value ...?",
                "image delete ?name name ...?",
                "image height name",
                "image inuse name",
                "image names",
                "image type name",
                "image types",
                "image width name",
            ],
            snippet: "",
            source: "Tk man page image.n",
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
