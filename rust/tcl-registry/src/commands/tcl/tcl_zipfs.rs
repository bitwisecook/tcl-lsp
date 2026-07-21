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

//! `zipfs` — mount and manipulate ZIP archives on the virtual
//! filesystem (Tcl 9.0+, TIP 430).
//!
//! Shipped as a built-in ensemble in Tcl 9.0; the 8.x profiles have
//! no `zipfs`, so the spec is gated `TCL90_PLUS`.  The public command
//! is the bare `zipfs`; the ensemble also answers to its fully-
//! qualified `::tcl::zipfs` name, so two forms are registered (the
//! same pattern as `tcl::process`).  Subcommand argument counts and
//! synopses follow the 9.0 `zipfs.n` manual page.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "zipfs subcommand ?arg ...?",
}];

fn make_spec(name: &'static str) -> CommandSpec {
    CommandSpec {
        name,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        subcommands: &SUBCOMMANDS,
        return_type: Some(TclType::String),
        forms: FORMS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet::brief(
            "Mount and manipulate ZIP archives on the virtual filesystem.",
            &["zipfs subcommand ?arg ...?"],
            "Tcl man page zipfs.n",
        )),
        ..CommandSpec::DEFAULT
    }
}

static SUBCOMMANDS: [SubCommand; 14] = [
    SubCommand {
        name: "canonical",
        arity: Arity::new(1, 2),
        detail: "Resolve a filename to its canonical zipfs path.",
        synopsis: "zipfs canonical ?mountpoint? filename",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::new(1, 1),
        detail: "Test whether a file exists in a mounted archive.",
        synopsis: "zipfs exists filename",
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "find",
        arity: Arity::new(1, 1),
        detail: "List the paths of every file under a directory.",
        synopsis: "zipfs find directoryName",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::new(1, 1),
        detail: "Return {archive uncompressed compressed offset} for a file.",
        synopsis: "zipfs info file",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "list",
        arity: Arity::new(0, 2),
        detail: "List mounted files, optionally filtered by -glob/-regexp pattern.",
        synopsis: "zipfs list ?(-glob|-regexp)? ?pattern?",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lmkimg",
        arity: Arity::new(2, 4),
        detail: "Build an executable image with an embedded archive from a file list.",
        synopsis: "zipfs lmkimg outfile inlist ?password? ?infile?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lmkzip",
        arity: Arity::new(2, 3),
        detail: "Create a ZIP archive from a list of files.",
        synopsis: "zipfs lmkzip outfile inlist ?password?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mkimg",
        arity: Arity::new(2, 5),
        detail: "Build an executable image with an embedded archive from a directory.",
        synopsis: "zipfs mkimg outfile indir ?strip? ?password? ?infile?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mkkey",
        arity: Arity::new(1, 1),
        detail: "Generate an obfuscated password string for an archive.",
        synopsis: "zipfs mkkey password",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mkzip",
        arity: Arity::new(2, 4),
        detail: "Create a ZIP archive from a directory's contents.",
        synopsis: "zipfs mkzip outfile indir ?strip? ?password?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mount",
        arity: Arity::new(0, 3),
        detail: "Mount a ZIP archive, or (no args) report current mounts.",
        synopsis: "zipfs mount ?zipfile? ?mountpoint? ?password?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mountdata",
        arity: Arity::new(2, 2),
        detail: "Mount ZIP archive content supplied as an in-memory data string.",
        synopsis: "zipfs mountdata data mountpoint",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "root",
        arity: Arity::new(0, 0),
        detail: "Return the zipfs root mount point for the platform.",
        synopsis: "zipfs root",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unmount",
        arity: Arity::new(1, 1),
        detail: "Dismount a previously mounted ZIP archive.",
        synopsis: "zipfs unmount mountpoint",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for the public global form `zipfs`.
pub fn spec() -> CommandSpec {
    make_spec("zipfs")
}

/// Command spec for the fully-qualified ensemble form `::tcl::zipfs`.
pub fn spec_qualified() -> CommandSpec {
    make_spec("::tcl::zipfs")
}
